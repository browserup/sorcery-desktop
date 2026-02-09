use anyhow::{Context, Result};
use std::path::PathBuf;

const APP_CONFIG_DIR: &str = "sorcery-desktop";

pub fn canonical_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("Could not find config directory")?;
    let sorcery_dir = config_dir.join(APP_CONFIG_DIR);
    std::fs::create_dir_all(&sorcery_dir)
        .context("Failed to create sorcery-desktop config directory")?;
    Ok(sorcery_dir)
}
