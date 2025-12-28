use std::process::Command;
use tracing::debug;

pub async fn detect_active_editor() -> Option<String> {
    #[cfg(target_os = "macos")]
    return detect_active_editor_macos().await;

    #[cfg(target_os = "windows")]
    return detect_active_editor_windows().await;

    #[cfg(target_os = "linux")]
    return detect_active_editor_linux().await;
}

#[cfg(target_os = "macos")]
async fn detect_active_editor_macos() -> Option<String> {
    let app_name = get_frontmost_app_native()?.to_lowercase();

    debug!("Detected frontmost app: {}", app_name);

    if app_name == "electron" {
        if let Some(editor) = detect_vscodium_via_ps().await {
            return Some(editor);
        }
    }

    if app_name.contains("iterm") || app_name.contains("terminal") {
        if let Some(editor) = detect_terminal_editor().await {
            return Some(editor);
        }
    }

    map_app_name_to_editor(&app_name)
}

#[cfg(target_os = "macos")]
fn get_frontmost_app_native() -> Option<String> {
    use cocoa::base::nil;
    use objc::runtime::{Class, Object};
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let cls = Class::get("NSWorkspace")?;
        let workspace: *mut Object = msg_send![cls, sharedWorkspace];
        let frontmost_app: *mut Object = msg_send![workspace, frontmostApplication];

        if frontmost_app.is_null() || frontmost_app == nil as *mut Object {
            return None;
        }

        let name: *mut Object = msg_send![frontmost_app, localizedName];
        if name.is_null() || name == nil as *mut Object {
            return None;
        }

        let utf8: *const std::ffi::c_char = msg_send![name, UTF8String];
        if utf8.is_null() {
            return None;
        }

        let c_str = std::ffi::CStr::from_ptr(utf8);
        c_str.to_str().ok().map(|s| s.to_string())
    }
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
async fn detect_active_editor_windows() -> Option<String> {
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
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let title = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_lowercase();

    debug!("Detected window title: {}", title);

    map_window_title_to_editor(&title)
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
async fn detect_active_editor_linux() -> Option<String> {
    if let Some(title) = try_xdotool().await {
        return map_window_title_to_editor(&title);
    }

    if let Some(title) = try_wmctrl().await {
        return map_window_title_to_editor(&title);
    }

    None
}

#[cfg(target_os = "linux")]
async fn try_xdotool() -> Option<String> {
    let output = Command::new("xdotool")
        .args(["getactivewindow", "getwindowname"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_lowercase(),
    )
}

#[cfg(target_os = "linux")]
async fn try_wmctrl() -> Option<String> {
    let output = Command::new("wmctrl")
        .args(["-a", ":ACTIVE:"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_lowercase(),
    )
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
