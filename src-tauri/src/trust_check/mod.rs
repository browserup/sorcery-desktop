use std::fs;
use std::path::Path;
use tracing::{debug, warn};

#[cfg(test)]
mod tests;

const MAX_TASKS_FILE_SIZE: u64 = 1024 * 1024; // 1 MB
const VIM_LOCAL_RC_FILES: &[&str] = &[".exrc", ".vimrc", ".gvimrc"];

#[derive(Debug, Clone)]
pub struct TrustScanResult {
    pub has_auto_tasks: bool,
    pub task_labels: Vec<String>,
    pub vim_local_rc_files: Vec<String>,
    pub scan_error: Option<String>,
}

fn scan_for_vim_local_rc(workspace_root: &Path) -> Vec<String> {
    VIM_LOCAL_RC_FILES
        .iter()
        .filter(|name| workspace_root.join(name).exists())
        .map(|name| (*name).to_string())
        .collect()
}

pub fn scan_workspace_for_auto_tasks(workspace_path: &Path) -> TrustScanResult {
    let vim_local_rc_files = scan_for_vim_local_rc(workspace_path);
    let tasks_file = workspace_path.join(".vscode").join("tasks.json");

    if !tasks_file.exists() {
        debug!(
            "No .vscode/tasks.json found in {}",
            workspace_path.display()
        );
        return TrustScanResult {
            has_auto_tasks: false,
            task_labels: Vec::new(),
            vim_local_rc_files,
            scan_error: None,
        };
    }

    let metadata = match fs::metadata(&tasks_file) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to read tasks.json metadata: {}", e);
            return TrustScanResult {
                has_auto_tasks: true,
                task_labels: Vec::new(),
                vim_local_rc_files,
                scan_error: Some(format!("Failed to read tasks.json: {}", e)),
            };
        }
    };

    if metadata.len() > MAX_TASKS_FILE_SIZE {
        warn!(
            "tasks.json exceeds maximum size ({} bytes > {} bytes)",
            metadata.len(),
            MAX_TASKS_FILE_SIZE
        );
        return TrustScanResult {
            has_auto_tasks: true,
            task_labels: Vec::new(),
            vim_local_rc_files,
            scan_error: Some("tasks.json exceeds maximum allowed size (1 MB)".to_string()),
        };
    }

    let content = match fs::read_to_string(&tasks_file) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read tasks.json: {}", e);
            return TrustScanResult {
                has_auto_tasks: true,
                task_labels: Vec::new(),
                vim_local_rc_files,
                scan_error: Some(format!("Failed to read tasks.json: {}", e)),
            };
        }
    };

    let mut result = parse_tasks_json(&content);
    result.vim_local_rc_files = vim_local_rc_files;
    result
}

fn parse_tasks_json(content: &str) -> TrustScanResult {
    let parsed = match jsonc_parser::parse_to_serde_value(content, &Default::default()) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return TrustScanResult {
                has_auto_tasks: false,
                task_labels: Vec::new(),
                vim_local_rc_files: Vec::new(),
                scan_error: None,
            };
        }
        Err(e) => {
            warn!("Failed to parse tasks.json: {}", e);
            return TrustScanResult {
                has_auto_tasks: true,
                task_labels: Vec::new(),
                vim_local_rc_files: Vec::new(),
                scan_error: Some(format!("Invalid JSON in tasks.json: {}", e)),
            };
        }
    };

    let tasks = match parsed.get("tasks") {
        Some(serde_json::Value::Array(arr)) => arr,
        _ => {
            return TrustScanResult {
                has_auto_tasks: false,
                task_labels: Vec::new(),
                vim_local_rc_files: Vec::new(),
                scan_error: None,
            };
        }
    };

    let mut auto_task_labels = Vec::new();

    for task in tasks {
        if is_auto_run_task(task) {
            let label = task
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("(unnamed task)")
                .to_string();
            auto_task_labels.push(label);
        }
    }

    TrustScanResult {
        has_auto_tasks: !auto_task_labels.is_empty(),
        task_labels: auto_task_labels,
        vim_local_rc_files: Vec::new(),
        scan_error: None,
    }
}

fn is_auto_run_task(task: &serde_json::Value) -> bool {
    task.get("runOptions")
        .and_then(|opts| opts.get("runOn"))
        .and_then(|run_on| run_on.as_str())
        .is_some_and(|s| s == "folderOpen")
}

pub fn needs_trust_check(workspace_path: &Path, is_trusted: bool) -> Option<TrustScanResult> {
    if is_trusted {
        debug!(
            "Workspace {} is trusted, skipping auto-task scan",
            workspace_path.display()
        );
        return None;
    }

    let result = scan_workspace_for_auto_tasks(workspace_path);

    if result.has_auto_tasks || !result.vim_local_rc_files.is_empty() || result.scan_error.is_some()
    {
        Some(result)
    } else {
        None
    }
}
