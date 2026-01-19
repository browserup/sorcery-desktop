// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod dialog_state;
mod dispatcher;
mod editors;
mod git_command_log;
mod path_validator;
mod protocol_handler;
mod protocol_registration;
mod settings;
mod tracker;
mod ui_utils;
mod workspace_mru;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Listener, Manager,
};
use tauri_plugin_deep_link::DeepLinkExt;
use tracing_subscriber::EnvFilter;

use crate::git_command_log::GIT_COMMAND_LOG;

#[cfg(target_os = "macos")]
fn hide_app() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    if let Some(mtm) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(mtm);
        app.hide(None);
    }
}

use dialog_state::DialogState;
use ui_utils::{build_dialog, DialogConfig};
#[cfg(target_os = "macos")]
use ui_utils::set_dark_titlebar;

async fn handle_protocol_result(
    result: Result<protocol_handler::HandleResult, anyhow::Error>,
    app_handle: &AppHandle,
    url: &str,
    duration: Duration,
) {
    let dialog_state = app_handle.state::<Arc<DialogState>>();

    match result {
        Ok(protocol_handler::HandleResult::Opened { file_path }) => {
            tracing::info!("Request: file opened successfully: {}", file_path);
            GIT_COMMAND_LOG.log_request(url, true, "opened", &file_path, duration);
            #[cfg(target_os = "macos")]
            hide_app();
        }
        Ok(protocol_handler::HandleResult::ShowChooser {
            matches,
            line,
            column,
        }) => {
            let match_count = matches.len();
            tracing::info!(
                "Request: showing workspace chooser with {} matches",
                match_count
            );
            GIT_COMMAND_LOG.log_request(
                url,
                true,
                "chooser",
                &format!("{} matching workspaces found", match_count),
                duration,
            );
            dialog_state.set_workspace_chooser(dialog_state::WorkspaceChooserData {
                matches,
                line,
                column,
            });
            let _ = build_dialog(
                app_handle,
                DialogConfig {
                    id: "workspace-chooser",
                    html_file: "workspace-chooser.html",
                    title: "Choose Workspace",
                    width: 600.0,
                    height: 500.0,
                },
            );
        }
        Ok(protocol_handler::HandleResult::ShowRevisionDialog {
            workspace,
            workspace_path,
            file_path,
            full_file_path,
            rev,
            line,
            column,
            current_ref,
            is_working_tree_clean,
            dirty_file_count,
            checkout_available,
            checkout_blocked_reason,
        }) => {
            tracing::info!("Request: showing revision dialog for {}@{}", file_path, rev);
            GIT_COMMAND_LOG.log_request(
                url,
                true,
                "revision_dialog",
                &format!("Revision {} requires checkout", rev),
                duration,
            );
            dialog_state.set_revision_dialog(dialog_state::RevisionDialogData {
                workspace,
                workspace_path: workspace_path.to_string_lossy().to_string(),
                file_path,
                full_file_path: full_file_path.to_string_lossy().to_string(),
                rev,
                line,
                column,
                current_ref,
                is_working_tree_clean,
                dirty_file_count,
                checkout_available,
                checkout_blocked_reason,
            });
            let _ = build_dialog(
                app_handle,
                DialogConfig {
                    id: "revision-handler",
                    html_file: "revision-handler.html",
                    title: "Open File at Revision",
                    width: 600.0,
                    height: 450.0,
                },
            );
        }
        Ok(protocol_handler::HandleResult::ShowCloneDialog {
            workspace_name,
            clone_path,
            remote_url,
            file_path,
            line,
            column,
            git_ref,
        }) => {
            tracing::info!(
                "Request: showing clone dialog for {} from {}",
                workspace_name,
                remote_url
            );
            GIT_COMMAND_LOG.log_request(
                url,
                true,
                "clone_dialog",
                &format!(
                    "Workspace '{}' not found, offering clone from {}",
                    workspace_name, remote_url
                ),
                duration,
            );
            let git_ref_str = git_ref.as_ref().map(|r| dialog_state::git_ref_display(r));
            dialog_state.set_clone_dialog(dialog_state::CloneDialogData {
                workspace_name,
                clone_path,
                remote_url,
                file_path,
                line,
                column,
                git_ref: git_ref_str,
                git_ref_kind: git_ref.clone(),
            });
            let _ = build_dialog(
                app_handle,
                DialogConfig {
                    id: "clone-dialog",
                    html_file: "clone-dialog.html",
                    title: "Clone Repository",
                    width: 520.0,
                    height: 380.0,
                },
            );
        }
        Ok(protocol_handler::HandleResult::ShowLargeFileDialog {
            file_path,
            file_size_bytes,
            line,
            column,
            editor_hint,
        }) => {
            let size_mb = file_size_bytes as f64 / (1024.0 * 1024.0);
            tracing::info!(
                "Request: showing large file warning for {} ({:.1} MB)",
                file_path,
                size_mb
            );
            GIT_COMMAND_LOG.log_request(
                url,
                true,
                "large_file_dialog",
                &format!("File is {:.1} MB, requesting confirmation", size_mb),
                duration,
            );
            dialog_state.set_large_file_dialog(dialog_state::LargeFileDialogData {
                file_path,
                file_size_bytes,
                line,
                column,
                editor_hint,
            });
            let _ = build_dialog(
                app_handle,
                DialogConfig {
                    id: "large-file-confirm",
                    html_file: "large-file-confirm.html",
                    title: "Large File Warning",
                    width: 500.0,
                    height: 280.0,
                },
            );
        }
        Ok(protocol_handler::HandleResult::OpenInBrowser { url: browser_url }) => {
            tracing::info!("Request: opening in browser: {}", browser_url);
            GIT_COMMAND_LOG.log_request(
                url,
                true,
                "browser",
                &format!("Opening in browser: {}", browser_url),
                duration,
            );
            if let Err(e) = open::that(&browser_url) {
                tracing::error!("Failed to open browser: {}", e);
            }
        }
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Request: failed to handle URL: {}", error_msg);
            GIT_COMMAND_LOG.log_request(url, false, "error", &error_msg, duration);
        }
    }
}

