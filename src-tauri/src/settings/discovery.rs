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
        let defaults_folder = self.settings_manager.get_default_workspaces_folder().await;
        let normalized_folder = self.get_normalized_workspaces_folder(&defaults_folder);
        let Some(folder) = normalized_folder else {
            debug!("No valid default_workspaces_folder configured, skipping sync");
            return Ok(SyncResult::default());
        };

        let discovered = self.scan_folder(&folder).await;

        self.settings_manager
            .modify(|settings| {
                if settings.defaults.default_workspaces_folder != defaults_folder {
                    debug!("default_workspaces_folder changed during sync, skipping update");
                    return Ok((SyncResult::default(), false));
                }

                let mut result = SyncResult::default();

                let ignored: HashSet<PathBuf> = settings
                    .defaults
                    .ignored_workspaces
                    .iter()
                    .filter_map(|p| self.normalize_path(p))
                    .collect();

                let existing_paths: HashSet<PathBuf> = settings
                    .workspaces
                    .iter()
                    .filter_map(|ws| ws.normalized_path.clone())
                    .collect();

                for repo in &discovered {
                    if ignored.contains(repo) || existing_paths.contains(repo) {
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

                let discovered_set: HashSet<&PathBuf> = discovered.iter().collect();
                let mut i = 0;
                while i < settings.workspaces.len() {
                    let ws = &settings.workspaces[i];
                    if ws.auto_discovered {
                        if let Some(ref path) = ws.normalized_path {
                            if !discovered_set.contains(path) {
                                let name = ws.name.clone().unwrap_or_else(|| ws.path.clone());
                                info!(
                                    "Removing auto-discovered workspace (no longer exists): {}",
                                    name
                                );
                                result.removed.push(name);
                                settings.workspaces.remove(i);
                                continue;
                            }
                        }
                    }
                    i += 1;
                }

                let changed = !result.added.is_empty() || !result.removed.is_empty();
                if changed {
                    info!(
                        "Workspace sync complete: {} added, {} removed",
                        result.added.len(),
                        result.removed.len()
                    );
                } else {
                    debug!("Workspace sync complete: no changes");
                }

                Ok((result, changed))
            })
            .await
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

    async fn scan_folder(&self, folder: &PathBuf) -> Vec<PathBuf> {
        let folder_for_blocking = folder.clone();
        let folder_for_error = folder.clone();
        tokio::task::spawn_blocking(move || {
            let folder = folder_for_blocking;
            debug!("Scanning default_workspaces_folder: {:?}", folder);

            let entries = match std::fs::read_dir(&folder) {
                Ok(entries) => entries,
                Err(e) => {
                    warn!(
                        "Failed to read default_workspaces_folder {:?}: {}",
                        folder, e
                    );
                    return Vec::new();
                }
            };

            let mut repos = Vec::new();

            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.starts_with('.') {
                    continue;
                }

                if path.join(".git").exists() {
                    repos.push(path);
                }
            }

            debug!("Found {} git repos in {:?}", repos.len(), folder);
            repos
        })
        .await
        .unwrap_or_else(move |e| {
            warn!(
                "Workspace scan task failed for {:?}: {}",
                folder_for_error,
                e.to_string()
            );
            Vec::new()
        })
    }
}
