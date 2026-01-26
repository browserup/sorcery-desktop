#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::process::Command;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use tracing::debug;

pub struct DetectionResult {
    pub editor_id: Option<String>,
    pub window_title: Option<String>,
}

pub async fn detect_active_editor() -> DetectionResult {
    #[cfg(target_os = "macos")]
    return detect_active_editor_macos().await;

    #[cfg(target_os = "windows")]
    return detect_active_editor_windows().await;

    #[cfg(target_os = "linux")]
    return detect_active_editor_linux().await;
}

#[cfg(target_os = "macos")]
async fn detect_active_editor_macos() -> DetectionResult {
    let (app_name, window_title) = get_frontmost_app_info_native();
    let app_name_lower = app_name.as_deref().map(str::to_lowercase);

    debug!(
        "Detected frontmost app: {:?}, title: {:?}",
        app_name, window_title
    );

    let editor_id = if let Some(ref name) = app_name_lower {
        if name == "electron" {
            detect_vscodium_via_ps()
                .await
                .or_else(|| map_app_name_to_editor(name))
        } else if name.contains("iterm") || name.contains("terminal") {
            detect_terminal_editor()
                .await
                .or_else(|| map_app_name_to_editor(name))
        } else {
            map_app_name_to_editor(name)
        }
    } else {
        None
    };

    DetectionResult {
        editor_id,
        window_title,
    }
}

#[cfg(target_os = "macos")]
fn get_frontmost_app_info_native() -> (Option<String>, Option<String>) {
    use objc2_app_kit::{NSRunningApplication, NSWorkspace};

    let workspace = NSWorkspace::sharedWorkspace();
    let frontmost_app: Option<objc2::rc::Retained<NSRunningApplication>> =
        workspace.frontmostApplication();

    let Some(app) = frontmost_app else {
        return (None, None);
    };

    let app_name = app.localizedName().map(|n| n.to_string());

    (app_name, None)
}

#[cfg(target_os = "macos")]
fn map_app_name_to_editor(app_name: &str) -> Option<String> {
    let editor_id = match app_name {
        s if s.contains("visual studio code") || s == "code" => "vscode",
        s if s.contains("cursor") => "cursor",
        s if s.contains("vscodium") => "vscodium",
        s if s == "roo" || s.starts_with("roo ") || s.ends_with(" roo") => "roo",
        s if s.contains("windsurf") => "windsurf",
        s if s.contains("intellij idea") || s == "idea" => "idea",
        s if s.contains("rubymine") => "rubymine",
        s if s.contains("pycharm") => "pycharm",
        s if s.contains("goland") => "goland",
        s if s.contains("webstorm") => "webstorm",
        s if s.contains("phpstorm") => "phpstorm",
        s if s.contains("rider") => "rider",
        s if s.contains("rustrover") => "rustrover",
        s if s.contains("clion") => "clion",
        s if s.contains("datagrip") => "datagrip",
        s if s.contains("appcode") => "appcode",
        s if s.contains("androidstudio") || s.contains("android studio") => "androidstudio",
        s if s.contains("fleet") => "fleet",
        s if s.contains("xcode") => "xcode",
        s if s.contains("eclipse") => "eclipse",
        s if s.contains("neovim") || s.contains("nvim") => "neovim",
        s if s.contains("macvim") => "vim",
        s if s.contains("vim") => "vim",
        s if s.contains("emacs") => "emacs",
        s if s.contains("zed") => "zed",
        s if s.contains("sublime text") => "sublime",
        _ => return None,
    };

    Some(editor_id.to_string())
}

#[cfg(target_os = "macos")]
async fn detect_vscodium_via_ps() -> Option<String> {
    let output = Command::new("ps").arg("aux").output().ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("VSCodium.app/Contents/MacOS/Electron") && !stdout.contains("Helper") {
        return Some("vscodium".to_string());
    }
    None
}

#[cfg(target_os = "macos")]
async fn detect_terminal_editor() -> Option<String> {
    let output = Command::new("ps").arg("aux").output().ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.contains(" nvim ") || stdout.contains(" neovim ") {
        return Some("neovim".to_string());
    }

    if stdout.contains("/vim ") || stdout.contains(" vim ") {
        return Some("vim".to_string());
    }

    None
}

#[cfg(target_os = "windows")]
async fn detect_active_editor_windows() -> DetectionResult {
    let ps_script = r#"
Add-Type @"
  using System;
  using System.Runtime.InteropServices;
  public class UserWindows {
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")]
    public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int count);
  }
