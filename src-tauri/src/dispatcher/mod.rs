use crate::editors::{EditorRegistry, OpenOptions};
use crate::path_validator::PathValidator;
use crate::settings::SettingsManager;
use crate::tracker::ActiveEditorTracker;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
        new_window: bool,
        editor_hint: Option<String>,
    ) -> Result<()> {
        info!("open() called with path: {}, line: {:?}, editor_hint: {:?}", path_str, line, editor_hint);

        let validated_path = self.path_validator.validate(path_str).await
            .context("Path validation failed")?;

        info!("Path validated: {}", validated_path.display());

        let editor_id = self.determine_editor(&validated_path, editor_hint).await?;
        info!("Determined editor: {}", editor_id);

        let manager = self.editor_registry.get(&editor_id)
            .ok_or_else(|| anyhow::anyhow!("Editor '{}' not found in registry", editor_id))?;

        let is_installed = manager.is_installed().await;
        info!("Editor '{}' is_installed: {}", editor_id, is_installed);

        if !is_installed {
            return Err(anyhow::anyhow!("Editor '{}' is not installed", editor_id));
        }

        let options = OpenOptions {
            line,
            column: None,
            new_window,
        };

        info!("Calling manager.open() for {}", editor_id);
        manager.open(&validated_path, &options).await
            .map_err(|e| anyhow::anyhow!("Failed to open in {}: {}", editor_id, e))?;

        info!("Successfully opened {} in {}", validated_path.display(), editor_id);
        Ok(())
    }

    async fn determine_editor(
        &self,
        path: &Path,
        editor_hint: Option<String>,
    ) -> Result<String> {
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

        if let Some(workspace) = self.settings_manager.get_workspace_for_path(path).await {
            debug!("Using workspace editor: {} for path {:?}", workspace.editor, path);
            return Ok(workspace.editor);
        }

        let default_editor = self.settings_manager.get_default_editor().await;
        debug!("Using default editor: {}", default_editor);
        Ok(default_editor)
    }

    // TODO: Implement deep link parsing per ai/11-deep-link-handler.md
    #[allow(dead_code)]
    pub async fn parse_deep_link(&self, url: &str) -> Result<DeepLinkRequest> {
        let parsed_url = url::Url::parse(url)
            .context("Failed to parse deep link URL")?;

        if parsed_url.scheme() != "hypredit" {
            return Err(anyhow::anyhow!("Invalid scheme: expected 'hypredit', got '{}'", parsed_url.scheme()));
        }

        let host = parsed_url.host_str()
            .ok_or_else(|| anyhow::anyhow!("Deep link missing host (workspace name)"))?;

        let mut path_segments: Vec<&str> = parsed_url.path_segments()
            .ok_or_else(|| anyhow::anyhow!("Deep link has no path"))?
            .collect();

        if path_segments.is_empty() {
            return Err(anyhow::anyhow!("Deep link has empty path"));
        }

        let file_part = path_segments.pop().unwrap();

        let (file_name, line) = if let Some(colon_pos) = file_part.rfind(':') {
            let (name, line_str) = file_part.split_at(colon_pos);
            let line_num = line_str[1..].parse::<usize>().ok();
            (name, line_num)
        } else {
            (file_part, None)
        };

        let mut full_path = PathBuf::new();
        for segment in &path_segments {
            full_path.push(segment);
        }
        full_path.push(file_name);

        let workspace_name = host.to_string();

        let settings = self.settings_manager.get().await;
        let workspace = settings.repos.iter()
            .find(|w| w.name.as_deref() == Some(&workspace_name))
            .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace_name))?;

        let workspace_root = workspace.normalized_path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Workspace '{}' has no normalized path", workspace_name))?;

        let absolute_path = workspace_root.join(&full_path);

        Ok(DeepLinkRequest {
            path: absolute_path.to_string_lossy().to_string(),
            line,
            editor: Some(workspace.editor.clone()),
        })
    }
}

// TODO: Implement deep link parsing per ai/11-deep-link-handler.md
#[allow(dead_code)]
#[derive(Debug)]
pub struct DeepLinkRequest {
    pub path: String,
    pub line: Option<usize>,
    pub editor: Option<String>,
}
