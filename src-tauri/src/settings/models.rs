use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub defaults: DefaultEditorConfig,

    #[serde(default, alias = "repos")]
    pub workspaces: Vec<WorkspaceConfig>,
}

impl Settings {
    /// Create settings with auto-detected `default_workspaces_folder`.
    /// Only use on first run - scans filesystem to find best candidate.
    #[must_use]
    pub fn with_detected_workspaces_folder() -> Self {
        Self {
            defaults: DefaultEditorConfig::with_detected_workspaces_folder(),
            workspaces: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct DefaultEditorConfig {
    #[serde(default = "default_editor")]
    pub editor: String,

    #[serde(default = "default_allow_non_workspace_files")]
    pub allow_non_workspace_files: bool,

    #[serde(default = "default_terminal")]
    pub preferred_terminal: String,

    #[serde(default = "default_workspaces_folder", alias = "repo_base_dir")]
    pub default_workspaces_folder: String,

    #[serde(default = "default_auto_switch_clean_branches")]
    pub auto_switch_clean_branches: bool,

    #[serde(default)]
    pub ignored_workspaces: Vec<String>,

    #[serde(default = "default_strip_git_diff_prefixes")]
    pub strip_git_diff_prefixes: bool,

    #[serde(default = "default_large_file_warning_mb")]
    pub large_file_warning_mb: u64,

    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,

    #[serde(default)]
    pub setup_completed: bool,
}

fn default_editor() -> String {
    "vscode".to_string()
}

const fn default_allow_non_workspace_files() -> bool {
    false
}

fn default_terminal() -> String {
    "auto".to_string()
}

fn count_git_repos(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().join(".git").exists())
                .count()
        })
        .unwrap_or(0)
}

fn default_workspaces_folder() -> String {
    "~/code".to_string()
}

fn detect_repo_base_dir() -> String {
    let home = dirs::home_dir().unwrap_or_default();
    let candidates = [
        "code", "Code", "repos", "Repos", "projects", "Projects", "dev", "Dev", "src", "apps",
        "Apps",
    ];

    let mut best_candidate: Option<(&str, usize)> = None;

    for candidate in candidates {
        let path = home.join(candidate);
        if path.is_dir() {
            let repo_count = count_git_repos(&path);
            if repo_count > 0 {
                match &best_candidate {
                    None => best_candidate = Some((candidate, repo_count)),
                    Some((_, best_count)) if repo_count > *best_count => {
                        best_candidate = Some((candidate, repo_count));
                    }
                    _ => {}
                }
            }
        }
    }

    best_candidate.map_or_else(|| "~/code".to_string(), |(name, _)| format!("~/{name}"))
}

const fn default_auto_switch_clean_branches() -> bool {
    true
}

const fn default_strip_git_diff_prefixes() -> bool {
    true
}

const fn default_large_file_warning_mb() -> u64 {
    5
}

const fn default_max_file_size_mb() -> u64 {
    50
}

impl Default for DefaultEditorConfig {
    fn default() -> Self {
        Self {
            editor: default_editor(),
            allow_non_workspace_files: default_allow_non_workspace_files(),
            preferred_terminal: default_terminal(),
            default_workspaces_folder: default_workspaces_folder(),
            auto_switch_clean_branches: default_auto_switch_clean_branches(),
            ignored_workspaces: Vec::new(),
            strip_git_diff_prefixes: default_strip_git_diff_prefixes(),
            large_file_warning_mb: default_large_file_warning_mb(),
            max_file_size_mb: default_max_file_size_mb(),
            setup_completed: false,
        }
    }
}

impl DefaultEditorConfig {
    #[must_use]
    pub fn with_detected_workspaces_folder() -> Self {
        Self {
            default_workspaces_folder: detect_repo_base_dir(),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceKind {
    Git,
    #[default]
    NonGit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceState {
    #[default]
    Present,
    Missing,
    Unavailable,
    IdentityDrift,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    #[default]
    Advisory,
    Enforced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspacePolicyMapping {
    pub workspace_key: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspacePolicyConfig {
    #[serde(default)]
    pub mode: PolicyMode,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mappings: Vec<WorkspacePolicyMapping>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_clone_roots: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_remote_hosts: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_remote_orgs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RepoIdentity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_remote: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_remotes: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_common_dir: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_branch: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_commit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub path: String,

    #[serde(default)]
    pub workspace_key: String,

    #[serde(default)]
    pub editor: String,

    #[serde(default)]
    pub auto_discovered: bool,

    #[serde(default)]
    pub trusted: bool,

    #[serde(default)]
    pub workspace_kind: WorkspaceKind,

    #[serde(default)]
    pub workspace_state: WorkspaceState,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_identity: Option<RepoIdentity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified_at: Option<i64>,

    #[serde(skip)]
    pub normalized_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LastSeenData {
    pub editors: HashMap<String, i64>,
    pub most_recent: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::count_git_repos;
    use tempfile::TempDir;

    #[test]
    fn count_git_repos_counts_worktree_style_git_dirs() {
        let root = TempDir::new().expect("temp dir");

        let classic_repo = root.path().join("classic");
        std::fs::create_dir_all(classic_repo.join(".git")).expect("classic repo");

        let worktree_repo = root.path().join("worktree");
        std::fs::create_dir_all(&worktree_repo).expect("worktree repo");
        std::fs::write(worktree_repo.join(".git"), "gitdir: /tmp/foo").expect("worktree git file");

        let non_repo = root.path().join("not_a_repo");
        std::fs::create_dir(&non_repo).expect("non repo dir");

        assert_eq!(count_git_repos(root.path()), 2);
    }
}
