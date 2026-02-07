use super::models::{RepoIdentity, Settings, WorkspaceConfig};
use super::policy::{PolicyDecision, WorkspacePolicy};
use super::{identity, WorkspaceState};
use crate::config_paths;
use anyhow::{Context, Result};
use serde_yaml_ng as serde_yaml;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct SettingsManager {
    config_path: PathBuf,
    policy_path: PathBuf,
    settings: Arc<RwLock<Settings>>,
    policy: Arc<RwLock<Option<WorkspacePolicy>>>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SettingsValidationError {
    #[error("Workspace mapped at '{workspace_path}' must have a non-empty workspace key")]
    EmptyWorkspaceKey { workspace_path: String },
    #[error(
        "Workspace key '{workspace_key}' must be unique (paths: '{first_path}' and '{second_path}')"
    )]
    DuplicateWorkspaceKey {
        workspace_key: String,
        first_path: String,
        second_path: String,
    },
    #[error(
        "Workspace path '{workspace_path}' is already mapped (keys: '{first_workspace_key}' and '{second_workspace_key}')"
    )]
    DuplicateWorkspacePath {
        workspace_path: String,
        first_workspace_key: String,
        second_workspace_key: String,
    },
    #[error("Workspace '{workspace_key}' violates policy: {reason}")]
    PolicyViolation {
        workspace_key: String,
        reason: String,
    },
}

#[allow(clippy::missing_errors_doc, clippy::significant_drop_tightening)]
impl SettingsManager {
    pub async fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        let policy_path = Self::policy_path_for_config(&config_path);

