use tauri::AppHandle;

pub struct DialogConfig {
    pub id: &'static str,
    pub html_file: &'static str,
    pub title: &'static str,
    pub width: f64,
    pub height: f64,
}

pub fn build_dialog(app_handle: &AppHandle, config: DialogConfig) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    activate_app();

    match tauri::WebviewWindowBuilder::new(
        app_handle,
        config.id,
        tauri::WebviewUrl::App(config.html_file.into()),
    )
    .title(config.title)
    .inner_size(config.width, config.height)
    .center()
    .resizable(false)
    .always_on_top(true)
    .focused(true)
    .accept_first_mouse(true)
    .build()
    {
        #[allow(unused_variables)]
        Ok(window) => {
            #[cfg(target_os = "macos")]
            set_dark_titlebar(&window);
            Ok(())
        }
        Err(e) => {
            let msg = format!("Failed to open {} dialog: {}", config.id, e);
            tracing::error!("{}", msg);
            Err(msg)
        }
    }
}

#[cfg(target_os = "macos")]
pub fn activate_app() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    if let Some(mtm) = MainThreadMarker::new() {
        let ns_app = NSApplication::sharedApplication(mtm);
        #[allow(deprecated)]
        ns_app.activateIgnoringOtherApps(true);
    }
}

#[cfg(target_os = "macos")]
pub fn set_dark_titlebar(window: &tauri::WebviewWindow) {
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSColor, NSWindow,
    };
    use tauri::Manager;

    let app_handle = window.app_handle().clone();
    let label = window.label().to_string();

    let _ = window.run_on_main_thread(move || {
        if let Some(win) = app_handle.get_webview_window(&label) {
            if let Ok(ns_window_ptr) = win.ns_window() {
                unsafe {
                    let ns_window: &NSWindow = &*(ns_window_ptr as *const NSWindow);

                    if let Some(appearance) =
                        NSAppearance::appearanceNamed(NSAppearanceNameDarkAqua)
                    {
                        ns_window.setAppearance(Some(&appearance));
                    }

                    let color = NSColor::colorWithRed_green_blue_alpha(0.071, 0.071, 0.071, 1.0);
                    ns_window.setBackgroundColor(Some(&color));
                }
            }
        }
    });
}
