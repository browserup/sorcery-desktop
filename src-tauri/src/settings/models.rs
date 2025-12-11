use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub defaults: DefaultEditorConfig,

    #[serde(default, alias = "repos")]
    pub workspaces: Vec<WorkspaceConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            defaults: DefaultEditorConfig::default(),
            workspaces: Vec::new(),
        }
    }
}

impl Settings {
    /// Create settings with auto-detected repo_base_dir.
    /// Only use on first run - scans filesystem to find best candidate.
    pub fn with_detected_repo_base() -> Self {
        Self {
            defaults: DefaultEditorConfig::with_detected_repo_base(),
            workspaces: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultEditorConfig {
    #[serde(default = "default_editor")]
    pub editor: String,

    #[serde(default = "default_allow_non_workspace_files")]
    pub allow_non_workspace_files: bool,

    #[serde(default = "default_terminal")]
    pub preferred_terminal: String,

    #[serde(default = "default_repo_base_dir")]
    pub repo_base_dir: String,

    #[serde(default = "default_auto_switch_clean_branches")]
    pub auto_switch_clean_branches: bool,
}

fn default_editor() -> String {
    "vscode".to_string()
}

fn default_allow_non_workspace_files() -> bool {
    false
}

fn default_terminal() -> String {
    "auto".to_string()
}

fn count_git_repos(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().join(".git").is_dir())
                .count()
        })
        .unwrap_or(0)
}

fn default_repo_base_dir() -> String {
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

    best_candidate
        .map(|(name, _)| format!("~/{}", name))
        .unwrap_or_else(|| "~/code".to_string())
}

fn default_auto_switch_clean_branches() -> bool {
    true
}

impl Default for DefaultEditorConfig {
    fn default() -> Self {
        Self {
            editor: default_editor(),
            allow_non_workspace_files: default_allow_non_workspace_files(),
            preferred_terminal: default_terminal(),
            repo_base_dir: default_repo_base_dir(),
            auto_switch_clean_branches: default_auto_switch_clean_branches(),
        }
    }
}

impl DefaultEditorConfig {
    pub fn with_detected_repo_base() -> Self {
        Self {
            repo_base_dir: detect_repo_base_dir(),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(default)]
    pub editor: String,

    #[serde(skip)]
    pub normalized_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LastSeenData {
    pub editors: HashMap<String, i64>,
    pub most_recent: Option<String>,
}
