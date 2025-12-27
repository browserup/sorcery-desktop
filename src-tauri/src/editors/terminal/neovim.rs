use super::super::traits::{EditorError, EditorInstance, EditorManager, EditorResult, OpenOptions};
use super::terminal_detector::TerminalApp;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

pub struct NeovimManager;

impl NeovimManager {
    pub fn new() -> Self {
        Self
    }

    async fn find_nvim_socket(&self, target_path: &Path) -> Option<PathBuf> {
        let sockets = self.gather_nvim_sockets().await;
        if sockets.is_empty() {
            return None;
        }

        let target = target_path.canonicalize().ok()?;

        for socket in &sockets {
            if let Some(cwd) = self.get_nvim_cwd(socket).await {
                if target.starts_with(&cwd) {
                    return Some(socket.clone());
                }
            }
        }

        sockets.first().cloned()
    }

    fn search_dir_for_sockets(
        &self,
        dir: &Path,
        sockets: &mut Vec<PathBuf>,
        depth: usize,
        max_depth: usize,
    ) {
        if depth >= max_depth {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if let Ok(metadata) = std::fs::metadata(&path) {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::FileTypeExt;
                        if metadata.file_type().is_socket() {
                            debug!("Found nvim socket at depth {}: {:?}", depth, path);
                            sockets.push(path);
                        } else if metadata.is_dir() {
                            debug!("Searching subdirectory at depth {}: {:?}", depth, path);
                            self.search_dir_for_sockets(&path, sockets, depth + 1, max_depth);
                        }
                    }
                }
            }
        }
    }

    async fn gather_nvim_sockets(&self) -> Vec<PathBuf> {
        use std::env;

        let mut sockets = Vec::new();
        let mut dirs = vec![PathBuf::from("/tmp")];

        if let Ok(tmpdir) = env::var("TMPDIR") {
            let tmpdir_path = PathBuf::from(&tmpdir);
            dirs.push(tmpdir_path.clone());
            debug!("Searching for nvim sockets in: /tmp and {}", tmpdir);
        }

        for dir in dirs {
            debug!("Checking directory: {:?}", dir);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.contains("nvim") {
                            debug!("Found nvim-related item: {:?}", path);
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::FileTypeExt;
                                    if metadata.file_type().is_socket() {
                                        debug!("Found nvim socket: {:?}", path);
                                        sockets.push(path);
                                    } else if metadata.is_dir() {
                                        debug!(
                                            "Found nvim directory, searching inside: {:?}",
                                            path
                                        );
                                        self.search_dir_for_sockets(&path, &mut sockets, 0, 2);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        debug!("Total nvim sockets found: {}", sockets.len());
        sockets
    }

    async fn get_nvim_cwd(&self, socket: &Path) -> Option<PathBuf> {
        let binary = self.find_binary().await?;

        let output = Command::new(&binary)
            .arg("--server")
            .arg(socket)
            .arg("--remote-expr")
            .arg("getcwd()")
            .output()
            .ok()?;

        if output.status.success() {
            let cwd_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !cwd_str.is_empty() {
                return Some(PathBuf::from(cwd_str));
            }
        }

        None
    }
}

#[async_trait]
impl EditorManager for NeovimManager {
    fn id(&self) -> &str {
        "neovim"
    }

    fn display_name(&self) -> &str {
        "Neovim"
    }

    fn supports_folders(&self) -> bool {
        true
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let candidates = vec![
                PathBuf::from("/opt/homebrew/bin/nvim"),
                PathBuf::from("/usr/local/bin/nvim"),
                PathBuf::from("/usr/bin/nvim"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found nvim at {:?}", path);
                    return Some(path);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let candidates = vec![
                PathBuf::from("/usr/bin/nvim"),
                PathBuf::from("/usr/local/bin/nvim"),
                PathBuf::from("/snap/bin/nvim"),
            ];

            for path in candidates {
                if path.exists() {
                    debug!("Found nvim at {:?}", path);
                    return Some(path);
                }
            }
        }

        if let Ok(output) = Command::new("which").arg("nvim").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = PathBuf::from(&path_str);
                    if path.exists() {
                        debug!("Found nvim via which: {:?}", path);
                        return Some(path);
                    }
                }
            }
        }
        None
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()> {
        tracing::info!("[NVIM-DEBUG] open() called for path: {:?}", path);
        let binary = self
            .find_binary()
            .await
            .ok_or(EditorError::BinaryNotFound)?;
        tracing::info!("[NVIM-DEBUG] Found binary: {:?}", binary);

        if let Some(socket) = self.find_nvim_socket(path).await {
            tracing::info!(
                "[NVIM-DEBUG] Found nvim socket: {:?}, trying to reuse",
                socket
            );

            let path_str = path.display().to_string();
            let escaped_path = path_str.replace('\\', "\\\\").replace(' ', "\\ ");
            let keys = match (options.line, options.column) {
                (Some(line), Some(column)) => {
                    format!(
                        ":e {}<CR>:call cursor({},{})<CR>",
                        escaped_path, line, column
                    )
                }
                (Some(line), None) => {
                    format!(":{}<CR>:e {}<CR>", line, escaped_path)
                }
                _ => {
                    format!(":e {}<CR>", escaped_path)
                }
            };

            tracing::info!("[NVIM-DEBUG] Sending keys to socket: {}", keys);
            let result = Command::new(&binary)
                .arg("--server")
                .arg(&socket)
                .arg("--remote-send")
                .arg(&keys)
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    tracing::info!("[NVIM-DEBUG] Successfully sent file to existing nvim instance");
                    return Ok(());
                }
                Ok(output) => {
                    tracing::info!(
                        "[NVIM-DEBUG] Failed to send to nvim socket, status: {:?}, stderr: {:?}",
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                Err(e) => tracing::info!("[NVIM-DEBUG] Error sending to nvim socket: {}", e),
            }
        } else {
            tracing::info!("[NVIM-DEBUG] No nvim socket found");
        }

        tracing::info!("[NVIM-DEBUG] Spawning new nvim instance");

        let mut nvim_args: Vec<String> = vec![];
        match (options.line, options.column) {
            (Some(line), Some(column)) => {
                nvim_args.push("-c".to_string());
                nvim_args.push(format!("call cursor({},{})", line, column));
            }
            (Some(line), None) => {
                nvim_args.push(format!("+{}", line));
            }
            _ => {}
        }
        nvim_args.push(path.display().to_string());

        debug!("Spawning nvim with args: {:?}", nvim_args);

        let terminal_pref = options.terminal_preference.as_deref();
        if let Some(terminal) = TerminalApp::detect_installed_with_preference(terminal_pref) {
            tracing::info!("[NVIM-DEBUG] Using terminal: {:?}", terminal);
            terminal.launch_editor("nvim", &nvim_args).map_err(|e| {
                tracing::error!("[NVIM-DEBUG] Terminal launch failed: {}", e);
                EditorError::LaunchFailed(e)
            })?;
            tracing::info!("[NVIM-DEBUG] Terminal launch succeeded");
        } else {
            tracing::error!("[NVIM-DEBUG] No terminal emulator found");
            return Err(EditorError::Other(
                "No terminal emulator found. Please install iTerm2, Alacritty, or another terminal.".to_string()
            ));
        }

        tracing::info!("[NVIM-DEBUG] open() completed successfully");
        Ok(())
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        Ok(Vec::new())
    }
}
