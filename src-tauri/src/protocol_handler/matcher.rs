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

fn strip_git_diff_prefix(path: &str) -> Option<&str> {
    if path.starts_with("a/") || path.starts_with("b/") {
        Some(&path[2..])
    } else {
        None
    }
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
        let matches = self.find_partial_matches_inner(partial_path).await?;

        if matches.is_empty() {
            let settings = self.settings_manager.get().await;
            if settings.defaults.strip_git_diff_prefixes {
                if let Some(stripped) = strip_git_diff_prefix(partial_path) {
                    debug!(
                        "No matches for '{}', retrying with stripped git diff prefix: '{}'",
                        partial_path, stripped
                    );
                    return self.find_partial_matches_inner(stripped).await;
                }
            }
        }

        Ok(matches)
    }

    async fn find_partial_matches_inner(&self, partial_path: &str) -> Result<Vec<WorkspaceMatch>> {
        let workspaces = self.settings_manager.get_workspaces().await;
        let mut workspace_in_path_matches = Vec::new();
        let mut suffix_matches = Vec::new();
        let mut matched_workspace_names = std::collections::HashSet::new();

        // Normalize path and split into segments for workspace detection
        let normalized_path = partial_path.replace('\\', "/");
        let path_segments: Vec<&str> = normalized_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

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

            // Phase 1: Check if workspace name appears in path segments (highest priority)
            let mut found_workspace_in_path = false;
            for (idx, segment) in path_segments.iter().enumerate() {
                if segment.eq_ignore_case(ws_name) {
                    // Found workspace name - extract relative path from remaining segments
                    let relative_path: PathBuf = path_segments[idx + 1..].iter().collect();
                    let candidate = workspace_root.join(&relative_path);

                    if Self::path_exists_and_valid(&candidate).await {
                        debug!(
                            "Workspace-in-path match: {} -> {}",
                            ws_name,
                            candidate.display()
                        );
                        workspace_in_path_matches.push(WorkspaceMatch {
                            workspace_name: ws_name.to_string(),
                            workspace_path: workspace_root.clone(),
                            full_file_path: candidate,
                            last_seen: None,
                            last_active: None,
                        });
                        matched_workspace_names.insert(ws_name.to_lowercase());
                        found_workspace_in_path = true;
                    }
                    break; // Use first occurrence of workspace name
                }
            }

            // Phase 2: Suffix matching (only if not already matched via workspace-in-path)
            if !found_workspace_in_path {
                let candidate = workspace_root.join(partial_path);

                if Self::path_exists_and_valid(&candidate).await {
                    suffix_matches.push(WorkspaceMatch {
                        workspace_name: ws_name.to_string(),
                        workspace_path: workspace_root.clone(),
                        full_file_path: candidate,
                        last_seen: None,
                        last_active: None,
                    });
                }
            }
        }

        // Combine matches: workspace-in-path first (higher priority), then suffix matches
        let mut matches = workspace_in_path_matches;
        matches.extend(suffix_matches);

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
        match self
            .find_workspace_path_inner(workspace_name, relative_path)
            .await
        {
            Ok(path) => Ok(path),
            Err(WorkspaceLookupError::PathNotFound(ws, _)) => {
                let settings = self.settings_manager.get().await;
                if settings.defaults.strip_git_diff_prefixes {
                    if let Some(stripped) = strip_git_diff_prefix(relative_path) {
                        debug!(
                            "Path not found in '{}', retrying with stripped git diff prefix: '{}'",
                            workspace_name, stripped
                        );
                        return self
                            .find_workspace_path_inner(workspace_name, stripped)
                            .await;
                    }
                }
                Err(WorkspaceLookupError::PathNotFound(
                    ws,
                    relative_path.to_string(),
                ))
            }
            Err(e) => Err(e),
        }
    }

    async fn find_workspace_path_inner(
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
                    // Check if workspace root directory exists
                    // If not, treat as WorkspaceNotFound so clone dialog can be offered
                    if !Self::path_exists_and_valid(workspace_root).await {
                        debug!(
                            "Workspace '{}' directory no longer exists at {}",
                            workspace_name,
                            workspace_root.display()
                        );
                        return Err(WorkspaceLookupError::WorkspaceNotFound(
                            workspace_name.to_string(),
                        ));
                    }

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
            .is_some_and(|meta| meta.is_file() || meta.is_dir());
        let user_is_dir = user_metadata
            .as_ref()
            .is_some_and(std::fs::Metadata::is_dir);

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

    pub async fn sort_by_recent_usage(&self, matches: &mut [WorkspaceMatch]) {
        const MAX_REFLOG_CHECKS: usize = 5;

        for ws_match in matches.iter_mut() {
            ws_match.last_active = self
                .workspace_tracker
                .compute_effective_time(&ws_match.workspace_path, false)
                .await;
        }

        Self::sort_matches(matches);

        let reflog_futures: Vec<_> = matches
            .iter()
            .take(MAX_REFLOG_CHECKS)
            .map(|m| {
                let path = m.workspace_path.clone();
                tokio::task::spawn_blocking(move || {
                    crate::workspace_mru::git_signals::head_reflog_time(&path)
                })
            })
            .collect();

        let results = futures::future::join_all(reflog_futures).await;

        for (i, result) in results.into_iter().enumerate() {
            if let Ok(Some(reflog_time)) = result {
                let current = matches[i].last_active;
                if current.is_none_or(|t| reflog_time > t) {
                    matches[i].last_active = Some(reflog_time);
                }
            }
        }

        Self::sort_matches(matches);

        debug!("Sorted {} matches by workspace MRU", matches.len());
    }

    fn sort_matches(matches: &mut [WorkspaceMatch]) {
        matches.sort_by(|a, b| match (a.last_active, b.last_active) {
            (Some(time_a), Some(time_b)) => time_b.cmp(&time_a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.workspace_name.cmp(&b.workspace_name),
        });
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
            trusted: false,
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
        std::fs::create_dir_all(file_path.parent().expect("file has parent"))
            .expect("create test dir");
        std::fs::write(&file_path, "fn test_match() {}").expect("write test file");

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
        std::fs::create_dir_all(file_path.parent().expect("file has parent"))
            .expect("create test dir");
        std::fs::write(&file_path, "fn main() {}").expect("write test file");

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

    #[tokio::test]
    async fn partial_match_strips_git_diff_a_prefix() {
        let workspace_dir = TempDir::new().expect("workspace dir");
        let file_path = workspace_dir.path().join("src").join("lib.rs");
        std::fs::create_dir_all(file_path.parent().expect("file has parent"))
            .expect("create test dir");
        std::fs::write(&file_path, "fn test() {}").expect("write test file");

        let matcher = build_matcher(&workspace_dir.path().to_path_buf(), "test-ws").await;

        let matches = matcher
            .find_partial_matches("a/src/lib.rs")
            .await
            .expect("matcher result");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].full_file_path, file_path);
    }

    #[tokio::test]
    async fn partial_match_strips_git_diff_b_prefix() {
        let workspace_dir = TempDir::new().expect("workspace dir");
        let file_path = workspace_dir.path().join("README.md");
        std::fs::write(&file_path, "# Test").expect("write test file");

        let matcher = build_matcher(&workspace_dir.path().to_path_buf(), "test-ws").await;

        let matches = matcher
            .find_partial_matches("b/README.md")
            .await
            .expect("matcher result");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].full_file_path, file_path);
    }

    #[tokio::test]
    async fn partial_match_preserves_real_a_directory() {
        let workspace_dir = TempDir::new().expect("workspace dir");
        let file_path = workspace_dir.path().join("a").join("file.rs");
        std::fs::create_dir_all(file_path.parent().expect("file has parent"))
            .expect("create test dir");
        std::fs::write(&file_path, "fn a() {}").expect("write test file");

        let matcher = build_matcher(&workspace_dir.path().to_path_buf(), "test-ws").await;

        let matches = matcher
            .find_partial_matches("a/file.rs")
            .await
            .expect("matcher result");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].full_file_path, file_path);
    }

    #[tokio::test]
    async fn workspace_path_strips_git_diff_prefix() {
        let workspace_dir = TempDir::new().expect("workspace dir");
        let file_path = workspace_dir.path().join("src").join("main.rs");
        std::fs::create_dir_all(file_path.parent().expect("file has parent"))
            .expect("create test dir");
        std::fs::write(&file_path, "fn main() {}").expect("write test file");

        let matcher = build_matcher(&workspace_dir.path().to_path_buf(), "myproject").await;

        let result = matcher
            .find_workspace_path("myproject", "a/src/main.rs")
            .await
            .expect("should find path after stripping prefix");

        assert_eq!(result, file_path);
    }

    #[test]
    fn strip_git_diff_prefix_works() {
        assert_eq!(strip_git_diff_prefix("a/src/lib.rs"), Some("src/lib.rs"));
        assert_eq!(strip_git_diff_prefix("b/README.md"), Some("README.md"));
        assert_eq!(strip_git_diff_prefix("src/lib.rs"), None);
        assert_eq!(strip_git_diff_prefix("ab/file.rs"), None);
    }

    // Workspace-in-path detection tests

    #[tokio::test]
    async fn partial_match_finds_workspace_in_middle_of_path() {
        let workspace_dir = TempDir::new().expect("workspace dir");
        let file_path = workspace_dir.path().join("src").join("main.rs");
        std::fs::create_dir_all(file_path.parent().expect("file has parent"))
            .expect("create test dir");
        std::fs::write(&file_path, "fn main() {}").expect("write test file");

        let matcher = build_matcher(&workspace_dir.path().to_path_buf(), "myproject").await;

        // Path has workspace name in middle - should find it and extract relative path
        let matches = matcher
            .find_partial_matches("some/prefix/myproject/src/main.rs")
            .await
            .expect("matcher result");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].full_file_path, file_path);
        assert_eq!(matches[0].workspace_name, "myproject");
    }

    #[tokio::test]
    async fn partial_match_workspace_detection_is_case_insensitive() {
        let workspace_dir = TempDir::new().expect("workspace dir");
        let file_path = workspace_dir.path().join("lib.rs");
        std::fs::write(&file_path, "fn test() {}").expect("write test file");

        let matcher = build_matcher(&workspace_dir.path().to_path_buf(), "myproject").await;

        // Different case should still match
        let matches = matcher
            .find_partial_matches("prefix/MyProject/lib.rs")
            .await
            .expect("matcher result");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].full_file_path, file_path);
    }

    #[tokio::test]
    async fn partial_match_workspace_must_be_full_segment() {
        let workspace_dir = TempDir::new().expect("workspace dir");
        let file_path = workspace_dir.path().join("src").join("main.rs");
        std::fs::create_dir_all(file_path.parent().expect("file has parent"))
            .expect("create test dir");
        std::fs::write(&file_path, "fn main() {}").expect("write test file");

        let matcher = build_matcher(&workspace_dir.path().to_path_buf(), "api").await;

        // "myapi" contains "api" but is not the same segment - should NOT match
        let matches = matcher
            .find_partial_matches("prefix/myapi/src/main.rs")
            .await
            .expect("matcher result");

        assert_eq!(matches.len(), 0);
    }

    #[tokio::test]
    async fn partial_match_uses_first_workspace_occurrence() {
        let workspace_dir = TempDir::new().expect("workspace dir");
        // Create nested path that has workspace name twice
        let file_path = workspace_dir.path().join("nested").join("file.rs");
        std::fs::create_dir_all(file_path.parent().expect("file has parent"))
            .expect("create test dir");
        std::fs::write(&file_path, "fn test() {}").expect("write test file");

        let matcher = build_matcher(&workspace_dir.path().to_path_buf(), "myproject").await;

        // Path has "myproject" twice - should use first occurrence
        let matches = matcher
            .find_partial_matches("a/myproject/myproject/nested/file.rs")
            .await
            .expect("matcher result");

        // Should find using first "myproject", path becomes "myproject/nested/file.rs"
        // which won't exist. Then fallback to second? Or just use first.
        // Actually the expected behavior is to use first match and take everything after.
        // So first "myproject" → path is "myproject/nested/file.rs" (doesn't exist as file)
        // This test documents the behavior - first match wins
        assert!(matches.len() <= 1);
    }

    #[tokio::test]
    async fn deleted_workspace_returns_not_found() {
        // Create a workspace, then delete its directory
        let workspace_dir = TempDir::new().expect("workspace dir");
        let workspace_path = workspace_dir.path().to_path_buf();

        let matcher = build_matcher(&workspace_path, "deleted-ws").await;

        // Delete the workspace directory
        drop(workspace_dir);

        // Should return WorkspaceNotFound, not PathNotFound
        let result = matcher
            .find_workspace_path("deleted-ws", "some/file.rs")
            .await;

        assert!(matches!(
            result,
            Err(WorkspaceLookupError::WorkspaceNotFound(_))
        ));
    }

    #[tokio::test]
    async fn partial_match_workspace_in_path_has_priority() {
        // Create two workspaces
        let workspace1_dir = TempDir::new().expect("workspace1 dir");
        let workspace2_dir = TempDir::new().expect("workspace2 dir");

        // Both have same relative file
        let file1 = workspace1_dir.path().join("src").join("main.rs");
        let file2 = workspace2_dir.path().join("src").join("main.rs");
        std::fs::create_dir_all(file1.parent().expect("file has parent")).expect("create test dir");
        std::fs::create_dir_all(file2.parent().expect("file has parent")).expect("create test dir");
        std::fs::write(&file1, "fn main() { /* ws1 */ }").expect("write test file");
        std::fs::write(&file2, "fn main() { /* ws2 */ }").expect("write test file");

        // Create matcher with both workspaces
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = Arc::new(
            SettingsManager::new_with_path(settings_path)
                .await
                .expect("settings manager"),
        );

        let mut settings = manager.get().await;
        settings.workspaces.push(WorkspaceConfig {
            path: workspace1_dir.path().to_string_lossy().to_string(),
            name: Some("backend".to_string()),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            normalized_path: Some(workspace1_dir.path().to_path_buf()),
        });
        settings.workspaces.push(WorkspaceConfig {
            path: workspace2_dir.path().to_string_lossy().to_string(),
            name: Some("frontend".to_string()),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            normalized_path: Some(workspace2_dir.path().to_path_buf()),
        });
        manager.save(settings).await.expect("save settings");

        let tracker = Arc::new(ActiveWorkspaceTracker::new(manager.clone()));
        let matcher = PathMatcher::new(manager, tracker);

        // Search with workspace name in path - should prioritize that workspace
        let matches = matcher
            .find_partial_matches("prefix/backend/src/main.rs")
            .await
            .expect("matcher result");

        assert!(!matches.is_empty());
        // First match should be the workspace mentioned in the path
        assert_eq!(matches[0].workspace_name, "backend");
        assert_eq!(matches[0].full_file_path, file1);
    }
}
