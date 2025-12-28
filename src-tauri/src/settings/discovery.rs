use super::models::WorkspaceConfig;
use super::SettingsManager;
use anyhow::Result;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncResult {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

pub struct WorkspaceSync {
    settings_manager: Arc<SettingsManager>,
}

impl WorkspaceSync {
    pub fn new(settings_manager: Arc<SettingsManager>) -> Self {
        Self { settings_manager }
    }

    /// Sync workspaces with the default_workspaces_folder.
    /// - Adds new repos as auto_discovered workspaces
    /// - Removes auto_discovered workspaces that no longer exist on disk
    /// - Respects ignored_workspaces list
    pub async fn sync(&self) -> Result<SyncResult> {
        let mut settings = self.settings_manager.get().await;
        let mut result = SyncResult::default();

        let folder = self.get_normalized_workspaces_folder(&settings.defaults.default_workspaces_folder);
        let Some(folder) = folder else {
            debug!("No valid default_workspaces_folder configured, skipping sync");
            return Ok(result);
        };

        // Build set of ignored paths (normalized)
        let ignored: HashSet<PathBuf> = settings
            .defaults
            .ignored_workspaces
            .iter()
            .filter_map(|p| self.normalize_path(p))
            .collect();

        // Build set of existing workspace paths (normalized)
        let existing_paths: HashSet<PathBuf> = settings
            .workspaces
            .iter()
            .filter_map(|ws| ws.normalized_path.clone())
            .collect();

        // Scan for git repos in the folder
        let discovered = self.scan_folder(&folder);

        // Add new repos
        for repo in &discovered {
            if ignored.contains(repo) {
                debug!("Skipping ignored workspace: {:?}", repo);
                continue;
            }
            if existing_paths.contains(repo) {
                continue;
            }

            let name = repo
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            info!("Adding auto-discovered workspace: {}", name);
            result.added.push(name.clone());

            settings.workspaces.push(WorkspaceConfig {
                path: repo.to_string_lossy().to_string(),
                name: Some(name),
                editor: String::new(),
                auto_discovered: true,
                normalized_path: Some(repo.clone()),
            });
        }

        // Remove auto_discovered workspaces that no longer exist
        let discovered_set: HashSet<&PathBuf> = discovered.iter().collect();
        let mut i = 0;
        while i < settings.workspaces.len() {
            let ws = &settings.workspaces[i];
            if ws.auto_discovered {
                if let Some(ref path) = ws.normalized_path {
                    if !discovered_set.contains(path) {
                        let name = ws.name.clone().unwrap_or_else(|| ws.path.clone());
                        info!("Removing auto-discovered workspace (no longer exists): {}", name);
                        result.removed.push(name);
                        settings.workspaces.remove(i);
                        continue;
                    }
                }
            }
            i += 1;
        }

        // Save if changes were made
        if !result.added.is_empty() || !result.removed.is_empty() {
            self.settings_manager.save(settings).await?;
            info!(
                "Workspace sync complete: {} added, {} removed",
                result.added.len(),
                result.removed.len()
            );
        } else {
            debug!("Workspace sync complete: no changes");
        }

        Ok(result)
    }

    fn get_normalized_workspaces_folder(&self, raw_path: &str) -> Option<PathBuf> {
        if raw_path.is_empty() {
            return None;
        }

        let expanded = shellexpand::tilde(raw_path);
        let path = PathBuf::from(expanded.as_ref());

        if path.exists() && path.is_dir() {
            Some(path)
        } else {
            warn!(
                "default_workspaces_folder '{}' does not exist or is not a directory",
                raw_path
            );
            None
        }
    }

    fn normalize_path(&self, raw_path: &str) -> Option<PathBuf> {
        if raw_path.is_empty() {
            return None;
        }
        let expanded = shellexpand::tilde(raw_path);
        Some(PathBuf::from(expanded.as_ref()))
    }

    fn scan_folder(&self, folder: &PathBuf) -> Vec<PathBuf> {
        debug!("Scanning default_workspaces_folder: {:?}", folder);

        let entries = match std::fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(e) => {
                warn!("Failed to read default_workspaces_folder {:?}: {}", folder, e);
                return Vec::new();
            }
        };

        let mut repos = Vec::new();

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            // Skip non-directories
            if !path.is_dir() {
                continue;
            }

            // Skip dot-prefixed folders
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }

            // Only include folders with .git
            let git_dir = path.join(".git");
            if !git_dir.exists() {
                continue;
            }

            repos.push(path);
        }

        debug!("Found {} git repos in {:?}", repos.len(), folder);
        repos
    }
}