const MIN_DEEP_LINK_INTERVAL_MS: u64 = 200;

struct DeepLinkThrottle {
    min_interval: Duration,
    last_allowed: Mutex<Option<Instant>>,
}

impl DeepLinkThrottle {
    fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last_allowed: Mutex::new(None),
        }
    }

    fn allow(&self) -> bool {
        let now = Instant::now();
        let mut guard = self
            .last_allowed
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());

        if let Some(last) = *guard {
            if now.duration_since(last) < self.min_interval {
                return false;
            }
        }

        *guard = Some(now);
        true
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting Sorcery Desktop...");

    let settings_manager = Arc::new(settings::SettingsManager::new().await?);
    let path_validator = Arc::new(path_validator::PathValidator::new(settings_manager.clone()));
    let editor_registry = Arc::new(editors::EditorRegistry::new());
    let workspace_tracker = Arc::new(workspace_mru::ActiveWorkspaceTracker::new(
        settings_manager.clone(),
    ));
    let tracker = Arc::new(
        tracker::ActiveEditorTracker::new(editor_registry.clone()).with_workspace_tracking(
            settings_manager.clone(),
            workspace_tracker.clone(),
        ),
    );
    let workspace_sync = Arc::new(settings::WorkspaceSync::new(settings_manager.clone()));
    let dispatcher = Arc::new(dispatcher::EditorDispatcher::new(
        settings_manager.clone(),
        path_validator.clone(),
        editor_registry.clone(),
        tracker.clone(),
    ));
    let protocol_handler = Arc::new(protocol_handler::ProtocolHandler::new(
        settings_manager.clone(),
        dispatcher.clone(),
        workspace_tracker.clone(),
    ));
    let dialog_state = Arc::new(DialogState::new());
    let deep_link_throttle = Arc::new(DeepLinkThrottle::new(Duration::from_millis(
        MIN_DEEP_LINK_INTERVAL_MS,
    )));

    settings_manager.load().await?;
    tracing::info!("Settings loaded");

    // Sync workspaces from default_workspaces_folder
    if let Err(e) = workspace_sync.sync().await {
        tracing::warn!("Failed to sync workspaces: {}", e);
    }

    tracker.load().await?;
    tracing::info!("Last seen data loaded");

    workspace_tracker.load().await?;
    tracing::info!("Workspace MRU data loaded");

    let tracker_handle = tracker.clone();
    tokio::spawn(async move {
        tracing::info!("Starting active editor tracker...");
        tracker_handle.start_polling().await;
    });

    tracing::info!("All services initialized");

    // Check protocol registration status on startup
    {
        let status = protocol_registration::ProtocolRegistration::get_status();
        if !status.is_registered {
            tracing::warn!("Protocol handler not registered: {}", status.details);
            #[cfg(target_os = "linux")]
            {
                tracing::info!("Attempting auto-registration...");
                if let Err(e) = protocol_registration::ProtocolRegistration::register() {
                    tracing::warn!("Failed to auto-register protocol handler: {}. You may need to run: xdg-mime default srcuri.desktop x-scheme-handler/srcuri", e);
                }
            }
        } else if !status.executables_match {
            tracing::warn!(
                "Protocol handler registered to different executable. Registered: {:?}, Current: {}",
                status.registered_executable,
                status.current_executable
            );
        } else {
            tracing::info!("Protocol handler registered correctly");
        }
    }

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let url = &args[1];
        if url.starts_with("srcuri://") {
            tracing::info!("Processing command-line URL: {}", url);
            match protocol_handler.handle_url(url).await {
                Ok(protocol_handler::HandleResult::Opened { file_path }) => {
                    tracing::info!("File opened successfully via command-line: {}", file_path);
                    return Ok(());
                }
                Ok(_) => {
                    tracing::warn!(
                        "Command-line URL requires UI interaction (not supported in CLI mode)"
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::error!("Failed to handle command-line URL: {}", e);
                    return Err(e.into());
                }
            }
        }
    }

    let protocol_handler_clone = protocol_handler.clone();
    let deep_link_throttle_for_setup = deep_link_throttle.clone();

    tauri::Builder::default()
        .setup(move |app| {
            let deep_link_throttle = deep_link_throttle_for_setup.clone();
            tracing::info!("Setting up Tauri app...");

            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            // Hide all windows - we run as a background service
            for (_label, window) in app.webview_windows() {
                let _ = window.hide();
            }

            let app_handle = app.handle().clone();
            let ph = protocol_handler_clone.clone();
            let ph_cold_start = protocol_handler_clone.clone();

            let throttle_for_listener = deep_link_throttle.clone();
            app.handle().listen("deep-link://new-url", move |event| {
                let payload = event.payload();
                let event_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                tracing::debug!(
                    "Deep link event received at {}ms - raw payload: {}",
                    event_time,
                    payload
                );

                let urls: Vec<String> = match serde_json::from_str(payload) {
                    Ok(urls) => urls,
                    Err(e) => {
                        tracing::error!("Failed to parse deep link payload: {}", e);
                        return;
                    }
                };

                if urls.is_empty() {
                    tracing::warn!("Received empty deep-link URL list");
                    return;
                }

                let url = urls[0].clone();
                if !throttle_for_listener.allow() {
                    tracing::warn!(
                        "Dropping deep-link URL due to 200ms rate limit: {}",
                        url
                    );
                    return;
                }
                tracing::debug!("Processing deep-link URL: {}", url);

                #[cfg(target_os = "macos")]
                hide_app();

                let app_handle = app_handle.clone();
                let ph = ph.clone();

                tauri::async_runtime::spawn(async move {
                    tracing::debug!("Spawned async task for URL: {}", url);
                    let start = std::time::Instant::now();
                    let result = ph.handle_url(&url).await;
                    tracing::debug!(
                        "handle_url completed in {:?}, result: {:?}",
                        start.elapsed(),
                        result.is_ok()
                    );
                    handle_protocol_result(result, &app_handle, &url, start.elapsed()).await;
                });
            });

            tracing::info!("Application ready");

            // Check for URLs that launched the app (cold start)
            // On macOS, URLs used to launch the app are delivered before the event listener is ready
            #[cfg(target_os = "macos")]
            {
                let throttle_for_cold_start = deep_link_throttle.clone();
                let deep_link = app.deep_link();
                if let Ok(Some(urls)) = deep_link.get_current() {
                    if let Some(url) = urls.first() {
                        let url_str = url.to_string();
                        tracing::info!("Processing cold-start URL: {}", url_str);
                        if !throttle_for_cold_start.allow() {
                            tracing::warn!(
                                "Dropping cold-start deep-link URL due to 200ms rate limit: {}",
                                url_str
                            );
                        } else {
                            hide_app();
                            let app_handle = app.handle().clone();
                            let ph = ph_cold_start.clone();
                            tauri::async_runtime::spawn(async move {
                                let start = std::time::Instant::now();
                                let result = ph.handle_url(&url_str).await;
                                handle_protocol_result(result, &app_handle, &url_str, start.elapsed())
                                    .await;
                            });
                        }
                    }
                }
            }

            let settings_item =
                MenuItem::with_id(app, "settings", "Open Sorcery", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

            // Create system tray icon
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Sorcery Desktop - Editor Link Handler")
                .icon(
                    app.default_window_icon()
                        .expect("app icon must be configured in tauri.conf.json")
                        .clone(),
                )
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "settings" => {
                            // Activate the app to bring it to the foreground
                            #[cfg(target_os = "macos")]
                            {
                                use objc2::MainThreadMarker;
                                use objc2_app_kit::NSApplication;
                                let mtm = MainThreadMarker::new()
                                    .expect("menu events run on main thread");
                                let ns_app = NSApplication::sharedApplication(mtm);
                                #[allow(deprecated)]
                                ns_app.activateIgnoringOtherApps(true);
                            }

                            if let Some(window) = app.get_webview_window("settings") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            } else {
                                match tauri::WebviewWindowBuilder::new(
                                    app,
                                    "settings",
                                    tauri::WebviewUrl::App("settings.html".into()),
                                )
                                .title("")
                                .inner_size(800.0, 600.0)
                                .center()
                                .resizable(true)
                                .focused(true)
                                .build()
                                {
                                    Ok(window) => {
                                        #[cfg(target_os = "macos")]
                                        set_dark_titlebar(&window);

                                        #[cfg(debug_assertions)]
                                        window.open_devtools();

                                        tracing::info!("Settings window opened");
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to open settings window: {}", e);
                                    }
                                }
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .manage(settings_manager)
        .manage(path_validator)
        .manage(editor_registry)
        .manage(tracker)
        .manage(workspace_tracker)
        .manage(dispatcher)
        .manage(protocol_handler)
        .manage(workspace_sync)
        .manage(dialog_state)
        .plugin(tauri_plugin_single_instance::init({
            let throttle = deep_link_throttle.clone();
            move |app, args, _cwd| {
                tracing::debug!("Single-instance callback triggered, args: {:?}", args);

            // Second instance launched - check if it's a URL or a direct app launch
            let mut handled_url = false;
            if args.len() > 1 {
                if let Some(url) = args.get(1) {
                    if url.starts_with("srcuri://") {
                        if !throttle.allow() {
                            handled_url = true;
                            tracing::warn!(
                                "Dropping single-instance deep-link URL due to 200ms rate limit: {}",
                                url
                            );
                        } else {
                            tracing::debug!("Forwarding URL to existing instance: {}", url);
                            if let Err(e) = app.emit("deep-link://new-url", vec![url.clone()]) {
                                tracing::error!("Failed to emit deep-link event: {}", e);
                            } else {
                                handled_url = true;
                            }
                        }
                    }
                }
            }

            // Only activate the app if user explicitly launched it (not via URL)
            if !handled_url {
                #[cfg(target_os = "macos")]
                {
                    use objc2::MainThreadMarker;
                    use objc2_app_kit::NSApplication;
                    let mtm = MainThreadMarker::new()
                        .expect("single-instance callback runs on main thread");
                    let ns_app = NSApplication::sharedApplication(mtm);
                    #[allow(deprecated)]
                    ns_app.activateIgnoringOtherApps(true);
                }
            }
        }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide window instead of closing, keep app running in tray
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::get_settings_path,
            commands::save_settings,
            commands::get_all_workspaces,
            commands::promote_workspace,
            commands::sync_workspaces,
            commands::delete_workspace,
            commands::get_editor_testbed_data,
            commands::test_open_file,
            commands::open_in_editor,
            commands::detect_source_folder,
            commands::get_workspace_chooser_data,
            commands::workspace_chosen,
            commands::workspace_chooser_cancelled,
            commands::get_revision_dialog_data,
            commands::get_git_revision_info,
            commands::open_file_at_revision,
            commands::revision_dialog_cancelled,
            commands::refresh_working_tree_status,
            commands::create_worktree_and_open,
            commands::get_git_command_history,
            commands::test_protocol_url,
            commands::get_clone_dialog_data,
            commands::clone_and_open,
            commands::update_clone_path,
            commands::clone_cancelled,
            commands::get_protocol_registration_status,
            commands::reregister_protocol,
            commands::get_logs_directory,
            commands::open_logs_directory,
            commands::get_large_file_dialog_data,
            commands::large_file_confirmed,
            commands::large_file_cancelled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
