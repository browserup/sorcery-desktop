/// Testing-only editor that performs no actual operations.
use super::traits::{EditorInstance, EditorManager, EditorResult, OpenOptions};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tracing::debug;

#[allow(dead_code)] // Only used by integration tests, not main binary
pub struct NullEditor;

impl Default for NullEditor {
    fn default() -> Self {
        Self
    }
}

impl NullEditor {
    #[allow(dead_code)] // Only used by integration tests
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for NullEditor {
    fn id(&self) -> &str {
        "null"
    }

    fn display_name(&self) -> &str {
        "Null Editor (Testing)"
    }

    fn supports_folders(&self) -> bool {
        true
    }

    async fn is_installed(&self) -> bool {
        true
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/dev/null"))
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()> {
        debug!(
            "NullEditor: simulated open for {:?} at line {:?}, column {:?}",
            path, options.line, options.column
        );
        Ok(())
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        Ok(vec![])
    }
}
