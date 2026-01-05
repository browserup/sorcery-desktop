pub mod git;
mod matcher;
mod parser;

pub use git::{GitHandler, WorkingTreeStatus};
pub use matcher::{PathMatcher, WorkspaceLookupError, WorkspaceMatch};
pub use parser::{GitRef, SrcuriParser, SrcuriRequest};

use crate::dispatcher::EditorDispatcher;
use crate::settings::SettingsManager;
use crate::workspace_mru::ActiveWorkspaceTracker;
use anyhow::{bail, Context, Result};
use std::sync::Arc;
use tracing::info;

pub struct ProtocolHandler {
    matcher: PathMatcher,
    settings_manager: Arc<SettingsManager>,
    dispatcher: Arc<EditorDispatcher>,
}

impl ProtocolHandler {
    pub fn new(
        settings_manager: Arc<SettingsManager>,
        dispatcher: Arc<EditorDispatcher>,
        workspace_tracker: Arc<ActiveWorkspaceTracker>,
    ) -> Self {
        Self {
            matcher: PathMatcher::new(settings_manager.clone(), workspace_tracker),
            settings_manager,
            dispatcher,
        }
    }

    pub async fn handle_url(&self, url: &str) -> Result<HandleResult> {
        info!("Handling srcuri URL: {}", url);

        let request = SrcuriParser::parse(url).context("Failed to parse srcuri URL")?;

        match request {
            SrcuriRequest::ImplicitWorkspace {
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
            SrcuriRequest::ExplicitWorkspace {
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
            SrcuriRequest::MatchPath {
                path,
                line,
                column,
                workspace_hint,
            } => self.handle_match_path(&path, line, column, workspace_hint.as_deref()).await,
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

    async fn handle_match_path(
        &self,
        path: &str,
        line: Option<usize>,
        column: Option<usize>,
        workspace_hint: Option<&str>,
    ) -> Result<HandleResult> {
        info!("Handling match path: {}", path);
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
            self.dispatcher
                .open(&file_path_str, line, column, false, None)
                .await?;

            return Ok(HandleResult::Opened {
                file_path: file_path_str,
            });
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
                self.dispatcher
                    .open(&file_path_str, line, column, false, None)
                    .await?;
                Ok(HandleResult::Opened {
                    file_path: file_path_str,
                })
            }
            Err(WorkspaceLookupError::WorkspaceNotFound(_)) if remote.is_some() => {
                let remote_url = remote.unwrap();
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
                self.dispatcher
                    .open(full_path, line, column, false, None)
                    .await?;
                return Ok(HandleResult::Opened {
                    file_path: full_path.to_string(),
                });
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
            self.dispatcher
                .open(&file_path_str, line, column, false, None)
                .await?;

            return Ok(HandleResult::Opened {
                file_path: file_path_str,
            });
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
                let remote_url = remote.unwrap();
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
            self.dispatcher
                .open(&file_path_str, line, column, false, None)
                .await?;
            return Ok(HandleResult::Opened {
                file_path: file_path_str,
            });
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
                self.dispatcher
                    .open(&file_path_str, line, column, false, None)
                    .await?;
                Ok(HandleResult::Opened {
                    file_path: file_path_str,
                })
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
    Opened { file_path: String },
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
    OpenInBrowser {
        url: String,
    },
}
