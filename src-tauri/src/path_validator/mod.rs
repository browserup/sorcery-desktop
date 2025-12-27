use crate::settings::SettingsManager;
use anyhow::{bail, Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::Arc;

static SUSPICIOUS_PATTERNS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(\.\./|\.\.\\|~|//|[\x00-\x1f]|[<>|?*;'`$&(){}\[\]"])"#).unwrap());

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

    pub async fn validate_any(&self, path_str: &str) -> Result<PathBuf> {
        tracing::debug!("Validating path (file or directory): {}", path_str);

        Self::sanitize(path_str).context("Sanitize failed")?;
        tracing::debug!("Path sanitized");

        let normalized = self.normalize(path_str).context("Normalize failed")?;
        tracing::debug!("Path normalized to: {}", normalized.display());

        self.verify_exists_any(&normalized)
            .context("Verification failed")?;
        tracing::debug!("Path exists verified");

        Ok(normalized)
    }

    fn sanitize(path: &str) -> Result<()> {
        if path.is_empty() {
            bail!("Path cannot be empty");
        }

        if path.len() > 4096 {
            bail!("Path too long (max 4096 characters)");
        }

        if SUSPICIOUS_PATTERNS.is_match(path) {
            bail!("Path contains suspicious patterns");
        }

        if path.contains("\\\\") {
            #[cfg(target_os = "windows")]
            {
                if !path.starts_with("\\\\") || path[2..].contains("\\\\") {
                    bail!("Path contains invalid backslash sequences");
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                bail!("Path contains invalid backslash sequences");
            }
        }

        #[cfg(target_os = "windows")]
        {
            let colon_count = path.chars().filter(|c| *c == ':').count();
            if colon_count > 1 {
                bail!("Path contains invalid ':' characters");
            }
            if let Some(idx) = path.find(':') {
                let drive_char = path.chars().next().unwrap_or_default();
                let next_char = path.chars().nth(idx + 1);
                let is_drive = idx == 1
                    && drive_char.is_ascii_alphabetic()
                    && matches!(next_char, Some('\\') | Some('/'));
                if !is_drive {
                    bail!("Path contains invalid ':' characters");
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if path.contains(':') {
                bail!("Path contains ':' characters");
            }
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

        let canonical = path
            .canonicalize()
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

    fn verify_exists_any(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            bail!("Path does not exist: {}", path.display());
        }

        if !path.is_file() && !path.is_dir() {
            bail!("Path is neither a file nor a directory: {}", path.display());
        }

        Ok(())
    }

    // TODO: Implement workspace-based security checks per ai/4-path-validation.md
    #[allow(dead_code)]
    async fn check_workspace_membership(&self, path: &Path) -> Result<()> {
        let settings = self.settings_manager.get().await;

        if settings.workspaces.is_empty() {
            return Ok(());
        }

        for workspace in &settings.workspaces {
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

        let canonical = path
            .canonicalize()
            .context("Failed to resolve workspace path (directory may not exist)")?;

        if !canonical.is_dir() {
            bail!("Workspace path must be a directory");
        }

        Ok(canonical)
    }
}

#[cfg(test)]
mod tests {
    use super::PathValidator;

    #[test]
    fn windows_drive_paths_allowed_on_windows() {
        if cfg!(target_os = "windows") {
            assert!(PathValidator::sanitize(r"C:\Users\example").is_ok());
        } else {
            assert!(PathValidator::sanitize(r"C:\Users\example").is_err());
        }
    }

    #[test]
    fn colon_in_paths_rejected_elsewhere() {
        if cfg!(not(target_os = "windows")) {
            assert!(PathValidator::sanitize("/tmp/file:bad").is_err());
        }
    }

    #[test]
    fn rejects_shell_metacharacters() {
        assert!(PathValidator::sanitize("/tmp/file;rm -rf /").is_err(), "semicolon");
        assert!(PathValidator::sanitize("/tmp/file'test.txt").is_err(), "single quote");
        assert!(PathValidator::sanitize("/tmp/file`whoami`.txt").is_err(), "backtick");
        assert!(PathValidator::sanitize("/tmp/$(curl x).txt").is_err(), "dollar sign");
        assert!(PathValidator::sanitize("/tmp/file&bg.txt").is_err(), "ampersand");
        assert!(PathValidator::sanitize("/tmp/file(sub).txt").is_err(), "open paren");
        assert!(PathValidator::sanitize("/tmp/file).txt").is_err(), "close paren");
        assert!(PathValidator::sanitize("/tmp/file{a,b}.txt").is_err(), "open brace");
        assert!(PathValidator::sanitize("/tmp/file}.txt").is_err(), "close brace");
        assert!(PathValidator::sanitize("/tmp/file[0].txt").is_err(), "open bracket");
        assert!(PathValidator::sanitize("/tmp/file].txt").is_err(), "close bracket");
        assert!(PathValidator::sanitize("/tmp/file\"quoted\".txt").is_err(), "double quote");
    }

    #[test]
    fn allows_safe_filenames() {
        assert!(PathValidator::sanitize("/tmp/file.txt").is_ok());
        assert!(PathValidator::sanitize("/tmp/my-file_name.rs").is_ok());
        assert!(PathValidator::sanitize("/tmp/CamelCase.java").is_ok());
        assert!(PathValidator::sanitize("/tmp/file.with.dots.md").is_ok());
        assert!(PathValidator::sanitize("/tmp/file 123.txt").is_ok(), "spaces allowed");
        assert!(PathValidator::sanitize("/tmp/file@domain.txt").is_ok(), "at sign allowed");
        assert!(PathValidator::sanitize("/tmp/file#tag.txt").is_ok(), "hash allowed");
        assert!(PathValidator::sanitize("/tmp/file%20encoded.txt").is_ok(), "percent allowed");
        assert!(PathValidator::sanitize("/tmp/file+plus.txt").is_ok(), "plus allowed");
        assert!(PathValidator::sanitize("/tmp/file=equals.txt").is_ok(), "equals allowed");
    }
}