"@
$handle = [UserWindows]::GetForegroundWindow()
$title = New-Object System.Text.StringBuilder 512
[UserWindows]::GetWindowText($handle, $title, 512)
$title.ToString()
"#;

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            ps_script,
        ])
        .output()
        .ok();

    let Some(output) = output else {
        return DetectionResult {
            editor_id: None,
            window_title: None,
        };
    };

    if !output.status.success() {
        return DetectionResult {
            editor_id: None,
            window_title: None,
        };
    }

    let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let title_lower = title.to_lowercase();

    debug!("Detected window title: {}", title);

    let editor_id = map_window_title_to_editor(&title_lower);

    DetectionResult {
        editor_id,
        window_title: Some(title),
    }
}

#[cfg(target_os = "windows")]
fn map_window_title_to_editor(title: &str) -> Option<String> {
    let editor_id = match title {
        s if s.contains("visual studio code") => "vscode",
        s if s.contains("cursor") => "cursor",
        s if s.contains("vscodium") => "vscodium",
        s if s.contains("roo code") => "roo",
        s if s.contains("windsurf") => "windsurf",
        s if s.contains("rubymine") => "rubymine",
        s if s.contains("goland") => "goland",
        s if s.contains("webstorm") => "webstorm",
        s if s.contains("pycharm") => "pycharm",
        s if s.contains("phpstorm") => "phpstorm",
        s if s.contains("rider") => "rider",
        s if s.contains("rustrover") => "rustrover",
        s if s.contains("clion") => "clion",
        s if s.contains("datagrip") => "datagrip",
        s if s.contains("intellij") => "idea",
        s if s.contains("android studio") => "androidstudio",
        s if s.contains("fleet") => "fleet",
        s if s.contains("eclipse") => "eclipse",
        s if s.contains("visual studio") => "visualstudio",
        s if s.contains("zed") => "zed",
        s if s.contains("sublime text") => "sublime",
        s if s.contains("notepad++") => "notepadplusplus",
        s if s.contains("vim") => "vim",
        s if s.contains("emacs") => "emacs",
        _ => return None,
    };

    Some(editor_id.to_string())
}

#[cfg(target_os = "linux")]
async fn detect_active_editor_linux() -> DetectionResult {
    let window_title = get_active_window_title_x11();
    let title_lower = window_title.as_ref().map(|t| t.to_lowercase());

    let editor_id = title_lower
        .as_ref()
        .and_then(|t| map_window_title_to_editor(t));

    DetectionResult {
        editor_id,
        window_title,
    }
}

#[cfg(target_os = "linux")]
fn get_active_window_title_x11() -> Option<String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};

    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;

    let net_active = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let prop = conn
        .get_property(false, root, net_active, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;

    if prop.format != 32 || prop.value.len() < 4 {
        return None;
    }
    let window_id = u32::from_ne_bytes(prop.value[0..4].try_into().ok()?);
    if window_id == 0 {
        return None;
    }

    let net_wm_name = conn
        .intern_atom(false, b"_NET_WM_NAME")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let utf8_string = conn
        .intern_atom(false, b"UTF8_STRING")
        .ok()?
        .reply()
        .ok()?
        .atom;

    let title_prop = conn
        .get_property(false, window_id, net_wm_name, utf8_string, 0, 1024)
        .ok()?
        .reply()
        .ok()?;

    if !title_prop.value.is_empty() {
        return String::from_utf8(title_prop.value).ok();
    }

    let wm_name = conn.intern_atom(false, b"WM_NAME").ok()?.reply().ok()?.atom;
    let title_prop = conn
        .get_property(false, window_id, wm_name, AtomEnum::STRING, 0, 1024)
        .ok()?
        .reply()
        .ok()?;

    Some(String::from_utf8_lossy(&title_prop.value).to_string())
}

#[cfg(target_os = "linux")]
fn map_window_title_to_editor(title: &str) -> Option<String> {
    let editor_id = match title {
        s if s.contains("visual studio code") => "vscode",
        s if s.contains("cursor") => "cursor",
        s if s.contains("vscodium") => "vscodium",
        s if s.contains("roo code") => "roo",
        s if s.contains("windsurf") => "windsurf",
        s if s.contains("neovim") || s.contains(" nvim") => "neovim",
        s if s.contains("vim") && !s.contains("nvim") => "vim",
        s if s.contains("emacs") => "emacs",
        s if s.contains("rubymine") => "rubymine",
        s if s.contains("goland") => "goland",
        s if s.contains("webstorm") => "webstorm",
        s if s.contains("pycharm") => "pycharm",
        s if s.contains("phpstorm") => "phpstorm",
        s if s.contains("rider") => "rider",
        s if s.contains("rustrover") => "rustrover",
        s if s.contains("clion") => "clion",
        s if s.contains("datagrip") => "datagrip",
        s if s.contains("intellij") => "idea",
        s if s.contains("android studio") => "androidstudio",
        s if s.contains("fleet") => "fleet",
        s if s.contains("eclipse") => "eclipse",
        s if s.contains("zed") => "zed",
        s if s.contains("sublime text") => "sublime",
        _ => return None,
    };

    Some(editor_id.to_string())
}
