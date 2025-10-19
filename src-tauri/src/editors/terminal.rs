use super::traits::{EditorManager, OpenOptions, EditorInstance, EditorResult, EditorError};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

pub struct VimManager;

impl VimManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EditorManager for VimManager {
    fn id(&self) -> &str {
        "vim"
    }

    fn display_name(&self) -> &str {
        "Vim"
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        if let Ok(output) = Command::new("which").arg("vim").output() {
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
        self.find_binary().await
            .ok_or(EditorError::BinaryNotFound)?;

        #[cfg(target_os = "macos")]
        {
            let mut cmd_str = format!("vim");

            if let Some(line) = options.line {
                cmd_str.push_str(&format!(" +{}", line));
            }

            cmd_str.push_str(&format!(" '{}'", path.display()));

            debug!("Opening in Terminal.app with vim: {}", cmd_str);

            Command::new("osascript")
                .arg("-e")
                .arg(format!("tell application \"Terminal\" to do script \"{}\"", cmd_str))
                .arg("-e")
                .arg("tell application \"Terminal\" to activate")
                .output()
                .map_err(|e| EditorError::LaunchFailed(e.to_string()))?;
        }

        #[cfg(not(target_os = "macos"))]
        {
            let binary = self.find_binary().await
                .ok_or(EditorError::BinaryNotFound)?;

            let mut args = vec![];

            if let Some(line) = options.line {
                args.push(format!("+{}", line));
            }

            args.push(path.display().to_string());

            Command::new(&binary)
                .args(&args)
                .spawn()
                .map_err(|e| EditorError::LaunchFailed(e.to_string()))?;
        }

        Ok(())
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        Ok(Vec::new())
    }
}

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

        // Try to find best match based on cwd
        let target = target_path.canonicalize().ok()?;

        for socket in &sockets {
            if let Some(cwd) = self.get_nvim_cwd(socket).await {
                if target.starts_with(&cwd) {
                    return Some(socket.clone());
                }
            }
        }

        // Return first available socket if no match
        sockets.first().cloned()
    }

    fn search_dir_for_sockets(&self, dir: &Path, sockets: &mut Vec<PathBuf>, depth: usize, max_depth: usize) {
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
                            // Check if it's a socket
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::FileTypeExt;
                                    if metadata.file_type().is_socket() {
                                        debug!("Found nvim socket: {:?}", path);
                                        sockets.push(path);
                                    } else if metadata.is_dir() {
                                        debug!("Found nvim directory, searching inside: {:?}", path);
                                        // Recursively search up to 2 levels deep for socket files
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

    async fn find_binary(&self) -> Option<PathBuf> {
        if let Ok(output) = Command::new("which").arg("nvim").output() {
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

        // Try to find and reuse existing nvim instance
        if let Some(socket) = self.find_nvim_socket(path).await {
            debug!("Found nvim socket: {:?}, trying to reuse", socket);

            let path_str = path.display().to_string();
            let escaped_path = path_str.replace('\\', "\\\\").replace(' ', "\\ ");
            let keys = if let Some(line) = options.line {
                format!(":{}<CR>:e {}<CR>", line, escaped_path)
            } else {
                format!(":e {}<CR>", escaped_path)
            };

            let result = Command::new(&binary)
                .arg("--server")
                .arg(&socket)
                .arg("--remote-send")
                .arg(&keys)
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    debug!("Successfully sent file to existing nvim instance");
                    return Ok(());
                }
                Ok(_) => debug!("Failed to send to nvim socket, will try fallback"),
                Err(e) => debug!("Error sending to nvim socket: {}, will try fallback", e),
            }
        }

        // Fallback: No nvim found or reuse failed - spawn new instance in Terminal
        debug!("No nvim socket found or reuse failed, spawning new instance");

        #[cfg(target_os = "macos")]
        {
            let mut args = vec![];
            if let Some(line) = options.line {
                args.push(format!("+{}", line));
            }
            args.push(path.display().to_string());

            let nvim_cmd = if args.is_empty() {
                "nvim".to_string()
            } else {
                format!("nvim {}", args.join(" "))
            };

            debug!("Spawning nvim in new terminal window: {}", nvim_cmd);

            use std::process::Stdio;

            // Use default terminal via 'open' command which respects system default
            // The -n flag opens a new instance, and we run nvim directly
            Command::new("open")
                .arg("-a")
                .arg("Terminal")  // Or could detect/use user preference
                .arg("-n")
                .args(&["--args", "bash", "-c", &format!("{}", nvim_cmd)])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| EditorError::LaunchFailed(e.to_string()))?;
        }

        #[cfg(not(target_os = "macos"))]
        {
            return Err(EditorError::Other(
                "No running nvim instance found. Please open nvim in your terminal first.".to_string()
            ));
        }

        Ok(())
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        Ok(Vec::new())
    }
}

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

    async fn find_binary(&self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let emacsclient = PathBuf::from("/Applications/Emacs.app/Contents/MacOS/bin/emacsclient");
            if emacsclient.exists() {
                return Some(emacsclient);
            }
        }

        if let Ok(output) = Command::new("which").arg("emacsclient").output() {
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
        use std::process::Stdio;

        let mut args = vec![];

        if let Some(line) = options.line {
            args.push(format!("+{}", line));
        }

        args.push(path.display().to_string());

        #[cfg(target_os = "macos")]
        {
            // Try 1: open -a Emacs with app bundle
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

            // Try 2: emacsclient -n
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

            // Try 3: emacs directly
            let emacs_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

            Command::new("emacs")
                .args(&emacs_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| EditorError::LaunchFailed(format!("All Emacs launch attempts failed: {}", e)))?;

            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            // Try 1: runemacs
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

            // Try 2: emacs
            let emacs_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

            Command::new("emacs")
                .args(&emacs_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| EditorError::LaunchFailed(format!("All Emacs launch attempts failed: {}", e)))?;

            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            // Try 1: emacsclient -c -n (for daemon)
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

            // Try 2: emacs GUI
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

            // Try 3: gnome-terminal with emacs -nw
            let mut terminal_args = vec!["--", "emacs", "-nw"];
            let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            terminal_args.extend(args_str);

            Command::new("gnome-terminal")
                .args(&terminal_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| EditorError::LaunchFailed(format!("All Emacs launch attempts failed: {}", e)))?;

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
            if stdout.contains("Emacs.app") || stdout.contains("/emacs ") || stdout.contains(" emacs ") {
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
