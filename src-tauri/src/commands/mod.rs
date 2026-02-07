use crate::dialog_state::DialogState;
pub use crate::dialog_state::{
    CloneDialogData, DangerousFileData, DangerousSettingData, LargeFileDialogData,
    RevisionDialogData, TrustDialogData, WorkspaceChooserData, WorkspaceConflictCandidateData,
    WorkspaceConflictDialogData, WorkspaceRepairDialogData,
};
use crate::dispatcher::EditorDispatcher;
use crate::editors::EditorRegistry;
use crate::git_command_log::{GitCommandLogEntry, GIT_COMMAND_LOG};
use crate::protocol_handler::{GitHandler, WorkingTreeStatus};
use crate::settings::{
    identity, PolicyDecision, Settings, SettingsManager, WorkspaceKind, WorkspaceState,
    WorkspaceSync,
};
use crate::tracker::ActiveEditorTracker;
#[cfg(target_os = "macos")]
use crate::ui_utils::{activate_app, set_dark_titlebar};
use crate::workspace_mru::ActiveWorkspaceTracker;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

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
#[allow(clippy::needless_pass_by_value)] // Tauri commands require owned State
pub fn get_settings_path(settings_manager: State<'_, Arc<SettingsManager>>) -> String {
    settings_manager.config_path().to_string_lossy().to_string()
}

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
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
    pub path: String,
    pub editor: Option<String>,
    pub is_discovered: bool,
    pub workspace_key: String,
    pub workspace_kind: String,
    pub workspace_state: String,
    pub primary_remote: Option<String>,
    pub last_verified_at: Option<i64>,
}

#[derive(Serialize)]
pub struct AllWorkspaces {
    pub explicit: Vec<WorkspaceDisplayInfo>,
    pub discovered: Vec<WorkspaceDisplayInfo>,
}

#[derive(Serialize)]
pub struct WorkspaceHealthSummary {
    pub healthy: usize,
    pub missing: usize,
    pub unavailable: usize,
    pub drifted: usize,
    pub conflict: usize,
}

#[must_use]
fn workspace_health_summary_from_counts(
    counts: &std::collections::HashMap<String, usize>,
) -> WorkspaceHealthSummary {
    WorkspaceHealthSummary {
        healthy: counts.get("healthy").copied().unwrap_or(0),
        missing: counts.get("missing").copied().unwrap_or(0),
        unavailable: counts.get("unavailable").copied().unwrap_or(0),
        drifted: counts.get("drifted").copied().unwrap_or(0),
        conflict: counts.get("conflict").copied().unwrap_or(0),
    }
}

