use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use tracing::debug;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TerminalApp {
    ITerm2,
    Alacritty,
    Kitty,
    WezTerm,
    Terminal,      // macOS Terminal.app
    GnomeTerminal, // Linux
    Konsole,       // Linux KDE
    Xterm,         // Linux fallback
}

impl TerminalApp {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "iterm2" | "iterm" => Some(Self::ITerm2),
            "alacritty" => Some(Self::Alacritty),
            "kitty" => Some(Self::Kitty),
            "wezterm" => Some(Self::WezTerm),
            "terminal" => Some(Self::Terminal),
            "gnome-terminal" | "gnome" => Some(Self::GnomeTerminal),
            "konsole" => Some(Self::Konsole),
            "xterm" => Some(Self::Xterm),
            "auto" | "" => None,
            _ => None,
        }
    }

    #[cfg(target_os = "macos")]
    pub fn detect_installed_with_preference(preferred: Option<&str>) -> Option<Self> {
        // If user has a preference, check if it's installed first
        if let Some(pref) = preferred {
            if pref != "auto" {
                if let Some(terminal) = Self::from_string(pref) {
                    if Self::is_installed(&terminal) {
                        debug!("Using preferred terminal: {:?}", terminal);
                        return Some(terminal);
                    } else {
                        debug!(
                            "Preferred terminal {:?} not installed, falling back to auto-detect",
                            terminal
                        );
                    }
                }
            }
        }

        let terminals = vec![
            (Self::ITerm2, "/Applications/iTerm.app"),
            (Self::Alacritty, "/Applications/Alacritty.app"),
            (Self::Kitty, "/Applications/kitty.app"),
            (Self::WezTerm, "/Applications/WezTerm.app"),
            (
                Self::Terminal,
                "/System/Applications/Utilities/Terminal.app",
            ),
        ];

        for (terminal, path) in terminals {
            if PathBuf::from(path).exists() {
                debug!("Found terminal: {:?} at {}", terminal, path);
                return Some(terminal);
            }
        }

        // Terminal.app always exists on macOS as fallback
        Some(Self::Terminal)
    }

    #[cfg(target_os = "macos")]
    fn is_installed(&self) -> bool {
        let path = match self {
            Self::ITerm2 => "/Applications/iTerm.app",
            Self::Alacritty => "/Applications/Alacritty.app",
            Self::Kitty => "/Applications/kitty.app",
            Self::WezTerm => "/Applications/WezTerm.app",
            Self::Terminal => "/System/Applications/Utilities/Terminal.app",
            _ => return false,
        };
        PathBuf::from(path).exists()
    }

    #[cfg(target_os = "linux")]
    pub fn detect_installed_with_preference(preferred: Option<&str>) -> Option<Self> {
        // If user has a preference, check if it's installed first
        if let Some(pref) = preferred {
            if pref != "auto" {
                if let Some(terminal) = Self::from_string(pref) {
                    if Self::is_installed(&terminal) {
                        debug!("Using preferred terminal: {:?}", terminal);
                        return Some(terminal);
                    } else {
                        debug!(
                            "Preferred terminal {:?} not installed, falling back to auto-detect",
                            terminal
                        );
                    }
                }
            }
        }

        let terminals = vec![
            (Self::Alacritty, "alacritty"),
            (Self::Kitty, "kitty"),
            (Self::WezTerm, "wezterm"),
            (Self::GnomeTerminal, "gnome-terminal"),
            (Self::Konsole, "konsole"),
            (Self::Xterm, "xterm"),
        ];

        for (terminal, bin) in terminals {
            if Self::is_command_available(bin) {
                debug!("Found terminal: {:?} via command {}", terminal, bin);
                return Some(terminal);
            }
        }

        None
    }

    #[cfg(target_os = "linux")]
    fn is_installed(&self) -> bool {
        let cmd = match self {
            Self::Alacritty => "alacritty",
            Self::Kitty => "kitty",
            Self::WezTerm => "wezterm",
            Self::GnomeTerminal => "gnome-terminal",
            Self::Konsole => "konsole",
            Self::Xterm => "xterm",
            _ => return false,
        };
        Self::is_command_available(cmd)
    }

    #[cfg(target_os = "linux")]
    fn is_command_available(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub fn launch_editor(&self, editor: &str, args: &[String]) -> Result<(), String> {
        debug!("Launching editor '{}' with args: {:?}", editor, args);
        match self {
            #[cfg(target_os = "macos")]
            Self::ITerm2 => self.launch_via_script("iTerm", editor, args),

            #[cfg(target_os = "macos")]
            Self::Terminal => self.launch_via_script("Terminal", editor, args),

            #[cfg(target_os = "macos")]
            Self::Alacritty => self.launch_alacritty_macos_direct(editor, args),

            #[cfg(target_os = "macos")]
            Self::Kitty => self.launch_kitty_macos_direct(editor, args),

            #[cfg(target_os = "macos")]
            Self::WezTerm => self.launch_wezterm_macos_direct(editor, args),

            #[cfg(target_os = "linux")]
            Self::Alacritty => self.launch_alacritty_linux_direct(editor, args),

            #[cfg(target_os = "linux")]
            Self::Kitty => self.launch_kitty_linux_direct(editor, args),

            #[cfg(target_os = "linux")]
            Self::WezTerm => self.launch_wezterm_linux_direct(editor, args),

            #[cfg(target_os = "linux")]
            Self::GnomeTerminal => self.launch_gnome_terminal_direct(editor, args),

            #[cfg(target_os = "linux")]
            Self::Konsole => self.launch_konsole_direct(editor, args),

            #[cfg(target_os = "linux")]
            Self::Xterm => self.launch_xterm_direct(editor, args),

            #[allow(unreachable_patterns)]
            _ => Err("Terminal not supported on this platform".to_string()),
        }
    }

    #[cfg(target_os = "macos")]
    fn launch_via_script(&self, app_name: &str, editor: &str, args: &[String]) -> Result<(), String> {
        use std::process::Stdio;
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let script_path = format!("/tmp/sorcery_launch_{}.sh", timestamp);

        let mut script_content = String::from("#!/bin/bash\n");
        script_content.push_str(&shell_escape::escape(editor.into()));
        for arg in args {
            script_content.push(' ');
            script_content.push_str(&shell_escape::escape(arg.into()));
        }
        script_content.push('\n');

        let mut file = fs::File::create(&script_path)
            .map_err(|e| format!("Failed to create launch script: {}", e))?;
        file.write_all(script_content.as_bytes())
            .map_err(|e| format!("Failed to write launch script: {}", e))?;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set script permissions: {}", e))?;

        Command::new("open")
            .arg("-a")
            .arg(app_name)
            .arg(&script_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch {}: {}", app_name, e))?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn launch_alacritty_macos_direct(&self, editor: &str, args: &[String]) -> Result<(), String> {
        use std::process::Stdio;

        let mut cmd = Command::new("open");
        cmd.arg("-a")
            .arg("Alacritty")
            .arg("-n")
            .arg("--args")
            .arg("-e")
            .arg(editor);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Alacritty: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn launch_kitty_macos_direct(&self, editor: &str, args: &[String]) -> Result<(), String> {
        use std::process::Stdio;

        let mut cmd = Command::new("open");
        cmd.arg("-a")
            .arg("kitty")
            .arg("-n")
            .arg("--args")
            .arg(editor);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Kitty: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn launch_wezterm_macos_direct(&self, editor: &str, args: &[String]) -> Result<(), String> {
        use std::process::Stdio;

        let mut cmd = Command::new("open");
        cmd.arg("-a")
            .arg("WezTerm")
            .arg("-n")
            .arg("--args")
            .arg("start")
            .arg("--")
            .arg(editor);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch WezTerm: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_alacritty_linux_direct(&self, editor: &str, args: &[String]) -> Result<(), String> {
        use std::process::Stdio;

        let mut cmd = Command::new("alacritty");
        cmd.arg("-e").arg(editor);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Alacritty: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_kitty_linux_direct(&self, editor: &str, args: &[String]) -> Result<(), String> {
        use std::process::Stdio;

        let mut cmd = Command::new("kitty");
        cmd.arg(editor);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Kitty: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_wezterm_linux_direct(&self, editor: &str, args: &[String]) -> Result<(), String> {
        use std::process::Stdio;

        let mut cmd = Command::new("wezterm");
        cmd.arg("start").arg("--").arg(editor);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch WezTerm: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_gnome_terminal_direct(&self, editor: &str, args: &[String]) -> Result<(), String> {
        use std::process::Stdio;

        let mut cmd = Command::new("gnome-terminal");
        cmd.arg("--").arg(editor);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch GNOME Terminal: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_konsole_direct(&self, editor: &str, args: &[String]) -> Result<(), String> {
        use std::process::Stdio;

        let mut cmd = Command::new("konsole");
        cmd.arg("-e").arg(editor);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Konsole: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_xterm_direct(&self, editor: &str, args: &[String]) -> Result<(), String> {
        use std::process::Stdio;

        let mut cmd = Command::new("xterm");
        cmd.arg("-e").arg(editor);

        for arg in args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch xterm: {}", e))?;

        Ok(())
    }
}
