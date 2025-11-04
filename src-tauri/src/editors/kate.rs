use super::traits::{EditorError, EditorInstance, EditorManager, EditorResult, OpenOptions};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

pub struct KateManager;

impl KateManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for KateManager {
    fn id(&self) -> &str {
        "kate"
    }

    fn display_name(&self) -> &str {
        "Kate"
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        if let Ok(output) = Command::new("which").arg("kate").output() {
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
            args.push("-l".to_string());
            args.push(line.to_string());
        }

        if let Some(column) = options.column {
            args.push("-c".to_string());
            args.push(column.to_string());
        }

        args.push("-u".to_string());
        args.push(path.display().to_string());

        debug!("Launching Kate with args: {:?}", args);

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
