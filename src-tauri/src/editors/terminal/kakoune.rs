use super::super::traits::{EditorError, EditorInstance, EditorManager, EditorResult, OpenOptions};
use super::terminal_detector::TerminalApp;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

pub struct KakouneManager;

impl KakouneManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for KakouneManager {
    fn id(&self) -> &str {
        "kakoune"
    }

    fn display_name(&self) -> &str {
        "Kakoune"
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let candidates = vec![
                PathBuf::from("/opt/homebrew/bin/kak"),
                PathBuf::from("/usr/local/bin/kak"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found kak at {:?}", path);
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let candidates = vec![
                PathBuf::from("/usr/bin/kak"),
                PathBuf::from("/usr/local/bin/kak"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found kak at {:?}", path);
                    return Some(path);
                }
            }
        }

        if let Ok(output) = Command::new("which").arg("kak").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(&path_str);
                    if path.exists() {
                        debug!("Found kak via which: {:?}", path);
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

        let mut kak_args: Vec<String> = vec![];
        match (options.line, options.column) {
            (Some(line), Some(column)) => {
                kak_args.push(format!("+{}:{}", line, column));
            }
            (Some(line), None) => {
                kak_args.push(format!("+{}:1", line));
            }
            _ => {}
        }
        kak_args.push(path.display().to_string());

        debug!("Opening kakoune with args: {:?}", kak_args);

        let terminal_pref = options.terminal_preference.as_deref();
        if let Some(terminal) = TerminalApp::detect_installed_with_preference(terminal_pref) {
            debug!("Using terminal: {:?}", terminal);
            terminal
                .launch_editor("kak", &kak_args)
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
