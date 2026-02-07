use crate::editors::{EditorRegistry, OpenOptions};
use crate::git_command_log::GIT_COMMAND_LOG;
use crate::path_validator::PathValidator;
use crate::settings::{SettingsManager, WorkspaceConfig};
use crate::tracker::ActiveEditorTracker;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tracing::{debug, info};

pub struct EditorDispatcher {
    settings_manager: Arc<SettingsManager>,
    path_validator: Arc<PathValidator>,
    editor_registry: Arc<EditorRegistry>,
    tracker: Arc<ActiveEditorTracker>,
}

#[derive(Debug, Error)]
pub enum EditorDispatchError {
    #[error("Path validation failed for '{path}'")]
    PathValidation {
        path: String,
        #[source]
        source: anyhow::Error,
    },
    #[error("Editor '{editor_id}' not found in registry")]
    EditorNotFound { editor_id: String },
    #[error("Editor '{display_name}' does not support opening folders. Try using a different editor like VS Code or a JetBrains IDE.")]
    EditorDoesNotSupportFolders {
        editor_id: String,
        display_name: String,
    },
    #[error("Editor '{editor_id}' is not installed")]
    EditorNotInstalled { editor_id: String },
    #[error("File is not in any configured workspace and opening non-workspace files is disabled. Enable 'Allow opening files outside of configured workspaces' in settings to open this file.")]
    NonWorkspaceFilesDisabled,
    #[error("Failed to open in {editor_id}: {source}")]
    OpenFailed {
        editor_id: String,
        #[source]
        source: crate::editors::EditorError,
    },
}

type Result<T> = std::result::Result<T, EditorDispatchError>;

