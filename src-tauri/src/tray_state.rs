use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tauri::image::Image;
use tauri::tray::TrayIcon;

const FLASH_DURATION_MS: u64 = 900;

struct TrayInner {
    handle: Option<TrayIcon>,
    normal: Image<'static>,
    active: Image<'static>,
}

pub struct TrayState {
    inner: Arc<Mutex<TrayInner>>,
}

impl TrayState {
    pub fn new(normal_icon: Image<'static>, active_icon: Image<'static>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TrayInner {
                handle: None,
                normal: normal_icon,
                active: active_icon,
            })),
        }
    }

    pub fn set_tray_icon(&self, tray: TrayIcon) {
        self.inner.lock().handle = Some(tray);
    }

    pub fn flash(&self) {
        let inner = Arc::clone(&self.inner);

        // Set to active icon immediately
        {
            let guard = inner.lock();
            let Some(tray) = guard.handle.as_ref() else {
                return;
            };

            if let Err(e) = tray.set_icon(Some(guard.active.clone())) {
                tracing::warn!("Failed to set active tray icon: {e}");
                return;
            }
        }

        // Spawn task to restore after delay
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(FLASH_DURATION_MS)).await;

            let guard = inner.lock();
            if let Some(tray) = guard.handle.as_ref() {
                if let Err(e) = tray.set_icon(Some(guard.normal.clone())) {
                    tracing::warn!("Failed to restore tray icon: {e}");
                }
            }
        });
    }
}
