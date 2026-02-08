// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod browser_detection;
mod commands;
mod config_paths;
mod dialog_state;
mod dispatcher;
mod editors;
mod git_command_log;
mod path_validator;
mod protocol_handler;
mod protocol_registration;
mod settings;
mod tracker;
mod tray_state;
mod trust_check;
mod ui_utils;
mod workspace_mru;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Listener, Manager,
};
#[cfg(target_os = "macos")]
use tauri_plugin_deep_link::DeepLinkExt;
use tokio::time::timeout;
use tracing_subscriber::EnvFilter;

use crate::git_command_log::GIT_COMMAND_LOG;
use crate::tray_state::TrayState;

fn load_png_as_image(bytes: &[u8]) -> Image<'static> {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .expect("Failed to decode PNG icon");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Image::new_owned(rgba.into_raw(), width, height)
}

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
#[cfg(target_os = "macos")]
use ui_utils::{activate_app, set_dark_titlebar};
use ui_utils::{build_dialog, DialogConfig};

#[allow(clippy::too_many_lines)] // Complex orchestration function with many dialog handlers
fn handle_protocol_result(
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

            let tray_state = app_handle.state::<Arc<TrayState>>();
            tray_state.flash();

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
                &format!("{match_count} matching workspaces found"),
                duration,
            );
            dialog_state.set_workspace_chooser(dialog_state::WorkspaceChooserData {
                matches,
                line,
                column,
            });
            let _ = build_dialog(
                app_handle,
                &DialogConfig {
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
                &format!("Revision {rev} requires checkout"),
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
                &DialogConfig {
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
            policy_violation,
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
                    "Workspace '{workspace_name}' not found, offering clone from {remote_url}"
                ),
                duration,
            );
            let git_ref_str = git_ref.as_ref().map(dialog_state::git_ref_display);
            dialog_state.set_clone_dialog(dialog_state::CloneDialogData {
                workspace_name,
                clone_path,
                remote_url,
                normalized_remote: None,
                policy_violation,
                file_path,
                line,
                column,
                git_ref: git_ref_str,
                clone_allowed: true,
                clone_validation_message: None,
                suggested_name: None,
                git_ref_kind: git_ref,
            });
            let _ = build_dialog(
                app_handle,
                &DialogConfig {
                    id: "clone-dialog",
                    html_file: "clone-dialog.html",
                    title: "Clone Repository",
                    width: 520.0,
                    height: 380.0,
                },
            );
        }
        Ok(protocol_handler::HandleResult::ShowWorkspaceRepairDialog {
            name,
            workspace_path,
            workspace_state,
            file_path,
            line,
            column,
        }) => {
            let workspace_state_label = format!("{workspace_state:?}").to_lowercase();
            tracing::info!(
                "Request: showing workspace repair dialog for '{}' in state '{}'",
                name,
                workspace_state_label
            );
            GIT_COMMAND_LOG.log_request(
                url,
                true,
                "workspace_repair_dialog",
                &format!("Workspace '{name}' is '{workspace_state_label}'"),
                duration,
            );
            dialog_state.set_workspace_repair_dialog(dialog_state::WorkspaceRepairDialogData {
                name,
                workspace_path,
                workspace_state: workspace_state_label,
                file_path,
                line,
                column,
                original_url: Some(url.to_string()),
            });
            let _ = build_dialog(
                app_handle,
                &DialogConfig {
                    id: "workspace-repair",
                    html_file: "workspace-repair.html",
                    title: "Workspace Needs Repair",
                    width: 620.0,
                    height: 440.0,
                },
            );
        }
        Ok(protocol_handler::HandleResult::ShowWorkspaceConflictDialog {
            workspace_name,
            requested_remote,
            existing_mappings,
            clone_path,
            file_path,
            line,
            column,
            git_ref,
            policy_violation,
        }) => {
            tracing::info!(
                "Request: showing workspace conflict dialog for '{}' and remote '{}'",
                workspace_name,
                requested_remote
            );
            GIT_COMMAND_LOG.log_request(
                url,
                true,
                "workspace_conflict_dialog",
                &format!(
                    "Workspace '{}' conflicts with existing mapping for remote '{}'",
                    workspace_name, requested_remote
                ),
                duration,
            );

            let candidates = existing_mappings
                .into_iter()
                .map(|workspace| dialog_state::WorkspaceConflictCandidateData {
                    name: crate::settings::identity::derive_workspace_name(&workspace),
                    workspace_path: workspace
                        .normalized_path
                        .as_ref()
                        .map(|path| path.to_string_lossy().to_string())
                        .unwrap_or(workspace.path),
                    workspace_state: format!("{:?}", workspace.workspace_state).to_lowercase(),
                    primary_remote: workspace
                        .repo_identity
                        .as_ref()
                        .and_then(|identity| identity.primary_remote.clone()),
                })
                .collect();

            dialog_state.set_workspace_conflict_dialog(dialog_state::WorkspaceConflictDialogData {
                name: workspace_name,
                requested_remote: requested_remote.clone(),
                normalized_remote: crate::settings::identity::normalize_remote_identity(
                    &requested_remote,
                ),
                policy_violation,
                clone_path,
                file_path,
                line,
                column,
                git_ref: git_ref.as_ref().map(dialog_state::git_ref_display),
                candidates,
                git_ref_kind: git_ref,
            });
            let _ = build_dialog(
                app_handle,
                &DialogConfig {
                    id: "workspace-conflict",
                    html_file: "workspace-conflict.html",
                    title: "Workspace Conflict",
                    width: 660.0,
                    height: 500.0,
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
            #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for display
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
                &format!("File is {size_mb:.1} MB, requesting confirmation"),
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
                &DialogConfig {
                    id: "large-file-confirm",
                    html_file: "large-file-confirm.html",
                    title: "Large File Warning",
                    width: 500.0,
                    height: 280.0,
                },
            );
        }
        Ok(protocol_handler::HandleResult::ShowTrustDialog {
            workspace_path,
            workspace_name,
            task_labels,
            vim_local_rc_files,
            dangerous_files,
            dangerous_settings,
            scan_error,
            pending_file_path,
            line,
            column,
            editor_hint,
        }) => {
            let task_count = task_labels.len();
            let vim_rc_count = vim_local_rc_files.len();
            let dangerous_files_count = dangerous_files.len();
            let dangerous_settings_count = dangerous_settings.len();
            tracing::info!(
                "Request: showing trust dialog for workspace '{}' ({} auto-run tasks, {} vim rc files, {} dangerous files, {} dangerous settings)",
                workspace_name,
                task_count,
                vim_rc_count,
                dangerous_files_count,
                dangerous_settings_count
            );
            GIT_COMMAND_LOG.log_request(
                url,
                true,
                "trust_dialog",
                &format!(
                    "Workspace '{workspace_name}' has {task_count} auto-run tasks, {vim_rc_count} vim rc files, {dangerous_files_count} dangerous files, {dangerous_settings_count} dangerous settings, requesting trust confirmation"
                ),
                duration,
            );
            dialog_state.set_trust_dialog(dialog_state::TrustDialogData {
                workspace_path: workspace_path.to_string_lossy().to_string(),
                workspace_name,
                task_labels,
                vim_local_rc_files,
                dangerous_files: dangerous_files
                    .into_iter()
                    .map(|f| dialog_state::DangerousFileData {
                        path: f.path,
                        reason: f.reason.to_string(),
                    })
                    .collect(),
                dangerous_settings: dangerous_settings
                    .into_iter()
                    .map(|s| dialog_state::DangerousSettingData {
                        key: s.key,
                        reason: s.reason.to_string(),
                    })
                    .collect(),
                scan_error,
                pending_file_path,
                line,
                column,
                editor_hint,
            });
            let _ = build_dialog(
                app_handle,
                &DialogConfig {
                    id: "trust-warning",
                    html_file: "trust-warning.html",
                    title: "Security Warning",
                    width: 600.0,
                    height: 520.0,
                },
            );
        }
        Ok(protocol_handler::HandleResult::OpenInBrowser { url: browser_url }) => {
            tracing::info!("Request: opening in browser: {}", browser_url);
            GIT_COMMAND_LOG.log_request(
                url,
                true,
                "browser",
                &format!("Opening in browser: {browser_url}"),
                duration,
            );
            if let Err(e) = open::that(&browser_url) {
                tracing::error!("Failed to open browser: {}", e);
            }
        }
        Ok(protocol_handler::HandleResult::Pong) => {
            tracing::info!("Request: ping received, Desktop is running");
            GIT_COMMAND_LOG.log_request(url, true, "ping", "Desktop is running", duration);
        }
        Ok(protocol_handler::HandleResult::HelloAck { version }) => {
            let version_str = version.as_deref().unwrap_or("unknown");
            tracing::info!("Request: extension hello from version {}", version_str);
            GIT_COMMAND_LOG.log_request(
                url,
                true,
                "hello",
                &format!("Extension version {version_str} registered"),
                duration,
            );
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
    #[allow(clippy::missing_const_for_fn)] // Mutex::new is not const
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
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if let Some(last) = *guard {
            if now.duration_since(last) < self.min_interval {
                return false;
            }
        }

        *guard = Some(now);
        true
    }
}

