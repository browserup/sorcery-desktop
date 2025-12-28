use crate::dispatcher::EditorDispatcher;
use crate::editors::EditorRegistry;
use crate::git_command_log::{GitCommandLogEntry, GIT_COMMAND_LOG};
use crate::protocol_handler::{GitHandler, GitRef, WorkingTreeStatus, WorkspaceMatch};
use crate::settings::{Settings, SettingsManager, WorkspaceSync};
use crate::tracker::ActiveEditorTracker;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

#[cfg(target_os = "macos")]
fn set_dark_titlebar(window: &tauri::WebviewWindow) {
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSString;
    use objc::{class, msg_send, sel, sel_impl};
    use tauri::Manager;

    // Get window label and app handle to use inside the closure
    let app_handle = window.app_handle().clone();
    let label = window.label().to_string();

    // Run on main thread since NSWindow APIs must be called from main thread
    let _ = window.run_on_main_thread(move || {
        if let Some(win) = app_handle.get_webview_window(&label) {
            if let Ok(ns_window) = win.ns_window() {
                unsafe {
                    let ns_window = ns_window as id;

                    // Get NSAppearance for dark mode
                    let appearance_name = cocoa::foundation::NSString::alloc(nil)
                        .init_str("NSAppearanceNameDarkAqua");
                    let appearance: id = msg_send![class!(NSAppearance), appearanceNamed: appearance_name];
                    let _: () = msg_send![ns_window, setAppearance: appearance];

                    // Set background color to match our theme (#121212)
                    let color: id = msg_send![class!(NSColor), colorWithRed:0.071 green:0.071 blue:0.071 alpha:1.0];
                    let _: () = msg_send![ns_window, setBackgroundColor: color];
                }
            }
        }
    });
}

#[derive(Serialize)]
pub struct EditorTestbedData {
    pub editors: Vec<EditorInfo>,
    pub last_seen: HashMap<String, i64>,
    pub most_recent: Option<String>,
    pub settings: Settings,
}

#[derive(Serialize)]
pub struct EditorInfo {
    pub editor_id: String,
    pub display_name: String,
    pub is_installed: bool,
    pub detected: bool,
    pub workspace: Option<String>,
    pub last_seen: Option<i64>,
}

