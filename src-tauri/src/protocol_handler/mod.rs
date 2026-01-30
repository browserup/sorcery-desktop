pub mod git;
mod matcher;
mod parser;

pub use git::{GitHandler, WorkingTreeStatus};
pub use matcher::{PathMatcher, WorkspaceLookupError, WorkspaceMatch};
pub use parser::{GitRef, SrcuriParser, SrcuriRequest};

use crate::dispatcher::EditorDispatcher;
use crate::settings::SettingsManager;
use crate::trust_check;
use crate::workspace_mru::ActiveWorkspaceTracker;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

pub struct ProtocolHandler {
    matcher: PathMatcher,
    settings_manager: Arc<SettingsManager>,
    dispatcher: Arc<EditorDispatcher>,
    workspace_tracker: Arc<ActiveWorkspaceTracker>,
}

impl ProtocolHandler {
    pub fn new(
        settings_manager: Arc<SettingsManager>,
        dispatcher: Arc<EditorDispatcher>,
        workspace_tracker: Arc<ActiveWorkspaceTracker>,
    ) -> Self {
        Self {
            matcher: PathMatcher::new(
                Arc::clone(&settings_manager),
                Arc::clone(&workspace_tracker),
            ),
            settings_manager,
            dispatcher,
            workspace_tracker,
        }
    }

    async fn open_with_size_check(
        &self,
        file_path: &str,
        line: Option<usize>,
        column: Option<usize>,
        editor_hint: Option<String>,
    ) -> Result<HandleResult> {
        let path = Path::new(file_path);

        if path.is_file() {
            if let Ok(metadata) = std::fs::metadata(path) {
                let file_size = metadata.len();
                let max_size = self.settings_manager.get_max_file_size_bytes().await;
                let warning_size = self.settings_manager.get_large_file_warning_bytes().await;

                if file_size > max_size {
                    let size_mb = file_size as f64 / (1024.0 * 1024.0);
                    let max_mb = max_size as f64 / (1024.0 * 1024.0);
                    bail!(
                        "File is too large ({:.1} MB). Maximum allowed size is {:.0} MB. \
                         You can increase this limit in Settings.",
                        size_mb,
                        max_mb
                    );
                }

                if file_size > warning_size {
                    return Ok(HandleResult::ShowLargeFileDialog {
                        file_path: file_path.to_string(),
                        file_size_bytes: file_size,
                        line,
                        column,
                        editor_hint,
                    });
                }
            }
        }

        self.dispatcher
            .open(file_path, line, column, false, editor_hint)
            .await?;

        if let Some(workspace_path) = self.find_workspace_for_path(file_path).await {
            self.workspace_tracker
                .record_workspace_seen(&workspace_path)
                .await;
        }

        Ok(HandleResult::Opened {
            file_path: file_path.to_string(),
        })
    }

