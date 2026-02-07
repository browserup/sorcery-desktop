use super::super::traits::{EditorError, EditorInstance, EditorManager, EditorResult, OpenOptions};
use super::terminal_detector::TerminalApp;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

pub struct VimManager;

impl VimManager {
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for VimManager {
    fn id(&self) -> &'static str {
        "vim"
    }

    fn display_name(&self) -> &'static str {
        "Vim"
    }

    fn supports_folders(&self) -> bool {
        true
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let candidates = vec![
                PathBuf::from("/opt/homebrew/bin/vim"),
                PathBuf::from("/usr/local/bin/vim"),
                PathBuf::from("/usr/bin/vim"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found vim at {:?}", path);
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let candidates = vec![
                PathBuf::from("/usr/bin/vim"),
                PathBuf::from("/usr/local/bin/vim"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found vim at {:?}", path);
                    return Some(path);
                }
            }
        }

        if let Ok(output) = Command::new("which").arg("vim").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(&path_str);
                    if path.exists() {
                        debug!("Found vim via which: {:?}", path);
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

        let mut vim_args: Vec<String> = vec![];
        match (options.line, options.column) {
            (Some(line), Some(column)) => {
                vim_args.push("-c".to_string());
                vim_args.push(format!("call cursor({line},{column})"));
            }
            (Some(line), None) => {
                vim_args.push(format!("+{line}"));
            }
            _ => {}
        }
        // Defense in depth: '--' terminates options so paths can't be interpreted as flags.
        // Primary protection is the absolute path requirement in path_validator which ensures
        // paths start with '/' or a drive letter, never '-'.
        vim_args.push("--".to_string());
        vim_args.push(path.display().to_string());

        debug!("Opening vim with args: {:?}", vim_args);

        let terminal_pref = options.terminal_preference.as_deref();
        if let Some(terminal) = TerminalApp::detect_installed_with_preference(terminal_pref) {
            debug!("Using terminal: {:?}", terminal);
            terminal
                .launch_editor("vim", &vim_args)
                .map_err(EditorError::LaunchFailed)?;
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