#[tauri::command]
pub async fn get_settings(
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<Settings, String> {
    Ok(settings_manager.get().await)
}

#[tauri::command]
pub fn get_settings_path(settings_manager: State<'_, Arc<SettingsManager>>) -> String {
    settings_manager.config_path().to_string_lossy().to_string()
}

#[tauri::command]
pub async fn save_settings(
    settings_manager: State<'_, Arc<SettingsManager>>,
    settings: Settings,
) -> Result<(), String> {
    settings_manager
        .save(settings)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct WorkspaceDisplayInfo {
    pub name: String,
    pub path: String,
    pub editor: Option<String>,
    pub is_discovered: bool,
}

#[derive(Serialize)]
pub struct AllWorkspaces {
    pub explicit: Vec<WorkspaceDisplayInfo>,
    pub discovered: Vec<WorkspaceDisplayInfo>,
}

#[tauri::command]
pub async fn get_all_workspaces(
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<AllWorkspaces, String> {
    let settings = settings_manager.get().await;

    let mut explicit = Vec::new();
    let mut discovered = Vec::new();

    for ws in &settings.workspaces {
        let name = ws.name.clone().unwrap_or_else(|| {
            ws.normalized_path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        });
        let info = WorkspaceDisplayInfo {
            name,
            path: ws.path.clone(),
            editor: if ws.editor.is_empty() {
                None
            } else {
                Some(ws.editor.clone())
            },
            is_discovered: ws.auto_discovered,
        };

        if ws.auto_discovered {
            discovered.push(info);
        } else {
            explicit.push(info);
        }
    }

    Ok(AllWorkspaces {
        explicit,
        discovered,
    })
}

#[tauri::command]
pub async fn promote_workspace(
    settings_manager: State<'_, Arc<SettingsManager>>,
    path: String,
    name: String,
) -> Result<(), String> {
    let mut settings = settings_manager.get().await;

    // Check if already exists
    let normalized_path = shellexpand::tilde(&path);
    let target_path = PathBuf::from(normalized_path.as_ref());

    for ws in &settings.workspaces {
        if let Some(ref existing) = ws.normalized_path {
            if existing == &target_path {
                return Err("Workspace already exists in explicit mappings".to_string());
            }
        }
    }

    settings
        .workspaces
        .push(crate::settings::WorkspaceConfig {
            path,
            name: Some(name),
            editor: String::new(),
            auto_discovered: false,
            normalized_path: Some(target_path),
        });

    settings_manager
        .save(settings)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_workspaces(
    workspace_sync: State<'_, Arc<WorkspaceSync>>,
) -> Result<crate::settings::SyncResult, String> {
    workspace_sync.sync().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_workspace(
    settings_manager: State<'_, Arc<SettingsManager>>,
    path: String,
) -> Result<(), String> {
    let mut settings = settings_manager.get().await;

    let normalized_path = shellexpand::tilde(&path);
    let target_path = PathBuf::from(normalized_path.as_ref());

    let mut found_index = None;
    let mut was_auto_discovered = false;

    for (i, ws) in settings.workspaces.iter().enumerate() {
        if let Some(ref existing) = ws.normalized_path {
            if existing == &target_path {
                found_index = Some(i);
                was_auto_discovered = ws.auto_discovered;
                break;
            }
        }
    }

    if let Some(index) = found_index {
        settings.workspaces.remove(index);

        // If it was auto-discovered, add to ignored list so it doesn't reappear
        if was_auto_discovered {
            settings.defaults.ignored_workspaces.push(path);
        }

        settings_manager
            .save(settings)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_editor_testbed_data(
    registry: State<'_, Arc<EditorRegistry>>,
    tracker: State<'_, Arc<ActiveEditorTracker>>,
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<EditorTestbedData, String> {
    let mut editors = Vec::new();
    let last_seen_data = tracker.get_last_seen_data().await;
    let settings = settings_manager.get().await;

    for editor_id in registry.list_editors() {
        if let Some(manager) = registry.get(&editor_id) {
            let is_installed = manager.is_installed().await;
            let instances = manager
                .get_running_instances()
                .await
                .ok()
                .unwrap_or_default();

            editors.push(EditorInfo {
                editor_id: editor_id.clone(),
                display_name: manager.display_name().to_string(),
                is_installed,
                detected: !instances.is_empty(),
                workspace: instances.first().and_then(|inst| inst.workspace.clone()),
                last_seen: last_seen_data.editors.get(&editor_id).copied(),
            });
        }
    }

    Ok(EditorTestbedData {
        editors,
        last_seen: last_seen_data.editors,
        most_recent: last_seen_data.most_recent,
        settings,
    })
}

#[tauri::command]
pub async fn test_open_file(
    dispatcher: State<'_, Arc<EditorDispatcher>>,
    editor_id: String,
    test_file_path: Option<String>,
) -> Result<String, String> {
    let file_path = test_file_path.unwrap_or_else(|| {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        format!("{}/README.md", manifest_dir.trim_end_matches("/src-tauri"))
    });

    dispatcher
        .open(&file_path, Some(50), None, true, Some(editor_id.clone()))
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!("Opened {} in {}", file_path, editor_id))
}

#[tauri::command]
pub async fn open_in_editor(
    dispatcher: State<'_, Arc<EditorDispatcher>>,
    path: String,
    line: Option<usize>,
    column: Option<usize>,
    new_window: bool,
    editor: Option<String>,
) -> Result<(), String> {
    dispatcher
        .open(&path, line, column, new_window, editor)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn detect_source_folder() -> Result<String, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Could not find home directory".to_string())?;

    let candidate_names = [
        "repos",
        "repositories",
        "code",
        "src",
        "source",
        "apps",
        "projects",
        "work",
        "developer",
        "dev",
        "development",
        "git",
        "git-repos",
    ];

    let mut best_folder: Option<PathBuf> = None;
    let mut max_git_count = 0;

    let mut check_candidate = |candidate_path: PathBuf| {
        if candidate_path.is_dir() {
            if let Ok(git_count) = count_git_repos(&candidate_path) {
                if git_count > max_git_count {
                    max_git_count = git_count;
                    best_folder = Some(candidate_path);
                }
            }
        }
    };

    // Scan home directory for common folder names
    if let Ok(entries) = std::fs::read_dir(&home_dir) {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                let file_name_lower = file_name.to_lowercase();
                if candidate_names.iter().any(|&name| file_name_lower == name) {
                    check_candidate(entry.path());
                }
            }
        }
    }

    // Unix/macOS specific paths
    #[cfg(not(target_os = "windows"))]
    {
        check_candidate(home_dir.join("workspace"));
        check_candidate(home_dir.join("github"));
        check_candidate(home_dir.join("Documents/projects"));
        check_candidate(home_dir.join("Documents/Code"));
        check_candidate(home_dir.join("Documents/GitHub"));
        check_candidate(home_dir.join("go/src"));
        check_candidate(home_dir.join("Sites"));
    }

    // Windows specific paths
    #[cfg(target_os = "windows")]
    {
        check_candidate(home_dir.join("source").join("repos"));
        check_candidate(home_dir.join("Documents").join("GitHub"));
        check_candidate(home_dir.join("Documents").join("Projects"));

        // Visual Studio project folders (2015-2025)
        let documents = home_dir.join("Documents");
        for year in 2015..=2025 {
            check_candidate(
                documents
                    .join(format!("Visual Studio {}", year))
                    .join("Projects"),
            );
        }

        // Common Windows dev root folders
        for root_path in &["C:\\dev", "C:\\src", "C:\\code", "C:\\Projects"] {
            check_candidate(PathBuf::from(root_path));
        }
    }

    let result = best_folder.unwrap_or_else(|| home_dir.join("code"));
    Ok(result.to_string_lossy().to_string())
}

fn count_git_repos(dir: &Path) -> Result<usize, std::io::Error> {
    let mut count = 0;

    if !dir.is_dir() {
        return Ok(0);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let git_path = path.join(".git");
            if git_path.exists() {
                count += 1;
            }
        }
    }

    Ok(count)
}

#[derive(Clone, Serialize)]
pub struct WorkspaceChooserData {
    pub matches: Vec<WorkspaceMatch>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

static WORKSPACE_CHOOSER_DATA: parking_lot::Mutex<Option<WorkspaceChooserData>> =
    parking_lot::Mutex::new(None);

#[tauri::command]
pub fn get_workspace_chooser_data() -> Result<WorkspaceChooserData, String> {
    WORKSPACE_CHOOSER_DATA
        .lock()
        .clone()
        .ok_or_else(|| "No chooser data available".to_string())
}

#[tauri::command]
pub async fn workspace_chosen(
    index: usize,
    dispatcher: State<'_, Arc<EditorDispatcher>>,
) -> Result<(), String> {
    let data = WORKSPACE_CHOOSER_DATA
        .lock()
        .clone()
        .ok_or_else(|| "No chooser data available".to_string())?;

    if index >= data.matches.len() {
        return Err("Invalid workspace index".to_string());
    }

    let workspace_match = &data.matches[index];

    dispatcher
        .open(
            &workspace_match.full_file_path.to_string_lossy(),
            data.line,
            data.column,
            false,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;

    *WORKSPACE_CHOOSER_DATA.lock() = None;

    Ok(())
}

#[tauri::command]
pub fn workspace_chooser_cancelled() -> Result<(), String> {
    *WORKSPACE_CHOOSER_DATA.lock() = None;
    Ok(())
}

pub fn set_workspace_chooser_data(data: WorkspaceChooserData) {
    *WORKSPACE_CHOOSER_DATA.lock() = Some(data);
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RevisionDialogData {
    pub workspace: String,
    pub workspace_path: String,
    pub file_path: String,
    pub full_file_path: String,
    pub rev: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub current_ref: String,
    pub is_working_tree_clean: bool,
    pub dirty_file_count: usize,
    pub checkout_available: bool,
    pub checkout_blocked_reason: Option<String>,
}

static REVISION_DIALOG_DATA: parking_lot::Mutex<Option<RevisionDialogData>> =
    parking_lot::Mutex::new(None);

#[tauri::command]
pub fn get_revision_dialog_data() -> Result<RevisionDialogData, String> {
    REVISION_DIALOG_DATA
        .lock()
        .clone()
        .ok_or_else(|| "No revision dialog data available".to_string())
}

#[tauri::command]
pub fn get_git_revision_info(workspace_path: String, rev: String) -> Result<String, String> {
    let path = PathBuf::from(&workspace_path);
    GitHandler::get_revision_info(&path, &rev).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_file_at_revision(
    workspace_path: String,
    file_path: String,
    rev: String,
    line: Option<usize>,
    column: Option<usize>,
    checkout: bool,
    dispatcher: State<'_, Arc<EditorDispatcher>>,
) -> Result<(), String> {
    let workspace = PathBuf::from(&workspace_path);

    if checkout {
        let current_ref = GitHandler::get_current_ref(&workspace).map_err(|e| e.to_string())?;

        tracing::info!("Checking out from {} to {}", current_ref, rev);

        GitHandler::checkout_revision(&workspace, &rev).map_err(|e| e.to_string())?;

        tracing::info!("Successfully checked out to {}", rev);

        let full_path = workspace.join(&file_path);

        dispatcher
            .open(&full_path.to_string_lossy(), line, column, false, None)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        let content = GitHandler::get_file_at_revision(&workspace, &file_path, &rev)
            .map_err(|e| e.to_string())?;

        let temp_dir = std::env::temp_dir();
        let file_name = format!(
            "{}@{}",
            file_path.replace("/", "_"),
            &rev[..7.min(rev.len())]
        );
        let temp_file = temp_dir.join(file_name);

        std::fs::write(&temp_file, content)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;

        dispatcher
            .open(&temp_file.to_string_lossy(), line, column, true, None)
            .await
            .map_err(|e| e.to_string())?;
    }

    *REVISION_DIALOG_DATA.lock() = None;

    Ok(())
}

#[tauri::command]
pub fn revision_dialog_cancelled() -> Result<(), String> {
    *REVISION_DIALOG_DATA.lock() = None;
    Ok(())
}

pub fn set_revision_dialog_data(data: RevisionDialogData) {
    *REVISION_DIALOG_DATA.lock() = Some(data);
}

#[tauri::command]
pub fn refresh_working_tree_status(workspace_path: String) -> Result<WorkingTreeStatus, String> {
    let path = PathBuf::from(&workspace_path);
    GitHandler::get_working_tree_status(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_worktree_and_open(
    dispatcher: State<'_, Arc<EditorDispatcher>>,
    workspace_path: String,
    workspace_name: String,
    branch_or_commit: String,
    file_path: String,
    line: Option<usize>,
    column: Option<usize>,
) -> Result<(), String> {
    let workspace = PathBuf::from(&workspace_path);

    // Create worktree (reuses existing if available)
    let worktree_path = GitHandler::create_worktree(&workspace, &workspace_name, &branch_or_commit)
        .map_err(|e| e.to_string())?;

    // Build full file path in worktree
    let full_path = worktree_path.join(&file_path);

    // Verify file exists
    if !full_path.exists() {
        return Err(format!(
            "File '{}' not found in worktree at {}",
            file_path,
            worktree_path.display()
        ));
    }

    // Open in editor
    dispatcher
        .open(&full_path.to_string_lossy(), line, column, false, None)
        .await
        .map_err(|e| e.to_string())?;

    // Clear revision dialog data since we're done
    *REVISION_DIALOG_DATA.lock() = None;

    Ok(())
}

#[tauri::command]
pub fn get_git_command_history() -> Result<Vec<GitCommandLogEntry>, String> {
    Ok(GIT_COMMAND_LOG.get_entries())
}

#[tauri::command]
pub async fn test_protocol_url(
    url: String,
    protocol_handler: State<'_, Arc<crate::protocol_handler::ProtocolHandler>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use crate::protocol_handler::HandleResult;
    use std::time::Instant;

    let start = Instant::now();
    let result = protocol_handler.handle_url(&url).await;
    let duration = start.elapsed();

    match result {
        Ok(HandleResult::Opened) => {
            GIT_COMMAND_LOG.log_request(&url, true, "opened", "File opened in editor", duration);
            Ok(())
        }
        Ok(HandleResult::ShowChooser {
            matches,
            line,
            column,
        }) => {
            let match_count = matches.len();
            GIT_COMMAND_LOG.log_request(
                &url,
                true,
                "chooser",
                &format!("{} matching workspaces found", match_count),
                duration,
            );
            set_workspace_chooser_data(WorkspaceChooserData {
                matches,
                line,
                column,
            });

            let window = tauri::WebviewWindowBuilder::new(
                &app,
                "workspace-chooser",
                tauri::WebviewUrl::App("workspace-chooser.html".into()),
            )
            .title("Choose Workspace")
            .inner_size(600.0, 500.0)
            .center()
            .resizable(false)
            .always_on_top(true)
            .focused(true)
            .build()
            .map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            set_dark_titlebar(&window);

            Ok(())
        }
        Ok(HandleResult::ShowRevisionDialog {
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
            GIT_COMMAND_LOG.log_request(
                &url,
                true,
                "revision_dialog",
                &format!("Revision {} requires checkout", rev),
                duration,
            );
            set_revision_dialog_data(RevisionDialogData {
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

            let window = tauri::WebviewWindowBuilder::new(
                &app,
                "revision-handler",
                tauri::WebviewUrl::App("revision-handler.html".into()),
            )
            .title("Open File at Revision")
            .inner_size(600.0, 450.0)
            .center()
            .resizable(false)
            .always_on_top(true)
            .focused(true)
            .build()
            .map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            set_dark_titlebar(&window);

            Ok(())
        }
        Ok(HandleResult::ShowCloneDialog {
            workspace_name,
            clone_path,
            remote_url,
            file_path,
            line,
            column,
            git_ref,
        }) => {
            GIT_COMMAND_LOG.log_request(
                &url,
                true,
                "clone_dialog",
                &format!(
                    "Workspace '{}' not found, offering clone from {}",
                    workspace_name, remote_url
                ),
                duration,
            );
            let git_ref_display = git_ref.as_ref().map(|r| git_ref_display(r));
            set_clone_dialog_data(CloneDialogData {
                workspace_name,
                clone_path,
                remote_url,
                file_path,
                line,
                column,
                git_ref: git_ref_display,
                git_ref_kind: git_ref.clone(),
            });

            let window = tauri::WebviewWindowBuilder::new(
                &app,
                "clone-dialog",
                tauri::WebviewUrl::App("clone-dialog.html".into()),
            )
            .title("Clone Repository")
            .inner_size(520.0, 380.0)
            .center()
            .resizable(false)
            .always_on_top(true)
            .focused(true)
            .build()
            .map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            set_dark_titlebar(&window);

            Ok(())
        }
        Ok(HandleResult::OpenInBrowser { url: browser_url }) => {
            GIT_COMMAND_LOG.log_request(
                &url,
                true,
                "browser",
                &format!("Opening in browser: {}", browser_url),
                duration,
            );
            if let Err(e) = open::that(&browser_url) {
                return Err(format!("Failed to open browser: {}", e));
            }
            Ok(())
        }
        Err(e) => {
            let error_msg = e.to_string();
            GIT_COMMAND_LOG.log_request(&url, false, "error", &error_msg, duration);
            Err(error_msg)
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CloneDialogData {
    pub workspace_name: String,
    pub clone_path: String,
    pub remote_url: String,
    pub file_path: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub git_ref: Option<String>,
    #[serde(skip)]
    pub git_ref_kind: Option<GitRef>,
}

static CLONE_DIALOG_DATA: parking_lot::Mutex<Option<CloneDialogData>> =
    parking_lot::Mutex::new(None);

pub fn set_clone_dialog_data(data: CloneDialogData) {
    *CLONE_DIALOG_DATA.lock() = Some(data);
}

pub(crate) fn git_ref_display(git_ref: &GitRef) -> String {
    match git_ref {
        GitRef::Branch(value) => value.clone(),
        GitRef::Tag(value) => format!("tag {}", value),
        GitRef::Commit(value) => format!("commit {}", value),
    }
}

#[tauri::command]
pub fn get_clone_dialog_data() -> Result<CloneDialogData, String> {
    CLONE_DIALOG_DATA
        .lock()
        .clone()
        .ok_or_else(|| "No clone dialog data available".to_string())
}

#[tauri::command]
pub async fn clone_and_open(
    dispatcher: State<'_, Arc<EditorDispatcher>>,
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<(), String> {
    let data = CLONE_DIALOG_DATA
        .lock()
        .clone()
        .ok_or_else(|| "No clone dialog data available".to_string())?;

    let target_path = PathBuf::from(&data.clone_path);

    GitHandler::clone_repo(&data.remote_url, &target_path, data.git_ref_kind.as_ref())
        .map_err(|e| e.to_string())?;

    // Add new workspace to settings
    let mut settings = settings_manager.get().await;
    settings.workspaces.push(crate::settings::WorkspaceConfig {
        path: data.clone_path.clone(),
        name: Some(data.workspace_name.clone()),
        editor: String::new(),
        auto_discovered: false,
        normalized_path: Some(target_path.clone()),
    });
    settings_manager
        .save(settings)
        .await
        .map_err(|e| format!("Failed to save workspace: {}", e))?;

    let full_file_path = target_path.join(&data.file_path);

    dispatcher
        .open(
            &full_file_path.to_string_lossy(),
            data.line,
            data.column,
            false,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;

    *CLONE_DIALOG_DATA.lock() = None;

    Ok(())
}

#[tauri::command]
pub fn update_clone_path(new_path: String) -> Result<(), String> {
    let mut data = CLONE_DIALOG_DATA.lock();
    if let Some(ref mut clone_data) = *data {
        clone_data.clone_path = new_path;
        Ok(())
    } else {
        Err("No clone dialog data available".to_string())
    }
}

#[tauri::command]
pub fn clone_cancelled() -> Result<(), String> {
    *CLONE_DIALOG_DATA.lock() = None;
    Ok(())
}

#[tauri::command]
pub fn get_protocol_registration_status(
) -> Result<crate::protocol_registration::ProtocolRegistrationStatus, String> {
    Ok(crate::protocol_registration::ProtocolRegistration::get_status())
}

#[tauri::command]
pub fn reregister_protocol() -> Result<String, String> {
    crate::protocol_registration::ProtocolRegistration::register().map_err(|e| e.to_string())?;
    Ok("Protocol re-registered successfully".to_string())
}

#[derive(Serialize)]
pub struct LogsDirectoryInfo {
    pub path: String,
    pub exists: bool,
}

#[tauri::command]
pub fn get_logs_directory() -> LogsDirectoryInfo {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let path = home.join("Library/Logs/DiagnosticReports");
            return LogsDirectoryInfo {
                exists: path.exists(),
                path: path.to_string_lossy().to_string(),
            };
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(local_app_data) = dirs::data_local_dir() {
            let path = local_app_data.join("CrashDumps");
            return LogsDirectoryInfo {
                exists: path.exists(),
                path: path.to_string_lossy().to_string(),
            };
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".local/share/sorcery-desktop/logs");
            return LogsDirectoryInfo {
                exists: path.exists(),
                path: path.to_string_lossy().to_string(),
            };
        }
    }

    LogsDirectoryInfo {
        path: "Unknown".to_string(),
        exists: false,
    }
}

#[tauri::command]
pub fn open_logs_directory() -> Result<(), String> {
    let info = get_logs_directory();

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&info.path)
            .spawn()
            .map_err(|e| format!("Failed to open logs directory: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&info.path)
            .spawn()
            .map_err(|e| format!("Failed to open logs directory: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&info.path)
            .spawn()
            .map_err(|e| format!("Failed to open logs directory: {}", e))?;
    }

    Ok(())
}
