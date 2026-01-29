pub mod git_signals;
mod models;

pub use models::{WorkspaceActivity, WorkspaceMruData};

use crate::settings::SettingsManager;
use anyhow::{Context, Result};
use serde_yaml_ng as serde_yaml;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub struct ActiveWorkspaceTracker {
    mru_data: Arc<RwLock<WorkspaceMruData>>,
    mru_path: PathBuf,
    #[allow(dead_code)]
    settings_manager: Arc<SettingsManager>,
}

impl ActiveWorkspaceTracker {
    pub fn new(settings_manager: Arc<SettingsManager>) -> Self {
        let mru_path = Self::get_mru_path()
            .unwrap_or_else(|_| PathBuf::from("/tmp/sorcery_desktop_workspace_mru.yaml"));

        Self {
            mru_data: Arc::new(RwLock::new(WorkspaceMruData::default())),
            mru_path,
            settings_manager,
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
        if !tokio::fs::try_exists(&self.mru_path).await.unwrap_or(false) {
            info!("No existing workspace MRU data found, starting fresh");
            return Ok(());
        }

        let contents = tokio::fs::read_to_string(&self.mru_path)
            .await
            .context("Failed to read workspace MRU file")?;

        let data: WorkspaceMruData = match serde_yaml::from_str(&contents) {
            Ok(data) => data,
            Err(e) => {
                warn!(
                    "Failed to parse workspace MRU data: {}. File contents ({} bytes): {:?}",
                    e,
                    contents.len(),
                    if contents.len() > 200 {
                        format!("{}...[truncated]", &contents[..200])
                    } else {
                        contents.clone()
                    }
                );

                // Backup corrupted file for debugging
                let backup_path = self.mru_path.with_extension("yaml.corrupted");
                if let Err(backup_err) = tokio::fs::rename(&self.mru_path, &backup_path).await {
                    warn!("Failed to backup corrupted file: {}", backup_err);
                } else {
                    warn!("Corrupted file backed up to {:?}", backup_path);
                }

                info!("Starting fresh with empty workspace MRU data");
                return Ok(());
            }
        };

        let mut current = self.mru_data.write().await;
        *current = data;

        info!("Workspace MRU data loaded from {:?}", self.mru_path);
        Ok(())
    }

    async fn save(&self) -> Result<()> {
        let data = self.mru_data.read().await.clone();

        let yaml_string =
            serde_yaml::to_string(&data).context("Failed to serialize workspace MRU data")?;

        // Atomic write: write to temp file, then rename
        let temp_path = self.mru_path.with_extension("yaml.tmp");
        tokio::fs::write(&temp_path, &yaml_string)
            .await
            .context("Failed to write temporary workspace MRU file")?;

        tokio::fs::rename(&temp_path, &self.mru_path)
            .await
            .context("Failed to rename temporary workspace MRU file")?;

        debug!("Workspace MRU data saved to {:?}", self.mru_path);
        Ok(())
    }

    pub async fn record_workspace_seen(&self, workspace_path: &Path) {
        let mut data = self.mru_data.write().await;
        data.workspaces.insert(
            workspace_path.to_path_buf(),
            WorkspaceActivity {
                last_seen: SystemTime::now(),
            },
        );
        drop(data);
        if let Err(e) = self.save().await {
            warn!("Failed to save workspace MRU data: {}", e);
        }
    }

    pub fn get_folder_mtime(workspace_path: &Path) -> Option<SystemTime> {
        std::fs::metadata(workspace_path).ok()?.modified().ok()
    }

    pub async fn get_last_seen(&self, workspace_path: &Path) -> Option<SystemTime> {
        let data = self.mru_data.read().await;
        data.workspaces
            .get(workspace_path)
            .map(|activity| activity.last_seen)
    }

    pub async fn compute_effective_time(
        &self,
        workspace_path: &Path,
        include_reflog: bool,
    ) -> Option<SystemTime> {
        let last_seen = self.get_last_seen(workspace_path).await;
        let folder_mtime = Self::get_folder_mtime(workspace_path);
        let reflog_time = if include_reflog {
            git_signals::head_reflog_time(workspace_path)
        } else {
            None
        };
        [last_seen, folder_mtime, reflog_time]
            .into_iter()
            .flatten()
            .max()
    }
}
