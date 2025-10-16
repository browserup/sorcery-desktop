use crate::settings::SettingsManager;
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use once_cell::sync::Lazy;

static SUSPICIOUS_PATTERNS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\.\./|\.\.\\|~|//|\\\\|[\x00-\x1f]|[<>:|?*])").unwrap()
});

static DANGEROUS_EXTENSIONS: &[&str] = &[
    ".exe", ".bat", ".cmd", ".sh", ".ps1", ".vbs", ".app", ".dmg",
];

pub struct PathValidator {
    settings_manager: Arc<SettingsManager>,
}

impl PathValidator {
    pub fn new(settings_manager: Arc<SettingsManager>) -> Self {
        Self { settings_manager }
    }

    pub async fn validate(&self, path_str: &str) -> Result<PathBuf> {
        tracing::debug!("Validating path: {}", path_str);

        self.sanitize(path_str)
            .context("Sanitize failed")?;
        tracing::debug!("Path sanitized");

        let normalized = self.normalize(path_str)
            .context("Normalize failed")?;
        tracing::debug!("Path normalized to: {}", normalized.display());

        self.verify_exists(&normalized)
            .context("Verification failed")?;
        tracing::debug!("Path exists verified");

        Ok(normalized)
    }

    fn sanitize(&self, path: &str) -> Result<()> {
        if path.is_empty() {
            bail!("Path cannot be empty");
        }

        if path.len() > 4096 {
            bail!("Path too long (max 4096 characters)");
        }

        if SUSPICIOUS_PATTERNS.is_match(path) {
            bail!("Path contains suspicious patterns");
        }

        for ext in DANGEROUS_EXTENSIONS {
            if path.to_lowercase().ends_with(ext) {
                bail!("Opening executable files is not allowed");
            }
        }

        Ok(())
    }

    fn normalize(&self, path: &str) -> Result<PathBuf> {
        let expanded = shellexpand::tilde(path);
        let path = Path::new(expanded.as_ref());

        if !path.is_absolute() {
            bail!("Path must be absolute");
        }

        let canonical = path.canonicalize()
            .context("Failed to resolve path (file may not exist)")?;

        #[cfg(target_os = "macos")]
        {
            let canonical_str = canonical.to_string_lossy();
            if canonical_str.starts_with("/private/") {
                if let Ok(stripped) = canonical.strip_prefix("/private") {
                    let mut absolute = PathBuf::from("/");
                    absolute.push(stripped);
                    return Ok(absolute);
                }
            }
        }

        Ok(canonical)
    }

    fn verify_exists(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            bail!("File does not exist: {}", path.display());
        }

        if !path.is_file() {
            bail!("Path is not a file: {}", path.display());
        }

        Ok(())
    }

    // TODO: Implement workspace-based security checks per ai/4-path-validation.md
    #[allow(dead_code)]
    async fn check_workspace_membership(&self, path: &Path) -> Result<()> {
        let settings = self.settings_manager.get().await;

        if settings.repos.is_empty() {
            return Ok(());
        }

        for workspace in &settings.repos {
            if let Some(normalized) = &workspace.normalized_path {
                if Self::is_under(path, normalized) {
                    return Ok(());
                }
            }
        }

        bail!(
            "File is not within any configured workspace: {}",
            path.display()
        );
    }

    #[allow(dead_code)]
    fn is_under(child: &Path, parent: &Path) -> bool {
        child.starts_with(parent)
    }

    // TODO: Implement workspace-based security checks per ai/4-path-validation.md
    #[allow(dead_code)]
    pub async fn validate_workspace_path(&self, path_str: &str) -> Result<PathBuf> {
        let expanded = shellexpand::tilde(path_str);
        let path = Path::new(expanded.as_ref());

        if !path.is_absolute() {
            bail!("Workspace path must be absolute");
        }

        let canonical = path.canonicalize()
            .context("Failed to resolve workspace path (directory may not exist)")?;

        if !canonical.is_dir() {
            bail!("Workspace path must be a directory");
        }

        Ok(canonical)
    }
}