#[allow(clippy::missing_errors_doc)]
impl EditorDispatcher {
    #[must_use]
    pub const fn new(
        settings_manager: Arc<SettingsManager>,
        path_validator: Arc<PathValidator>,
        editor_registry: Arc<EditorRegistry>,
        tracker: Arc<ActiveEditorTracker>,
    ) -> Self {
        Self {
            settings_manager,
            path_validator,
            editor_registry,
            tracker,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub async fn open(
        &self,
        path_str: &str,
        line: Option<usize>,
        column: Option<usize>,
        new_window: bool,
        editor_hint: Option<String>,
    ) -> Result<()> {
        let start = Instant::now();
        info!(
            "open() called with path: {}, line: {:?}, column: {:?}, editor_hint: {:?}",
            path_str, line, column, editor_hint
        );

        let validated_path =
            self.path_validator
                .validate_any(path_str)
                .await
                .map_err(|source| EditorDispatchError::PathValidation {
                    path: path_str.to_string(),
                    source,
                })?;

        let is_directory = validated_path.is_dir();
        info!(
            "Path validated: {} (is_directory: {})",
            validated_path.display(),
            is_directory
        );

        let workspace = self
            .settings_manager
            .get_workspace_for_path(&validated_path)
            .await;
        let workspace_root = workspace.as_ref().and_then(|w| w.normalized_path.clone());
        info!(
            "Workspace lookup for {:?}: workspace_root={:?}",
            validated_path, workspace_root
        );

        let editor_id = self
            .determine_editor(&validated_path, editor_hint, workspace.as_ref())
            .await?;
        info!("Determined editor: {}", editor_id);

        let manager = self.editor_registry.get(&editor_id).ok_or_else(|| {
            EditorDispatchError::EditorNotFound {
                editor_id: editor_id.clone(),
            }
        })?;

        if is_directory && !manager.supports_folders() {
            let duration = start.elapsed();
            GIT_COMMAND_LOG.log_editor_launch(
                &editor_id,
                path_str,
                line,
                workspace_root.as_deref(),
                false,
                Some(&format!(
                    "Editor '{editor_id}' does not support opening folders"
                )),
                duration,
            );
            return Err(EditorDispatchError::EditorDoesNotSupportFolders {
                editor_id: editor_id.clone(),
                display_name: manager.display_name().to_string(),
            });
        }

        let is_installed = manager.is_installed().await;
        info!("Editor '{}' is_installed: {}", editor_id, is_installed);

        if !is_installed {
            let duration = start.elapsed();
            GIT_COMMAND_LOG.log_editor_launch(
                &editor_id,
                path_str,
                line,
                workspace_root.as_deref(),
                false,
                Some(&format!("Editor '{editor_id}' is not installed")),
                duration,
            );
            return Err(EditorDispatchError::EditorNotInstalled {
                editor_id: editor_id.clone(),
            });
        }

        let terminal_preference = self.settings_manager.get_preferred_terminal().await;

        let options = OpenOptions {
            line: if is_directory { None } else { line },
            column: if is_directory { None } else { column },
            new_window,
            terminal_preference: Some(terminal_preference),
            workspace_root,
        };

        info!("Calling manager.open() for {}", editor_id);
        let result = manager.open(&validated_path, &options).await;

        let duration = start.elapsed();

        match &result {
            Ok(()) => {
                info!(
                    "Successfully opened {} in {}",
                    validated_path.display(),
                    editor_id
                );
                GIT_COMMAND_LOG.log_editor_launch(
                    &editor_id,
                    path_str,
                    line,
                    options.workspace_root.as_deref(),
                    true,
                    None,
                    duration,
                );
            }
            Err(e) => {
                GIT_COMMAND_LOG.log_editor_launch(
                    &editor_id,
                    path_str,
                    line,
                    options.workspace_root.as_deref(),
                    false,
                    Some(&e.to_string()),
                    duration,
                );
            }
        }

        result.map_err(|source| EditorDispatchError::OpenFailed {
            editor_id: editor_id.clone(),
            source,
        })
    }

    async fn determine_editor(
        &self,
        path: &Path,
        editor_hint: Option<String>,
        workspace: Option<&WorkspaceConfig>,
    ) -> Result<String> {
        let in_workspace = workspace.is_some();

        if !in_workspace && !self.settings_manager.allows_non_workspace_files().await {
            return Err(EditorDispatchError::NonWorkspaceFilesDisabled);
        }

        if let Some(hint) = editor_hint {
            if hint == "most-recent" {
                if let Some(recent) = self.tracker.get_most_recent_editor().await {
                    debug!("Using most recent editor: {}", recent);
                    return Ok(recent);
                }
            } else {
                debug!("Using editor hint: {}", hint);
                return Ok(hint);
            }
        }

        if let Some(ws) = workspace {
            if !ws.editor.is_empty() {
                debug!("Using workspace editor: {} for path {:?}", ws.editor, path);
                return Ok(ws.editor.clone());
            }
            debug!("Workspace editor is empty, falling back to default");
        }

        let default_editor = self.settings_manager.get_default_editor().await;
        debug!("Using default editor: {}", default_editor);
        Ok(default_editor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_validator::PathValidator;
    use crate::settings::Settings;
    use tempfile::TempDir;

    async fn build_dispatcher(allow_non_workspace_files: bool) -> EditorDispatcher {
        let temp_dir = TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let settings_manager = Arc::new(
            SettingsManager::new_with_path(settings_path)
                .await
                .expect("settings manager"),
        );

        let mut settings = Settings::default();
        settings.defaults.allow_non_workspace_files = allow_non_workspace_files;
        settings_manager
            .save(settings)
            .await
            .expect("save settings");

        let path_validator = Arc::new(PathValidator::new());
        let editor_registry = Arc::new(EditorRegistry::new());
        let tracker = Arc::new(ActiveEditorTracker::new(Arc::clone(&editor_registry)));

        EditorDispatcher::new(settings_manager, path_validator, editor_registry, tracker)
    }

    #[tokio::test]
    async fn editor_hint_does_not_bypass_non_workspace_policy() {
        let dispatcher = build_dispatcher(false).await;

        let result = dispatcher
            .determine_editor(Path::new("/tmp/file.rs"), Some("vscode".to_string()), None)
            .await;

        assert!(matches!(
            result,
            Err(EditorDispatchError::NonWorkspaceFilesDisabled)
        ));
    }

    #[tokio::test]
    async fn editor_hint_is_used_when_non_workspace_allowed() {
        let dispatcher = build_dispatcher(true).await;

        let result = dispatcher
            .determine_editor(Path::new("/tmp/file.rs"), Some("vscode".to_string()), None)
            .await
            .expect("editor should resolve");

        assert_eq!(result, "vscode");
    }
}