#[tauri::command]
pub async fn get_all_workspaces(
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<AllWorkspaces, String> {
    let settings = settings_manager.get().await;

    let mut explicit = Vec::new();
    let mut discovered = Vec::new();

    for ws in &settings.workspaces {
        let info = WorkspaceDisplayInfo {
            path: ws.path.clone(),
            editor: if ws.editor.is_empty() {
                None
            } else {
                Some(ws.editor.clone())
            },
            is_discovered: ws.auto_discovered,
            workspace_key: identity::derive_workspace_key(ws),
            workspace_kind: match ws.workspace_kind {
                WorkspaceKind::Git => "git".to_string(),
                WorkspaceKind::NonGit => "folder".to_string(),
            },
            workspace_state: match ws.workspace_state {
                WorkspaceState::Present => "healthy".to_string(),
                WorkspaceState::Missing => "missing".to_string(),
                WorkspaceState::Unavailable => "unavailable".to_string(),
                WorkspaceState::IdentityDrift => "drifted".to_string(),
                WorkspaceState::Conflict => "conflict".to_string(),
            },
            primary_remote: ws
                .repo_identity
                .as_ref()
                .and_then(|identity| identity.primary_remote.clone()),
            last_verified_at: ws.last_verified_at,
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
pub async fn get_workspace_health_summary(
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<WorkspaceHealthSummary, String> {
    let counts = settings_manager.get_workspace_health_counts().await;
    Ok(workspace_health_summary_from_counts(&counts))
}

#[tauri::command]
pub async fn reconcile_workspace_states(
    app: tauri::AppHandle,
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<WorkspaceHealthSummary, String> {
    let changed = settings_manager
        .reconcile_workspace_states()
        .await
        .map_err(|e| e.to_string())?;
    let counts = settings_manager.get_workspace_health_counts().await;
    if changed {
        app.emit("workspace-health-updated", &counts)
            .map_err(|e| e.to_string())?;
    }
    Ok(workspace_health_summary_from_counts(&counts))
}

#[tauri::command]
pub async fn promote_workspace(
    settings_manager: State<'_, Arc<SettingsManager>>,
    path: String,
    workspace_key: String,
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

    settings.workspaces.push(crate::settings::WorkspaceConfig {
        path,
        workspace_key,
        editor: String::new(),
        auto_discovered: false,
        trusted: false,
        workspace_kind: WorkspaceKind::NonGit,
        workspace_state: WorkspaceState::Present,
        repo_identity: None,
        last_verified_at: None,
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
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.parent().unwrap_or(manifest_dir);
        repo_root.join("README.md").to_string_lossy().to_string()
    });

    dispatcher
        .open(&file_path, Some(50), None, true, Some(editor_id.clone()))
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!("Opened {file_path} in {editor_id}"))
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

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands require owned State
pub fn get_workspace_chooser_data(
    dialog_state: State<'_, Arc<DialogState>>,
) -> Result<WorkspaceChooserData, String> {
    dialog_state
        .take_workspace_chooser()
        .ok_or_else(|| "No chooser data available".to_string())
}

#[tauri::command]
pub async fn workspace_chosen(
    index: usize,
    dispatcher: State<'_, Arc<EditorDispatcher>>,
    dialog_state: State<'_, Arc<DialogState>>,
    workspace_tracker: State<'_, Arc<ActiveWorkspaceTracker>>,
) -> Result<(), String> {
    let data = dialog_state
        .take_workspace_chooser()
        .ok_or_else(|| "No chooser data available".to_string())?;

    if index >= data.matches.len() {
        return Err("Invalid workspace index".to_string());
    }

    let workspace_match = &data.matches[index];

    workspace_tracker
        .record_workspace_seen(&workspace_match.workspace_path)
        .await;

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

    Ok(())
}

#[tauri::command]
#[allow(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::missing_const_for_fn
)]
pub fn workspace_chooser_cancelled(
    _dialog_state: State<'_, Arc<DialogState>>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands require owned State
pub fn get_revision_dialog_data(
    dialog_state: State<'_, Arc<DialogState>>,
) -> Result<RevisionDialogData, String> {
    dialog_state
        .take_revision_dialog()
        .ok_or_else(|| "No revision dialog data available".to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands require owned String
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
            file_path.replace('/', "_"),
            &rev[..7.min(rev.len())]
        );
        let temp_file = temp_dir.join(file_name);

        std::fs::write(&temp_file, content)
            .map_err(|e| format!("Failed to write temp file: {e}"))?;

        dispatcher
            .open(&temp_file.to_string_lossy(), line, column, true, None)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
#[allow(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::missing_const_for_fn
)]
pub fn revision_dialog_cancelled(_dialog_state: State<'_, Arc<DialogState>>) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands require owned String
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

    Ok(())
}

#[tauri::command]
#[allow(clippy::unnecessary_wraps)]
pub fn get_git_command_history() -> Result<Vec<GitCommandLogEntry>, String> {
    Ok(GIT_COMMAND_LOG.get_entries())
}

#[tauri::command]
#[allow(clippy::too_many_lines)] // Complex orchestration function
pub async fn test_protocol_url(
    url: String,
    protocol_handler: State<'_, Arc<crate::protocol_handler::ProtocolHandler>>,
    dialog_state: State<'_, Arc<DialogState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use crate::dialog_state::git_ref_display;
    use crate::protocol_handler::HandleResult;
    use std::time::Instant;

    let start = Instant::now();
    let result = protocol_handler.handle_url(&url).await;
    let duration = start.elapsed();

    match result {
        Ok(HandleResult::Opened { file_path }) => {
            GIT_COMMAND_LOG.log_request(&url, true, "opened", &file_path, duration);
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
                &format!("{match_count} matching workspaces found"),
                duration,
            );
            dialog_state.set_workspace_chooser(WorkspaceChooserData {
                matches,
                line,
                column,
            });

            #[cfg(target_os = "macos")]
            activate_app();

            #[allow(unused_variables)]
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
            .accept_first_mouse(true)
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
                &format!("Revision {rev} requires checkout"),
                duration,
            );
            dialog_state.set_revision_dialog(RevisionDialogData {
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

            #[cfg(target_os = "macos")]
            activate_app();

            #[allow(unused_variables)]
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
            .accept_first_mouse(true)
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
            policy_violation,
        }) => {
            GIT_COMMAND_LOG.log_request(
                &url,
                true,
                "clone_dialog",
                &format!(
                    "Workspace '{workspace_name}' not found, offering clone from {remote_url}"
                ),
                duration,
            );
            let git_ref_str = git_ref.as_ref().map(git_ref_display);
            dialog_state.set_clone_dialog(CloneDialogData {
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
                suggested_workspace_key: None,
                git_ref_kind: git_ref,
            });

            #[cfg(target_os = "macos")]
            activate_app();

            #[allow(unused_variables)]
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
            .accept_first_mouse(true)
            .build()
            .map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            set_dark_titlebar(&window);

            Ok(())
        }
        Ok(HandleResult::ShowWorkspaceRepairDialog {
            workspace_key,
            workspace_path,
            workspace_state,
            file_path,
            line,
            column,
        }) => {
            let workspace_state_label = format!("{workspace_state:?}").to_lowercase();
            GIT_COMMAND_LOG.log_request(
                &url,
                true,
                "workspace_repair_dialog",
                &format!("Workspace '{workspace_key}' is '{workspace_state_label}'"),
                duration,
            );
            dialog_state.set_workspace_repair_dialog(WorkspaceRepairDialogData {
                workspace_key,
                workspace_path,
                workspace_state: workspace_state_label,
                file_path,
                line,
                column,
                original_url: Some(url.clone()),
            });

            #[cfg(target_os = "macos")]
            activate_app();

            #[allow(unused_variables)]
            let window = tauri::WebviewWindowBuilder::new(
                &app,
                "workspace-repair",
                tauri::WebviewUrl::App("workspace-repair.html".into()),
            )
            .title("Workspace Needs Repair")
            .inner_size(620.0, 440.0)
            .center()
            .resizable(false)
            .always_on_top(true)
            .focused(true)
            .accept_first_mouse(true)
            .build()
            .map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            set_dark_titlebar(&window);

            Ok(())
        }
        Ok(HandleResult::ShowWorkspaceConflictDialog {
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
            GIT_COMMAND_LOG.log_request(
                &url,
                true,
                "workspace_conflict_dialog",
                &format!(
                    "Workspace '{workspace_name}' conflicts with existing mapping for remote '{}'",
                    requested_remote
                ),
                duration,
            );
            dialog_state.set_workspace_conflict_dialog(WorkspaceConflictDialogData {
                workspace_key: workspace_name,
                requested_remote: requested_remote.clone(),
                normalized_remote: identity::normalize_remote_identity(&requested_remote),
                policy_violation,
                clone_path,
                file_path,
                line,
                column,
                git_ref: git_ref.as_ref().map(git_ref_display),
                candidates: existing_mappings
                    .into_iter()
                    .map(|workspace| WorkspaceConflictCandidateData {
                        workspace_key: identity::derive_workspace_key(&workspace),
                        workspace_path: workspace
                            .normalized_path
                            .as_ref()
                            .map(|path| path.to_string_lossy().to_string())
                            .unwrap_or(workspace.path),
                        workspace_state: format!("{:?}", workspace.workspace_state).to_lowercase(),
                        primary_remote: workspace
                            .repo_identity
                            .as_ref()
                            .and_then(|repo| repo.primary_remote.clone()),
                    })
                    .collect(),
                git_ref_kind: git_ref,
            });

            #[cfg(target_os = "macos")]
            activate_app();

            #[allow(unused_variables)]
            let window = tauri::WebviewWindowBuilder::new(
                &app,
                "workspace-conflict",
                tauri::WebviewUrl::App("workspace-conflict.html".into()),
            )
            .title("Workspace Conflict")
            .inner_size(660.0, 500.0)
            .center()
            .resizable(false)
            .always_on_top(true)
            .focused(true)
            .accept_first_mouse(true)
            .build()
            .map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            set_dark_titlebar(&window);

            Ok(())
        }
        Ok(HandleResult::ShowLargeFileDialog {
            file_path,
            file_size_bytes,
            line,
            column,
            editor_hint,
        }) => {
            #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for display
            let size_mb = file_size_bytes as f64 / (1024.0 * 1024.0);
            GIT_COMMAND_LOG.log_request(
                &url,
                true,
                "large_file_dialog",
                &format!("File is {size_mb:.1} MB, requesting confirmation"),
                duration,
            );
            dialog_state.set_large_file_dialog(LargeFileDialogData {
                file_path,
                file_size_bytes,
                line,
                column,
                editor_hint,
            });

            #[cfg(target_os = "macos")]
            activate_app();

            #[allow(unused_variables)]
            let window = tauri::WebviewWindowBuilder::new(
                &app,
                "large-file-confirm",
                tauri::WebviewUrl::App("large-file-confirm.html".into()),
            )
            .title("Large File Warning")
            .inner_size(500.0, 280.0)
            .center()
            .resizable(false)
            .always_on_top(true)
            .focused(true)
            .accept_first_mouse(true)
            .build()
            .map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            set_dark_titlebar(&window);

            Ok(())
        }
        Ok(HandleResult::ShowTrustDialog {
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
            GIT_COMMAND_LOG.log_request(
                &url,
                true,
                "trust_dialog",
                &format!(
                    "Workspace '{workspace_name}' has {task_count} auto-run tasks, {vim_rc_count} vim rc files, {dangerous_files_count} dangerous files, {dangerous_settings_count} dangerous settings, requesting trust confirmation"
                ),
                duration,
            );
            dialog_state.set_trust_dialog(TrustDialogData {
                workspace_path: workspace_path.to_string_lossy().to_string(),
                workspace_name,
                task_labels,
                vim_local_rc_files,
                dangerous_files: dangerous_files
                    .into_iter()
                    .map(|f| DangerousFileData {
                        path: f.path,
                        reason: f.reason.to_string(),
                    })
                    .collect(),
                dangerous_settings: dangerous_settings
                    .into_iter()
                    .map(|s| DangerousSettingData {
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

            #[cfg(target_os = "macos")]
            activate_app();

            #[allow(unused_variables)]
            let window = tauri::WebviewWindowBuilder::new(
                &app,
                "trust-warning",
                tauri::WebviewUrl::App("trust-warning.html".into()),
            )
            .title("Security Warning")
            .inner_size(600.0, 520.0)
            .center()
            .resizable(false)
            .always_on_top(true)
            .focused(true)
            .accept_first_mouse(true)
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
                &format!("Opening in browser: {browser_url}"),
                duration,
            );
            if let Err(e) = open::that(&browser_url) {
                return Err(format!("Failed to open browser: {e}"));
            }
            Ok(())
        }
        Ok(HandleResult::Pong) => {
            GIT_COMMAND_LOG.log_request(&url, true, "ping", "Desktop is running", duration);
            Ok(())
        }
        Ok(HandleResult::HelloAck { version }) => {
            let version_str = version.as_deref().unwrap_or("unknown");
            GIT_COMMAND_LOG.log_request(
                &url,
                true,
                "hello",
                &format!("Extension version {version_str} registered"),
                duration,
            );
            Ok(())
        }
        Err(e) => {
            let error_msg = e.to_string();
            GIT_COMMAND_LOG.log_request(&url, false, "error", &error_msg, duration);
            Err(error_msg)
        }
    }
}

#[tauri::command]
pub async fn get_clone_dialog_data(
    dialog_state: State<'_, Arc<DialogState>>,
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<CloneDialogData, String> {
    let data = dialog_state
        .peek_clone_dialog()
        .ok_or_else(|| "No clone dialog data available".to_string())?;
    let settings = settings_manager.get().await;
    let enriched =
        enrich_clone_dialog_data(data, &settings, settings_manager.inner().as_ref()).await;
    dialog_state.set_clone_dialog(enriched.clone());
    Ok(enriched)
}

#[tauri::command]
pub async fn clone_and_open(
    app: tauri::AppHandle,
    dispatcher: State<'_, Arc<EditorDispatcher>>,
    settings_manager: State<'_, Arc<SettingsManager>>,
    dialog_state: State<'_, Arc<DialogState>>,
) -> Result<(), String> {
    let data = dialog_state
        .take_clone_dialog()
        .ok_or_else(|| "No clone dialog data available".to_string())?;
    let settings = settings_manager.get().await;
    let data = enrich_clone_dialog_data(data, &settings, settings_manager.inner().as_ref()).await;

    if !data.clone_allowed {
        return Err(data
            .clone_validation_message
            .unwrap_or_else(|| "Clone path is not valid".to_string()));
    }

    let target_path = PathBuf::from(&data.clone_path);
    let remote_url = data.remote_url.clone();
    let git_ref_kind = data.git_ref_kind.clone();

    // Run clone in a blocking task since it does I/O
    let app_handle = app.clone();
    let clone_result = tokio::task::spawn_blocking(move || {
        GitHandler::clone_repo_with_progress(
            &remote_url,
            &target_path,
            git_ref_kind.as_ref(),
            |output| {
                let _ = app_handle.emit("clone-progress", output);
            },
        )
    })
    .await
    .map_err(|e| format!("Clone task failed: {e}"))?;

    clone_result.map_err(|e| e.to_string())?;

    let target_path = PathBuf::from(&data.clone_path);
    let workspace_key = data
        .suggested_workspace_key
        .clone()
        .unwrap_or_else(|| derive_workspace_key_from_path(&target_path, &data.workspace_name));

    // Add new workspace to settings
    let mut settings = settings_manager.get().await;
    settings.workspaces.push(crate::settings::WorkspaceConfig {
        path: data.clone_path.clone(),
        workspace_key,
        editor: String::new(),
        auto_discovered: false,
        trusted: false,
        workspace_kind: WorkspaceKind::Git,
        workspace_state: WorkspaceState::Present,
        repo_identity: None,
        last_verified_at: None,
        normalized_path: Some(target_path.clone()),
    });
    settings_manager
        .save(settings)
        .await
        .map_err(|e| format!("Failed to save workspace: {e}"))?;

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

    Ok(())
}

#[tauri::command]
pub async fn update_clone_path(
    new_path: String,
    dialog_state: State<'_, Arc<DialogState>>,
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<CloneDialogData, String> {
    if dialog_state.update_clone_path(&new_path) {
        let data = dialog_state
            .peek_clone_dialog()
            .ok_or_else(|| "No clone dialog data available".to_string())?;
        let settings = settings_manager.get().await;
        let enriched =
            enrich_clone_dialog_data(data, &settings, settings_manager.inner().as_ref()).await;
        dialog_state.set_clone_dialog(enriched.clone());
        Ok(enriched)
    } else {
        Err("No clone dialog data available".to_string())
    }
}

#[tauri::command]
#[allow(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::missing_const_for_fn
)]
pub fn clone_cancelled(_dialog_state: State<'_, Arc<DialogState>>) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands require owned State
pub fn get_workspace_repair_dialog_data(
    dialog_state: State<'_, Arc<DialogState>>,
) -> Result<WorkspaceRepairDialogData, String> {
    dialog_state
        .take_workspace_repair_dialog()
        .ok_or_else(|| "No workspace repair dialog data available".to_string())
}

#[tauri::command]
pub async fn rename_workspace_key(
    workspace_key: String,
    new_workspace_key: String,
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<(), String> {
    rename_workspace_key_impl(
        settings_manager.inner().as_ref(),
        &workspace_key,
        &new_workspace_key,
    )
    .await
}

async fn rename_workspace_key_impl(
    settings_manager: &SettingsManager,
    workspace_key: &str,
    new_workspace_key: &str,
) -> Result<(), String> {
    let desired_key = new_workspace_key.trim();
    if desired_key.is_empty() {
        return Err("Workspace key cannot be empty".to_string());
    }

    settings_manager
        .modify(|settings| {
            let lookup_key = identity::canonical_workspace_key_for_lookup(workspace_key);
            let mut found = false;

            for workspace in &mut settings.workspaces {
                if !workspace_matches_lookup_key(workspace, &lookup_key) {
                    continue;
                }

                let current_key = identity::derive_workspace_key(workspace);
                if current_key == desired_key {
                    return Ok(((), false));
                }

                workspace.workspace_key = desired_key.to_string();
                workspace.auto_discovered = false;
                found = true;
                break;
            }

            if !found {
                return Err(anyhow::anyhow!("Workspace key '{workspace_key}' not found"));
            }

            Ok(((), true))
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rebind_workspace_path(
    workspace_key: String,
    new_path: String,
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<(), String> {
    rebind_workspace_path_impl(settings_manager.inner().as_ref(), &workspace_key, &new_path).await
}

async fn rebind_workspace_path_impl(
    settings_manager: &SettingsManager,
    workspace_key: &str,
    new_path: &str,
) -> Result<(), String> {
    settings_manager
        .modify(|settings| {
            let lookup_key = identity::canonical_workspace_key_for_lookup(workspace_key);

            for workspace in &mut settings.workspaces {
                if !workspace_matches_lookup_key(workspace, &lookup_key) {
                    continue;
                }

                workspace.path = new_path.to_string();
                workspace.auto_discovered = false;
                workspace.trusted = false;
                return Ok(((), true));
            }

            Err(anyhow::anyhow!("Workspace key '{workspace_key}' not found"))
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn forget_workspace_by_key(
    workspace_key: String,
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<(), String> {
    forget_workspace_by_key_impl(settings_manager.inner().as_ref(), &workspace_key).await
}

async fn forget_workspace_by_key_impl(
    settings_manager: &SettingsManager,
    workspace_key: &str,
) -> Result<(), String> {
    settings_manager
        .modify(|settings| {
            let lookup_key = identity::canonical_workspace_key_for_lookup(workspace_key);
            let original_len = settings.workspaces.len();

            settings
                .workspaces
                .retain(|workspace| !workspace_matches_lookup_key(workspace, &lookup_key));

            if settings.workspaces.len() == original_len {
                return Err(anyhow::anyhow!("Workspace key '{workspace_key}' not found"));
            }

            Ok(((), true))
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands require owned State
pub fn get_workspace_conflict_dialog_data(
    dialog_state: State<'_, Arc<DialogState>>,
) -> Result<WorkspaceConflictDialogData, String> {
    dialog_state
        .peek_workspace_conflict_dialog()
        .ok_or_else(|| "No workspace conflict dialog data available".to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri commands include AppHandle and State dependencies
pub async fn workspace_conflict_open_existing(
    app: tauri::AppHandle,
    workspace_path: String,
    file_path: String,
    line: Option<usize>,
    column: Option<usize>,
    settings_manager: State<'_, Arc<SettingsManager>>,
    protocol_handler: State<'_, Arc<crate::protocol_handler::ProtocolHandler>>,
    dialog_state: State<'_, Arc<DialogState>>,
) -> Result<(), String> {
    use crate::protocol_handler::HandleResult;

    let full_path = resolve_conflict_open_target_path(&workspace_path, &file_path)?;
    ensure_workspace_policy_allows_path(
        settings_manager.inner().as_ref(),
        Path::new(&workspace_path),
    )
    .await?;

    let result = protocol_handler
        .open_resolved_path(&full_path.to_string_lossy(), line, column)
        .await
        .map_err(|e| e.to_string())?;

    match result {
        HandleResult::Opened { .. } => Ok(()),
        HandleResult::ShowLargeFileDialog {
            file_path,
            file_size_bytes,
            line,
            column,
            editor_hint,
        } => {
            dialog_state.set_large_file_dialog(LargeFileDialogData {
                file_path,
                file_size_bytes,
                line,
                column,
                editor_hint,
            });

            #[cfg(target_os = "macos")]
            activate_app();

            if let Some(window) = app.get_webview_window("large-file-confirm") {
                let _ = window.show();
                let _ = window.set_focus();
                return Ok(());
            }

            #[allow(unused_variables)]
            let window = tauri::WebviewWindowBuilder::new(
                &app,
                "large-file-confirm",
                tauri::WebviewUrl::App("large-file-confirm.html".into()),
            )
            .title("Large File Warning")
            .inner_size(500.0, 280.0)
            .center()
            .resizable(false)
            .always_on_top(true)
            .focused(true)
            .accept_first_mouse(true)
            .build()
            .map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            set_dark_titlebar(&window);

            Ok(())
        }
        HandleResult::ShowTrustDialog {
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
        } => {
            dialog_state.set_trust_dialog(TrustDialogData {
                workspace_path: workspace_path.to_string_lossy().to_string(),
                workspace_name,
                task_labels,
                vim_local_rc_files,
                dangerous_files: dangerous_files
                    .into_iter()
                    .map(|file| DangerousFileData {
                        path: file.path,
                        reason: file.reason.to_string(),
                    })
                    .collect(),
                dangerous_settings: dangerous_settings
                    .into_iter()
                    .map(|setting| DangerousSettingData {
                        key: setting.key,
                        reason: setting.reason.to_string(),
                    })
                    .collect(),
                scan_error,
                pending_file_path,
                line,
                column,
                editor_hint,
            });

            #[cfg(target_os = "macos")]
            activate_app();

            if let Some(window) = app.get_webview_window("trust-warning") {
                let _ = window.show();
                let _ = window.set_focus();
                return Ok(());
            }

            #[allow(unused_variables)]
            let window = tauri::WebviewWindowBuilder::new(
                &app,
                "trust-warning",
                tauri::WebviewUrl::App("trust-warning.html".into()),
            )
            .title("Security Warning")
            .inner_size(600.0, 520.0)
            .center()
            .resizable(false)
            .always_on_top(true)
            .focused(true)
            .accept_first_mouse(true)
            .build()
            .map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            set_dark_titlebar(&window);

            Ok(())
        }
        other => Err(format!(
            "Unexpected result while opening workspace mapping: {other:?}"
        )),
    }
}

fn resolve_conflict_open_target_path(
    workspace_path: &str,
    file_path: &str,
) -> Result<PathBuf, String> {
    let full_path = PathBuf::from(workspace_path).join(file_path);
    if full_path.exists() {
        return Ok(full_path);
    }

    Err(format!(
        "File '{}' not found under workspace '{}'",
        file_path, workspace_path
    ))
}

async fn ensure_workspace_policy_allows_path(
    settings_manager: &SettingsManager,
    workspace_path: &Path,
) -> Result<(), String> {
    if let Some(mapping) = settings_manager
        .get_workspace_for_path(workspace_path)
        .await
    {
        if let PolicyDecision::EnforcedViolation(violation) =
            settings_manager.evaluate_workspace_policy(&mapping).await
        {
            return Err(violation.to_string());
        }
    }

    Ok(())
}

#[tauri::command]
pub fn workspace_conflict_open_clone_dialog(
    app: tauri::AppHandle,
    dialog_state: State<'_, Arc<DialogState>>,
) -> Result<(), String> {
    let conflict_data = dialog_state
        .take_workspace_conflict_dialog()
        .ok_or_else(|| "No workspace conflict dialog data available".to_string())?;

    dialog_state.set_clone_dialog(CloneDialogData {
        workspace_name: conflict_data.workspace_key,
        clone_path: conflict_data.clone_path,
        remote_url: conflict_data.requested_remote,
        normalized_remote: conflict_data.normalized_remote,
        policy_violation: conflict_data.policy_violation,
        file_path: conflict_data.file_path,
        line: conflict_data.line,
        column: conflict_data.column,
        git_ref: conflict_data.git_ref,
        clone_allowed: true,
        clone_validation_message: None,
        suggested_workspace_key: None,
        git_ref_kind: conflict_data.git_ref_kind,
    });

    if let Some(window) = app.get_webview_window("clone-dialog") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    activate_app();

    #[allow(unused_variables)]
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
    .accept_first_mouse(true)
    .build()
    .map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    set_dark_titlebar(&window);

    Ok(())
}

fn derive_workspace_key_from_path(path: &Path, fallback: &str) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map_or_else(|| fallback.to_string(), ToString::to_string)
}

fn workspace_matches_lookup_key(
    workspace: &crate::settings::WorkspaceConfig,
    lookup_key: &str,
) -> bool {
    identity::canonical_workspace_key_for_lookup(&identity::derive_workspace_key(workspace))
        == lookup_key
}

async fn enrich_clone_dialog_data(
    mut data: CloneDialogData,
    settings: &Settings,
    settings_manager: &SettingsManager,
) -> CloneDialogData {
    let target_path = PathBuf::from(&data.clone_path);
    let normalized_remote = identity::normalize_remote_identity(&data.remote_url);
    data.normalized_remote = normalized_remote.clone();
    data.policy_violation = None;

    let desired_lookup = identity::canonical_workspace_key_for_lookup(&data.workspace_name);
    let key_matches: Vec<_> = settings
        .workspaces
        .iter()
        .filter(|workspace| {
            identity::canonical_workspace_key_for_lookup(&workspace.workspace_key) == desired_lookup
        })
        .collect();

    let same_remote_key_exists = normalized_remote.as_ref().is_some_and(|remote| {
        key_matches.iter().any(|workspace| {
            workspace
                .repo_identity
                .as_ref()
                .is_some_and(|identity| identity::remote_matches_identity(remote, identity))
        })
    });

    let different_remote_key_exists = !key_matches.is_empty() && !same_remote_key_exists;
    let target_exists = target_path.exists();

    data.clone_allowed = true;
    data.clone_validation_message = None;
    data.suggested_workspace_key = None;

    if same_remote_key_exists {
        data.clone_allowed = false;
        data.clone_validation_message = Some(
            "This workspace key already maps to the same remote. Open the existing mapping instead."
                .to_string(),
        );
        return data;
    }

    if target_exists {
        data.clone_allowed = false;
        data.clone_validation_message =
            Some("Clone target already exists. Choose a new location before cloning.".to_string());
        return data;
    }

    if different_remote_key_exists {
        let suggested_key = derive_workspace_key_from_path(&target_path, &data.workspace_name);
        let suggested_key =
            if identity::canonical_workspace_key_for_lookup(&suggested_key) == desired_lookup {
                format!("{suggested_key}-clone")
            } else {
                suggested_key
            };

        data.suggested_workspace_key = Some(suggested_key.clone());
        data.clone_validation_message = Some(format!(
            "Workspace key collision detected. Clone will be saved as '{suggested_key}'."
        ));
    }

    match settings_manager
        .evaluate_clone_policy(&data.workspace_name, Some(&data.remote_url), &target_path)
        .await
    {
        PolicyDecision::Allowed => {}
        PolicyDecision::AdvisoryViolation(violation) => {
            data.policy_violation = Some(violation.to_string());
        }
        PolicyDecision::EnforcedViolation(violation) => {
            let reason = violation.to_string();
            data.policy_violation = Some(reason.clone());
            data.clone_allowed = false;
            data.clone_validation_message = Some(reason);
        }
    }

    data
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands require owned State
pub fn get_large_file_dialog_data(
    dialog_state: State<'_, Arc<DialogState>>,
) -> Result<LargeFileDialogData, String> {
    dialog_state
        .take_large_file_dialog()
        .ok_or_else(|| "No large file dialog data available".to_string())
}

#[tauri::command]
pub async fn large_file_confirmed(
    file_path: String,
    line: Option<usize>,
    column: Option<usize>,
    editor_hint: Option<String>,
    dispatcher: State<'_, Arc<EditorDispatcher>>,
) -> Result<(), String> {
    dispatcher
        .open(&file_path, line, column, false, editor_hint)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::unnecessary_wraps, clippy::missing_const_for_fn)]
pub fn large_file_cancelled() -> Result<(), String> {
    Ok(())
}

#[tauri::command]
#[allow(clippy::unnecessary_wraps)]
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
            let path = home.join(".local/share/sorcery/logs");
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
            .map_err(|e| format!("Failed to open logs directory: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&info.path)
            .spawn()
            .map_err(|e| format!("Failed to open logs directory: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&info.path)
            .spawn()
            .map_err(|e| format!("Failed to open logs directory: {e}"))?;
    }

    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri commands require owned State
pub fn get_trust_dialog_data(
    dialog_state: State<'_, Arc<DialogState>>,
) -> Result<TrustDialogData, String> {
    dialog_state
        .take_trust_dialog()
        .ok_or_else(|| "No trust dialog data available".to_string())
}

#[tauri::command]
pub async fn trust_confirmed(
    workspace_path: String,
    file_path: String,
    line: Option<usize>,
    column: Option<usize>,
    editor_hint: Option<String>,
    settings_manager: State<'_, Arc<SettingsManager>>,
    dispatcher: State<'_, Arc<EditorDispatcher>>,
) -> Result<(), String> {
    let workspace = PathBuf::from(&workspace_path);

    settings_manager
        .trust_workspace(&workspace)
        .await
        .map_err(|e| format!("Failed to trust workspace: {e}"))?;

    tracing::info!("Workspace '{}' marked as trusted", workspace_path);

    dispatcher
        .open(&file_path, line, column, false, editor_hint)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::unnecessary_wraps)]
pub fn trust_cancelled() -> Result<(), String> {
    tracing::info!("Trust dialog cancelled");
    Ok(())
}

#[derive(Serialize)]
pub struct SetupEditorInfo {
    pub editor_id: String,
    pub display_name: String,
    pub is_installed: bool,
}

#[derive(Serialize)]
pub struct FolderSuggestion {
    pub path: String,
    pub repo_count: usize,
}

#[derive(Serialize)]
pub struct SetupData {
    pub editors: Vec<SetupEditorInfo>,
    pub current_editor: String,
    pub detected_folder: String,
    pub folder_suggestions: Vec<FolderSuggestion>,
}

#[tauri::command]
pub async fn get_setup_data(
    registry: State<'_, Arc<EditorRegistry>>,
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<SetupData, String> {
    let settings = settings_manager.get().await;

    let mut editors = Vec::new();
    for editor_id in registry.list_editors() {
        if let Some(manager) = registry.get(&editor_id) {
            let is_installed = manager.is_installed().await;
            editors.push(SetupEditorInfo {
                editor_id: editor_id.clone(),
                display_name: manager.display_name().to_string(),
                is_installed,
            });
        }
    }

    // Sort: installed first, then alphabetically by display name
    editors.sort_by(|a, b| match (a.is_installed, b.is_installed) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.display_name.cmp(&b.display_name),
    });

    let detected_folder = detect_source_folder()
        .await
        .unwrap_or_else(|_| "~/code".to_string());

    let mut folder_suggestions = Vec::new();
    let home_dir = dirs::home_dir().ok_or_else(|| "Could not find home directory".to_string())?;

    let candidates = ["code", "repos", "projects", "dev", "src", "apps", "work"];
    for candidate in candidates {
        let path = home_dir.join(candidate);
        if path.is_dir() {
            let repo_count = count_git_repos(&path).unwrap_or(0);
            if repo_count > 0 {
                folder_suggestions.push(FolderSuggestion {
                    path: path.to_string_lossy().to_string(),
                    repo_count,
                });
            }
        }
    }

    // Sort by repo count descending
    folder_suggestions.sort_by(|a, b| b.repo_count.cmp(&a.repo_count));

    // Include current setting if different
    let current_folder = settings.defaults.default_workspaces_folder.clone();
    let expanded_current = shellexpand::tilde(&current_folder).to_string();
    if !folder_suggestions
        .iter()
        .any(|f| f.path == expanded_current)
    {
        let path = PathBuf::from(&expanded_current);
        let repo_count = if path.is_dir() {
            count_git_repos(&path).unwrap_or(0)
        } else {
            0
        };
        folder_suggestions.insert(
            0,
            FolderSuggestion {
                path: expanded_current,
                repo_count,
            },
        );
    }

    let best_folder = folder_suggestions
        .first()
        .map_or(detected_folder, |f| f.path.clone());

    Ok(SetupData {
        editors,
        current_editor: settings.defaults.editor,
        detected_folder: best_folder,
        folder_suggestions,
    })
}

#[tauri::command]
pub async fn complete_setup(
    editor: String,
    workspaces_folder: String,
    settings_manager: State<'_, Arc<SettingsManager>>,
    workspace_sync: State<'_, Arc<crate::settings::WorkspaceSync>>,
) -> Result<(), String> {
    let mut settings = settings_manager.get().await;

    settings.defaults.editor = editor;
    settings.defaults.default_workspaces_folder = workspaces_folder;
    settings.defaults.setup_completed = true;

    settings_manager
        .save(settings)
        .await
        .map_err(|e| e.to_string())?;

    // Sync workspaces from the selected folder
    if let Err(e) = workspace_sync.sync().await {
        tracing::warn!("Failed to sync workspaces after setup: {}", e);
    }

    tracing::info!("Setup completed successfully");
    Ok(())
}

#[tauri::command]
pub async fn is_setup_needed(
    settings_manager: State<'_, Arc<SettingsManager>>,
) -> Result<bool, String> {
    Ok(settings_manager.is_setup_needed().await)
}

#[tauri::command]
pub fn detect_browsers() -> Vec<crate::browser_detection::BrowserInfo> {
    crate::browser_detection::BrowserDetector::detect_browsers()
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_workspace_policy_allows_path, forget_workspace_by_key_impl,
        rebind_workspace_path_impl, rename_workspace_key_impl, resolve_conflict_open_target_path,
    };
    use crate::settings::{
        Settings, SettingsManager, WorkspaceConfig, WorkspaceKind, WorkspaceState,
    };
    use std::path::{Path, PathBuf};

    fn explicit_workspace(path: &Path, key: &str) -> WorkspaceConfig {
        WorkspaceConfig {
            path: path.to_string_lossy().to_string(),
            workspace_key: key.to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        }
    }

    async fn initialize_manager(
        workspaces: Vec<WorkspaceConfig>,
    ) -> (tempfile::TempDir, SettingsManager) {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let settings_path = temp_dir.path().join("settings.yaml");
        let manager = SettingsManager::new_with_path(settings_path)
            .await
            .expect("settings manager");
        let settings = Settings {
            workspaces,
            ..Settings::default()
        };
        manager.save(settings).await.expect("save settings");
        (temp_dir, manager)
    }

    fn path_string(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }

    #[tokio::test]
    async fn rename_workspace_key_updates_mapping() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let workspace_path = temp_dir.path().join("rails");
        std::fs::create_dir_all(&workspace_path).expect("workspace dir");

        let (_temp_dir, manager) =
            initialize_manager(vec![explicit_workspace(&workspace_path, "rails")]).await;

        rename_workspace_key_impl(&manager, "rails", "rails-upstream")
            .await
            .expect("rename key");

        let settings = manager.get().await;
        assert_eq!(settings.workspaces.len(), 1);
        assert_eq!(settings.workspaces[0].workspace_key, "rails-upstream");
    }

    #[tokio::test]
    async fn rename_workspace_key_rejects_duplicates() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let first_path = temp_dir.path().join("rails");
        let second_path = temp_dir.path().join("myapp");
        std::fs::create_dir_all(&first_path).expect("first path");
        std::fs::create_dir_all(&second_path).expect("second path");

        let (_temp_dir, manager) = initialize_manager(vec![
            explicit_workspace(&first_path, "rails"),
            explicit_workspace(&second_path, "myapp"),
        ])
        .await;

        let error = rename_workspace_key_impl(&manager, "myapp", "rails")
            .await
            .expect_err("rename should fail");
        assert!(error.contains("must be unique"));
    }

    #[tokio::test]
    async fn rebind_workspace_path_updates_path_and_resets_flags() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let old_path = temp_dir.path().join("old");
        let new_path = temp_dir.path().join("new");
        std::fs::create_dir_all(&old_path).expect("old path");
        std::fs::create_dir_all(&new_path).expect("new path");

        let mut workspace = explicit_workspace(&old_path, "repo");
        workspace.auto_discovered = true;
        workspace.trusted = true;

        let (_temp_dir, manager) = initialize_manager(vec![workspace]).await;

        rebind_workspace_path_impl(&manager, "repo", &path_string(&new_path))
            .await
            .expect("rebind");

        let settings = manager.get().await;
        let saved = &settings.workspaces[0];
        assert_eq!(saved.path, path_string(&new_path));
        assert!(!saved.auto_discovered);
        assert!(!saved.trusted);
    }

    #[tokio::test]
    async fn forget_workspace_by_key_removes_only_matching_workspace() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let first_path = temp_dir.path().join("repo-a");
        let second_path = temp_dir.path().join("repo-b");
        std::fs::create_dir_all(&first_path).expect("first path");
        std::fs::create_dir_all(&second_path).expect("second path");

        let (_temp_dir, manager) = initialize_manager(vec![
            explicit_workspace(&first_path, "repo-a"),
            explicit_workspace(&second_path, "repo-b"),
        ])
        .await;

        forget_workspace_by_key_impl(&manager, "repo-a")
            .await
            .expect("forget workspace");

        let settings = manager.get().await;
        assert_eq!(settings.workspaces.len(), 1);
        assert_eq!(settings.workspaces[0].workspace_key, "repo-b");
    }

    #[tokio::test]
    async fn forget_workspace_by_key_returns_error_for_unknown_key() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let workspace_path = temp_dir.path().join("repo");
        std::fs::create_dir_all(&workspace_path).expect("workspace path");

        let (_temp_dir, manager) =
            initialize_manager(vec![explicit_workspace(&workspace_path, "repo")]).await;

        let error = forget_workspace_by_key_impl(&manager, "unknown")
            .await
            .expect_err("missing key should fail");
        assert!(error.contains("not found"));
    }

    #[tokio::test]
    async fn conflict_rebind_flow_resolves_file_in_new_path() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let old_path = temp_dir.path().join("old-location");
        let new_path = temp_dir.path().join("new-location");
        let file_relative = PathBuf::from("src/main.rs");
        std::fs::create_dir_all(&old_path).expect("old path");
        std::fs::create_dir_all(new_path.join("src")).expect("new path");
        std::fs::write(new_path.join(&file_relative), "fn main() {}\n").expect("write file");

        let (_temp_dir, manager) =
            initialize_manager(vec![explicit_workspace(&old_path, "workspace")]).await;

        rebind_workspace_path_impl(&manager, "workspace", &path_string(&new_path))
            .await
            .expect("rebind workspace");

        let full_path = resolve_conflict_open_target_path(
            &path_string(&new_path),
            file_relative.to_string_lossy().as_ref(),
        )
        .expect("resolved path");
        assert_eq!(full_path, new_path.join(&file_relative));

        ensure_workspace_policy_allows_path(&manager, Path::new(&path_string(&new_path)))
            .await
            .expect("policy check");
    }
}
