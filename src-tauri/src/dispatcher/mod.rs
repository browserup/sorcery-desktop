use crate::editors::{EditorRegistry, OpenOptions};
use crate::git_command_log::GIT_COMMAND_LOG;
use crate::path_validator::PathValidator;
use crate::settings::SettingsManager;
use crate::tracker::ActiveEditorTracker;
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

pub struct EditorDispatcher {
    settings_manager: Arc<SettingsManager>,
    path_validator: Arc<PathValidator>,
    editor_registry: Arc<EditorRegistry>,
    tracker: Arc<ActiveEditorTracker>,
}

impl EditorDispatcher {
    pub fn new(
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

        let validated_path = self
            .path_validator
            .validate_any(path_str)
            .await
            .context("Path validation failed")?;

        let is_directory = validated_path.is_dir();
        info!(
            "Path validated: {} (is_directory: {})",
            validated_path.display(),
            is_directory
        );

        let editor_id = self.determine_editor(&validated_path, editor_hint).await?;
        info!("Determined editor: {}", editor_id);

        let manager = self
            .editor_registry
            .get(&editor_id)
            .ok_or_else(|| anyhow::anyhow!("Editor '{}' not found in registry", editor_id))?;

        if is_directory && !manager.supports_folders() {
            let duration = start.elapsed();
            GIT_COMMAND_LOG.log_editor_launch(
                &editor_id,
                path_str,
                line,
                false,
                Some(&format!(
                    "Editor '{}' does not support opening folders",
                    editor_id
                )),
                duration,
            );
            return Err(anyhow::anyhow!(
                "Editor '{}' does not support opening folders. Try using a different editor like VS Code or a JetBrains IDE.",
                manager.display_name()
            ));
        }

        let is_installed = manager.is_installed().await;
        info!("Editor '{}' is_installed: {}", editor_id, is_installed);

        if !is_installed {
            let duration = start.elapsed();
            GIT_COMMAND_LOG.log_editor_launch(
                &editor_id,
                path_str,
                line,
                false,
                Some(&format!("Editor '{}' is not installed", editor_id)),
                duration,
            );
            return Err(anyhow::anyhow!("Editor '{}' is not installed", editor_id));
        }

        let terminal_preference = self.settings_manager.get_preferred_terminal().await;

        let options = OpenOptions {
            line: if is_directory { None } else { line },
            column: if is_directory { None } else { column },
            new_window,
            terminal_preference: Some(terminal_preference),
        };

        info!("Calling manager.open() for {}", editor_id);
        let result = manager.open(&validated_path, &options).await;

        let duration = start.elapsed();

        match &result {
            Ok(_) => {
                info!(
                    "Successfully opened {} in {}",
                    validated_path.display(),
                    editor_id
                );
                GIT_COMMAND_LOG.log_editor_launch(&editor_id, path_str, line, true, None, duration);
            }
            Err(e) => {
                GIT_COMMAND_LOG.log_editor_launch(
                    &editor_id,
                    path_str,
                    line,
                    false,
                    Some(&e.to_string()),
                    duration,
                );
            }
        }

        result.map_err(|e| anyhow::anyhow!("Failed to open in {}: {}", editor_id, e))
    }

    async fn determine_editor(&self, path: &Path, editor_hint: Option<String>) -> Result<String> {
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

        let in_workspace =
            if let Some(workspace) = self.settings_manager.get_workspace_for_path(path).await {
                if !workspace.editor.is_empty() {
                    debug!(
                        "Using workspace editor: {} for path {:?}",
                        workspace.editor, path
                    );
                    return Ok(workspace.editor);
                }
                debug!("Workspace editor is empty, falling back to default");
                true
            } else {
                false
            };

        if !in_workspace && !self.settings_manager.allows_non_workspace_files().await {
            return Err(anyhow::anyhow!(
                "File is not in any configured workspace and opening non-workspace files is disabled. \
                Enable 'Allow opening files outside of configured workspaces' in settings to open this file."
            ));
        }

        let default_editor = self.settings_manager.get_default_editor().await;
        debug!("Using default editor: {}", default_editor);
        Ok(default_editor)
    }
}
