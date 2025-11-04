use super::super::traits::{EditorError, EditorInstance, EditorManager, EditorResult, OpenOptions};
use super::terminal_detector::TerminalApp;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

pub struct MicroManager;

impl MicroManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for MicroManager {
    fn id(&self) -> &str {
        "micro"
    }

    fn display_name(&self) -> &str {
        "Micro"
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let candidates = vec![
                PathBuf::from("/opt/homebrew/bin/micro"),
                PathBuf::from("/usr/local/bin/micro"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found micro at {:?}", path);
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let candidates = vec![
                PathBuf::from("/usr/bin/micro"),
                PathBuf::from("/usr/local/bin/micro"),
                PathBuf::from("/snap/bin/micro"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found micro at {:?}", path);
                    return Some(path);
                }
            }
        }

        if let Ok(output) = Command::new("which").arg("micro").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(&path_str);
                    if path.exists() {
                        debug!("Found micro via which: {:?}", path);
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

        let mut micro_args = vec![];
        match (options.line, options.column) {
            (Some(line), Some(column)) => {
                micro_args.push(format!("'{}':{}:{}", path.display(), line, column));
            }
            (Some(line), None) => {
                micro_args.push(format!("'{}':{}", path.display(), line));
            }
            _ => {
                micro_args.push(format!("'{}'", path.display()));
            }
        }

        let micro_cmd = format!("micro {}", micro_args.join(" "));

        debug!("Opening in terminal with micro: {}", micro_cmd);

        let terminal_pref = options.terminal_preference.as_deref();
        if let Some(terminal) = TerminalApp::detect_installed_with_preference(terminal_pref) {
            debug!("Using terminal: {:?}", terminal);
            terminal
                .launch_command(&micro_cmd)
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
