pub mod git;
mod matcher;
mod parser;

pub use git::{GitHandler, WorkingTreeStatus};
pub use matcher::{PathMatcher, WorkspaceLookupError, WorkspaceMatch};
pub use parser::{GitRef, SrcuriParser, SrcuriRequest};

use crate::dispatcher::EditorDispatcher;
use crate::settings::{identity, PolicyDecision, SettingsManager, WorkspaceConfig, WorkspaceState};
use crate::trust_check;
use crate::workspace_mru::ActiveWorkspaceTracker;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

pub struct ProtocolHandler {
    matcher: PathMatcher,
    settings_manager: Arc<SettingsManager>,
    dispatcher: Arc<EditorDispatcher>,
    workspace_tracker: Arc<ActiveWorkspaceTracker>,
}

#[derive(Debug)]
enum WorkspaceResolution {
    Matched(Box<WorkspaceConfig>),
    RemoteMismatch(Vec<WorkspaceConfig>),
    NotConfigured,
}

impl ProtocolHandler {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
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

    async fn scan_workspace_trust(
        workspace_path: &Path,
        is_trusted: bool,
    ) -> Option<trust_check::TrustScanResult> {
        let workspace_path_for_task = workspace_path.to_path_buf();
        let workspace_path_for_log = workspace_path.to_path_buf();

        match tokio::task::spawn_blocking(move || {
            trust_check::needs_trust_check(&workspace_path_for_task, is_trusted)
        })
        .await
        {
            Ok(scan_result) => scan_result,
            Err(error) => {
                let workspace_display = workspace_path_for_log.display();
                warn!("Trust scan task failed for {workspace_display}: {error}");
                Some(trust_check::TrustScanResult {
                    has_auto_tasks: false,
                    task_labels: Vec::new(),
                    vim_local_rc_files: Vec::new(),
                    dangerous_files: Vec::new(),
                    dangerous_settings: Vec::new(),
                    scan_error: Some(format!(
                        "Failed to scan workspace for trust checks: {error}"
                    )),
                })
            }
        }
    }

