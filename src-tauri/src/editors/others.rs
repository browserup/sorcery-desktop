use super::traits::{EditorError, EditorInstance, EditorManager, EditorResult, OpenOptions};
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

    fn supports_folders(&self) -> bool {
        true
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
        self.find_binary()
            .await
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

    fn supports_folders(&self) -> bool {
        true
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let candidates = vec![
                PathBuf::from("/Applications/Zed.app/Contents/MacOS/cli"),
                PathBuf::from("/usr/local/bin/zed"),
                PathBuf::from("/opt/homebrew/bin/zed"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found Zed at {:?}", path);
                    return Some(path);
                }
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
        let binary = self
            .find_binary()
            .await
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

pub struct GeditManager;

impl GeditManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for GeditManager {
    fn id(&self) -> &str {
        "gedit"
    }

    fn display_name(&self) -> &str {
        "Gedit"
    }

    fn supports_folders(&self) -> bool {
        false
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            let candidates = vec![
                PathBuf::from("/usr/bin/gedit"),
                PathBuf::from("/usr/local/bin/gedit"),
            ];

            for path in candidates {
                if path.exists() {
                    return Some(path);
                }
            }
        }

        if let Ok(output) = Command::new("which").arg("gedit").output() {
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
        let binary = self
            .find_binary()
            .await
            .ok_or(EditorError::BinaryNotFound)?;

        let mut args = vec![];

        if let Some(line) = options.line {
            args.push(format!("+{}", line));
        }

        args.push(path.display().to_string());

        debug!("Launching Gedit with args: {:?}", args);

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

#[async_trait]
impl EditorManager for SublimeManager {
    fn id(&self) -> &str {
        "sublime"
    }

    fn display_name(&self) -> &str {
        "Sublime Text"
    }

    fn supports_folders(&self) -> bool {
        true
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let subl =
                PathBuf::from("/Applications/Sublime Text.app/Contents/SharedSupport/bin/subl");
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
        let binary = self
            .find_binary()
            .await
            .ok_or(EditorError::BinaryNotFound)?;

        let mut args = vec![];

        let file_arg = match (options.line, options.column) {
            (Some(line), Some(column)) => format!("{}:{}:{}", path.display(), line, column),
            (Some(line), None) => format!("{}:{}", path.display(), line),
            _ => path.display().to_string(),
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