    async fn open_with_trust_and_size_check(
        &self,
        file_path: &str,
        line: Option<usize>,
        column: Option<usize>,
        editor_hint: Option<String>,
    ) -> Result<HandleResult> {
        let path = Path::new(file_path);

        if let Some(workspace_path) = self.find_workspace_for_path(file_path).await {
            let is_trusted = self
                .settings_manager
                .is_workspace_trusted(&workspace_path)
                .await;

            if let Some(scan_result) = trust_check::needs_trust_check(&workspace_path, is_trusted) {
                let workspace_name = workspace_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                info!(
                    "Trust check triggered for workspace '{}': {} auto-run tasks found",
                    workspace_name,
                    scan_result.task_labels.len()
                );

                return Ok(HandleResult::ShowTrustDialog {
                    workspace_path,
                    workspace_name,
                    task_labels: scan_result.task_labels,
                    vim_local_rc_files: scan_result.vim_local_rc_files,
                    dangerous_files: scan_result.dangerous_files,
                    dangerous_settings: scan_result.dangerous_settings,
                    scan_error: scan_result.scan_error,
                    pending_file_path: file_path.to_string(),
                    line,
                    column,
                    editor_hint,
                });
            }
        } else if path.is_file() {
            if let Some(parent) = path.parent() {
                let git_root = GitHandler::find_git_root(parent);
                if let Some(workspace_path) = git_root {
                    let is_trusted = self
                        .settings_manager
                        .is_workspace_trusted(&workspace_path)
                        .await;

                    if let Some(scan_result) =
                        trust_check::needs_trust_check(&workspace_path, is_trusted)
                    {
                        let workspace_name = workspace_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        info!(
                            "Trust check triggered for workspace '{}': {} auto-run tasks found",
                            workspace_name,
                            scan_result.task_labels.len()
                        );

                        return Ok(HandleResult::ShowTrustDialog {
                            workspace_path,
                            workspace_name,
                            task_labels: scan_result.task_labels,
                            vim_local_rc_files: scan_result.vim_local_rc_files,
                            dangerous_files: scan_result.dangerous_files,
                            dangerous_settings: scan_result.dangerous_settings,
                            scan_error: scan_result.scan_error,
                            pending_file_path: file_path.to_string(),
                            line,
                            column,
                            editor_hint,
                        });
                    }
                }
            }
        }

        self.open_with_size_check(file_path, line, column, editor_hint)
            .await
    }

    async fn find_workspace_for_path(&self, file_path: &str) -> Option<PathBuf> {
        let path = std::path::Path::new(file_path);
        for ws in self.settings_manager.get_workspaces().await {
            if let Some(ref ws_path) = ws.normalized_path {
                if path.starts_with(ws_path) {
                    return Some(ws_path.clone());
                }
            }
        }
        None
    }

    pub async fn handle_url(&self, url: &str) -> Result<HandleResult> {
        info!("Handling srcuri URL: {}", url);

        let request = SrcuriParser::parse(url).context("Failed to parse srcuri URL")?;

        match request {
            SrcuriRequest::Ping => {
                info!("Ping request received - Desktop is running");
                Ok(HandleResult::Pong)
            }
            SrcuriRequest::Hello { version } => {
                if let Some(ref v) = version {
                    info!("Hello request received from extension version {}", v);
                } else {
                    info!("Hello request received from extension (no version)");
                }
                Ok(HandleResult::HelloAck { version })
            }
            SrcuriRequest::ImplicitWorkspace {
                workspace,
                path,
                line,
                column,
                git_ref,
                remote,
            }
            | SrcuriRequest::ExplicitWorkspace {
                workspace,
                path,
                line,
                column,
                git_ref,
                remote,
            } => {
                if let Some(ref git_ref) = git_ref {
                    self.handle_revision_path(
                        &workspace,
                        &path,
                        git_ref,
                        line,
                        column,
                        remote.as_deref(),
                    )
                    .await
                } else {
                    self.handle_workspace_path(&workspace, &path, line, column, remote.as_deref())
                        .await
                }
            }
            SrcuriRequest::RelativePath {
                path,
                line,
                column,
                workspace_hint,
            } => {
                self.handle_rel_path(&path, line, column, workspace_hint.as_deref())
                    .await
            }
            SrcuriRequest::AnyPath {
                path,
                line,
                column,
                workspace_hint,
            } => {
                self.handle_any_path(&path, line, column, workspace_hint.as_deref())
                    .await
            }
            SrcuriRequest::AbsolutePath {
                full_path,
                line,
                column,
            } => self.handle_absolute_path(&full_path, line, column).await,
            SrcuriRequest::ExternalUrl {
                provider,
                repo_name,
                provider_path,
                path,
                line,
                column,
                git_ref,
                workspace_override,
                fragment,
            } => {
                self.handle_external_url(
                    &provider,
                    &repo_name,
                    &provider_path,
                    &path,
                    line,
                    column,
                    git_ref,
                    workspace_override.as_deref(),
                    fragment.as_deref(),
                )
                .await
            }
        }
    }