    async fn trust_dialog_if_needed(
        &self,
        workspace_path: PathBuf,
        pending_file_path: &str,
        line: Option<usize>,
        column: Option<usize>,
        editor_hint: Option<String>,
    ) -> Option<HandleResult> {
        let is_trusted = self
            .settings_manager
            .is_workspace_trusted(&workspace_path)
            .await;
        let scan_result = Self::scan_workspace_trust(&workspace_path, is_trusted).await?;

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

        Some(HandleResult::ShowTrustDialog {
            workspace_path,
            workspace_name,
            task_labels: scan_result.task_labels,
            vim_local_rc_files: scan_result.vim_local_rc_files,
            dangerous_files: scan_result.dangerous_files,
            dangerous_settings: scan_result.dangerous_settings,
            scan_error: scan_result.scan_error,
            pending_file_path: pending_file_path.to_string(),
            line,
            column,
            editor_hint,
        })
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
                    #[allow(clippy::cast_precision_loss)]
                    let size_mb = file_size as f64 / (1024.0 * 1024.0);
                    #[allow(clippy::cast_precision_loss)]
                    let max_mb = max_size as f64 / (1024.0 * 1024.0);
                    bail!(
                        "File is too large ({size_mb:.1} MB). Maximum allowed size is {max_mb:.0} MB. \
                         You can increase this limit in Settings."
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

        if let Some(workspace) = self.settings_manager.get_workspace_for_path(path).await {
            let workspace_key = identity::derive_workspace_key(&workspace);
            if let Some(policy_violation) = self
                .enforced_workspace_policy_violation(&workspace_key, &workspace)
                .await
            {
                bail!("{policy_violation}");
            }

            if let Some(workspace_path) = workspace.normalized_path {
                if let Some(dialog) = self
                    .trust_dialog_if_needed(
                        workspace_path,
                        file_path,
                        line,
                        column,
                        editor_hint.clone(),
                    )
                    .await
                {
                    return Ok(dialog);
                }
            }
        } else if path.is_file() {
            if let Some(parent) = path.parent() {
                let git_root = GitHandler::find_git_root(parent);
                if let Some(workspace_path) = git_root {
                    if let Some(dialog) = self
                        .trust_dialog_if_needed(
                            workspace_path,
                            file_path,
                            line,
                            column,
                            editor_hint.clone(),
                        )
                        .await
                    {
                        return Ok(dialog);
                    }
                }
            }
        }

        self.open_with_size_check(file_path, line, column, editor_hint)
            .await
    }

    async fn find_workspace_for_path(&self, file_path: &str) -> Option<PathBuf> {
        let path = Path::new(file_path);
        self.settings_manager
            .get_workspace_for_path(path)
            .await
            .and_then(|workspace| workspace.normalized_path)
    }

    async fn resolve_workspace(
        &self,
        workspace_key: &str,
        remote: Option<&str>,
    ) -> WorkspaceResolution {
        let (remote_matches, key_matches) = self
            .settings_manager
            .resolve_workspace_by_key(workspace_key, remote)
            .await;

        if let Some(remote) = remote {
            if let Some(matched) = remote_matches
                .into_iter()
                .find(|workspace| workspace.workspace_state == WorkspaceState::Present)
                .or_else(|| key_matches.first().cloned())
            {
                if matched
                    .repo_identity
                    .as_ref()
                    .is_some_and(|identity| identity::remote_matches_identity(remote, identity))
                {
                    return WorkspaceResolution::Matched(Box::new(matched));
                }
            }

            if !key_matches.is_empty() {
                return WorkspaceResolution::RemoteMismatch(key_matches);
            }

            return WorkspaceResolution::NotConfigured;
        }

        if let Some(matched) = key_matches
            .iter()
            .find(|workspace| workspace.workspace_state == WorkspaceState::Present)
            .cloned()
            .or_else(|| key_matches.first().cloned())
        {
            WorkspaceResolution::Matched(Box::new(matched))
        } else {
            WorkspaceResolution::NotConfigured
        }
    }

    fn suggest_clone_path(
        workspace_name: &str,
        default_folder: &str,
        remote: Option<&str>,
        existing_mappings: &[WorkspaceConfig],
    ) -> PathBuf {
        let repo_base = shellexpand::tilde(default_folder);
        let base_path = PathBuf::from(repo_base.as_ref());
        let default_path = base_path.join(workspace_name);

        let normalized_existing_paths: Vec<PathBuf> = existing_mappings
            .iter()
            .filter_map(|workspace| workspace.normalized_path.clone())
            .collect();

        if !normalized_existing_paths.contains(&default_path) {
            return default_path;
        }

        let owner_hint = remote
            .and_then(identity::normalize_remote_identity)
            .and_then(|normalized| {
                let mut pieces = normalized.split('/');
                let _host = pieces.next()?;
                pieces.next().map(ToString::to_string)
            })
            .unwrap_or_else(|| "fork".to_string());

        let owner_candidate = base_path.join(format!("{workspace_name}-{owner_hint}"));
        if !normalized_existing_paths.contains(&owner_candidate) {
            return owner_candidate;
        }

        base_path.join(format!("{workspace_name}-clone"))
    }

    async fn select_most_recent_workspace(
        &self,
        candidates: Vec<WorkspaceConfig>,
    ) -> Option<WorkspaceConfig> {
        let mut best_candidate: Option<WorkspaceConfig> = None;
        let mut best_time = None;

        for candidate in candidates {
            let candidate_time = if let Some(path) = candidate.normalized_path.as_ref() {
                self.workspace_tracker
                    .compute_effective_time(path, false)
                    .await
            } else {
                None
            };

            if best_candidate.is_none() || candidate_time > best_time {
                best_time = candidate_time;
                best_candidate = Some(candidate);
            }
        }

        best_candidate
    }

    async fn select_workspace_for_revision(
        &self,
        mapping: &WorkspaceConfig,
        git_ref: &GitRef,
    ) -> Option<WorkspaceConfig> {
        let repo_identity = mapping.repo_identity.as_ref()?;
        let mut candidates = self
            .settings_manager
            .get_workspaces_in_repo_group(repo_identity)
            .await;

        candidates.retain(|candidate| {
            candidate.workspace_state == WorkspaceState::Present
                && candidate
                    .normalized_path
                    .as_ref()
                    .is_some_and(|path| path.exists())
        });

        if candidates.is_empty() {
            return None;
        }

        let target_ref = match git_ref {
            GitRef::Commit(value) | GitRef::Branch(value) | GitRef::Tag(value) => value.as_str(),
        };

        let mut exact_ref_matches = Vec::new();
        for candidate in &candidates {
            let Some(workspace_path) = candidate.normalized_path.as_ref() else {
                continue;
            };
            if GitHandler::should_skip_revision_dialog(workspace_path, target_ref).unwrap_or(false)
            {
                exact_ref_matches.push(candidate.clone());
            }
        }

        if !exact_ref_matches.is_empty() {
            return self.select_most_recent_workspace(exact_ref_matches).await;
        }

        self.select_most_recent_workspace(candidates).await
    }

    async fn enforced_workspace_policy_violation(
        &self,
        workspace_key: &str,
        mapping: &WorkspaceConfig,
    ) -> Option<String> {
        match self
            .settings_manager
            .evaluate_workspace_policy(mapping)
            .await
        {
            PolicyDecision::Allowed => None,
            PolicyDecision::AdvisoryViolation(violation) => {
                warn!(
                    "Policy advisory for workspace '{}' mapping '{}': {}",
                    workspace_key,
                    identity::derive_workspace_key(mapping),
                    violation
                );
                None
            }
            PolicyDecision::EnforcedViolation(violation) => {
                warn!(
                    "Policy denied workspace '{}' mapping '{}': {}",
                    workspace_key,
                    identity::derive_workspace_key(mapping),
                    violation
                );
                Some(violation.to_string())
            }
        }
    }

    async fn enforced_clone_policy_violation(
        &self,
        workspace_key: &str,
        remote: Option<&str>,
        clone_path: &Path,
    ) -> Option<String> {
        match self
            .settings_manager
            .evaluate_clone_policy(workspace_key, remote, clone_path)
            .await
        {
            PolicyDecision::Allowed => None,
            PolicyDecision::AdvisoryViolation(violation) => {
                warn!("Policy advisory for clone '{workspace_key}': {violation}");
                None
            }
            PolicyDecision::EnforcedViolation(violation) => {
                warn!("Policy denied clone '{workspace_key}': {violation}");
                Some(violation.to_string())
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
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

    #[allow(clippy::missing_errors_doc)]
    pub async fn open_resolved_path(
        &self,
        full_path: &str,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Result<HandleResult> {
        self.open_with_trust_and_size_check(full_path, line, column, None)
            .await
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
            bail!("File '{path}' not found in any configured workspace");
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
            let workspace_key = identity::derive_workspace_key(&workspace);
            if workspace_key.eq_ignore_ascii_case(first) {
                return Some((workspace_key, remainder));
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
            return matches!(bytes.get(2), Some(b'/' | b'\\'));
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

        match self.resolve_workspace(workspace, remote).await {
            WorkspaceResolution::Matched(mapping) => {
                let mapping = *mapping;

                if mapping.workspace_state != WorkspaceState::Present {
                    let workspace_path = mapping.normalized_path.as_ref().map_or_else(
                        || mapping.path.clone(),
                        |path| path.to_string_lossy().to_string(),
                    );
                    return Ok(HandleResult::ShowWorkspaceRepairDialog {
                        workspace_key: workspace.to_string(),
                        workspace_path,
                        workspace_state: mapping.workspace_state,
                        file_path: path.to_string(),
                        line,
                        column,
                    });
                }

                if let Some(policy_violation) = self
                    .enforced_workspace_policy_violation(workspace, &mapping)
                    .await
                {
                    let default_folder =
                        self.settings_manager.get_default_workspaces_folder().await;
                    let clone_path = Self::suggest_clone_path(
                        workspace,
                        &default_folder,
                        remote,
                        std::slice::from_ref(&mapping),
                    );
                    let requested_remote = remote
                        .map(ToString::to_string)
                        .or_else(|| {
                            mapping
                                .repo_identity
                                .as_ref()
                                .and_then(|identity| identity.primary_remote.clone())
                        })
                        .unwrap_or_else(|| "not-provided".to_string());

                    return Ok(HandleResult::ShowWorkspaceConflictDialog {
                        workspace_name: workspace.to_string(),
                        requested_remote,
                        existing_mappings: vec![mapping],
                        clone_path: clone_path.to_string_lossy().to_string(),
                        file_path: path.to_string(),
                        line,
                        column,
                        git_ref: None,
                        policy_violation: Some(policy_violation),
                    });
                }

                match self.matcher.find_workspace_path(workspace, path).await {
                    Ok(full_path) => {
                        let file_path_str = full_path.to_string_lossy().to_string();
                        self.open_with_trust_and_size_check(&file_path_str, line, column, None)
                            .await
                    }
                    Err(e) => Err(e.into()),
                }
            }
            WorkspaceResolution::RemoteMismatch(existing) => {
                let remote_url = remote.expect("resolution guarantees remote");
                let default_folder = self.settings_manager.get_default_workspaces_folder().await;
                let clone_path =
                    Self::suggest_clone_path(workspace, &default_folder, remote, &existing);
                let policy_violation = self
                    .enforced_clone_policy_violation(workspace, Some(remote_url), &clone_path)
                    .await;

                Ok(HandleResult::ShowWorkspaceConflictDialog {
                    workspace_name: workspace.to_string(),
                    requested_remote: remote_url.to_string(),
                    existing_mappings: existing,
                    clone_path: clone_path.to_string_lossy().to_string(),
                    file_path: path.to_string(),
                    line,
                    column,
                    git_ref: None,
                    policy_violation,
                })
            }
            WorkspaceResolution::NotConfigured => {
                if let Some(remote_url) = remote {
                    let default_folder =
                        self.settings_manager.get_default_workspaces_folder().await;
                    let clone_path =
                        Self::suggest_clone_path(workspace, &default_folder, remote, &[]);
                    let policy_violation = self
                        .enforced_clone_policy_violation(workspace, Some(remote_url), &clone_path)
                        .await;

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
                        git_ref: None,
                        policy_violation,
                    });
                }

                match self.matcher.find_workspace_path(workspace, path).await {
                    Ok(full_path) => {
                        let file_path_str = full_path.to_string_lossy().to_string();
                        self.open_with_trust_and_size_check(&file_path_str, line, column, None)
                            .await
                    }
                    Err(e) => Err(e.into()),
                }
            }
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
            }
            bail!(
                "File '{full_path}' not found in any workspace and non-workspace files are disabled"
            );
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

        let matched_mapping = match self.resolve_workspace(workspace, remote).await {
            WorkspaceResolution::Matched(mapping) => {
                let mapping = *mapping;
                if mapping.workspace_state != WorkspaceState::Present {
                    let workspace_path = mapping.normalized_path.as_ref().map_or_else(
                        || mapping.path.clone(),
                        |path| path.to_string_lossy().to_string(),
                    );
                    return Ok(HandleResult::ShowWorkspaceRepairDialog {
                        workspace_key: workspace.to_string(),
                        workspace_path,
                        workspace_state: mapping.workspace_state,
                        file_path: path.to_string(),
                        line,
                        column,
                    });
                }
                mapping
            }
            WorkspaceResolution::RemoteMismatch(existing) => {
                let remote_url = remote.expect("resolution guarantees remote");
                let default_folder = self.settings_manager.get_default_workspaces_folder().await;
                let clone_path =
                    Self::suggest_clone_path(workspace, &default_folder, remote, &existing);
                let policy_violation = self
                    .enforced_clone_policy_violation(workspace, Some(remote_url), &clone_path)
                    .await;

                return Ok(HandleResult::ShowWorkspaceConflictDialog {
                    workspace_name: workspace.to_string(),
                    requested_remote: remote_url.to_string(),
                    existing_mappings: existing,
                    clone_path: clone_path.to_string_lossy().to_string(),
                    file_path: path.to_string(),
                    line,
                    column,
                    git_ref: Some(git_ref.clone()),
                    policy_violation,
                });
            }
            WorkspaceResolution::NotConfigured => {
                if let Some(remote_url) = remote {
                    let default_folder =
                        self.settings_manager.get_default_workspaces_folder().await;
                    let clone_path =
                        Self::suggest_clone_path(workspace, &default_folder, remote, &[]);
                    let policy_violation = self
                        .enforced_clone_policy_violation(workspace, Some(remote_url), &clone_path)
                        .await;

                    return Ok(HandleResult::ShowCloneDialog {
                        workspace_name: workspace.to_string(),
                        clone_path: clone_path.to_string_lossy().to_string(),
                        remote_url: remote_url.to_string(),
                        file_path: path.to_string(),
                        line,
                        column,
                        git_ref: Some(git_ref.clone()),
                        policy_violation,
                    });
                }

                return Err(WorkspaceLookupError::WorkspaceNotFound(workspace.to_string()).into());
            }
        };

        let selected_mapping = self
            .select_workspace_for_revision(&matched_mapping, git_ref)
            .await
            .unwrap_or(matched_mapping);

        if let Some(policy_violation) = self
            .enforced_workspace_policy_violation(workspace, &selected_mapping)
            .await
        {
            let requested_remote = remote
                .map(ToString::to_string)
                .or_else(|| {
                    selected_mapping
                        .repo_identity
                        .as_ref()
                        .and_then(|identity| identity.primary_remote.clone())
                })
                .unwrap_or_else(|| "not-provided".to_string());
            let default_folder = self.settings_manager.get_default_workspaces_folder().await;
            let clone_path = Self::suggest_clone_path(
                workspace,
                &default_folder,
                remote,
                std::slice::from_ref(&selected_mapping),
            );

            return Ok(HandleResult::ShowWorkspaceConflictDialog {
                workspace_name: workspace.to_string(),
                requested_remote,
                existing_mappings: vec![selected_mapping],
                clone_path: clone_path.to_string_lossy().to_string(),
                file_path: path.to_string(),
                line,
                column,
                git_ref: Some(git_ref.clone()),
                policy_violation: Some(policy_violation),
            });
        }

        let workspace_root = selected_mapping
            .normalized_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(&selected_mapping.path));
        let full_path = workspace_root.join(path);

        let selected_workspace_key = identity::derive_workspace_key(&selected_mapping);
        if !selected_workspace_key.eq_ignore_ascii_case(workspace) {
            info!(
                "Revision resolution switched workspace from '{}' to '{}'",
                workspace, selected_workspace_key
            );
        }

        let git_root = GitHandler::find_git_root(&workspace_root).ok_or_else(|| {
            anyhow::anyhow!("Could not find git repository for workspace '{workspace}'")
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
        if let Ok(full_path) = self.matcher.find_workspace_path(workspace_name, path).await {
            info!("Found matching workspace '{workspace_name}', opening locally");

            // If git_ref specified, delegate to revision handling
            if let Some(ref git_ref) = git_ref {
                let remote = format!("https://{provider}");
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
        } else {
            let mut url = String::from("https://srcuri.com/");
            url.push_str(provider_path.trim_start_matches('/'));
            if let Some(frag) = fragment {
                url.push('#');
                url.push_str(frag);
            }

            info!("No matching workspace, opening in browser: {url}");
            Ok(HandleResult::OpenInBrowser { url })
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
        policy_violation: Option<String>,
    },
    ShowWorkspaceRepairDialog {
        workspace_key: String,
        workspace_path: String,
        workspace_state: WorkspaceState,
        file_path: String,
        line: Option<usize>,
        column: Option<usize>,
    },
    ShowWorkspaceConflictDialog {
        workspace_name: String,
        requested_remote: String,
        existing_mappings: Vec<WorkspaceConfig>,
        clone_path: String,
        file_path: String,
        line: Option<usize>,
        column: Option<usize>,
        git_ref: Option<GitRef>,
        policy_violation: Option<String>,
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
