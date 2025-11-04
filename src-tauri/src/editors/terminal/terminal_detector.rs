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

    pub fn launch_command(&self, command: &str) -> Result<(), String> {
        match self {
            #[cfg(target_os = "macos")]
            Self::ITerm2 => self.launch_iterm2(command),

            #[cfg(target_os = "macos")]
            Self::Alacritty => self.launch_alacritty_macos(command),

            #[cfg(target_os = "macos")]
            Self::Kitty => self.launch_kitty_macos(command),

            #[cfg(target_os = "macos")]
            Self::WezTerm => self.launch_wezterm_macos(command),

            #[cfg(target_os = "macos")]
            Self::Terminal => self.launch_terminal_app(command),

            #[cfg(target_os = "linux")]
            Self::Alacritty => self.launch_alacritty_linux(command),

            #[cfg(target_os = "linux")]
            Self::Kitty => self.launch_kitty_linux(command),

            #[cfg(target_os = "linux")]
            Self::WezTerm => self.launch_wezterm_linux(command),

            #[cfg(target_os = "linux")]
            Self::GnomeTerminal => self.launch_gnome_terminal(command),

            #[cfg(target_os = "linux")]
            Self::Konsole => self.launch_konsole(command),

            #[cfg(target_os = "linux")]
            Self::Xterm => self.launch_xterm(command),

            #[allow(unreachable_patterns)]
            _ => Err("Terminal not supported on this platform".to_string()),
        }
    }

    #[cfg(target_os = "macos")]
    fn launch_iterm2(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        let script = format!(
            "tell application \"iTerm\"\n\
             activate\n\
             create window with default profile\n\
             tell current session of current window\n\
                 write text \"{}\"\n\
             end tell\n\
             end tell",
            command.replace("\"", "\\\"")
        );

        Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch iTerm2: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn launch_alacritty_macos(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        Command::new("open")
            .arg("-a")
            .arg("Alacritty")
            .arg("-n")
            .arg("--args")
            .arg("-e")
            .arg("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Alacritty: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn launch_kitty_macos(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        Command::new("open")
            .arg("-a")
            .arg("kitty")
            .arg("-n")
            .arg("--args")
            .arg("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Kitty: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn launch_wezterm_macos(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        Command::new("open")
            .arg("-a")
            .arg("WezTerm")
            .arg("-n")
            .arg("--args")
            .arg("start")
            .arg("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch WezTerm: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn launch_terminal_app(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        let script = format!(
            "tell application \"Terminal\"\n\
             activate\n\
             do script \"{}\"\n\
             end tell",
            command.replace("\"", "\\\"")
        );

        Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Terminal: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_alacritty_linux(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        Command::new("alacritty")
            .arg("-e")
            .arg("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Alacritty: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_kitty_linux(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        Command::new("kitty")
            .arg("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Kitty: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_wezterm_linux(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        Command::new("wezterm")
            .arg("start")
            .arg("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch WezTerm: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_gnome_terminal(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        Command::new("gnome-terminal")
            .arg("--")
            .arg("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch GNOME Terminal: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_konsole(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        Command::new("konsole")
            .arg("-e")
            .arg("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch Konsole: {}", e))?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_xterm(&self, command: &str) -> Result<(), String> {
        use std::process::Stdio;

        Command::new("xterm")
            .arg("-e")
            .arg("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to launch xterm: {}", e))?;

        Ok(())
    }
}
