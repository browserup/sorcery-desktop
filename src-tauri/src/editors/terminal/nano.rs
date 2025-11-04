use super::super::traits::{EditorError, EditorInstance, EditorManager, EditorResult, OpenOptions};
use super::terminal_detector::TerminalApp;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

pub struct NanoManager;

impl NanoManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for NanoManager {
    fn id(&self) -> &str {
        "nano"
    }

    fn display_name(&self) -> &str {
        "Nano"
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let candidates = vec![
                PathBuf::from("/opt/homebrew/bin/nano"),
                PathBuf::from("/usr/local/bin/nano"),
                PathBuf::from("/usr/bin/nano"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found nano at {:?}", path);
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let candidates = vec![
                PathBuf::from("/usr/bin/nano"),
                PathBuf::from("/usr/local/bin/nano"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found nano at {:?}", path);
                    return Some(path);
                }
            }
        }

        if let Ok(output) = Command::new("which").arg("nano").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(&path_str);
                    if path.exists() {
                        debug!("Found nano via which: {:?}", path);
                        return Some(path);
                    }
                }
            }
        }
        None
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()> {
        self.find_binary()
            .await
            .ok_or(EditorError::BinaryNotFound)?;

        let mut nano_args = vec![];
        if let Some(line) = options.line {
            nano_args.push(format!("+{}", line));
        }
        nano_args.push(format!("'{}'", path.display()));

        let nano_cmd = format!("nano {}", nano_args.join(" "));

        debug!("Opening in terminal with nano: {}", nano_cmd);

        let terminal_pref = options.terminal_preference.as_deref();
        if let Some(terminal) = TerminalApp::detect_installed_with_preference(terminal_pref) {
            debug!("Using terminal: {:?}", terminal);
            terminal
                .launch_command(&nano_cmd)
                .map_err(|e| EditorError::LaunchFailed(e))?;
        } else {
            return Err(EditorError::Other(
                "No terminal emulator found. Please install iTerm2, Alacritty, or another terminal.".to_string()
            ));
        }

        Ok(())
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        Ok(Vec::new())
    }
}
