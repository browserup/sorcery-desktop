use super::traits::{EditorManager, OpenOptions, EditorInstance, EditorResult, EditorError};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

#[cfg(target_os = "macos")]
pub struct XcodeManager;

#[cfg(target_os = "macos")]
impl XcodeManager {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "macos")]
#[async_trait]
impl EditorManager for XcodeManager {
    fn id(&self) -> &str {
        "xcode"
    }

    fn display_name(&self) -> &str {
        "Xcode"
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        let xcode_path = PathBuf::from("/Applications/Xcode.app/Contents/MacOS/Xcode");
        if xcode_path.exists() {
            Some(xcode_path)
        } else {
            None
        }
    }

    async fn open(&self, path: &Path, _options: &OpenOptions) -> EditorResult<()> {
        self.find_binary().await
            .ok_or(EditorError::BinaryNotFound)?;

        debug!("Opening in Xcode: {:?}", path);

        Command::new("open")
            .arg("-a")
            .arg("Xcode")
            .arg(path)
            .spawn()
            .map_err(|e| EditorError::LaunchFailed(e.to_string()))?;

        Ok(())
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        Ok(Vec::new())
    }
}

pub struct ZedManager;

impl ZedManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for ZedManager {
    fn id(&self) -> &str {
        "zed"
    }

    fn display_name(&self) -> &str {
        "Zed"
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let zed_cli = PathBuf::from("/usr/local/bin/zed");
            if zed_cli.exists() {
                return Some(zed_cli);
            }
        }

        if let Ok(output) = Command::new("which").arg("zed").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }

        None
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()> {
        let binary = self.find_binary().await
            .ok_or(EditorError::BinaryNotFound)?;

        let mut args = vec![];

        let file_arg = if let Some(line) = options.line {
            format!("{}:{}", path.display(), line)
        } else {
            path.display().to_string()
        };

        args.push(file_arg);

        debug!("Launching Zed with args: {:?}", args);

        Command::new(&binary)
            .args(&args)
            .spawn()
            .map_err(|e| EditorError::LaunchFailed(e.to_string()))?;

        Ok(())
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        Ok(Vec::new())
    }
}

pub struct SublimeManager;

impl SublimeManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for SublimeManager {
    fn id(&self) -> &str {
        "sublime"
    }

    fn display_name(&self) -> &str {
        "Sublime Text"
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let subl = PathBuf::from("/Applications/Sublime Text.app/Contents/SharedSupport/bin/subl");
            if subl.exists() {
                return Some(subl);
            }

            let subl_usr = PathBuf::from("/usr/local/bin/subl");
            if subl_usr.exists() {
                return Some(subl_usr);
            }
        }

        #[cfg(target_os = "windows")]
        {
            let subl = PathBuf::from("C:\\Program Files\\Sublime Text\\subl.exe");
            if subl.exists() {
                return Some(subl);
            }
        }

        if let Ok(output) = Command::new("which").arg("subl").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }

        None
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()> {
        let binary = self.find_binary().await
            .ok_or(EditorError::BinaryNotFound)?;

        let mut args = vec![];

        let file_arg = if let Some(line) = options.line {
            format!("{}:{}", path.display(), line)
        } else {
            path.display().to_string()
        };

        args.push(file_arg);

        debug!("Launching Sublime Text with args: {:?}", args);

        Command::new(&binary)
            .args(&args)
            .spawn()
            .map_err(|e| EditorError::LaunchFailed(e.to_string()))?;

        Ok(())
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        Ok(Vec::new())
    }
}
