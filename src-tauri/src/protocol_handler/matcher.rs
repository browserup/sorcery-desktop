use crate::settings::SettingsManager;
use crate::workspace_mru::ActiveWorkspaceTracker;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMatch {
    pub workspace_name: String,
    pub workspace_path: PathBuf,
    pub full_file_path: PathBuf,
    pub last_seen: Option<i64>,
    #[serde(skip)]
    pub last_active: Option<SystemTime>,
}

pub struct PathMatcher {
    settings_manager: Arc<SettingsManager>,
    workspace_tracker: Arc<ActiveWorkspaceTracker>,
}

impl PathMatcher {
    pub fn new(
        settings_manager: Arc<SettingsManager>,
        workspace_tracker: Arc<ActiveWorkspaceTracker>,
    ) -> Self {
        Self {
            settings_manager,
            workspace_tracker,
        }
    }

    pub async fn find_partial_matches(&self, partial_path: &str) -> Result<Vec<WorkspaceMatch>> {
        let settings = self.settings_manager.get().await;
        let mut matches = Vec::new();

        for workspace in &settings.workspaces {
            if let Some(workspace_root) = &workspace.normalized_path {
                let candidate = workspace_root.join(partial_path);

                if candidate.exists() && (candidate.is_file() || candidate.is_dir()) {
                    matches.push(WorkspaceMatch {
                        workspace_name: workspace.name.clone().unwrap_or_else(|| {
                            workspace_root
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string()
                        }),
                        workspace_path: workspace_root.clone(),
                        full_file_path: candidate,
                        last_seen: None,
                        last_active: None,
                    });
                }
            }
        }

        debug!(
            "Found {} matches for partial path '{}'",
            matches.len(),
            partial_path
        );
        Ok(matches)
    }

    pub async fn find_workspace_path(
        &self,
        workspace_name: &str,
        relative_path: &str,
    ) -> Result<PathBuf> {
        let settings = self.settings_manager.get().await;

        for workspace in &settings.workspaces {
            let ws_name = workspace.name.as_deref().unwrap_or_else(|| {
                workspace
                    .normalized_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
            });

            if ws_name.eq_ignore_case(workspace_name) {
                if let Some(workspace_root) = &workspace.normalized_path {
                    let full_path = workspace_root.join(relative_path);

                    if full_path.exists() && (full_path.is_file() || full_path.is_dir()) {
                        debug!(
                            "Found workspace match: {} -> {}",
                            workspace_name,
                            full_path.display()
                        );
                        return Ok(full_path);
                    } else {
                        bail!(
                            "Path not found in workspace '{}': {}",
                            workspace_name,
                            relative_path
                        );
                    }
                }
            }
        }

        bail!("Workspace '{}' not found in configuration", workspace_name);
    }

    pub async fn find_full_path_matches(&self, full_path: &str) -> Result<Vec<WorkspaceMatch>> {
        info!("Scanning full path for workspace fragments: {}", full_path);

        let settings = self.settings_manager.get().await;
        let mut matches = Vec::new();

        for workspace in &settings.workspaces {
            let ws_name = workspace.name.as_deref().unwrap_or_else(|| {
                workspace
                    .normalized_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
            });

            if let Some(fragment_start) = full_path.find(&format!("/{}/", ws_name)) {
                let fragment = &full_path[fragment_start + ws_name.len() + 2..];

                info!(
                    "Found workspace '{}' in path, checking fragment: {}",
                    ws_name, fragment
                );

                if let Some(workspace_root) = &workspace.normalized_path {
                    let candidate = workspace_root.join(fragment);

                    if candidate.exists() && (candidate.is_file() || candidate.is_dir()) {
                        info!("Match found: {}", candidate.display());
                        matches.push(WorkspaceMatch {
                            workspace_name: ws_name.to_string(),
                            workspace_path: workspace_root.clone(),
                            full_file_path: candidate,
                            last_seen: None,
                            last_active: None,
                        });
                    }
                }
            }
        }

        if matches.is_empty() {
            debug!("No workspace fragments found in path, checking if path exists as-is");
            let path = PathBuf::from(full_path);
            if path.exists() && (path.is_file() || path.is_dir()) {
                matches.push(WorkspaceMatch {
                    workspace_name: if path.is_dir() {
                        "Non-workspace folder"
                    } else {
                        "Non-workspace file"
                    }
                    .to_string(),
                    workspace_path: path.parent().unwrap_or(&path).to_path_buf(),
                    full_file_path: path,
                    last_seen: None,
                    last_active: None,
                });
            }
        }

        debug!(
            "Found {} matches for full path '{}'",
            matches.len(),
            full_path
        );
        Ok(matches)
    }

    pub async fn sort_by_recent_usage(&self, matches: &mut Vec<WorkspaceMatch>) {
        for ws_match in matches.iter_mut() {
            ws_match.last_active = self
                .workspace_tracker
                .get_workspace_last_active(&ws_match.workspace_path)
                .await;
        }

        matches.sort_by(|a, b| match (a.last_active, b.last_active) {
            (Some(time_a), Some(time_b)) => time_b.cmp(&time_a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.workspace_name.cmp(&b.workspace_name),
        });

        debug!("Sorted {} matches by workspace MRU", matches.len());
    }
}

trait StrExt {
    fn eq_ignore_case(&self, other: &str) -> bool;
}

impl StrExt for str {
    fn eq_ignore_case(&self, other: &str) -> bool {
        self.to_lowercase() == other.to_lowercase()
    }
}
