use super::traits::{EditorManager, OpenOptions, EditorInstance, EditorResult, EditorError};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, warn};

pub struct VSCodeManager {
    id: String,
    display_name: String,
    cli_name: String,
    macos_app_name: String,
    #[cfg(target_os = "windows")]
    windows_exe_name: String,
}

impl VSCodeManager {
    pub fn new(id: &str, display_name: &str, cli_name: &str, macos_app_name: &str, _windows_exe_name: &str) -> Self {
        Self {
            id: id.to_string(),
            display_name: display_name.to_string(),
            cli_name: cli_name.to_string(),
            macos_app_name: macos_app_name.to_string(),
            #[cfg(target_os = "windows")]
            windows_exe_name: _windows_exe_name.to_string(),
        }
    }

    #[cfg(target_os = "macos")]
    async fn find_binary_macos(&self) -> Option<PathBuf> {
        let candidates = vec![
            PathBuf::from(format!("/Applications/{}.app/Contents/Resources/app/bin/{}",
                self.macos_app_name, self.cli_name)),
            PathBuf::from(format!("/usr/local/bin/{}", self.cli_name)),
            PathBuf::from(format!("/opt/homebrew/bin/{}", self.cli_name)),
        ];

        for path in candidates {
            if path.exists() {
                debug!("Found {} at {:?}", self.display_name, path);
                return Some(path);
            }
        }

        if let Ok(output) = Command::new("which").arg(&self.cli_name).output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(path_str);
                    if path.exists() {
                        debug!("Found {} via which: {:?}", self.display_name, path);
                        return Some(path);
                    }
                }
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    async fn find_binary_windows(&self) -> Option<PathBuf> {
        let candidates = vec![
            PathBuf::from(format!("C:\\Program Files\\{}\\bin\\{}.cmd",
                self.windows_exe_name, self.cli_name)),
            PathBuf::from(format!("C:\\Program Files (x86)\\{}\\bin\\{}.cmd",
                self.windows_exe_name, self.cli_name)),
        ];

        for path in candidates {
            if path.exists() {
                debug!("Found {} at {:?}", self.display_name, path);
                return Some(path);
            }
        }

        if let Ok(output) = Command::new("where").arg(&format!("{}.cmd", self.cli_name)).output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(path_str.lines().next().unwrap_or(""));
                    if path.exists() {
                        debug!("Found {} via where: {:?}", self.display_name, path);
                        return Some(path);
                    }
                }
            }
        }

        None
    }

    #[cfg(target_os = "linux")]
    async fn find_binary_linux(&self) -> Option<PathBuf> {
        let candidates = vec![
            PathBuf::from(format!("/usr/local/bin/{}", self.cli_name)),
            PathBuf::from(format!("/usr/bin/{}", self.cli_name)),
            PathBuf::from(format!("/snap/bin/{}", self.cli_name)),
        ];

        for path in candidates {
            if path.exists() {
                debug!("Found {} at {:?}", self.display_name, path);
                return Some(path);
            }
        }

        if let Ok(output) = Command::new("which").arg(&self.cli_name).output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(path_str);
                    if path.exists() {
                        debug!("Found {} via which: {:?}", self.display_name, path);
                        return Some(path);
                    }
                }
            }
        }

        None
    }
}

#[async_trait]
impl EditorManager for VSCodeManager {
    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        return self.find_binary_macos().await;

        #[cfg(target_os = "windows")]
        return self.find_binary_windows().await;

        #[cfg(target_os = "linux")]
        return self.find_binary_linux().await;
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()> {
        let binary = self.find_binary().await
            .ok_or(EditorError::BinaryNotFound)?;

        let mut args = vec![];

        if !options.new_window {
            args.push("--reuse-window");
        } else {
            args.push("--new-window");
        }

        let goto_arg = if let Some(line) = options.line {
            let col = options.column.unwrap_or(1);
            format!("--goto {}:{}:{}", path.display(), line, col)
        } else {
            path.display().to_string()
        };

        args.push(&goto_arg);

        debug!("Launching {} with args: {:?}", self.display_name, args);

        let result = Command::new(&binary)
            .args(&args)
            .output();

        match result {
            Ok(output) if output.status.success() => {
                return Ok(());
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                debug!("Failed to launch {} with primary binary: {}", self.display_name, stderr);
            }
            Err(e) => {
                debug!("Failed to exec {} with primary binary: {}", self.display_name, e);
            }
        }

        // macOS fallback: If CLI command fails and app bundle exists, try using app bundle CLI path
        #[cfg(target_os = "macos")]
        {
            let app_path = PathBuf::from(format!("/Applications/{}.app", self.macos_app_name));
            if app_path.exists() {
                let cli_path = app_path.join("Contents/Resources/app/bin").join(&self.cli_name);
                debug!("Trying app bundle fallback at {:?}", cli_path);

                if cli_path.exists() {
                    let fallback_result = Command::new(&cli_path)
                        .args(&args)
                        .output();

                    match fallback_result {
                        Ok(output) if output.status.success() => {
                            debug!("Successfully launched {} via app bundle fallback", self.display_name);
                            return Ok(());
                        }
                        Ok(output) => {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            warn!("App bundle fallback also failed: {}", stderr);
                        }
                        Err(e) => {
                            warn!("Failed to exec via app bundle: {}", e);
                        }
                    }
                }
            }
        }

        Err(EditorError::LaunchFailed(format!("Failed to launch {}", self.display_name)))
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        #[cfg(target_os = "macos")]
        {
            let pattern = format!("/Applications/{}.app", self.macos_app_name);

            let output = Command::new("ps")
                .arg("aux")
                .output()
                .map_err(|e| EditorError::Other(e.to_string()))?;

            if !output.status.success() {
                return Ok(Vec::new());
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.to_lowercase().contains(&pattern.to_lowercase()) {
                return Ok(vec![EditorInstance {
                    pid: 0,
                    workspace: Some("detected (workspace unknown)".to_string()),
                    window_title: None,
                }]);
            }

            Ok(Vec::new())
        }

        #[cfg(target_os = "windows")]
        {
            let pattern = format!("{}.exe", self.windows_exe_name);

            let output = Command::new("tasklist")
                .output()
                .map_err(|e| EditorError::Other(e.to_string()))?;

            if !output.status.success() {
                return Ok(Vec::new());
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.to_lowercase().contains(&pattern.to_lowercase()) {
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
            if stdout.to_lowercase().contains(&self.cli_name.to_lowercase()) {
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