    async fn handle_rel_path(
        &self,
        path: &str,
        line: Option<usize>,
        column: Option<usize>,
        workspace_hint: Option<&str>,
    ) -> Result<HandleResult> {
        info!("Handling rel path: {}", path);
        if let Some(hint) = workspace_hint {
            info!("  workspace hint: {}", hint);
        }

        let mut matches = self.matcher.find_partial_matches(path).await?;

        if matches.is_empty() {
            bail!("File '{}' not found in any configured workspace", path);
        }

        if matches.len() == 1 {
            let workspace_match = &matches[0];
            info!(
                "Single match found, opening: {}",
                workspace_match.full_file_path.display()
            );

            let file_path_str = workspace_match.full_file_path.to_string_lossy().to_string();
            return self
                .open_with_trust_and_size_check(&file_path_str, line, column, None)
                .await;
        }

        self.matcher.sort_by_recent_usage(&mut matches).await;

        info!(
            "Multiple matches found ({}), showing chooser",
            matches.len()
        );
        Ok(HandleResult::ShowChooser {
            matches,
            line,
            column,
        })
    }

    async fn handle_any_path(
        &self,
        path: &str,
        line: Option<usize>,
        column: Option<usize>,
        workspace_hint: Option<&str>,
    ) -> Result<HandleResult> {
        info!("Handling any path: {}", path);
        if let Some(hint) = workspace_hint {
            info!("  workspace hint: {}", hint);
        }

        if Self::is_absolute_any_path(path) {
            return self.handle_absolute_path(path, line, column).await;
        }

        if let Some(hint) = workspace_hint {
            return self
                .handle_workspace_path(hint, path, line, column, None)
                .await;
        }

        if let Some((workspace, relative_path)) = self.extract_leading_workspace_path(path).await {
            return self
                .handle_workspace_path(&workspace, &relative_path, line, column, None)
                .await;
        }

        self.handle_rel_path(path, line, column, None).await
    }

    async fn extract_leading_workspace_path(&self, path: &str) -> Option<(String, String)> {
        let normalized = path.replace('\\', "/");
        let mut segments = normalized.split('/').filter(|s| !s.is_empty());
        let first = segments.next()?;
        let remainder = segments.collect::<Vec<_>>().join("/");

        for workspace in self.settings_manager.get_workspaces().await {
            let ws_name = workspace.name.as_deref().or_else(|| {
                Path::new(&workspace.path)
                    .file_name()
                    .and_then(|name| name.to_str())
            });

            if let Some(ws_name) = ws_name {
                if ws_name.eq_ignore_ascii_case(first) {
                    return Some((ws_name.to_string(), remainder));
                }
            }
        }

        None
    }

    fn is_absolute_any_path(path: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        if path.starts_with('/') || path.starts_with('\\') || path.starts_with('~') {
            return true;
        }

        let bytes = path.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
            if bytes.len() == 2 {
                return true;
            }
            return matches!(bytes.get(2), Some(b'/') | Some(b'\\'));
        }

