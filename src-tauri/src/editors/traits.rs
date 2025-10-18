use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EditorError {
    #[error("Editor binary not found")]
    BinaryNotFound,

    #[error("Failed to launch editor: {0}")]
    LaunchFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type EditorResult<T> = Result<T, EditorError>;

#[derive(Debug, Clone, Serialize)]
pub struct OpenOptions {
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub new_window: bool,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            line: None,
            column: None,
            new_window: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorInstance {
    pub pid: u32,
    pub workspace: Option<String>,
    pub window_title: Option<String>,
}

#[async_trait]
pub trait EditorManager: Send + Sync {
    fn id(&self) -> &str;

    fn display_name(&self) -> &str;

    async fn is_installed(&self) -> bool {
        self.find_binary().await.is_some()
    }

    async fn find_binary(&self) -> Option<PathBuf>;

    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()>;

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>>;
}
