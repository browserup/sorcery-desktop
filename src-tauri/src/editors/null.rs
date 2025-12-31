/// # NullEditor - A Testing-Only Editor Implementation
///
/// This editor exists solely for testing purposes. It implements the EditorManager
/// trait but performs no actual operations - no processes are spawned, no files
/// are opened.
///
/// ## Why This Exists
///
/// Integration tests for the protocol handler and dispatcher need to exercise
/// the full code path (URL parsing → path validation → editor selection → open)
/// without actually launching real editors like VS Codium or VS Code.
///
/// Before NullEditor, tests would spawn multiple VS Codium instances, which:
/// - Slowed down test execution
/// - Left editor windows open after tests
/// - Required specific editors to be installed
/// - Made tests flaky in CI environments
///
/// ## Usage
///
/// In tests, set the default editor to "null":
/// ```text
/// settings.defaults.editor = "null".to_string();
/// ```
///
/// For tests that actually need to verify editor launching behavior,
/// use the editor_launch_tests.rs tests which are gated behind
/// `#[cfg(feature = "docker-tests")]`.
use super::traits::{EditorInstance, EditorManager, EditorResult, OpenOptions};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tracing::debug;

#[allow(dead_code)] // Only used by integration tests, not main binary
pub struct NullEditor;

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
