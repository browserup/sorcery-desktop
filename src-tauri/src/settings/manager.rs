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

        // Only scan for repo directories on first run (no settings file).
        // When settings exist, load() will replace this with file contents anyway.
        let initial = if config_path.exists() {
            Settings::default()
        } else {
            Settings::with_detected_workspaces_folder()
        };

        Ok(Self {
            config_path,
            settings: Arc::new(RwLock::new(initial)),
        })
    }

    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not find config directory")?;

        let sorcery_dir = config_dir.join("sorcery-desktop");
        std::fs::create_dir_all(&sorcery_dir)
            .context("Failed to create sorcery-desktop config directory")?;

        Ok(sorcery_dir.join("settings.yaml"))
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub async fn load(&self) -> Result<()> {
        if !self.config_path.exists() {
            info!("No existing settings file found, using defaults");
            return Ok(());
        }

        let contents = tokio::fs::read_to_string(&self.config_path)
            .await
            .context("Failed to read settings file")?;

        let mut settings: Settings =
            serde_yaml::from_str(&contents).context("Failed to parse YAML settings")?;

        self.normalize_workspace_paths(&mut settings).await?;

        let mut current = self.settings.write().await;
        *current = settings;

        info!("Settings loaded from {:?}", self.config_path);
        Ok(())
    }

    pub async fn save(&self, mut settings: Settings) -> Result<()> {
        let yaml_string =
            serde_yaml::to_string(&settings).context("Failed to serialize settings to YAML")?;

        tokio::fs::write(&self.config_path, yaml_string)
            .await
            .context("Failed to write settings file")?;

        // Normalize paths before storing in memory
        self.normalize_workspace_paths(&mut settings).await?;

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

        for workspace in &settings.workspaces {
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

    pub async fn allows_non_workspace_files(&self) -> bool {
        let settings = self.settings.read().await;
        settings.defaults.allow_non_workspace_files
    }

    pub async fn get_preferred_terminal(&self) -> String {
        let settings = self.settings.read().await;
        settings.defaults.preferred_terminal.clone()
    }

    async fn normalize_workspace_paths(&self, settings: &mut Settings) -> Result<()> {
        for workspace in &mut settings.workspaces {
            match Self::normalize_path(&workspace.path) {
                Ok(normalized) => {
                    workspace.normalized_path = Some(normalized);
                }
                Err(e) => {
                    warn!("Failed to normalize path '{}': {}", workspace.path, e);
                }
            }
        }

        // Validate workspace names
        Self::validate_workspace_names(settings);

        Ok(())
    }

    /// Validate workspace names and warn about those containing dots.
    /// Workspace names with dots are ambiguous with provider hostnames (e.g., github.com).
    /// Use ?workspace= escape hatch in URLs to reference dot-containing workspace names.
    fn validate_workspace_names(settings: &Settings) {
        for workspace in &settings.workspaces {
            let name = workspace
                .name
                .as_ref()
                .map(|n| n.as_str())
                .unwrap_or_else(|| {
                    // Derive name from path (last component)
                    std::path::Path::new(&workspace.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                });

            if name.contains('.') {
                warn!(
                    "Workspace '{}' contains a dot in its name. \
                     This may be confused with provider hostnames (e.g., github.com). \
                     Consider renaming, or use ?workspace={} in URLs to reference it explicitly.",
                    name, name
                );
            }
        }
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
