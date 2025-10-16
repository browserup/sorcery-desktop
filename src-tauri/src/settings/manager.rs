use super::models::{Settings, WorkspaceConfig};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct SettingsManager {
    config_path: PathBuf,
    settings: Arc<RwLock<Settings>>,
}

impl SettingsManager {
    pub async fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        Ok(Self {
            config_path,
            settings: Arc::new(RwLock::new(Settings::default())),
        })
    }

    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?;

        let hypredit_dir = config_dir.join("hypredit");
        std::fs::create_dir_all(&hypredit_dir)
            .context("Failed to create hypredit config directory")?;

        Ok(hypredit_dir.join("settings.yaml"))
    }

    pub async fn load(&self) -> Result<()> {
        if !self.config_path.exists() {
            info!("No existing settings file found, using defaults");
            return Ok(());
        }

        let contents = tokio::fs::read_to_string(&self.config_path).await
            .context("Failed to read settings file")?;

        let mut settings: Settings = serde_yaml::from_str(&contents)
            .context("Failed to parse YAML settings")?;

        self.normalize_workspace_paths(&mut settings).await?;

        let mut current = self.settings.write().await;
        *current = settings;

        info!("Settings loaded from {:?}", self.config_path);
        Ok(())
    }

    pub async fn save(&self, settings: Settings) -> Result<()> {
        let yaml_string = serde_yaml::to_string(&settings)
            .context("Failed to serialize settings to YAML")?;

        tokio::fs::write(&self.config_path, yaml_string).await
            .context("Failed to write settings file")?;

        let mut current = self.settings.write().await;
        *current = settings;

        info!("Settings saved to {:?}", self.config_path);
        Ok(())
    }

    pub async fn get(&self) -> Settings {
        self.settings.read().await.clone()
    }

    pub async fn get_workspace_for_path(&self, path: &Path) -> Option<WorkspaceConfig> {
        let settings = self.settings.read().await;

        for workspace in &settings.repos {
            if let Some(normalized) = &workspace.normalized_path {
                if path.starts_with(normalized) {
                    return Some(workspace.clone());
                }
            }
        }

        None
    }

    pub async fn get_default_editor(&self) -> String {
        let settings = self.settings.read().await;
        settings.defaults.editor.clone()
    }

    async fn normalize_workspace_paths(&self, settings: &mut Settings) -> Result<()> {
        for workspace in &mut settings.repos {
            match Self::normalize_path(&workspace.path) {
                Ok(normalized) => {
                    workspace.normalized_path = Some(normalized);
                }
                Err(e) => {
                    warn!("Failed to normalize path '{}': {}", workspace.path, e);
                }
            }
        }
        Ok(())
    }

    fn normalize_path(path: &str) -> Result<PathBuf> {
        let expanded = shellexpand::tilde(path);
        let path = Path::new(expanded.as_ref());

        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            std::env::current_dir()
                .context("Failed to get current directory")?
                .join(path)
                .canonicalize()
                .context("Failed to canonicalize path")
        }
    }
}
