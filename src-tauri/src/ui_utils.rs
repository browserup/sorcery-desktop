#[cfg(target_os = "macos")]
pub fn set_dark_titlebar(window: &tauri::WebviewWindow) {
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSString;
    use objc::{class, msg_send, sel, sel_impl};
    use tauri::Manager;

    let app_handle = window.app_handle().clone();
    let label = window.label().to_string();

    let _ = window.run_on_main_thread(move || {
        if let Some(win) = app_handle.get_webview_window(&label) {
            if let Ok(ns_window) = win.ns_window() {
                unsafe {
                    let ns_window = ns_window as id;

                    let appearance_name =
                        cocoa::foundation::NSString::alloc(nil).init_str("NSAppearanceNameDarkAqua");
                    let appearance: id =
                        msg_send![class!(NSAppearance), appearanceNamed: appearance_name];
                    let _: () = msg_send![ns_window, setAppearance: appearance];

                    let color: id =
                        msg_send![class!(NSColor), colorWithRed:0.071 green:0.071 blue:0.071 alpha:1.0];
                    let _: () = msg_send![ns_window, setBackgroundColor: color];
                }
            }
        }
    });
}
