use anyhow::Result;
use serde::Serialize;

#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[derive(Serialize, Clone, Debug)]
pub struct ProtocolRegistrationStatus {
    pub is_registered: bool,
    pub registered_executable: Option<String>,
    pub current_executable: String,
    pub executables_match: bool,
    pub platform: String,
    pub details: String,
}

/// Platform-specific protocol registration
#[allow(dead_code)]
pub struct ProtocolRegistration;

impl ProtocolRegistration {
    /// Get detailed protocol registration status
    pub fn get_status() -> ProtocolRegistrationStatus {
        #[cfg(target_os = "linux")]
        return Self::get_status_linux();

        #[cfg(target_os = "macos")]
        return Self::get_status_macos();

        #[cfg(target_os = "windows")]
        return Self::get_status_windows();

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return ProtocolRegistrationStatus {
            is_registered: false,
            registered_executable: None,
            current_executable: std::env::current_exe()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string()),
            executables_match: false,
            platform: "unsupported".to_string(),
            details: "Platform not supported for protocol registration".to_string(),
        };
    }

    /// Check if the protocol handler is registered
    #[cfg(target_os = "linux")]
    pub fn is_registered() -> bool {
        Self::is_registered_linux()
    }

    /// Register the protocol handler
    pub fn register() -> Result<()> {
        #[cfg(target_os = "linux")]
        return Self::register_linux();

        #[cfg(target_os = "macos")]
        return Self::register_macos();

        #[cfg(target_os = "windows")]
        anyhow::bail!("On Windows, protocol registration is handled by the installer. Please run the MSI installer.");

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        anyhow::bail!("Protocol registration not supported on this platform.");
    }

    #[cfg(target_os = "macos")]
    fn register_macos() -> Result<()> {
        use std::process::Command;

        let current_exe = std::env::current_exe()?;
        let exe_str = current_exe.to_string_lossy();

        // Find the .app bundle containing this executable
        let app_bundle = if exe_str.contains(".app/Contents/MacOS/") {
            let parts: Vec<&str> = exe_str.split(".app/Contents/MacOS/").collect();
            if !parts.is_empty() {
                format!("{}.app", parts[0])
            } else {
                anyhow::bail!(
                    "Could not determine app bundle path from executable: {}",
                    exe_str
                );
            }
        } else {
            anyhow::bail!("Executable is not inside an app bundle: {}", exe_str);
        };

        if !std::path::Path::new(&app_bundle).exists() {
            anyhow::bail!("App bundle not found: {}", app_bundle);
        }

        tracing::info!("Re-registering app bundle with lsregister: {}", app_bundle);

        let status = Command::new("/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister")
            .args(["-f", &app_bundle])
            .status()?;

        if status.success() {
            tracing::info!("Successfully re-registered protocol handler");
            Ok(())
        } else {
            anyhow::bail!("lsregister failed with exit code: {:?}", status.code());
        }
    }

    #[cfg(target_os = "linux")]
    fn get_status_linux() -> ProtocolRegistrationStatus {
        use std::fs;
        use std::process::Command;

        let current_exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let is_registered = Self::is_registered_linux();
        let mut registered_exe = None;
        let mut details = String::new();

        if is_registered {
            if let Ok(desktop_path) = Self::get_desktop_file_path() {
                if let Ok(content) = fs::read_to_string(&desktop_path) {
                    for line in content.lines() {
                        if line.starts_with("Exec=") {
                            let exec_line = line.trim_start_matches("Exec=");
                            let exe_path = exec_line.split_whitespace().next().unwrap_or("");
                            registered_exe = Some(exe_path.to_string());
                            break;
                        }
                    }
                }
                details = format!("Desktop file: {}", desktop_path.display());
            }
        } else {
            details = "Protocol not registered. Run the app to auto-register.".to_string();
        }

        let executables_match = if let Some(ref reg_exe) = registered_exe {
            reg_exe == &current_exe
        } else {
            false
        };

        ProtocolRegistrationStatus {
            is_registered,
            registered_executable: registered_exe,
            current_executable: current_exe,
            executables_match,
            platform: "linux".to_string(),
            details,
        }
    }

    #[cfg(target_os = "linux")]
    fn is_registered_linux() -> bool {
        use std::process::Command;

        // Check if srcuri.desktop is the default handler for srcuri://
        let output = Command::new("xdg-mime")
            .args(["query", "default", "x-scheme-handler/srcuri"])
            .output();

        if let Ok(output) = output {
            let handler = String::from_utf8_lossy(&output.stdout);
            handler.trim() == "srcuri.desktop"
        } else {
            false
        }
    }

    #[cfg(target_os = "linux")]
    fn register_linux() -> Result<()> {
        use std::fs;
        use std::process::Command;

        tracing::info!("Registering srcuri:// protocol handler for Linux");

        // Ensure .desktop file exists
        let desktop_file_path = Self::get_desktop_file_path()?;

        if !desktop_file_path.exists() {
            Self::create_desktop_file(&desktop_file_path)?;
        }

        // Register as default handler
        let status = Command::new("xdg-mime")
            .args(["default", "srcuri.desktop", "x-scheme-handler/srcuri"])
            .status()?;

        if status.success() {
            tracing::info!("Successfully registered srcuri:// protocol handler");
            Ok(())
        } else {
            anyhow::bail!("Failed to register protocol handler with xdg-mime");
        }
    }

    #[cfg(target_os = "linux")]
    fn get_desktop_file_path() -> Result<PathBuf> {
        // Check user applications first
        if let Some(home) = dirs::home_dir() {
            let user_apps = home.join(".local/share/applications/srcuri.desktop");
            if user_apps.exists() {
                return Ok(user_apps);
            }
        }

        // Check system applications
        let system_apps = PathBuf::from("/usr/share/applications/srcuri.desktop");
        if system_apps.exists() {
            return Ok(system_apps);
        }

        // Default to user location (we'll create it there)
        if let Some(home) = dirs::home_dir() {
            let user_apps = home.join(".local/share/applications/srcuri.desktop");

            // Ensure directory exists
            if let Some(parent) = user_apps.parent() {
                std::fs::create_dir_all(parent)?;
            }

            Ok(user_apps)
        } else {
            anyhow::bail!("Could not determine home directory");
        }
    }

    #[cfg(target_os = "linux")]
    fn create_desktop_file(path: &PathBuf) -> Result<()> {
        use std::fs;

        tracing::info!("Creating desktop file at {:?}", path);

        // Get the executable path
        let exe_path = std::env::current_exe()?;
        let exe_path_str = exe_path.to_string_lossy();

        let desktop_content = format!(
            r#"[Desktop Entry]
Version=1.0
Type=Application
Name=Sorcery Desktop
Comment=Editor-agnostic deep link handler
Exec={} %u
Icon=sorcery
Terminal=false
Categories=Development;Utility;
MimeType=x-scheme-handler/srcuri;
StartupWMClass=sorcery-desktop
"#,
            exe_path_str
        );

        fs::write(path, desktop_content)?;

        // Update desktop database
        if let Some(parent) = path.parent() {
            let _ = std::process::Command::new("update-desktop-database")
                .arg(parent)
                .status();
        }

        tracing::info!("Desktop file created successfully");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn get_status_macos() -> ProtocolRegistrationStatus {
        use std::process::Command;

        let current_exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let mut is_registered = false;
        let mut registered_exe = None;
        let mut details = String::from("Checking default handler...");

        // Much faster approach: Use LSCopyDefaultHandlerForURLScheme equivalent
        // We'll check common app locations first
        let home = std::env::var("HOME").unwrap_or_default();
        let possible_paths = vec![
            "/Applications/Sorcery Desktop.app".to_string(),
            "/Applications/srcuri.app".to_string(),
            format!("{}/Applications/Sorcery Desktop.app", home),
            format!("{}/Applications/srcuri.app", home),
        ];

        for app_path in &possible_paths {
            let plist_path = format!("{}/Contents/Info.plist", app_path);
            if std::path::Path::new(&plist_path).exists() {
                // Check if this plist has srcuri in CFBundleURLSchemes
                let output = Command::new("defaults")
                    .args(["read", &plist_path, "CFBundleURLTypes"])
                    .output();

                if let Ok(output) = output {
                    let plist_content = String::from_utf8_lossy(&output.stdout);
                    if plist_content.contains("srcuri") {
                        is_registered = true;
                        // Get actual executable name from CFBundleExecutable
                        let exe_name = Command::new("defaults")
                            .args(["read", &plist_path, "CFBundleExecutable"])
                            .output()
                            .ok()
                            .and_then(|o| String::from_utf8(o.stdout).ok())
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|| "sorcery-desktop".to_string());
                        let exe_path = format!("{}/Contents/MacOS/{}", app_path, exe_name);
                        registered_exe = Some(exe_path);
                        details = format!("App bundle: {}", app_path);
                        break;
                    }
                }
            }
        }

        // If not found in common locations, check if current executable is in an app bundle
        if !is_registered && current_exe.contains(".app/Contents/MacOS/") {
            let parts: Vec<&str> = current_exe.split(".app/Contents/MacOS/").collect();
            if !parts.is_empty() {
                let app_bundle = format!("{}.app", parts[0]);
                let plist_path = format!("{}/Contents/Info.plist", app_bundle);

                if std::path::Path::new(&plist_path).exists() {
                    let output = Command::new("defaults")
                        .args(["read", &plist_path, "CFBundleURLTypes"])
                        .output();

                    if let Ok(output) = output {
                        let plist_content = String::from_utf8_lossy(&output.stdout);
                        if plist_content.contains("srcuri") {
                            is_registered = true;
                            registered_exe = Some(current_exe.clone());
                            details = format!("App bundle: {}", app_bundle);
                        }
                    }
                }
            }
        }

        if !is_registered {
            details = "Protocol not registered. Run 'make install' or register via installer."
                .to_string();
        }

        let executables_match = if let Some(ref reg_exe) = registered_exe {
            reg_exe == &current_exe
        } else {
            false
        };

        ProtocolRegistrationStatus {
            is_registered,
            registered_executable: registered_exe,
            current_executable: current_exe,
            executables_match,
            platform: "macos".to_string(),
            details,
        }
    }

    #[cfg(target_os = "windows")]
    fn get_status_windows() -> ProtocolRegistrationStatus {
        use std::process::Command;

        let current_exe = std::env::current_exe()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let mut is_registered = false;
        let mut registered_exe = None;
        let mut details = String::from("Checking Windows Registry...");

        let output = Command::new("reg")
            .args([
                "query",
                "HKEY_CLASSES_ROOT\\srcuri\\shell\\open\\command",
                "/ve",
            ])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let reg_output = String::from_utf8_lossy(&output.stdout);
                is_registered = true;

                for line in reg_output.lines() {
                    if line.contains("REG_SZ") {
                        let parts: Vec<&str> = line.split("REG_SZ").collect();
                        if parts.len() > 1 {
                            let command = parts[1].trim();
                            let exe_path = command
                                .trim_start_matches('"')
                                .split('"')
                                .next()
                                .unwrap_or("")
                                .to_string();
                            registered_exe = Some(exe_path);
                            details = format!("Registry: HKEY_CLASSES_ROOT\\srcuri");
                            break;
                        }
                    }
                }
            }
        }

        if !is_registered {
            details = "Protocol not registered. Run the MSI installer.".to_string();
        }

        let executables_match = if let Some(ref reg_exe) = registered_exe {
            reg_exe == &current_exe
        } else {
            false
        };

        ProtocolRegistrationStatus {
            is_registered,
            registered_executable: registered_exe,
            current_executable: current_exe,
            executables_match,
            platform: "windows".to_string(),
            details,
        }
    }
}