        false
    }

    async fn handle_workspace_path(
        &self,
        workspace: &str,
        path: &str,
        line: Option<usize>,
        column: Option<usize>,
        remote: Option<&str>,
    ) -> Result<HandleResult> {
        info!("Handling workspace path: {}/{}", workspace, path);

        match self.matcher.find_workspace_path(workspace, path).await {
            Ok(full_path) => {
                let file_path_str = full_path.to_string_lossy().to_string();
                self.open_with_trust_and_size_check(&file_path_str, line, column, None)
                    .await
            }
            Err(WorkspaceLookupError::WorkspaceNotFound(_)) if remote.is_some() => {
                let remote_url = remote.expect("guard ensures Some");
                let default_folder = self.settings_manager.get_default_workspaces_folder().await;
                let repo_base = shellexpand::tilde(&default_folder);
                let clone_path = std::path::PathBuf::from(repo_base.as_ref()).join(workspace);

                info!(
                    "Workspace '{}' not found, offering to clone from {}",
                    workspace, remote_url
                );

                Ok(HandleResult::ShowCloneDialog {
                    workspace_name: workspace.to_string(),
                    clone_path: clone_path.to_string_lossy().to_string(),
                    remote_url: remote_url.to_string(),
                    file_path: path.to_string(),
                    line,
                    column,
                    git_ref: None,
                })
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn handle_absolute_path(
        &self,
        full_path: &str,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Result<HandleResult> {
        info!("Handling absolute path: {}", full_path);

        let mut matches = self.matcher.find_full_path_matches(full_path).await?;

        if matches.is_empty() {
            if self.settings_manager.allows_non_workspace_files().await {
                info!("No workspace matches, attempting to open as absolute path");
                return self
                    .open_with_trust_and_size_check(full_path, line, column, None)
                    .await;
            } else {
                bail!(
                    "File '{}' not found in any workspace and non-workspace files are disabled",
                    full_path
                );
            }
        }

        if matches.len() == 1 {
            let workspace_match = &matches[0];
            info!(
                "Single match found, opening: {}",
                workspace_match.full_file_path.display()
            );

            let file_path_str = workspace_match.full_file_path.to_string_lossy().to_string();
            return self
                .open_with_trust_and_size_check(&file_path_str, line, column, None)
                .await;
        }

        self.matcher.sort_by_recent_usage(&mut matches).await;

        info!(
            "Multiple matches found ({}), showing chooser",
            matches.len()
        );
        Ok(HandleResult::ShowChooser {
            matches,
            line,
            column,
        })
    }

    async fn handle_revision_path(
        &self,
        workspace: &str,
        path: &str,
        git_ref: &GitRef,
        line: Option<usize>,
        column: Option<usize>,
        remote: Option<&str>,
    ) -> Result<HandleResult> {
        let rev = match git_ref {
            GitRef::Commit(s) | GitRef::Branch(s) | GitRef::Tag(s) => s.as_str(),
        };

        info!("Handling revision path: {}/{} @ {}", workspace, path, rev);

        let full_path = match self.matcher.find_workspace_path(workspace, path).await {
            Ok(p) => p,
            Err(WorkspaceLookupError::WorkspaceNotFound(_)) if remote.is_some() => {
                let remote_url = remote.expect("guard ensures Some");
                let default_folder = self.settings_manager.get_default_workspaces_folder().await;
                let repo_base = shellexpand::tilde(&default_folder);
                let clone_path = std::path::PathBuf::from(repo_base.as_ref()).join(workspace);

                info!(
                    "Workspace '{}' not found, offering to clone from {}",
                    workspace, remote_url
                );

                return Ok(HandleResult::ShowCloneDialog {
                    workspace_name: workspace.to_string(),
                    clone_path: clone_path.to_string_lossy().to_string(),
                    remote_url: remote_url.to_string(),
                    file_path: path.to_string(),
                    line,
                    column,
                    git_ref: Some(git_ref.clone()),
                });
            }
            Err(e) => return Err(e.into()),
        };

        let workspace_path = full_path
            .parent()
            .context("Could not determine workspace path")?;

        let git_root = GitHandler::find_git_root(workspace_path).ok_or_else(|| {
            anyhow::anyhow!(
                "Could not find git repository for workspace '{}'",
                workspace
            )
        })?;

        GitHandler::validate_revision(&git_root, rev)?;

        if GitHandler::should_skip_revision_dialog(&git_root, rev)? {
            info!("Already on target revision {}, opening directly", rev);
            let file_path_str = full_path.to_string_lossy().to_string();
            return self
                .open_with_trust_and_size_check(&file_path_str, line, column, None)
                .await;
        }

        let current_ref = GitHandler::get_current_ref(&git_root)?;
        let (checkout_available, checkout_blocked_reason, status) =
            GitHandler::get_revision_dialog_state(&git_root, path, rev)?;

        Ok(HandleResult::ShowRevisionDialog {
            workspace: workspace.to_string(),
            workspace_path: git_root,
            file_path: path.to_string(),
            full_file_path: full_path,
            rev: rev.to_string(),
            line,
            column,
            current_ref,
            is_working_tree_clean: status.is_clean,
            dirty_file_count: status.modified_count,
            checkout_available,
            checkout_blocked_reason,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_external_url(
        &self,
        provider: &str,
        repo_name: &str,
        provider_path: &str,
        path: &str,
        line: Option<usize>,
        column: Option<usize>,
        git_ref: Option<GitRef>,
        workspace_override: Option<&str>,
        fragment: Option<&str>,
    ) -> Result<HandleResult> {
        let workspace_name = workspace_override.unwrap_or(repo_name);
        info!(
            "Handling external URL: {} (workspace: {})",
            provider, workspace_name
        );

        // Try to find a workspace matching the workspace name
        match self.matcher.find_workspace_path(workspace_name, path).await {
            Ok(full_path) => {
                info!(
                    "Found matching workspace '{}', opening locally",
                    workspace_name
                );

                // If git_ref specified, delegate to revision handling
                if let Some(ref git_ref) = git_ref {
                    let remote = format!("https://{}", provider);
                    return self
                        .handle_revision_path(
                            workspace_name,
                            path,
                            git_ref,
                            line,
                            column,
                            Some(&remote),
                        )
                        .await;
                }

                let file_path_str = full_path.to_string_lossy().to_string();
                self.open_with_trust_and_size_check(&file_path_str, line, column, None)
                    .await
            }
            Err(_) => {
                let mut url = String::from("https://srcuri.com/");
                url.push_str(provider_path.trim_start_matches('/'));
                if let Some(frag) = fragment {
                    url.push('#');
                    url.push_str(frag);
                }

                info!("No matching workspace, opening in browser: {}", url);
                Ok(HandleResult::OpenInBrowser { url })
            }
        }
    }
}

#[derive(Debug)]
pub enum HandleResult {
    Opened {
        file_path: String,
    },
    ShowChooser {
        matches: Vec<WorkspaceMatch>,
        line: Option<usize>,
        column: Option<usize>,
    },
    ShowRevisionDialog {
        workspace: String,
        workspace_path: std::path::PathBuf,
        file_path: String,
        full_file_path: std::path::PathBuf,
        rev: String,
        line: Option<usize>,
        column: Option<usize>,
        current_ref: String,
        is_working_tree_clean: bool,
        dirty_file_count: usize,
        checkout_available: bool,
        checkout_blocked_reason: Option<String>,
    },
    ShowCloneDialog {
        workspace_name: String,
        clone_path: String,
        remote_url: String,
        file_path: String,
        line: Option<usize>,
        column: Option<usize>,
        git_ref: Option<GitRef>,
    },
    ShowLargeFileDialog {
        file_path: String,
        file_size_bytes: u64,
        line: Option<usize>,
        column: Option<usize>,
        editor_hint: Option<String>,
    },
    ShowTrustDialog {
        workspace_path: PathBuf,
        workspace_name: String,
        task_labels: Vec<String>,
        vim_local_rc_files: Vec<String>,
        dangerous_files: Vec<trust_check::DangerousFile>,
        dangerous_settings: Vec<trust_check::DangerousSetting>,
        scan_error: Option<String>,
        pending_file_path: String,
        line: Option<usize>,
        column: Option<usize>,
        editor_hint: Option<String>,
    },
    OpenInBrowser {
        url: String,
    },
    /// Extension ping: used to check if Desktop is installed
    Pong,
    /// Extension hello: extension registered itself with Desktop
    HelloAck {
        version: Option<String>,
    },
}
