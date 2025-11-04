use super::super::traits::{EditorError, EditorInstance, EditorManager, EditorResult, OpenOptions};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

pub struct EmacsManager;

impl EmacsManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for EmacsManager {
    fn id(&self) -> &str {
        "emacs"
    }

    fn display_name(&self) -> &str {
        "Emacs"
    }

    fn supports_folders(&self) -> bool {
        true
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let candidates = vec![
                PathBuf::from("/Applications/Emacs.app/Contents/MacOS/bin/emacsclient"),
                PathBuf::from("/opt/homebrew/bin/emacsclient"),
                PathBuf::from("/usr/local/bin/emacsclient"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found emacsclient at {:?}", path);
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let candidates = vec![
                PathBuf::from("/usr/bin/emacsclient"),
                PathBuf::from("/usr/local/bin/emacsclient"),
                PathBuf::from("/snap/bin/emacsclient"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found emacsclient at {:?}", path);
                    return Some(path);
                }
            }
        }

        if let Ok(output) = Command::new("which").arg("emacsclient").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(&path_str);
                    if path.exists() {
                        debug!("Found emacsclient via which: {:?}", path);
                        return Some(path);
                    }
                }
            }
        }

        None
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()> {
        use std::process::Stdio;

        let mut args = vec![];

        match (options.line, options.column) {
            (Some(line), Some(column)) => {
                args.push(format!("+{}:{}", line, column));
            }
            (Some(line), None) => {
                args.push(format!("+{}", line));
            }
            _ => {}
        }

        args.push(path.display().to_string());

        #[cfg(target_os = "macos")]
        {
            debug!("Trying to open Emacs.app on macOS");
            let mut cmd_args = vec!["-a", "Emacs", "--args"];
            let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            cmd_args.extend(args_str);

            let result = Command::new("open")
                .args(&cmd_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            if result.is_ok() {
                debug!("Successfully launched Emacs.app");
                return Ok(());
            }

            debug!("Emacs.app launch failed, trying emacsclient");

            let emacsclient_args: Vec<&str> = vec!["-n"]
                .into_iter()
                .chain(args.iter().map(|s| s.as_str()))
                .collect();

            let result = Command::new("emacsclient")
                .args(&emacsclient_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            if result.is_ok() {
                debug!("Successfully launched via emacsclient");
                return Ok(());
            }

            debug!("emacsclient failed, trying emacs command");

            let emacs_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

            Command::new("emacs")
                .args(&emacs_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| {
                    EditorError::LaunchFailed(format!("All Emacs launch attempts failed: {}", e))
                })?;

            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            debug!("Trying runemacs on Windows");
            let runemacs_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

            let result = Command::new("runemacs")
                .args(&runemacs_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            if result.is_ok() {
                debug!("Successfully launched via runemacs");
                return Ok(());
            }

            debug!("runemacs failed, trying emacs command");

            let emacs_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

            Command::new("emacs")
                .args(&emacs_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| {
                    EditorError::LaunchFailed(format!("All Emacs launch attempts failed: {}", e))
                })?;

            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            debug!("Trying emacsclient -c -n on Linux");
            let mut client_args = vec!["-c", "-n"];
            let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            client_args.extend(args_str);

            let result = Command::new("emacsclient")
                .args(&client_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            if result.is_ok() {
                debug!("Successfully launched via emacsclient");
                return Ok(());
            }

            debug!("emacsclient failed, trying emacs GUI");

            let emacs_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

            let result = Command::new("emacs")
                .args(&emacs_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            if result.is_ok() {
                debug!("Successfully launched Emacs GUI");
                return Ok(());
            }

            debug!("Emacs GUI failed, trying gnome-terminal with emacs -nw");

            let mut terminal_args = vec!["--", "emacs", "-nw"];
            let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            terminal_args.extend(args_str);

            Command::new("gnome-terminal")
                .args(&terminal_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| {
                    EditorError::LaunchFailed(format!("All Emacs launch attempts failed: {}", e))
                })?;

            return Ok(());
        }
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("tasklist")
                .output()
                .map_err(|e| EditorError::Other(e.to_string()))?;

            if !output.status.success() {
                return Ok(Vec::new());
            }

            let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
            if stdout.contains("emacs.exe") || stdout.contains("runemacs.exe") {
                return Ok(vec![EditorInstance {
                    pid: 0,
                    workspace: Some("detected (workspace unknown)".to_string()),
                    window_title: None,
                }]);
            }

            Ok(Vec::new())
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("ps")
                .arg("aux")
                .output()
                .map_err(|e| EditorError::Other(e.to_string()))?;

            if !output.status.success() {
                return Ok(Vec::new());
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("Emacs.app")
                || stdout.contains("/emacs ")
                || stdout.contains(" emacs ")
            {
                return Ok(vec![EditorInstance {
                    pid: 0,
                    workspace: Some("detected (workspace unknown)".to_string()),
                    window_title: None,
                }]);
            }

            Ok(Vec::new())
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("ps")
                .arg("aux")
                .output()
                .map_err(|e| EditorError::Other(e.to_string()))?;

            if !output.status.success() {
                return Ok(Vec::new());
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("/emacs") || stdout.contains(" emacs") {
                return Ok(vec![EditorInstance {
                    pid: 0,
                    workspace: Some("detected (workspace unknown)".to_string()),
                    window_title: None,
                }]);
            }

            Ok(Vec::new())
        }
    }
}
