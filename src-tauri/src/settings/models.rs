use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub defaults: DefaultEditorConfig,

    #[serde(default)]
    pub repos: Vec<WorkspaceConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            defaults: DefaultEditorConfig::default(),
            repos: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultEditorConfig {
    #[serde(default = "default_editor")]
    pub editor: String,
}

fn default_editor() -> String {
    "vscode".to_string()
}

impl Default for DefaultEditorConfig {
    fn default() -> Self {
        Self {
            editor: default_editor(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    pub editor: String,

    #[serde(skip)]
    pub normalized_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LastSeenData {
    pub editors: HashMap<String, i64>,
    pub most_recent: Option<String>,
}