async fn refresh_and_emit_workspace_health(
    settings_manager: &Arc<settings::SettingsManager>,
    app_handle: &AppHandle,
) -> Result<(), anyhow::Error> {
    let changed = settings_manager.refresh_workspace_states().await?;
    if !changed {
        return Ok(());
    }

    let counts = settings_manager.get_workspace_health_counts().await;
    app_handle.emit("workspace-health-updated", counts)?;
    Ok(())
}

#[tokio::main]
#[allow(clippy::too_many_lines, clippy::large_stack_frames)] // App initialization and tauri macro
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting Sorcery Desktop...");

    let settings_manager = Arc::new(settings::SettingsManager::new().await?);
    let path_validator = Arc::new(path_validator::PathValidator::new());
    let editor_registry = Arc::new(editors::EditorRegistry::new());
    let workspace_tracker = Arc::new(workspace_mru::ActiveWorkspaceTracker::new(Arc::clone(
        &settings_manager,
    )));
    let tracker = Arc::new(
        tracker::ActiveEditorTracker::new(Arc::clone(&editor_registry)).with_workspace_tracking(
            Arc::clone(&settings_manager),
            Arc::clone(&workspace_tracker),
        ),
    );
    let workspace_sync = Arc::new(settings::WorkspaceSync::new(Arc::clone(&settings_manager)));
    let dispatcher = Arc::new(dispatcher::EditorDispatcher::new(
        Arc::clone(&settings_manager),
        Arc::clone(&path_validator),
        Arc::clone(&editor_registry),
        Arc::clone(&tracker),
    ));
    let protocol_handler = Arc::new(protocol_handler::ProtocolHandler::new(
        Arc::clone(&settings_manager),
        Arc::clone(&dispatcher),
        Arc::clone(&workspace_tracker),
    ));
    let dialog_state = Arc::new(DialogState::new());
    let deep_link_throttle = Arc::new(DeepLinkThrottle::new(Duration::from_millis(
        MIN_DEEP_LINK_INTERVAL_MS,
    )));

    // Load tray icons for animation
    let normal_icon = load_png_as_image(include_bytes!("../icons/32x32.png"));
    let active_icon = load_png_as_image(include_bytes!("../icons/32x32_active.png"));
    let tray_state = Arc::new(TrayState::new(normal_icon, active_icon));

    settings_manager.load().await?;
    tracing::info!("Settings loaded");

    // Sync workspaces from default_workspaces_folder
    if let Err(e) = workspace_sync.sync().await {
        tracing::warn!("Failed to sync workspaces: {}", e);
    }

    if let Err(e) = settings_manager.refresh_workspace_states().await {
        tracing::warn!("Failed to refresh workspace states: {}", e);
    }

    let (workspace_change_tx, workspace_change_rx) = tokio::sync::mpsc::unbounded_channel();
    let workspace_watch_service = match settings::WorkspaceWatchService::new(
        Arc::clone(&settings_manager),
        workspace_change_tx,
    )
    .await
    {
        Ok(service) => Some(Arc::new(tokio::sync::Mutex::new(service))),
        Err(error) => {
            tracing::warn!("Failed to start workspace file watcher: {}", error);
            None
        }
    };

    tracker.load().await?;
    tracing::info!("Last seen data loaded");

    workspace_tracker.load().await?;
    tracing::info!("Workspace MRU data loaded");

    let tracker_handle = Arc::clone(&tracker);
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

    let protocol_handler_clone = Arc::clone(&protocol_handler);
    let deep_link_throttle_for_setup = Arc::clone(&deep_link_throttle);
    let tray_state_for_setup = Arc::clone(&tray_state);
    let settings_manager_for_setup = Arc::clone(&settings_manager);
    let workspace_watch_service_for_setup = workspace_watch_service.clone();
    let setup_needed = settings_manager.is_setup_needed().await;

    tauri::Builder::default()
        .setup(move |app| {
            let deep_link_throttle = Arc::clone(&deep_link_throttle_for_setup);
            let settings_manager_for_setup = Arc::clone(&settings_manager_for_setup);
            let mut workspace_change_rx = workspace_change_rx;
            tracing::info!("Setting up Tauri app...");

            // Show setup wizard if this is first run
            if setup_needed {
                tracing::info!("First run detected, showing setup wizard");
                #[cfg(target_os = "macos")]
                activate_app();

                match tauri::WebviewWindowBuilder::new(
                    app,
                    "setup",
                    tauri::WebviewUrl::App("setup.html".into()),
                )
                .title("Welcome to Sorcery Desktop")
                .inner_size(500.0, 550.0)
                .center()
                .resizable(false)
                .focused(true)
                .accept_first_mouse(true)
                .build()
                {
                    #[allow(unused_variables)]
                    Ok(window) => {
                        #[cfg(target_os = "macos")]
                        set_dark_titlebar(&window);
                        tracing::info!("Setup window opened");
                    }
                    Err(e) => {
                        tracing::error!("Failed to open setup window: {}", e);
                    }
                }
            }

            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            // Hide all windows - we run as a background service
            for (_label, window) in app.webview_windows() {
                let _ = window.hide();
            }

            let app_handle = app.handle().clone();
            let ph = Arc::clone(&protocol_handler_clone);
            #[cfg(target_os = "macos")]
            let ph_cold_start = Arc::clone(&protocol_handler_clone);

            let settings_manager_for_periodic = Arc::clone(&settings_manager_for_setup);
            let app_handle_for_periodic = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(60));
                loop {
                    ticker.tick().await;
                    if let Err(error) = refresh_and_emit_workspace_health(
                        &settings_manager_for_periodic,
                        &app_handle_for_periodic,
                    )
                    .await
                    {
                        tracing::warn!("Background workspace refresh failed: {}", error);
                    }
                }
            });

            if let Some(workspace_watch_service) = workspace_watch_service_for_setup.as_ref() {
                let workspace_watch_service = Arc::clone(workspace_watch_service);
                let settings_manager_for_watch_refresh = Arc::clone(&settings_manager_for_setup);
                tauri::async_runtime::spawn(async move {
                    let mut ticker = tokio::time::interval(Duration::from_secs(30));
                    loop {
                        ticker.tick().await;
                        let settings = settings_manager_for_watch_refresh.get().await;
                        let mut watcher = workspace_watch_service.lock().await;
                        watcher.refresh_watch_roots_for_settings(&settings);
                    }
                });
            }

            let app_handle_for_watch_events = app_handle.clone();
            let settings_manager_for_watch_events = Arc::clone(&settings_manager_for_setup);
            tauri::async_runtime::spawn(async move {
                const WORKSPACE_WATCH_DEBOUNCE: Duration = Duration::from_millis(750);
                while workspace_change_rx.recv().await.is_some() {
                    loop {
                        match timeout(WORKSPACE_WATCH_DEBOUNCE, workspace_change_rx.recv()).await {
                            Ok(Some(_)) => continue,
                            Ok(None) => return,
                            Err(_) => break,
                        }
                    }

                    if let Err(error) = refresh_and_emit_workspace_health(
                        &settings_manager_for_watch_events,
                        &app_handle_for_watch_events,
                    )
                    .await
                    {
                        tracing::warn!(
                            "Workspace watch refresh failed after filesystem event: {}",
                            error
                        );
                    }
                }
            });

            let throttle_for_listener = Arc::clone(&deep_link_throttle);
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
                let ph = Arc::clone(&ph);

                tauri::async_runtime::spawn(async move {
                    tracing::debug!("Spawned async task for URL: {}", url);
                    let start = std::time::Instant::now();
                    let result = ph.handle_url(&url).await;
                    tracing::debug!(
                        "handle_url completed in {:?}, result: {:?}",
                        start.elapsed(),
                        result.is_ok()
                    );
                    handle_protocol_result(result, &app_handle, &url, start.elapsed());
                });
            });

            tracing::info!("Application ready");

            // Check for URLs that launched the app (cold start)
            // On macOS, URLs used to launch the app are delivered before the event listener is ready
            #[cfg(target_os = "macos")]
            {
                let throttle_for_cold_start = Arc::clone(&deep_link_throttle);
                let deep_link = app.deep_link();
                if let Ok(Some(urls)) = deep_link.get_current() {
                    if let Some(url) = urls.first() {
                        let url_str = url.to_string();
                        tracing::info!("Processing cold-start URL: {}", url_str);
                        if throttle_for_cold_start.allow() {
                            hide_app();
                            let app_handle = app.handle().clone();
                            let ph = Arc::clone(&ph_cold_start);
                            tauri::async_runtime::spawn(async move {
                                let start = std::time::Instant::now();
                                let result = ph.handle_url(&url_str).await;
                                handle_protocol_result(result, &app_handle, &url_str, start.elapsed());
                            });
                        } else {
                            tracing::warn!(
                                "Dropping cold-start deep-link URL due to 200ms rate limit: {}",
                                url_str
                            );
                        }
                    }
                }
            }

            let settings_item =
                MenuItem::with_id(app, "settings", "Open Sorcery Desktop", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

            // Create system tray icon
            let tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Sorcery Desktop - Editor Link Handler")
                .icon(
                    app.default_window_icon()
                        .expect("app icon must be configured in tauri.conf.json") // Invariant: app icon is always configured.
                        .clone(),
                )
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "settings" => {
                            #[cfg(target_os = "macos")]
                            activate_app();

                            if let Some(window) = app.get_webview_window("settings") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            } else {
                                match tauri::WebviewWindowBuilder::new(
                                    app,
                                    "settings",
                                    tauri::WebviewUrl::App("settings.html".into()),
                                )
                                .title("Sorcery Desktop")
                                .inner_size(800.0, 600.0)
                                .center()
                                .resizable(true)
                                .focused(true)
                                .accept_first_mouse(true)
                                .build()
                                {
                                    #[allow(unused_variables)]
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

            tray_state_for_setup.set_tray_icon(tray);

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
        .manage(tray_state)
        .plugin(tauri_plugin_single_instance::init({
            let throttle = Arc::clone(&deep_link_throttle);
            move |app, args, _cwd| {
                tracing::debug!("Single-instance callback triggered, args: {:?}", args);

            // Second instance launched - check if it's a URL or a direct app launch
            let mut handled_url = false;
            if args.len() > 1 {
                if let Some(url) = args.get(1) {
                    if url.starts_with("srcuri://") {
                        if throttle.allow() {
                            tracing::debug!("Forwarding URL to existing instance: {}", url);
                            if let Err(e) = app.emit("deep-link://new-url", vec![url.clone()]) {
                                tracing::error!("Failed to emit deep-link event: {}", e);
                            } else {
                                handled_url = true;
                            }
                        } else {
                            handled_url = true;
                            tracing::warn!(
                                "Dropping single-instance deep-link URL due to 200ms rate limit: {}",
                                url
                            );
                        }
                    }
                }
            }

            // Only activate the app if user explicitly launched it (not via URL)
            if !handled_url {
                #[cfg(target_os = "macos")]
                activate_app();
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
            commands::get_app_version,
            commands::save_settings,
            commands::get_all_workspaces,
            commands::get_workspace_health_summary,
            commands::refresh_workspace_states,
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
            commands::get_workspace_repair_dialog_data,
            commands::rename_workspace,
            commands::change_workspace_folder,
            commands::remove_workspace,
            commands::get_workspace_conflict_dialog_data,
            commands::workspace_conflict_open_existing,
            commands::workspace_conflict_open_clone_dialog,
            commands::get_protocol_registration_status,
            commands::reregister_protocol,
            commands::get_logs_directory,
            commands::open_logs_directory,
            commands::get_large_file_dialog_data,
            commands::large_file_confirmed,
            commands::large_file_cancelled,
            commands::get_trust_dialog_data,
            commands::trust_confirmed,
            commands::trust_cancelled,
            commands::get_setup_data,
            commands::complete_setup,
            commands::is_setup_needed,
            commands::detect_browsers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
