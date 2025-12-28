mod fs_signal;
mod git_signals;
mod models;
mod probe;
mod process;

pub use models::{WorkspaceActivity, WorkspaceMruData};

use crate::settings::SettingsManager;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use sysinfo::System;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

pub struct ActiveWorkspaceTracker {
    mru_data: Arc<RwLock<WorkspaceMruData>>,
    mru_path: PathBuf,
    settings_manager: Arc<SettingsManager>,
    system: Arc<RwLock<System>>,
}

impl ActiveWorkspaceTracker {
    pub fn new(settings_manager: Arc<SettingsManager>) -> Self {
        let mru_path = Self::get_mru_path()
            .unwrap_or_else(|_| PathBuf::from("/tmp/sorcery_desktop_workspace_mru.yaml"));

        Self {
            mru_data: Arc::new(RwLock::new(WorkspaceMruData::default())),
            mru_path,
            settings_manager,
            system: Arc::new(RwLock::new(System::new())),
        }
    }

    fn get_mru_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not find config directory")?;

        let sorcery_dir = config_dir.join("sorcery");
        std::fs::create_dir_all(&sorcery_dir)
            .context("Failed to create sorcery config directory")?;

        Ok(sorcery_dir.join("workspace_mru.yaml"))
    }

    pub async fn load(&self) -> Result<()> {
        if !self.mru_path.exists() {
            info!("No existing workspace MRU data found, starting fresh");
            return Ok(());
        }

        let contents = tokio::fs::read_to_string(&self.mru_path)
            .await
            .context("Failed to read workspace MRU file")?;

        let data: WorkspaceMruData =
            serde_yaml::from_str(&contents).context("Failed to parse YAML workspace MRU data")?;

        let mut current = self.mru_data.write().await;
        *current = data;

        info!("Workspace MRU data loaded from {:?}", self.mru_path);
        Ok(())
    }

    async fn save(&self) -> Result<()> {
        let data = self.mru_data.read().await.clone();

        let yaml_string =
            serde_yaml::to_string(&data).context("Failed to serialize workspace MRU data")?;

        tokio::fs::write(&self.mru_path, yaml_string)
            .await
            .context("Failed to write workspace MRU file")?;

        info!("Workspace MRU data saved to {:?}", self.mru_path);
        Ok(())
    }

    pub async fn start_polling(self: Arc<Self>) {
        info!("Starting workspace MRU tracking (60s interval)");

        let mut ticker = interval(Duration::from_secs(60));

        loop {
            ticker.tick().await;
            self.update_workspace_activity().await;
        }
    }

    async fn update_workspace_activity(&self) {
        let settings = self.settings_manager.get().await;

        {
            let mut sys = self.system.write().await;
            process::refresh_process_snapshot(&mut sys);
        }

        let sys = self.system.read().await;

        for workspace_config in &settings.workspaces {
            if let Some(workspace_path) = &workspace_config.normalized_path {
                let probe_result = probe::probe_workspace(workspace_path, &sys);

                if let Some(last_active) = probe_result.last_active {
                    let mut mru_data = self.mru_data.write().await;
                    mru_data
                        .workspaces
                        .insert(workspace_path.clone(), WorkspaceActivity { last_active });
                }
            }
        }

        if let Err(e) = self.save().await {
            warn!("Failed to save workspace MRU data: {}", e);
        }
    }

    pub async fn get_workspace_last_active(&self, workspace_path: &PathBuf) -> Option<SystemTime> {
        let data = self.mru_data.read().await;
        data.workspaces
            .get(workspace_path)
            .map(|activity| activity.last_active)
    }
}
