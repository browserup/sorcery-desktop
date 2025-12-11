use super::traits::{EditorError, EditorInstance, EditorManager, EditorResult, OpenOptions};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use tracing::{debug, warn};

struct BinaryCache {
    path: Option<PathBuf>,
    timestamp: SystemTime,
}

pub struct JetBrainsManager {
    id: String,
    display_name: String,
    toolbox_id: String,
    cache: RwLock<Option<BinaryCache>>,
}

impl JetBrainsManager {
    pub fn new(id: &str, display_name: &str, toolbox_id: &str) -> Self {
        Self {
            id: id.to_string(),
            display_name: display_name.to_string(),
            toolbox_id: toolbox_id.to_string(),
            cache: RwLock::new(None),
        }
    }

    fn cache_ttl() -> Duration {
        Duration::from_secs(300)
    }

    fn get_cached_binary(&self) -> Option<PathBuf> {
        let cache = self.cache.read();
        if let Some(cached) = cache.as_ref() {
            if cached.timestamp.elapsed().ok()? < Self::cache_ttl() {
                return cached.path.clone();
            }
        }
        None
    }

    fn cache_binary(&self, path: Option<PathBuf>) {
        let mut cache = self.cache.write();
        *cache = Some(BinaryCache {
            path,
            timestamp: SystemTime::now(),
        });
    }

    #[cfg(target_os = "macos")]
    async fn find_toolbox_binary_macos(&self) -> Option<PathBuf> {
        let toolbox_apps =
            dirs::home_dir()?.join("Library/Application Support/JetBrains/Toolbox/apps");

        if !toolbox_apps.exists() {
            return None;
        }

        let product_dir = toolbox_apps.join(&self.toolbox_id);
        let app_name = format!("{}.app", self.display_name);

        if product_dir.exists() {
            for channel in &["ch-0", "ch-1"] {
                let channel_dir = product_dir.join(channel);
                if !channel_dir.exists() {
                    continue;
                }

                if let Some(latest_version) = Self::find_latest_subdir(&channel_dir) {
                    let app_path = latest_version.join(&app_name);
                    if app_path.exists() {
                        debug!(
                            "Found {} Toolbox installation at {:?}",
                            self.display_name, app_path
                        );
                        return Some(app_path);
                    }
                }
            }
        }

        // Fallback heuristic: search across all Toolbox products
        self.find_any_toolbox_mac_app(&toolbox_apps, &app_name)
    }

    #[cfg(target_os = "macos")]
    fn find_any_toolbox_mac_app(&self, toolbox_root: &Path, app_name: &str) -> Option<PathBuf> {
        let products = std::fs::read_dir(toolbox_root).ok()?;

        for product in products.filter_map(|e| e.ok()) {
            if !product.path().is_dir() {
                continue;
            }

            for channel in &["ch-0", "ch-1"] {
                let channel_dir = product.path().join(channel);
                if let Some(latest_version) = Self::find_latest_subdir(&channel_dir) {
                    let app_path = latest_version.join(app_name);
                    if app_path.exists() {
                        debug!(
                            "Found {} via heuristic search at {:?}",
                            self.display_name, app_path
                        );
                        return Some(app_path);
                    }
                }
            }
        }

        None
    }

