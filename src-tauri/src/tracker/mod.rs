mod detector;

use crate::editors::EditorRegistry;
use crate::settings::{LastSeenData, SettingsManager};
use crate::workspace_mru::ActiveWorkspaceTracker;
use anyhow::{Context, Result};
use serde_yaml_ng as serde_yaml;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

pub struct ActiveEditorTracker {
    last_seen: Arc<RwLock<LastSeenData>>,
    last_seen_path: PathBuf,
    #[allow(dead_code)]
    registry: Arc<EditorRegistry>,
    settings_manager: Option<Arc<SettingsManager>>,
    workspace_tracker: Option<Arc<ActiveWorkspaceTracker>>,
}

#[allow(clippy::missing_errors_doc, clippy::significant_drop_tightening)]
impl ActiveEditorTracker {
    pub fn new(registry: Arc<EditorRegistry>) -> Self {
        let last_seen_path = Self::get_last_seen_path()
            .unwrap_or_else(|_| PathBuf::from("/tmp/sorcery_last_seen.yaml"));

        Self {
            last_seen: Arc::new(RwLock::new(LastSeenData::default())),
            last_seen_path,
            registry,
            settings_manager: None,
            workspace_tracker: None,
        }
    }

    #[must_use]
    pub fn with_workspace_tracking(
        mut self,
        settings_manager: Arc<SettingsManager>,
        workspace_tracker: Arc<ActiveWorkspaceTracker>,
    ) -> Self {
        self.settings_manager = Some(settings_manager);
        self.workspace_tracker = Some(workspace_tracker);
        self
    }

    fn get_last_seen_path() -> Result<PathBuf> {
        Ok(crate::config_paths::canonical_config_dir()?.join("last_seen.yaml"))
    }

    pub async fn load(&self) -> Result<()> {
        if !tokio::fs::try_exists(&self.last_seen_path)
            .await
            .unwrap_or(false)
        {
            info!("No existing last_seen data found, starting fresh");
            return Ok(());
        }

        let contents = tokio::fs::read_to_string(&self.last_seen_path)
            .await
            .context("Failed to read last_seen file")?;

        let data: LastSeenData =
            serde_yaml::from_str(&contents).context("Failed to parse YAML last_seen data")?;

        let mut current = self.last_seen.write().await;
        *current = data;

        info!("Last seen data loaded from {:?}", self.last_seen_path);
        Ok(())
    }

    async fn save(&self) -> Result<()> {
        let data = self.last_seen.read().await.clone();

        let yaml_string =
            serde_yaml::to_string(&data).context("Failed to serialize last_seen data to YAML")?;

        // Atomic write: write to temp file, then rename
        let temp_path = self.last_seen_path.with_extension("yaml.tmp");
        tokio::fs::write(&temp_path, &yaml_string)
            .await
            .context("Failed to write temporary last_seen file")?;

        tokio::fs::rename(&temp_path, &self.last_seen_path)
            .await
            .context("Failed to rename temporary last_seen file")?;

        debug!("Last seen data saved to {:?}", self.last_seen_path);
        Ok(())
    }

    pub async fn start_polling(self: Arc<Self>) {
        info!("Starting active editor tracking (15s interval)");

        let mut ticker = interval(Duration::from_secs(15));

        loop {
            ticker.tick().await;
            self.update_last_seen().await;
        }
    }

    async fn update_last_seen(&self) {
        let detection = detector::detect_active_editor().await;

        if let Some(ref editor_id) = detection.editor_id {
            let timestamp = chrono::Utc::now().timestamp_millis();

            let changed = {
                let last_seen = self.last_seen.read().await;
                last_seen.most_recent.as_ref() != Some(editor_id)
            };

            if changed {
                debug!("Detected active editor: {} at {}", editor_id, timestamp);

                {
                    let mut last_seen = self.last_seen.write().await;
                    last_seen.editors.insert(editor_id.clone(), timestamp);
                    last_seen.most_recent = Some(editor_id.clone());
                }

                if let Err(e) = self.save().await {
                    warn!("Failed to save last_seen data: {}", e);
                }
            }
        }

        if let Some(ref title) = detection.window_title {
            if let Some(ws_path) = self.extract_workspace_from_title(title).await {
                if let Some(ref tracker) = self.workspace_tracker {
                    tracker.record_workspace_seen(&ws_path).await;
                }
            }
        }
    }

    async fn extract_workspace_from_title(&self, title: &str) -> Option<PathBuf> {
        let settings_manager = self.settings_manager.as_ref()?;
        let title_lower = title.to_lowercase();

        for ws in settings_manager.get_workspaces().await {
            let workspace_name = crate::settings::identity::derive_workspace_name(&ws);
            if title_lower.contains(&workspace_name.to_lowercase()) {
                return ws.normalized_path.clone();
            }
        }
        None
    }

    pub async fn get_last_seen_data(&self) -> LastSeenData {
        self.last_seen.read().await.clone()
    }

    pub async fn get_most_recent_editor(&self) -> Option<String> {
        self.last_seen.read().await.most_recent.clone()
    }
}
