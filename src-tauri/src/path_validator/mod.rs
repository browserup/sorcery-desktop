use anyhow::{bail, Context, Result};
use regex::Regex;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

// Blocks shell/HTML metacharacters that could enable command or DOM injection.
// Parentheses and square brackets are intentionally omitted—they're common in
// macOS-generated directory names and Git repo folders and are safe because we
// never invoke a shell when launching editors.
static SUSPICIOUS_PATTERNS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(\.\./|\.\.\\|//|[\x00-\x1f]|[<>|?*;'`$&{}"]|#)"#)
        .expect("SUSPICIOUS_PATTERNS regex is valid")
});

// Binary formats that can't be meaningfully edited as text.
// Script files (.sh, .bat, .ps1, etc.) are intentionally NOT blocked -
// they're source code and opening them in an editor doesn't execute them.
static DANGEROUS_EXTENSIONS: &[&str] = &[".exe", ".app", ".dmg"];

#[derive(Default)]
pub struct PathValidator;

impl PathValidator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    #[allow(clippy::unused_async, clippy::missing_errors_doc)]
    pub async fn validate_any(&self, path_str: &str) -> Result<PathBuf> {
        tracing::debug!("Validating path (file or directory): {}", path_str);

        Self::sanitize(path_str).context("Sanitize failed")?;
        tracing::debug!("Path sanitized");

        let normalized = Self::normalize(path_str).context("Normalize failed")?;
        tracing::debug!("Path normalized to: {}", normalized.display());

        Self::verify_exists_any(&normalized).context("Verification failed")?;
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

        if path.contains('~') && !path.starts_with('~') {
            bail!("Path contains invalid '~' characters");
        }

        let expanded: Cow<'_, str> = if path.starts_with('~') {
            Cow::Owned(shellexpand::tilde(path).into_owned())
        } else {
            Cow::Borrowed(path)
        };
        let input = expanded.as_ref();

        #[cfg(target_os = "windows")]
        {
            if input.starts_with("//") || input.starts_with("\\\\") {
                // UNC paths trigger automatic SMB auth and can leak NTLM credentials.
                bail!("Network paths (UNC) are not supported for security reasons");
            }
        }

        let normalized_for_scan: Cow<'_, str> = if input.contains("//") {
            Cow::Owned(Self::collapse_forward_slashes(input))
        } else {
            Cow::Borrowed(input)
        };

        if SUSPICIOUS_PATTERNS.is_match(normalized_for_scan.as_ref()) {
            bail!("Path contains suspicious patterns");
        }

        if input.contains("\\\\") {
            #[cfg(target_os = "windows")]
            {
                // Block UNC paths (\\server\share\...) - they trigger automatic SMB
                // authentication which could leak NTLM credentials to attacker servers
                if input.starts_with("\\\\") {
                    bail!("Network paths (UNC) are not supported for security reasons");
                }
                bail!("Path contains invalid backslash sequences");
            }
            #[cfg(not(target_os = "windows"))]
            {
                bail!("Path contains invalid backslash sequences");
            }
        }

        #[cfg(target_os = "windows")]
        {
            let colon_count = input.chars().filter(|c| *c == ':').count();
            if colon_count > 1 {
                bail!("Path contains invalid ':' characters");
            }
            if let Some(idx) = input.find(':') {
                let drive_char = input.chars().next().unwrap_or_default();
                let next_char = input.chars().nth(idx + 1);
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
            if input.contains(':') {
                bail!("Path contains ':' characters");
            }
        }

        for ext in DANGEROUS_EXTENSIONS {
            if input.to_lowercase().ends_with(ext) {
                bail!("Opening executable files is not allowed");
            }
        }

        Ok(())
    }

    fn collapse_forward_slashes(input: &str) -> String {
        let mut collapsed = String::with_capacity(input.len());
        let mut previous_was_slash = false;

        for character in input.chars() {
            if character == '/' {
                if previous_was_slash {
                    continue;
                }
                previous_was_slash = true;
            } else {
                previous_was_slash = false;
            }
            collapsed.push(character);
        }

        collapsed
    }

    fn normalize(path: &str) -> Result<PathBuf> {
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

    fn verify_exists_any(path: &Path) -> Result<()> {
        if !path.exists() {
            bail!("Path does not exist: {}", path.display());
        }

        if !path.is_file() && !path.is_dir() {
            bail!("Path is neither a file nor a directory: {}", path.display());
        }

        Ok(())
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
    fn rejects_unc_paths_on_windows() {
        // UNC paths could leak NTLM credentials via automatic SMB authentication
        if cfg!(target_os = "windows") {
            assert!(
                PathValidator::sanitize(r"\\server\share\file.txt").is_err(),
                "UNC path should be rejected"
            );
            assert!(
                PathValidator::sanitize(r"\\attacker.com\share\secrets").is_err(),
                "Attacker UNC path should be rejected"
            );
            assert!(
                PathValidator::sanitize("//server/share/file.txt").is_err(),
                "Forward-slash UNC path should be rejected"
            );
        }
    }

    #[test]
    fn allows_redundant_forward_slashes_on_unix_like_paths() {
        if cfg!(not(target_os = "windows")) {
            assert!(
                PathValidator::sanitize("//private/var/folders/test.rs").is_ok(),
                "leading redundant slash should normalize"
            );
            assert!(
                PathValidator::sanitize("/tmp//nested///file.rs").is_ok(),
                "internal redundant slashes should normalize"
            );
        }
    }

    #[test]
    fn rejects_shell_metacharacters() {
        assert!(
            PathValidator::sanitize("/tmp/file;rm -rf /").is_err(),
            "semicolon"
        );
        assert!(
            PathValidator::sanitize("/tmp/file'test.txt").is_err(),
            "single quote"
        );
        assert!(
            PathValidator::sanitize("/tmp/file`whoami`.txt").is_err(),
            "backtick"
        );
        assert!(
            PathValidator::sanitize("/tmp/$(curl x).txt").is_err(),
            "dollar sign"
        );
        assert!(
            PathValidator::sanitize("/tmp/file&bg.txt").is_err(),
            "ampersand"
        );
        assert!(
            PathValidator::sanitize("/tmp/file{a,b}.txt").is_err(),
            "open brace"
        );
        assert!(
            PathValidator::sanitize("/tmp/file}.txt").is_err(),
            "close brace"
        );
        assert!(
            PathValidator::sanitize("/tmp/file\"quoted\".txt").is_err(),
            "double quote"
        );
        assert!(
            PathValidator::sanitize("/tmp/file#tag.txt").is_err(),
            "hash"
        );
    }

    #[test]
    fn allows_common_special_characters() {
        assert!(
            PathValidator::sanitize("/tmp/file(sub).txt").is_ok(),
            "open paren ok"
        );
        assert!(
            PathValidator::sanitize("/tmp/file).txt").is_ok(),
            "close paren ok"
        );
        assert!(
            PathValidator::sanitize("/tmp/file[0].txt").is_ok(),
            "open bracket ok"
        );
        assert!(
            PathValidator::sanitize("/tmp/file].txt").is_ok(),
            "close bracket ok"
        );
    }

    #[test]
    fn leading_tilde_supported_mid_path_rejected() {
        assert!(
            PathValidator::sanitize("~/code/file.txt").is_ok(),
            "leading tilde ok"
        );
        assert!(
            PathValidator::sanitize("/tmp/foo~bar.txt").is_err(),
            "mid-path tilde rejected"
        );
    }

    #[test]
    fn allows_safe_filenames() {
        assert!(PathValidator::sanitize("/tmp/file.txt").is_ok());
        assert!(PathValidator::sanitize("/tmp/my-file_name.rs").is_ok());
        assert!(PathValidator::sanitize("/tmp/CamelCase.java").is_ok());
        assert!(PathValidator::sanitize("/tmp/file.with.dots.md").is_ok());
        assert!(
            PathValidator::sanitize("/tmp/file 123.txt").is_ok(),
            "spaces allowed"
        );
        assert!(
            PathValidator::sanitize("/tmp/file@domain.txt").is_ok(),
            "at sign allowed"
        );
        assert!(
            PathValidator::sanitize("/tmp/file%20encoded.txt").is_ok(),
            "percent allowed"
        );
        assert!(
            PathValidator::sanitize("/tmp/file+plus.txt").is_ok(),
            "plus allowed"
        );
        assert!(
            PathValidator::sanitize("/tmp/file=equals.txt").is_ok(),
            "equals allowed"
        );
    }
}