    fn find_latest_subdir(dir: &Path) -> Option<PathBuf> {
        if !dir.exists() {
            return None;
        }

        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                let metadata = e.metadata().ok()?;
                let mtime = metadata.modified().ok()?;
                Some((e.path(), mtime))
            })
            .collect();

        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.first().map(|(path, _)| path.clone())
    }

    #[cfg(target_os = "macos")]
    async fn find_standalone_binary_macos(&self) -> Option<PathBuf> {
        let app_name = format!("{}.app", self.display_name);

        // Check both /Applications and ~/Applications
        let candidates = vec![
            PathBuf::from("/Applications").join(&app_name),
            dirs::home_dir()?.join("Applications").join(&app_name),
        ];

        for app_path in candidates {
            if app_path.exists() {
                debug!("Found {} standalone at {:?}", self.display_name, app_path);
                return Some(app_path);
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    async fn find_toolbox_binary_windows(&self) -> Option<PathBuf> {
        let toolbox_apps = dirs::data_local_dir()?.join("JetBrains\\Toolbox\\apps");

        if toolbox_apps.exists() {
            let product_dir = toolbox_apps.join(&self.toolbox_id);

            if product_dir.exists() {
                for channel in &["ch-0", "ch-1"] {
                    let channel_dir = product_dir.join(channel);
                    if let Some(latest_version) = Self::find_latest_subdir(&channel_dir) {
                        let bin_dir = latest_version.join("bin");
                        if let Some(exe) = Self::pick_windows_exe(&bin_dir, &self.id) {
                            debug!(
                                "Found {} Toolbox installation at {:?}",
                                self.display_name, exe
                            );
                            return Some(exe);
                        }
                    }
                }
            }

            // Fallback heuristic search
            if let Some(exe) = self.find_any_toolbox_windows_exe(&toolbox_apps) {
                return Some(exe);
            }
        }

        // Standalone installations in Program Files
        self.find_standalone_binary_windows()
    }

    #[cfg(target_os = "windows")]
    fn find_any_toolbox_windows_exe(&self, toolbox_root: &Path) -> Option<PathBuf> {
        let products = std::fs::read_dir(toolbox_root).ok()?;

        for product in products.filter_map(|e| e.ok()) {
            if !product.path().is_dir() {
                continue;
            }

            for channel in &["ch-0", "ch-1"] {
                let channel_dir = product.path().join(channel);
                if let Some(latest_version) = Self::find_latest_subdir(&channel_dir) {
                    let bin_dir = latest_version.join("bin");
                    if let Some(exe) = Self::pick_windows_exe(&bin_dir, &self.id) {
                        debug!(
                            "Found {} via heuristic search at {:?}",
                            self.display_name, exe
                        );
                        return Some(exe);
                    }
                }
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    fn find_standalone_binary_windows(&self) -> Option<PathBuf> {
        use std::env;

        let pf = env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
        let pf86 =
            env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());

        let candidates = vec![
            PathBuf::from(&pf).join("JetBrains\\IntelliJ IDEA\\bin\\idea64.exe"),
            PathBuf::from(&pf).join("JetBrains\\IntelliJ IDEA Community Edition\\bin\\idea64.exe"),
            PathBuf::from(&pf).join("JetBrains\\RubyMine\\bin\\rubymine64.exe"),
            PathBuf::from(&pf).join("JetBrains\\GoLand\\bin\\goland64.exe"),
            PathBuf::from(&pf).join("JetBrains\\WebStorm\\bin\\webstorm64.exe"),
            PathBuf::from(&pf).join("JetBrains\\PyCharm\\bin\\pycharm64.exe"),
            PathBuf::from(&pf).join("JetBrains\\PhpStorm\\bin\\phpstorm64.exe"),
            PathBuf::from(&pf).join("JetBrains\\Rider\\bin\\rider64.exe"),
            PathBuf::from(&pf).join("JetBrains\\CLion\\bin\\clion64.exe"),
            PathBuf::from(&pf86).join("JetBrains\\IntelliJ IDEA\\bin\\idea64.exe"),
            PathBuf::from(&pf86).join("JetBrains\\RubyMine\\bin\\rubymine64.exe"),
            PathBuf::from(&pf86).join("JetBrains\\GoLand\\bin\\goland64.exe"),
        ];

        for candidate in candidates {
            if candidate.exists() {
                let candidate_str = candidate.to_string_lossy().to_lowercase();
                if candidate_str.contains(&self.id)
                    || (self.id == "intellij" && candidate_str.contains("idea"))
                {
                    debug!("Found {} standalone at {:?}", self.display_name, candidate);
                    return Some(candidate);
                }
            }
        }

        // Fallback: return first existing for IntelliJ
        if self.id == "intellij" || self.id == "idea" {
            for candidate in candidates {
                if candidate.exists() && candidate.to_string_lossy().contains("idea") {
                    return Some(candidate);
                }
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    fn pick_windows_exe(bin_dir: &Path, editor_id: &str) -> Option<PathBuf> {
        if !bin_dir.exists() {
            return None;
        }

        let files: Vec<_> = std::fs::read_dir(bin_dir)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .to_string_lossy()
                    .to_lowercase()
                    .ends_with("64.exe")
            })
            .collect();

        // Prefer matching name
        for file in &files {
            let name = file.file_name();
            let name_lower = name.to_string_lossy().to_lowercase();
            if name_lower.starts_with(editor_id) {
                return Some(file.path());
            }
        }

        // Fallback to idea64.exe for intellij/idea
        if editor_id == "intellij" || editor_id == "idea" {
            for file in &files {
                let name = file.file_name();
                if name.to_string_lossy().to_lowercase().starts_with("idea") {
                    return Some(file.path());
                }
            }
        }

        // Last resort: first 64.exe
        files.first().map(|e| e.path())
    }

    #[cfg(target_os = "linux")]
    async fn find_toolbox_binary_linux(&self) -> Option<PathBuf> {
        let toolbox_apps = dirs::data_local_dir()?.join("JetBrains/Toolbox/apps");

        if toolbox_apps.exists() {
            let product_dir = toolbox_apps.join(&self.toolbox_id);

            if product_dir.exists() {
                for channel in &["ch-0", "ch-1"] {
                    let channel_dir = product_dir.join(channel);
                    if let Some(latest_version) = Self::find_latest_subdir(&channel_dir) {
                        let bin_dir = latest_version.join("bin");
                        if let Some(script) = Self::pick_linux_script(&bin_dir, &self.id) {
                            debug!(
                                "Found {} Toolbox installation at {:?}",
                                self.display_name, script
                            );
                            return Some(script);
                        }
                    }
                }
            }

            // Fallback heuristic search
            if let Some(script) = self.find_any_toolbox_linux_script(&toolbox_apps) {
                return Some(script);
            }
        }

        None
    }

    #[cfg(target_os = "linux")]
    fn find_any_toolbox_linux_script(&self, toolbox_root: &Path) -> Option<PathBuf> {
        let products = std::fs::read_dir(toolbox_root).ok()?;

        for product in products.filter_map(|e| e.ok()) {
            if !product.path().is_dir() {
                continue;
            }

            for channel in &["ch-0", "ch-1"] {
                let channel_dir = product.path().join(channel);
                if let Some(latest_version) = Self::find_latest_subdir(&channel_dir) {
                    let bin_dir = latest_version.join("bin");
                    if let Some(script) = Self::pick_linux_script(&bin_dir, &self.id) {
                        debug!(
                            "Found {} via heuristic search at {:?}",
                            self.display_name, script
                        );
                        return Some(script);
                    }
                }
            }
        }

        None
    }

    #[cfg(target_os = "linux")]
    fn pick_linux_script(bin_dir: &Path, editor_id: &str) -> Option<PathBuf> {
        if !bin_dir.exists() {
            return None;
        }

        let files: Vec<_> = std::fs::read_dir(bin_dir)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().to_string_lossy().ends_with(".sh"))
            .collect();

        // Prefer matching name
        for file in &files {
            let name = file.file_name();
            let name_lower = name.to_string_lossy().to_lowercase();
            if name_lower.starts_with(editor_id) {
                return Some(file.path());
            }
        }

        // Fallback to idea.sh for intellij/idea
        if editor_id == "intellij" || editor_id == "idea" {
            for file in &files {
                let name = file.file_name();
                if name.to_string_lossy().to_lowercase().starts_with("idea") {
                    return Some(file.path());
                }
            }
        }

        // Last resort: first .sh
        files.first().map(|e| e.path())
    }

    #[cfg(target_os = "linux")]
    async fn find_standalone_binary_linux(&self) -> Option<PathBuf> {
        // Check common locations and PATH
        let candidates = vec![
            PathBuf::from(format!("/usr/local/bin/{}", self.toolbox_id)),
            PathBuf::from(format!("/usr/bin/{}", self.toolbox_id)),
            PathBuf::from(format!("/snap/bin/{}", self.toolbox_id)),
            PathBuf::from(format!("/opt/{}/bin/{}.sh", self.toolbox_id, self.toolbox_id)),
        ];

        for path in candidates {
            if path.exists() {
                debug!("Found {} standalone at {:?}", self.display_name, path);
                return Some(path);
            }
        }

        // Fallback: use `which` to find in PATH
        if let Ok(output) = Command::new("which").arg(&self.toolbox_id).output() {
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

    fn spawn_editor(&self, binary: &Path, args: &[String]) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            use std::process::Stdio;

            // Use -n to always open a new instance so arguments are properly passed
            // Without -n, if the app is already running, it just activates it and ignores args
            let mut cmd = Command::new("open");
            cmd.arg("-n").arg("-a").arg(binary).arg("--args");

            for arg in args {
                cmd.arg(arg);
            }

            cmd.stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| e.to_string())?;
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Stdio;

            // Use cmd.exe with START to properly detach
            let binary_str = binary.to_string_lossy();
            let mut cmd = Command::new("cmd.exe");
            cmd.arg("/c")
                .arg("start")
                .arg("\"\"") // Empty window title
                .arg(&*binary_str);

            for arg in args {
                cmd.arg(arg);
            }

            cmd.stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| e.to_string())?;
        }

        #[cfg(target_os = "linux")]
        {
            use std::process::Stdio;

            Command::new(binary)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

#[async_trait]
impl EditorManager for JetBrainsManager {
    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn supports_folders(&self) -> bool {
        true
    }

    async fn find_binary(&self) -> Option<PathBuf> {
        if let Some(cached) = self.get_cached_binary() {
            return Some(cached);
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(path) = self.find_toolbox_binary_macos().await {
                self.cache_binary(Some(path.clone()));
                return Some(path);
            }

            if let Some(path) = self.find_standalone_binary_macos().await {
                self.cache_binary(Some(path.clone()));
                return Some(path);
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(path) = self.find_toolbox_binary_windows().await {
                self.cache_binary(Some(path.clone()));
                return Some(path);
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(path) = self.find_toolbox_binary_linux().await {
                self.cache_binary(Some(path.clone()));
                return Some(path);
            }

            if let Some(path) = self.find_standalone_binary_linux().await {
                self.cache_binary(Some(path.clone()));
                return Some(path);
            }
        }

        self.cache_binary(None);
        None
    }

    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()> {
        let binary = self
            .find_binary()
            .await
            .ok_or(EditorError::BinaryNotFound)?;

        let mut args = vec![];

        if let Some(line) = options.line {
            args.push("--line".to_string());
            args.push(line.to_string());
        }

        if let Some(column) = options.column {
            args.push("--column".to_string());
            args.push(column.to_string());
        }

        args.push(path.display().to_string());

        debug!("Launching {} with args: {:?}", self.display_name, args);

        // Try to launch, with auto-retry on failure
        let launch_result = self.spawn_editor(&binary, &args);

        if let Err(e) = launch_result {
            warn!(
                "Failed to launch {}: {}. Invalidating cache and retrying...",
                self.display_name, e
            );

            // Invalidate cache
            self.cache_binary(None);

            // Retry with fresh binary discovery
            if let Some(retry_binary) = self.find_binary().await {
                debug!("Retrying with fresh binary: {:?}", retry_binary);
                return self
                    .spawn_editor(&retry_binary, &args)
                    .map_err(|e| EditorError::LaunchFailed(format!("Retry failed: {}", e)));
            }

            return Err(EditorError::LaunchFailed(e.to_string()));
        }

        Ok(())
    }

    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>> {
        #[cfg(target_os = "macos")]
        {
            let pattern = format!("/Applications/{}.app", self.display_name);
            let output = Command::new("pgrep")
                .arg("-f")
                .arg(&pattern)
                .output()
                .map_err(|e| EditorError::Other(e.to_string()))?;

            if !output.status.success() {
                return Ok(Vec::new());
            }

            let pids_str = String::from_utf8_lossy(&output.stdout);
            let instances: Vec<EditorInstance> = pids_str
                .lines()
                .filter_map(|line| line.parse::<u32>().ok())
                .map(|pid| EditorInstance {
                    pid,
                    workspace: None,
                    window_title: None,
                })
                .collect();

            Ok(instances)
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(Vec::new())
        }
    }
}
