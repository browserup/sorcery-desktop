use crate::settings::SettingsManager;
use crate::workspace_mru::ActiveWorkspaceTracker;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;
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

#[derive(Debug, Error)]
pub enum WorkspaceLookupError {
    #[error("Workspace '{0}' not found in configuration")]
    WorkspaceNotFound(String),
    #[error("Path '{1}' not found in workspace '{0}'")]
    PathNotFound(String, String),
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

    async fn path_exists_and_valid(path: &PathBuf) -> bool {
        match tokio::fs::metadata(path).await {
            Ok(meta) => meta.is_file() || meta.is_dir(),
            Err(_) => false,
        }
    }

    pub async fn find_partial_matches(&self, partial_path: &str) -> Result<Vec<WorkspaceMatch>> {
        let workspaces = self.settings_manager.get_workspaces().await;
        let mut matches = Vec::new();

        for workspace in &workspaces {
            if let Some(workspace_root) = &workspace.normalized_path {
                let candidate = workspace_root.join(partial_path);

                if Self::path_exists_and_valid(&candidate).await {
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
    ) -> Result<PathBuf, WorkspaceLookupError> {
        let workspaces = self.settings_manager.get_workspaces().await;

        for workspace in &workspaces {
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

                    if Self::path_exists_and_valid(&full_path).await {
                        debug!(
                            "Found workspace match: {} -> {}",
                            workspace_name,
                            full_path.display()
                        );
                        return Ok(full_path);
                    } else {
                        return Err(WorkspaceLookupError::PathNotFound(
                            workspace_name.to_string(),
                            relative_path.to_string(),
                        ));
                    }
                }
            }
        }

        Err(WorkspaceLookupError::WorkspaceNotFound(
            workspace_name.to_string(),
        ))
    }

    pub async fn find_full_path_matches(&self, full_path: &str) -> Result<Vec<WorkspaceMatch>> {
        info!("Scanning full path for workspace fragments: {}", full_path);

        let workspaces = self.settings_manager.get_workspaces().await;
        let mut matches = Vec::new();
        let normalized_input = full_path.replace('\\', "/");
        let path_segments: Vec<&str> = normalized_input
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        let user_path = PathBuf::from(full_path);
        let user_metadata = tokio::fs::metadata(&user_path).await.ok();
        let user_path_valid = user_metadata
            .as_ref()
            .map(|meta| meta.is_file() || meta.is_dir())
            .unwrap_or(false);
        let user_is_dir = user_metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);

        for workspace in &workspaces {
            let ws_name = workspace.name.as_deref().unwrap_or_else(|| {
                workspace
                    .normalized_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
            });

            let Some(workspace_root) = &workspace.normalized_path else {
                continue;
            };

            if user_path_valid && user_path.starts_with(workspace_root) {
                info!(
                    "Full path '{}' is inside workspace '{}'",
                    full_path, ws_name
                );

                matches.push(WorkspaceMatch {
                    workspace_name: ws_name.to_string(),
                    workspace_path: workspace_root.clone(),
                    full_file_path: user_path.clone(),
                    last_seen: None,
                    last_active: None,
                });
                continue;
            }

            for (idx, segment) in path_segments.iter().enumerate() {
                if segment.eq_ignore_case(ws_name) {
                    let mut fragment = PathBuf::new();
                    for seg in &path_segments[idx + 1..] {
                        fragment.push(seg);
                    }

                    let candidate = workspace_root.join(&fragment);

                    if Self::path_exists_and_valid(&candidate).await {
                        info!("Match found: {}", candidate.display());
                        matches.push(WorkspaceMatch {
                            workspace_name: ws_name.to_string(),
                            workspace_path: workspace_root.clone(),
                            full_file_path: candidate,
                            last_seen: None,
                            last_active: None,
                        });
                    }
                    break;
                }
            }
        }

        if matches.is_empty() {
            debug!("No workspace fragments found in path, checking if path exists as-is");
            if user_path_valid {
                matches.push(WorkspaceMatch {
                    workspace_name: if user_is_dir {
                        "Non-workspace folder"
                    } else {
                        "Non-workspace file"
                    }
                    .to_string(),
                    workspace_path: user_path.parent().unwrap_or(&user_path).to_path_buf(),
                    full_file_path: user_path,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{SettingsManager, WorkspaceConfig};
    use crate::workspace_mru::ActiveWorkspaceTracker;
    use tempfile::TempDir;

    async fn build_matcher(workspace_dir: &PathBuf, workspace_name: &str) -> PathMatcher {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = Arc::new(
            SettingsManager::new_with_path(settings_path)
                .await
                .expect("settings manager"),
        );

        let mut settings = manager.get().await;
        settings.workspaces.push(WorkspaceConfig {
            path: workspace_dir.to_string_lossy().to_string(),
            name: Some(workspace_name.to_string()),
            editor: String::new(),
            auto_discovered: false,
            normalized_path: Some(workspace_dir.clone()),
        });
        manager.save(settings).await.expect("save settings");

        let tracker = Arc::new(ActiveWorkspaceTracker::new(manager.clone()));
        PathMatcher::new(manager, tracker)
    }

    #[tokio::test]
    async fn full_path_match_detects_direct_path() {
        let workspace_dir = TempDir::new().expect("workspace dir");
        let file_path = workspace_dir.path().join("src").join("lib.rs");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "fn test_match() {}").unwrap();

        let matcher = build_matcher(
            &workspace_dir.path().to_path_buf(),
            "workspace-direct-match",
        )
        .await;

        let matches = matcher
            .find_full_path_matches(&file_path.to_string_lossy())
            .await
            .expect("matcher result");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].full_file_path, file_path);
        assert_eq!(matches[0].workspace_name, "workspace-direct-match");
    }

    #[tokio::test]
    async fn full_path_match_detects_windows_like_alias() {
        let workspace_dir = TempDir::new().expect("workspace dir");
        let file_path = workspace_dir.path().join("src").join("main.rs");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let matcher = build_matcher(&workspace_dir.path().to_path_buf(), "my-workspace").await;

        let remote_path = "D:\\Code\\my-workspace\\src\\main.rs";
        let matches = matcher
            .find_full_path_matches(remote_path)
            .await
            .expect("matcher result");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].full_file_path, file_path);
        assert_eq!(matches[0].workspace_name, "my-workspace");
    }
}
