mod detector;

use crate::editors::EditorRegistry;
use crate::settings::LastSeenData;
use anyhow::{Context, Result};
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
}

impl ActiveEditorTracker {
    pub fn new(registry: Arc<EditorRegistry>) -> Self {
        let last_seen_path = Self::get_last_seen_path()
            .unwrap_or_else(|_| PathBuf::from("/tmp/sorcery_desktop_last_seen.yaml"));

        Self {
            last_seen: Arc::new(RwLock::new(LastSeenData::default())),
            last_seen_path,
            registry,
        }
    }

    fn get_last_seen_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not find config directory")?;

        let sorcery_dir = config_dir.join("sorcery");
        std::fs::create_dir_all(&sorcery_dir)
            .context("Failed to create sorcery config directory")?;

        Ok(sorcery_dir.join("last_seen.yaml"))
    }

    pub async fn load(&self) -> Result<()> {
        if !self.last_seen_path.exists() {
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

        tokio::fs::write(&self.last_seen_path, yaml_string)
            .await
            .context("Failed to write last_seen file")?;

        debug!("Last seen data saved to {:?}", self.last_seen_path);
        Ok(())
    }

    pub async fn start_polling(self: Arc<Self>) {
        info!("Starting active editor tracking (10s interval)");

        let mut ticker = interval(Duration::from_secs(10));

        loop {
            ticker.tick().await;
            self.update_last_seen().await;
        }
    }

    async fn update_last_seen(&self) {
        if let Some(editor_id) = detector::detect_active_editor().await {
            let timestamp = chrono::Utc::now().timestamp_millis();

            debug!("Detected active editor: {} at {}", editor_id, timestamp);

            {
                let mut last_seen = self.last_seen.write().await;
                last_seen.editors.insert(editor_id.clone(), timestamp);
                last_seen.most_recent = Some(editor_id);
            }

            if let Err(e) = self.save().await {
                warn!("Failed to save last_seen data: {}", e);
            }
        }
    }

    pub async fn get_last_seen_data(&self) -> LastSeenData {
        self.last_seen.read().await.clone()
    }

    pub async fn get_most_recent_editor(&self) -> Option<String> {
        self.last_seen.read().await.most_recent.clone()
    }
}
