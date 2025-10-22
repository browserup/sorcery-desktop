use crate::dispatcher::EditorDispatcher;
use crate::editors::EditorRegistry;
use crate::settings::{Settings, SettingsManager};
use crate::tracker::ActiveEditorTracker;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

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
pub async fn save_settings(
    settings_manager: State<'_, Arc<SettingsManager>>,
    settings: Settings,
) -> Result<(), String> {
    settings_manager.save(settings).await
        .map_err(|e| e.to_string())
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
            let instances = manager.get_running_instances().await.ok().unwrap_or_default();

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
        .open(&file_path, Some(50), true, Some(editor_id.clone()))
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!("Opened {} in {}", file_path, editor_id))
}

#[tauri::command]
pub async fn open_in_editor(
    dispatcher: State<'_, Arc<EditorDispatcher>>,
    path: String,
    line: Option<usize>,
    new_window: bool,
    editor: Option<String>,
) -> Result<(), String> {
    dispatcher
        .open(&path, line, new_window, editor)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn detect_source_folder() -> Result<String, String> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?;

    let candidate_names = [
        "code", "repos", "development", "developer", "workspace",
        "src", "dev", "apps", "projects", "project", "github"
    ];

    let mut best_folder: Option<PathBuf> = None;
    let mut max_git_count = 0;

    if let Ok(entries) = std::fs::read_dir(&home_dir) {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                let file_name_lower = file_name.to_lowercase();

                if candidate_names.iter().any(|&name| file_name_lower == name) {
                    let candidate_path = entry.path();

                    if !candidate_path.is_dir() {
                        continue;
                    }

                    if let Ok(git_count) = count_git_repos(&candidate_path) {
                        if git_count > max_git_count {
                            max_git_count = git_count;
                            best_folder = Some(candidate_path);
                        }
                    }
                }
            }
        }
    }

    let result = best_folder.unwrap_or(home_dir);
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