        // Only scan for repo directories on first run (no settings file).
        // When settings exist, load() will replace this with file contents anyway.
        let initial = if tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
            Settings::default()
        } else {
            Settings::with_detected_workspaces_folder()
        };

        let manager = Self {
            config_path,
            policy_path,
            settings: Arc::new(RwLock::new(initial)),
            policy: Arc::new(RwLock::new(None)),
        };

        manager.reload_policy().await?;
        Ok(manager)
    }

    /// Create a `SettingsManager` with a custom config path.
    /// Use this in integration tests to avoid polluting the user's real settings.
    #[allow(dead_code, clippy::unused_async)] // Used by integration tests, not main binary
    pub async fn new_with_path(config_path: PathBuf) -> Result<Self> {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory for tests")?;
        }

        let policy_path = Self::policy_path_for_config(&config_path);

        let manager = Self {
            config_path,
            policy_path,
            settings: Arc::new(RwLock::new(Settings::default())),
            policy: Arc::new(RwLock::new(None)),
        };

        manager.reload_policy().await?;
        Ok(manager)
    }

    fn get_config_path() -> Result<PathBuf> {
        Ok(config_paths::canonical_config_dir()?.join("settings.yaml"))
    }

    fn policy_path_for_config(config_path: &Path) -> PathBuf {
        config_path.parent().map_or_else(
            || PathBuf::from("policy.yaml"),
            |parent| parent.join("policy.yaml"),
        )
    }

    #[must_use]
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub async fn load(&self) -> Result<()> {
        self.reload_policy().await?;

        if !tokio::fs::try_exists(&self.config_path)
            .await
            .unwrap_or(false)
        {
            info!("No existing settings file found, using defaults");
            return Ok(());
        }

        let contents = tokio::fs::read_to_string(&self.config_path)
            .await
            .context("Failed to read settings file")?;

        let mut settings: Settings =
            serde_yaml::from_str(&contents).context("Failed to parse YAML settings")?;

        Self::normalize_workspace_paths(&mut settings);
        Self::validate_workspace_uniqueness(&settings).map_err(anyhow::Error::from)?;
        self.validate_workspace_policy(&settings).await?;

        let mut current = self.settings.write().await;
        *current = settings;

        info!("Settings loaded from {:?}", self.config_path);
        Ok(())
    }

    pub async fn save(&self, mut settings: Settings) -> Result<()> {
        // Normalize paths before persisting or caching in memory
        Self::normalize_workspace_paths(&mut settings);
        Self::validate_workspace_uniqueness(&settings).map_err(anyhow::Error::from)?;
        self.validate_workspace_policy(&settings).await?;

        self.persist_to_disk(&settings).await?;

        let mut current = self.settings.write().await;
        *current = settings;

        info!("Settings saved to {:?}", self.config_path);
        Ok(())
    }

    pub async fn modify<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Settings) -> Result<(R, bool)>,
    {
        let mut settings = self.settings.write().await;
        let (result, dirty) = f(&mut settings)?;

        if dirty {
            Self::normalize_workspace_paths(&mut settings);
            Self::validate_workspace_uniqueness(&settings).map_err(anyhow::Error::from)?;
            self.validate_workspace_policy(&settings).await?;
            self.persist_to_disk(&settings).await?;
            info!("Settings saved to {:?}", self.config_path);
        }

        Ok(result)
    }

    pub async fn get(&self) -> Settings {
        self.settings.read().await.clone()
    }

    pub async fn reload_policy(&self) -> Result<()> {
        let loaded = Self::load_policy_file(&self.policy_path).await?;
        let mut policy = self.policy.write().await;
        *policy = loaded;
        Ok(())
    }

    pub async fn evaluate_workspace_policy(&self, workspace: &WorkspaceConfig) -> PolicyDecision {
        let policy = self.policy.read().await;
        policy.as_ref().map_or(PolicyDecision::Allowed, |policy| {
            policy.evaluate_workspace(workspace)
        })
    }

    pub async fn evaluate_clone_policy(
        &self,
        workspace_key: &str,
        remote: Option<&str>,
        target_path: &Path,
    ) -> PolicyDecision {
        let policy = self.policy.read().await;
        policy.as_ref().map_or(PolicyDecision::Allowed, |policy| {
            policy.evaluate_clone_request(workspace_key, remote, target_path)
        })
    }

    pub async fn get_workspace_for_path(&self, path: &Path) -> Option<WorkspaceConfig> {
        let lookup_path = Self::normalize_lookup_path(path);
        let settings = self.settings.read().await;

        for workspace in &settings.workspaces {
            if workspace.workspace_state == WorkspaceState::Present {
                if let Some(normalized) = &workspace.normalized_path {
                    if lookup_path.starts_with(normalized) {
                        return Some(workspace.clone());
                    }
                }
            }
        }

        None
    }

    pub async fn resolve_workspace_by_key(
        &self,
        key: &str,
        remote: Option<&str>,
    ) -> (Vec<WorkspaceConfig>, Vec<WorkspaceConfig>) {
        let mut matched = Vec::new();
        let mut key_only_matches = Vec::new();
        let lookup_key = identity::canonical_workspace_key_for_lookup(key);
        let settings = self.settings.read().await;

        for workspace in &settings.workspaces {
            let workspace_key = identity::derive_workspace_key(workspace);
            if identity::canonical_workspace_key_for_lookup(&workspace_key) != lookup_key {
                continue;
            }

            key_only_matches.push(workspace.clone());

            if let Some(remote) = remote {
                if workspace
                    .repo_identity
                    .as_ref()
                    .is_some_and(|repo| identity::remote_matches_identity(remote, repo))
                {
                    matched.push(workspace.clone());
                }
            } else {
                matched.push(workspace.clone());
            }
        }

        (matched, key_only_matches)
    }

    pub async fn get_workspaces_in_repo_group(
        &self,
        repo_identity: &RepoIdentity,
    ) -> Vec<WorkspaceConfig> {
        let Some(group_key) = identity::repo_group_key(repo_identity) else {
            return Vec::new();
        };
        let settings = self.settings.read().await;

        settings
            .workspaces
            .iter()
            .filter(|workspace| {
                workspace
                    .repo_identity
                    .as_ref()
                    .and_then(identity::repo_group_key)
                    .is_some_and(|candidate_key| candidate_key == group_key)
            })
            .cloned()
            .collect()
    }

    pub async fn reconcile_workspace_states(&self) -> Result<bool> {
        let mut settings = self.settings.write().await;
        let before = settings.workspaces.clone();
        Self::normalize_workspace_paths(&mut settings);
        let changed = before != settings.workspaces;

        if changed {
            self.persist_to_disk(&settings).await?;
            info!("Settings saved to {:?}", self.config_path);
        }

        Ok(changed)
    }

    pub async fn get_workspace_health_counts(&self) -> HashMap<String, usize> {
        let settings = self.settings.read().await;
        let mut counts: HashMap<String, usize> = HashMap::new();

        for workspace in &settings.workspaces {
            let key = match workspace.workspace_state {
                WorkspaceState::Present => "healthy",
                WorkspaceState::Missing => "missing",
                WorkspaceState::Unavailable => "unavailable",
                WorkspaceState::IdentityDrift => "drifted",
                WorkspaceState::Conflict => "conflict",
            };

            *counts.entry(key.to_string()).or_insert(0) += 1;
        }

        counts
    }

    pub async fn is_workspace_trusted(&self, workspace_path: &Path) -> bool {
        let lookup_path = Self::normalize_lookup_path(workspace_path);
        let settings = self.settings.read().await;
        for workspace in &settings.workspaces {
            if workspace.workspace_state != WorkspaceState::Present {
                continue;
            }
            if let Some(ref normalized) = workspace.normalized_path {
                if lookup_path.starts_with(normalized) {
                    return workspace.trusted;
                }
            }
        }
        false
    }

    pub async fn get_default_editor(&self) -> String {
        let settings = self.settings.read().await;
        settings.defaults.editor.clone()
    }

    pub async fn allows_non_workspace_files(&self) -> bool {
        let settings = self.settings.read().await;
        settings.defaults.allow_non_workspace_files
    }

    pub async fn get_preferred_terminal(&self) -> String {
        let settings = self.settings.read().await;
        settings.defaults.preferred_terminal.clone()
    }

    pub async fn get_workspaces(&self) -> Vec<WorkspaceConfig> {
        self.settings.read().await.workspaces.clone()
    }

    pub async fn get_default_workspaces_folder(&self) -> String {
        self.settings
            .read()
            .await
            .defaults
            .default_workspaces_folder
            .clone()
    }

    pub async fn get_large_file_warning_bytes(&self) -> u64 {
        self.settings.read().await.defaults.large_file_warning_mb * 1024 * 1024
    }

    pub async fn get_max_file_size_bytes(&self) -> u64 {
        self.settings.read().await.defaults.max_file_size_mb * 1024 * 1024
    }

    pub async fn trust_workspace(&self, workspace_path: &Path) -> Result<()> {
        let lookup_path = Self::normalize_lookup_path(workspace_path);
        self.modify(|settings| {
            for workspace in &mut settings.workspaces {
                if workspace.workspace_state != WorkspaceState::Present {
                    continue;
                }
                if let Some(ref normalized) = workspace.normalized_path {
                    if lookup_path.starts_with(normalized) {
                        workspace.trusted = true;
                        return Ok(((), true));
                    }
                }
            }
            Ok(((), false))
        })
        .await
    }

    async fn load_policy_file(policy_path: &Path) -> Result<Option<WorkspacePolicy>> {
        if !tokio::fs::try_exists(policy_path).await.unwrap_or(false) {
            return Ok(None);
        }

        let contents = tokio::fs::read_to_string(policy_path)
            .await
            .with_context(|| format!("Failed to read policy file '{}'", policy_path.display()))?;

        let config: super::WorkspacePolicyConfig = serde_yaml::from_str(&contents)
            .with_context(|| format!("Failed to parse policy YAML '{}'", policy_path.display()))?;

        let policy = WorkspacePolicy::from_config(config)
            .with_context(|| format!("Invalid policy config '{}'", policy_path.display()))?;

        info!("Workspace policy loaded from '{}'", policy_path.display());
        Ok(Some(policy))
    }

    async fn validate_workspace_policy(&self, settings: &Settings) -> Result<()> {
        let policy = self.policy.read().await;
        let Some(policy) = policy.as_ref() else {
            return Ok(());
        };

        for workspace in &settings.workspaces {
            let decision = policy.evaluate_workspace(workspace);
            match decision {
                PolicyDecision::Allowed => {}
                PolicyDecision::AdvisoryViolation(violation) => {
                    let workspace_key = identity::derive_workspace_key(workspace);
                    warn!(
                        "Policy advisory for workspace '{}': {}",
                        workspace_key, violation
                    );
                }
                PolicyDecision::EnforcedViolation(violation) => {
                    let workspace_key = identity::derive_workspace_key(workspace);
                    return Err(SettingsValidationError::PolicyViolation {
                        workspace_key,
                        reason: violation.to_string(),
                    }
                    .into());
                }
            }
        }

        Ok(())
    }

    fn normalize_workspace_paths(settings: &mut Settings) {
        for workspace in &mut settings.workspaces {
            let previous_key = workspace.workspace_key.clone();
            let previous_kind = workspace.workspace_kind;
            let previous_state = workspace.workspace_state;
            let previous_repo_identity = workspace.repo_identity.clone();
            let previous_normalized_path = workspace.normalized_path.clone();

            let workspace_key = identity::derive_workspace_key(workspace);
            workspace.workspace_key = workspace_key.clone();

            match Self::normalize_path(&workspace.path) {
                Ok(normalized) => {
                    let inspection = identity::inspect_workspace(&normalized);
                    workspace.normalized_path = Some(normalized);
                    workspace.workspace_state = inspection.workspace_state;

                    if matches!(
                        workspace.workspace_state,
                        WorkspaceState::Missing | WorkspaceState::Unavailable
                    ) {
                        // Preserve last known git classification and identity while unavailable so
                        // we can detect identity drift if the path later reappears.
                        workspace.workspace_kind = if previous_repo_identity.is_some() {
                            super::WorkspaceKind::Git
                        } else {
                            inspection.workspace_kind
                        };
                        workspace.repo_identity =
                            previous_repo_identity.clone().or(inspection.repo_identity);
                    } else {
                        workspace.workspace_kind = inspection.workspace_kind;
                        workspace.repo_identity = inspection.repo_identity;
                    }
                }
                Err(e) => {
                    let path = &workspace.path;
                    warn!("Failed to normalize path '{path}': {e}");
                    workspace.normalized_path = None;
                    workspace.workspace_kind = if previous_repo_identity.is_some() {
                        super::WorkspaceKind::Git
                    } else {
                        super::WorkspaceKind::NonGit
                    };
                    workspace.workspace_state = WorkspaceState::Unavailable;
                    workspace.repo_identity = previous_repo_identity.clone();
                }
            }

            if workspace.workspace_state == WorkspaceState::Present {
                let kind_drifted_from_git = matches!(previous_kind, super::WorkspaceKind::Git)
                    && !matches!(workspace.workspace_kind, super::WorkspaceKind::Git);
                let primary_remote_changed = previous_repo_identity
                    .as_ref()
                    .and_then(|identity| identity.primary_remote.as_ref())
                    != workspace
                        .repo_identity
                        .as_ref()
                        .and_then(|identity| identity.primary_remote.as_ref());
                let had_previous_identity = previous_repo_identity.is_some();

                if kind_drifted_from_git || (had_previous_identity && primary_remote_changed) {
                    workspace.workspace_state = WorkspaceState::IdentityDrift;
                    workspace.trusted = false;
                }
            }

            let changed = workspace.workspace_key != previous_key
                || workspace.workspace_kind != previous_kind
                || workspace.workspace_state != previous_state
                || workspace.repo_identity != previous_repo_identity
                || workspace.normalized_path != previous_normalized_path;

            if changed || workspace.last_verified_at.is_none() {
                workspace.last_verified_at = Some(identity::now_timestamp_millis());
            }
        }

        Self::mark_workspace_conflicts(settings);

        // Validate workspace keys
        Self::validate_workspace_keys(settings);
    }

    /// Validate workspace keys and warn about those containing dots.
    /// Workspace keys with dots are ambiguous with provider hostnames (e.g., github.com).
    /// Use ?workspace= escape hatch in URLs to reference dot-containing workspace keys.
    fn validate_workspace_keys(settings: &Settings) {
        for workspace in &settings.workspaces {
            let key = identity::derive_workspace_key(workspace);

            if key.contains('.') {
                warn!(
                    "Workspace '{}' contains a dot in its key. \
                     This may be confused with provider hostnames (e.g., github.com). \
                     Consider renaming, or use ?workspace={} in URLs to reference it explicitly.",
                    key, key
                );
            }
        }
    }

    fn mark_workspace_conflicts(settings: &mut Settings) {
        let mut conflicted_indices: HashSet<usize> = HashSet::new();

        let mut key_to_indices: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, workspace) in settings.workspaces.iter().enumerate() {
            let key = identity::canonical_workspace_key_for_lookup(
                &identity::derive_workspace_key(workspace),
            );
            key_to_indices.entry(key).or_default().push(index);
        }

        for indices in key_to_indices.values() {
            if indices.len() > 1 {
                for index in indices {
                    conflicted_indices.insert(*index);
                }
            }
        }

        let mut path_to_indices: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        for (index, workspace) in settings.workspaces.iter().enumerate() {
            if let Some(path) = workspace.normalized_path.as_ref() {
                path_to_indices.entry(path.clone()).or_default().push(index);
            }
        }

        for indices in path_to_indices.values() {
            if indices.len() > 1 {
                for index in indices {
                    conflicted_indices.insert(*index);
                }
            }
        }

        for (index, workspace) in settings.workspaces.iter_mut().enumerate() {
            if conflicted_indices.contains(&index) {
                workspace.workspace_state = WorkspaceState::Conflict;
            }
        }
    }

    fn normalize_path(path: &str) -> Result<PathBuf> {
        let expanded = shellexpand::tilde(path);
        let path = Path::new(expanded.as_ref());

        let normalized = if path.is_absolute() {
            if path.exists() {
                path.canonicalize().context("Failed to canonicalize path")?
            } else {
                path.to_path_buf()
            }
        } else {
            std::env::current_dir()
                .context("Failed to get current directory")?
                .join(path)
                .canonicalize()
                .context("Failed to canonicalize path")?
        };

        #[cfg(target_os = "macos")]
        {
            let normalized_str = normalized.to_string_lossy();
            if normalized_str.starts_with("/private/") {
                if let Ok(stripped) = normalized.strip_prefix("/private") {
                    let mut absolute = PathBuf::from("/");
                    absolute.push(stripped);
                    return Ok(absolute);
                }
            }
        }

        Ok(normalized)
    }

    fn normalize_lookup_path(path: &Path) -> PathBuf {
        let normalized = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        #[cfg(target_os = "macos")]
        {
            let normalized_str = normalized.to_string_lossy();
            if normalized_str.starts_with("/private/") {
                if let Ok(stripped) = normalized.strip_prefix("/private") {
                    let mut absolute = PathBuf::from("/");
                    absolute.push(stripped);
                    return absolute;
                }
            }
        }

        normalized
    }

    fn validate_workspace_uniqueness(
        settings: &Settings,
    ) -> std::result::Result<(), SettingsValidationError> {
        let mut workspace_keys: HashMap<String, String> = HashMap::new();

        for workspace in &settings.workspaces {
            let workspace_key = identity::derive_workspace_key(workspace);
            if workspace_key.is_empty() {
                return Err(SettingsValidationError::EmptyWorkspaceKey {
                    workspace_path: workspace.path.clone(),
                });
            }
            let canonical_key = identity::canonical_workspace_key_for_lookup(&workspace_key);
            let path = workspace.path.clone();

            if let Some(existing_path) = workspace_keys.insert(canonical_key.clone(), path.clone())
            {
                return Err(SettingsValidationError::DuplicateWorkspaceKey {
                    workspace_key: canonical_key,
                    first_path: existing_path,
                    second_path: path,
                });
            }
        }

        let mut normalized_paths: HashMap<PathBuf, String> = HashMap::new();

        for workspace in &settings.workspaces {
            let Some(path) = workspace.normalized_path.as_ref() else {
                continue;
            };
            if !path.exists() {
                continue;
            }

            let workspace_key = identity::derive_workspace_key(workspace);
            if let Some(existing_workspace_key) =
                normalized_paths.insert(path.clone(), workspace_key.clone())
            {
                return Err(SettingsValidationError::DuplicateWorkspacePath {
                    workspace_path: path.to_string_lossy().to_string(),
                    first_workspace_key: existing_workspace_key,
                    second_workspace_key: workspace_key,
                });
            }
        }

        Ok(())
    }

    async fn persist_to_disk(&self, settings: &Settings) -> Result<()> {
        let yaml_string =
            serde_yaml::to_string(settings).context("Failed to serialize settings to YAML")?;

        // Atomic write: write to temp file, then rename
        let temp_path = self.config_path.with_extension("yaml.tmp");
        tokio::fs::write(&temp_path, &yaml_string)
            .await
            .context("Failed to write temporary settings file")?;

        tokio::fs::rename(&temp_path, &self.config_path)
            .await
            .context("Failed to rename temporary settings file")?;

        Ok(())
    }

    pub async fn is_setup_needed(&self) -> bool {
        let settings = self.settings.read().await;
        !settings.defaults.setup_completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::WorkspaceKind;
    use std::process::Command;
    use tempfile::TempDir;

    fn workspace_config(path: String) -> WorkspaceConfig {
        let workspace_key = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .map_or_else(|| "workspace".to_string(), ToString::to_string);
        WorkspaceConfig {
            path,
            workspace_key,
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        }
    }

    #[tokio::test]
    async fn load_rejects_empty_workspace_keys() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let invalid_workspace_path = temp_dir.path().join("repo");

        let yaml = format!(
            r#"
defaults:
  editor: "vscode"
workspaces:
  - path: "{}"
    workspace_key: ""
"#,
            invalid_workspace_path.to_string_lossy()
        );
        std::fs::write(&settings_path, yaml).expect("write settings yaml");

        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let error = manager
            .load()
            .await
            .expect_err("empty workspace key must be rejected on load");
        let validation_error = error
            .downcast_ref::<SettingsValidationError>()
            .expect("validation error");
        assert!(matches!(
            validation_error,
            SettingsValidationError::EmptyWorkspaceKey { workspace_path }
                if workspace_path == &invalid_workspace_path.to_string_lossy().to_string()
        ));
    }

    #[tokio::test]
    async fn load_accepts_legacy_name_field_as_workspace_key() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let workspace_path = temp_dir.path().join("repo");
        std::fs::create_dir_all(&workspace_path).expect("workspace path");

        let yaml = format!(
            r#"
defaults:
  editor: "vscode"
workspaces:
  - path: "{}"
    name: "repo"
"#,
            workspace_path.to_string_lossy()
        );
        std::fs::write(&settings_path, yaml).expect("write settings yaml");

        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");
        manager.load().await.expect("legacy name field should load");

        let settings = manager.get().await;
        assert_eq!(settings.workspaces.len(), 1);
        assert_eq!(settings.workspaces[0].workspace_key, "repo");
    }

    #[tokio::test]
    async fn save_normalizes_existing_absolute_workspace_path() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_dir = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_dir).expect("create workspace dir");
        let noncanonical = workspace_dir.join("..").join("workspace");
        let noncanonical_str = noncanonical.to_string_lossy().to_string();

        let mut settings = Settings::default();
        settings
            .workspaces
            .push(workspace_config(noncanonical_str.clone()));
        manager.save(settings).await.expect("save settings");

        let initial = manager.get().await;
        let initial_remote = initial.workspaces[0]
            .repo_identity
            .as_ref()
            .and_then(|identity| identity.primary_remote.clone());
        assert_eq!(initial_remote, None);

        let saved = manager.get().await;
        let normalized = saved.workspaces[0]
            .normalized_path
            .as_ref()
            .expect("normalized path");
        let expected =
            SettingsManager::normalize_path(&noncanonical_str).expect("normalize expected path");

        assert_eq!(normalized, &expected);
    }

    #[tokio::test]
    async fn save_preserves_nonexistent_absolute_workspace_path() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let missing_path = temp_dir.path().join("does-not-exist");
        let missing_str = missing_path.to_string_lossy().to_string();

        let mut settings = Settings::default();
        settings.workspaces.push(workspace_config(missing_str));
        manager.save(settings).await.expect("save settings");

        let saved = manager.get().await;
        let normalized = saved.workspaces[0]
            .normalized_path
            .as_ref()
            .expect("normalized path");

        assert_eq!(normalized, &missing_path);
    }

    #[tokio::test]
    async fn duplicate_workspace_keys_are_rejected_on_save() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_a = temp_dir.path().join("workspace-a");
        let workspace_b = temp_dir.path().join("workspace-b");
        std::fs::create_dir_all(&workspace_a).expect("create workspace a");
        std::fs::create_dir_all(&workspace_b).expect("create workspace b");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_a.to_string_lossy().to_string(),
            workspace_key: "rails".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_b.to_string_lossy().to_string(),
            workspace_key: "rails".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });

        let error = manager
            .save(settings)
            .await
            .expect_err("duplicate workspace keys must be rejected");
        let validation_error = error
            .downcast_ref::<SettingsValidationError>()
            .expect("validation error");
        assert!(matches!(
            validation_error,
            SettingsValidationError::DuplicateWorkspaceKey { workspace_key, .. }
                if workspace_key == "rails"
        ));
    }

    #[tokio::test]
    async fn duplicate_present_workspace_paths_are_rejected_on_save() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_path = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_path).expect("create workspace");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_path.to_string_lossy().to_string(),
            workspace_key: "workspace-a".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_path.to_string_lossy().to_string(),
            workspace_key: "workspace-b".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });

        let error = manager
            .save(settings)
            .await
            .expect_err("duplicate workspace paths must be rejected");
        let validation_error = error
            .downcast_ref::<SettingsValidationError>()
            .expect("validation error");
        assert!(matches!(
            validation_error,
            SettingsValidationError::DuplicateWorkspacePath {
                first_workspace_key,
                second_workspace_key,
                ..
            } if first_workspace_key == "workspace-a" && second_workspace_key == "workspace-b"
        ));
    }

    #[tokio::test]
    async fn empty_workspace_keys_are_rejected_on_save() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let first_workspace = temp_dir.path().join("apps").join("myapp");
        let second_workspace = temp_dir.path().join("other").join("myapp");
        std::fs::create_dir_all(&first_workspace).expect("create first workspace");
        std::fs::create_dir_all(&second_workspace).expect("create second workspace");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: first_workspace.to_string_lossy().to_string(),
            workspace_key: String::new(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        settings.workspaces.push(WorkspaceConfig {
            path: second_workspace.to_string_lossy().to_string(),
            workspace_key: String::new(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });

        let error = manager
            .save(settings)
            .await
            .expect_err("empty workspace key must be rejected");
        let validation_error = error
            .downcast_ref::<SettingsValidationError>()
            .expect("validation error");
        assert!(matches!(
            validation_error,
            SettingsValidationError::EmptyWorkspaceKey { workspace_path }
                if workspace_path == &first_workspace.to_string_lossy().to_string()
        ));
    }

    #[tokio::test]
    async fn workspace_key_is_trimmed_on_save() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_path = temp_dir.path().join("nexty");
        std::fs::create_dir_all(&workspace_path).expect("create workspace");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_path.to_string_lossy().to_string(),
            workspace_key: "  nexty  ".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        manager.save(settings).await.expect("save settings");

        let saved = manager.get().await;
        assert_eq!(saved.workspaces.len(), 1);
        assert_eq!(saved.workspaces[0].workspace_key, "nexty");
    }

    #[tokio::test]
    async fn non_git_workspace_is_first_class_mapping() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_path = temp_dir.path().join("apps").join("perforce-project");
        std::fs::create_dir_all(&workspace_path).expect("create workspace");
        std::fs::write(workspace_path.join(".p4config"), "P4CLIENT=myclient")
            .expect("write perforce marker");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_path.to_string_lossy().to_string(),
            workspace_key: "perforce-project".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        manager.save(settings).await.expect("save settings");

        let saved = manager.get().await;
        assert_eq!(saved.workspaces.len(), 1);
        assert_eq!(saved.workspaces[0].workspace_kind, WorkspaceKind::NonGit);
        assert_eq!(saved.workspaces[0].workspace_state, WorkspaceState::Present);
        assert!(saved.workspaces[0].repo_identity.is_none());
    }

    #[tokio::test]
    async fn duplicate_key_is_rejected_when_existing_mapping_is_missing() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let missing_workspace = temp_dir.path().join("old").join("myapp");
        let present_workspace = temp_dir.path().join("apps").join("myapp");
        std::fs::create_dir_all(&present_workspace).expect("create present workspace");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: missing_workspace.to_string_lossy().to_string(),
            workspace_key: "myapp".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        settings.workspaces.push(WorkspaceConfig {
            path: present_workspace.to_string_lossy().to_string(),
            workspace_key: "myapp".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });

        let error = manager
            .save(settings)
            .await
            .expect_err("duplicate key must be rejected even if one mapping is missing");
        let validation_error = error
            .downcast_ref::<SettingsValidationError>()
            .expect("validation error");
        assert!(matches!(
            validation_error,
            SettingsValidationError::DuplicateWorkspaceKey { workspace_key, .. }
                if workspace_key == "myapp"
        ));
    }

    #[tokio::test]
    async fn reconcile_marks_identity_drift_when_remote_changes() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_path = temp_dir.path().join("repo");
        std::fs::create_dir_all(&workspace_path).expect("create workspace dir");

        run_git(&workspace_path, &["init"]);
        run_git(
            &workspace_path,
            &["config", "user.email", "test@example.com"],
        );
        run_git(&workspace_path, &["config", "user.name", "Test User"]);
        run_git(
            &workspace_path,
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/org/repo-a.git",
            ],
        );

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_path.to_string_lossy().to_string(),
            workspace_key: "repo".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: true,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        manager.save(settings).await.expect("save settings");

        run_git(
            &workspace_path,
            &[
                "remote",
                "set-url",
                "origin",
                "https://github.com/org/repo-b.git",
            ],
        );

        manager
            .reconcile_workspace_states()
            .await
            .expect("reconcile workspace states");

        let refreshed = manager.get().await;
        assert_eq!(refreshed.workspaces.len(), 1);
        let workspace = &refreshed.workspaces[0];
        let refreshed_remote = workspace
            .repo_identity
            .as_ref()
            .and_then(|identity| identity.primary_remote.clone());
        assert_eq!(refreshed_remote, Some("github.com/org/repo-b".to_string()));
        assert_eq!(workspace.workspace_state, WorkspaceState::IdentityDrift);
        assert!(!workspace.trusted);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn is_workspace_trusted_handles_symlink_alias_paths() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_real_path = temp_dir.path().join("workspace-real");
        std::fs::create_dir_all(&workspace_real_path).expect("create workspace dir");

        let workspace_link_path = temp_dir.path().join("workspace-link");
        std::os::unix::fs::symlink(&workspace_real_path, &workspace_link_path)
            .expect("create symlink");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_real_path.to_string_lossy().to_string(),
            workspace_key: "workspace".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: true,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        manager.save(settings).await.expect("save settings");

        assert!(manager.is_workspace_trusted(&workspace_real_path).await);
        assert!(manager.is_workspace_trusted(&workspace_link_path).await);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn trust_workspace_handles_symlink_alias_paths() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_real_path = temp_dir.path().join("workspace-real");
        std::fs::create_dir_all(&workspace_real_path).expect("create workspace dir");

        let workspace_link_path = temp_dir.path().join("workspace-link");
        std::os::unix::fs::symlink(&workspace_real_path, &workspace_link_path)
            .expect("create symlink");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_real_path.to_string_lossy().to_string(),
            workspace_key: "workspace".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        manager.save(settings).await.expect("save settings");

        manager
            .trust_workspace(&workspace_link_path)
            .await
            .expect("trust workspace");

        let saved = manager.get().await;
        assert!(saved.workspaces[0].trusted);
    }

    #[tokio::test]
    async fn get_workspaces_in_repo_group_returns_matching_worktrees() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let repo_path = temp_dir.path().join("repo");
        let feature_path = temp_dir.path().join("repo-feature");
        std::fs::create_dir_all(&repo_path).expect("create repo dir");

        run_git(&repo_path, &["init"]);
        run_git(&repo_path, &["config", "user.email", "test@example.com"]);
        run_git(&repo_path, &["config", "user.name", "Test User"]);
        std::fs::write(repo_path.join("README.md"), "hello").expect("write readme");
        run_git(&repo_path, &["add", "README.md"]);
        run_git(&repo_path, &["commit", "-m", "init"]);
        run_git(&repo_path, &["branch", "-M", "main"]);
        run_git(&repo_path, &["branch", "feature"]);
        run_git(
            &repo_path,
            &[
                "worktree",
                "add",
                feature_path.to_string_lossy().as_ref(),
                "feature",
            ],
        );

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: repo_path.to_string_lossy().to_string(),
            workspace_key: "repo-main".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::Git,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        settings.workspaces.push(WorkspaceConfig {
            path: feature_path.to_string_lossy().to_string(),
            workspace_key: "repo-feature".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::Git,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        manager.save(settings).await.expect("save settings");

        let saved = manager.get().await;
        let repo_identity = saved
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_key == "repo-main")
            .and_then(|workspace| workspace.repo_identity.clone())
            .expect("repo identity");

        let grouped = manager.get_workspaces_in_repo_group(&repo_identity).await;
        assert_eq!(grouped.len(), 2);
    }

    #[tokio::test]
    async fn save_rejects_enforced_policy_violation() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_path = temp_dir.path().join("rails");
        std::fs::create_dir_all(&workspace_path).expect("create workspace dir");
        run_git(&workspace_path, &["init"]);
        run_git(
            &workspace_path,
            &["config", "user.email", "test@example.com"],
        );
        run_git(&workspace_path, &["config", "user.name", "Test User"]);
        run_git(
            &workspace_path,
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/company/rails.git",
            ],
        );

        let policy_path = temp_dir.path().join("policy.yaml");
        std::fs::write(
            &policy_path,
            r#"
mode: enforced
mappings:
  - workspace_key: rails
    remote: github.com/rails/rails
"#,
        )
        .expect("write policy");
        manager.reload_policy().await.expect("reload policy");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_path.to_string_lossy().to_string(),
            workspace_key: "rails".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::Git,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });

        let error = manager
            .save(settings)
            .await
            .expect_err("enforced policy mismatch should fail");
        let validation = error
            .downcast_ref::<SettingsValidationError>()
            .expect("validation error");

        assert!(matches!(
            validation,
            SettingsValidationError::PolicyViolation { workspace_key, .. }
                if workspace_key == "rails"
        ));
    }

    #[tokio::test]
    async fn advisory_policy_allows_save_with_warning() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_path = temp_dir.path().join("rails");
        std::fs::create_dir_all(&workspace_path).expect("create workspace dir");
        run_git(&workspace_path, &["init"]);
        run_git(
            &workspace_path,
            &["config", "user.email", "test@example.com"],
        );
        run_git(&workspace_path, &["config", "user.name", "Test User"]);
        run_git(
            &workspace_path,
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/company/rails.git",
            ],
        );

        let policy_path = temp_dir.path().join("policy.yaml");
        std::fs::write(
            &policy_path,
            r#"
mode: advisory
mappings:
  - workspace_key: rails
    remote: github.com/rails/rails
"#,
        )
        .expect("write policy");
        manager.reload_policy().await.expect("reload policy");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_path.to_string_lossy().to_string(),
            workspace_key: "rails".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::Git,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });

        manager
            .save(settings)
            .await
            .expect("advisory policy should allow");
    }

    #[tokio::test]
    async fn missing_mappings_persist_for_manual_and_discovered_workspaces() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let manual_workspace = temp_dir.path().join("manual-workspace");
        let discovered_workspace = temp_dir.path().join("apps").join("discovered-workspace");
        std::fs::create_dir_all(&manual_workspace).expect("create manual workspace");
        std::fs::create_dir_all(&discovered_workspace).expect("create discovered workspace");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: manual_workspace.to_string_lossy().to_string(),
            workspace_key: "manual-workspace".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        settings.workspaces.push(WorkspaceConfig {
            path: discovered_workspace.to_string_lossy().to_string(),
            workspace_key: "discovered-workspace".to_string(),
            editor: String::new(),
            auto_discovered: true,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        manager.save(settings).await.expect("save settings");

        std::fs::remove_dir_all(&manual_workspace).expect("delete manual workspace");
        std::fs::remove_dir_all(&discovered_workspace).expect("delete discovered workspace");

        manager
            .reconcile_workspace_states()
            .await
            .expect("reconcile workspace states");

        let saved = manager.get().await;
        assert_eq!(saved.workspaces.len(), 2);

        let manual = saved
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_key == "manual-workspace")
            .expect("manual workspace");
        assert_eq!(manual.workspace_state, WorkspaceState::Missing);
        assert!(!manual.auto_discovered);

        let discovered = saved
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_key == "discovered-workspace")
            .expect("discovered workspace");
        assert_eq!(discovered.workspace_state, WorkspaceState::Missing);
        assert!(discovered.auto_discovered);
    }

    #[tokio::test]
    async fn reconcile_marks_identity_drift_when_repo_is_recreated_with_different_remote() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let workspace_path = temp_dir.path().join("repo");
        initialize_git_workspace_with_remote(&workspace_path, "https://github.com/org/repo-a.git");

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_path.to_string_lossy().to_string(),
            workspace_key: "repo".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: true,
            workspace_kind: WorkspaceKind::Git,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        manager.save(settings).await.expect("save settings");

        std::fs::remove_dir_all(&workspace_path).expect("delete workspace");
        manager
            .reconcile_workspace_states()
            .await
            .expect("reconcile missing workspace");

        let missing = manager.get().await;
        let missing_workspace = &missing.workspaces[0];
        assert_eq!(missing_workspace.workspace_state, WorkspaceState::Missing);
        let missing_remote = missing_workspace
            .repo_identity
            .as_ref()
            .and_then(|identity| identity.primary_remote.as_deref());
        assert_eq!(missing_remote, Some("github.com/org/repo-a"));

        initialize_git_workspace_with_remote(&workspace_path, "https://github.com/org/repo-b.git");
        manager
            .reconcile_workspace_states()
            .await
            .expect("reconcile recreated workspace");

        let recreated = manager.get().await;
        let recreated_workspace = &recreated.workspaces[0];
        assert_eq!(
            recreated_workspace.workspace_state,
            WorkspaceState::IdentityDrift
        );
        assert!(!recreated_workspace.trusted);
        let recreated_remote = recreated_workspace
            .repo_identity
            .as_ref()
            .and_then(|identity| identity.primary_remote.as_deref());
        assert_eq!(recreated_remote, Some("github.com/org/repo-b"));
    }

    #[tokio::test]
    async fn transition_to_worktrees_adds_candidates_without_losing_existing_mapping() {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");

        let repo_path = temp_dir.path().join("repo-main");
        let worktree_path = temp_dir.path().join("repo-feature");
        std::fs::create_dir_all(&repo_path).expect("create repo dir");

        run_git(&repo_path, &["init"]);
        run_git(&repo_path, &["config", "user.email", "test@example.com"]);
        run_git(&repo_path, &["config", "user.name", "Test User"]);
        std::fs::write(repo_path.join("README.md"), "hello").expect("write readme");
        run_git(&repo_path, &["add", "README.md"]);
        run_git(&repo_path, &["commit", "-m", "init"]);
        run_git(&repo_path, &["branch", "-M", "main"]);
        run_git(&repo_path, &["branch", "feature"]);

        let mut settings = Settings::default();
        settings.workspaces.push(WorkspaceConfig {
            path: repo_path.to_string_lossy().to_string(),
            workspace_key: "repo-main".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::Git,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
        manager.save(settings).await.expect("save main mapping");

        run_git(
            &repo_path,
            &[
                "worktree",
                "add",
                worktree_path.to_string_lossy().as_ref(),
                "feature",
            ],
        );

        manager
            .modify(|settings| {
                settings.workspaces.push(WorkspaceConfig {
                    path: worktree_path.to_string_lossy().to_string(),
                    workspace_key: "repo-feature".to_string(),
                    editor: String::new(),
                    auto_discovered: false,
                    trusted: false,
                    workspace_kind: WorkspaceKind::Git,
                    workspace_state: WorkspaceState::Present,
                    repo_identity: None,
                    last_verified_at: None,
                    normalized_path: None,
                });
                Ok(((), true))
            })
            .await
            .expect("save worktree mapping");

        let saved = manager.get().await;
        assert_eq!(saved.workspaces.len(), 2);

        let repo_identity = saved
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_key == "repo-main")
            .and_then(|workspace| workspace.repo_identity.clone())
            .expect("repo identity");

        let grouped = manager.get_workspaces_in_repo_group(&repo_identity).await;
        assert_eq!(grouped.len(), 2);
    }

    fn initialize_git_workspace_with_remote(path: &Path, remote: &str) {
        std::fs::create_dir_all(path).expect("create repo dir");
        run_git(path, &["init"]);
        run_git(path, &["config", "user.email", "test@example.com"]);
        run_git(path, &["config", "user.name", "Test User"]);
        run_git(path, &["remote", "add", "origin", remote]);
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("run git");
        assert!(status.success(), "git command failed: {:?}", args);
    }
}
