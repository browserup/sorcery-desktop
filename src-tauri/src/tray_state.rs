use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tauri::image::Image;
use tauri::tray::TrayIcon;

const FLASH_DURATION_MS: u64 = 600;

struct TrayInner {
    tray_icon: Option<TrayIcon>,
    normal_icon: Image<'static>,
    active_icon: Image<'static>,
}

pub struct TrayState {
    inner: Arc<Mutex<TrayInner>>,
}

impl TrayState {
    pub fn new(normal_icon: Image<'static>, active_icon: Image<'static>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TrayInner {
                tray_icon: None,
                normal_icon,
                active_icon,
            })),
        }
    }

    pub fn set_tray_icon(&self, tray: TrayIcon) {
        self.inner.lock().tray_icon = Some(tray);
    }

    pub fn flash(&self) {
        let inner = Arc::clone(&self.inner);

        // Set to active icon immediately
        {
            let guard = inner.lock();
            let Some(tray) = guard.tray_icon.as_ref() else {
                return;
            };

            if let Err(e) = tray.set_icon(Some(guard.active_icon.clone())) {
                tracing::warn!("Failed to set active tray icon: {}", e);
                return;
            }
        }

        // Spawn task to restore after delay
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(FLASH_DURATION_MS)).await;

            let guard = inner.lock();
            if let Some(tray) = guard.tray_icon.as_ref() {
                if let Err(e) = tray.set_icon(Some(guard.normal_icon.clone())) {
                    tracing::warn!("Failed to restore tray icon: {}", e);
                }
            }
        });
    }
}
