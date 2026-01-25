use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct BrowserInfo {
    pub id: String,
    pub name: String,
    pub is_installed: bool,
}

pub struct BrowserDetector;

impl BrowserDetector {
    pub fn detect_browsers() -> Vec<BrowserInfo> {
        vec![
            BrowserInfo {
                id: "chrome".to_string(),
                name: "Google Chrome".to_string(),
                is_installed: Self::is_chrome_installed(),
            },
            BrowserInfo {
                id: "firefox".to_string(),
                name: "Mozilla Firefox".to_string(),
                is_installed: Self::is_firefox_installed(),
            },
            BrowserInfo {
                id: "edge".to_string(),
                name: "Microsoft Edge".to_string(),
                is_installed: Self::is_edge_installed(),
            },
        ]
    }

    pub fn is_chrome_installed() -> bool {
        Self::chrome_path().is_some()
    }

    pub fn is_firefox_installed() -> bool {
        Self::firefox_path().is_some()
    }

    pub fn is_edge_installed() -> bool {
        Self::edge_path().is_some()
    }

    #[cfg(target_os = "macos")]
    fn chrome_path() -> Option<PathBuf> {
        let candidates = [
            "/Applications/Google Chrome.app",
            "/Applications/Google Chrome Canary.app",
        ];

        for path in candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        if let Some(home) = dirs::home_dir() {
            let user_chrome = home.join("Applications/Google Chrome.app");
            if user_chrome.exists() {
                return Some(user_chrome);
            }
        }

        None
    }

    #[cfg(target_os = "linux")]
    fn chrome_path() -> Option<PathBuf> {
        let candidates = [
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/snap/bin/chromium",
        ];

        for path in candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    fn chrome_path() -> Option<PathBuf> {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(chrome_key) =
            hklm.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe")
        {
            if let Ok(path) = chrome_key.get_value::<String, _>("") {
                let p = PathBuf::from(&path);
                if p.exists() {
                    return Some(p);
                }
            }
        }

        let candidates = [
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ];

        if let Some(local_app_data) = dirs::data_local_dir() {
            let user_chrome = local_app_data.join(r"Google\Chrome\Application\chrome.exe");
            if user_chrome.exists() {
                return Some(user_chrome);
            }
        }

        for path in candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    #[cfg(target_os = "macos")]
    fn firefox_path() -> Option<PathBuf> {
        let candidates = [
            "/Applications/Firefox.app",
            "/Applications/Firefox Developer Edition.app",
            "/Applications/Firefox Nightly.app",
        ];

        for path in candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    #[cfg(target_os = "linux")]
    fn firefox_path() -> Option<PathBuf> {
        let candidates = [
            "/usr/bin/firefox",
            "/usr/bin/firefox-esr",
            "/snap/bin/firefox",
        ];

        for path in candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    fn firefox_path() -> Option<PathBuf> {
        let candidates = [
            r"C:\Program Files\Mozilla Firefox\firefox.exe",
            r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe",
        ];

        for path in candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    #[cfg(target_os = "macos")]
    fn edge_path() -> Option<PathBuf> {
        let p = PathBuf::from("/Applications/Microsoft Edge.app");
        if p.exists() {
            Some(p)
        } else {
            None
        }
    }

    #[cfg(target_os = "linux")]
    fn edge_path() -> Option<PathBuf> {
        let candidates = ["/usr/bin/microsoft-edge", "/usr/bin/microsoft-edge-stable"];

        for path in candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    fn edge_path() -> Option<PathBuf> {
        let candidates = [
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ];

        for path in candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_browsers_returns_list() {
        let browsers = BrowserDetector::detect_browsers();
        assert_eq!(browsers.len(), 3);
        assert_eq!(browsers[0].id, "chrome");
        assert_eq!(browsers[1].id, "firefox");
        assert_eq!(browsers[2].id, "edge");
    }
}
