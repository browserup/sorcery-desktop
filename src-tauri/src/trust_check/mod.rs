use std::fs;
use std::path::Path;
use tracing::{debug, warn};

#[cfg(test)]
mod tests;

const MAX_TASKS_FILE_SIZE: u64 = 1024 * 1024; // 1 MB
const MAX_SETTINGS_FILE_SIZE: u64 = 1024 * 1024; // 1 MB
const VIM_LOCAL_RC_FILES: &[&str] = &[".exrc", ".vimrc", ".gvimrc"];

const DANGEROUS_FILE_PATTERNS: &[(&str, &str)] = &[
    // Ruby LSP - auto-loads addon.rb files matching this pattern
    ("**/ruby_lsp/**/addon.rb", "Ruby LSP auto-loads this file"),
];

const DANGEROUS_SETTINGS_KEYS: &[(&str, &str)] = &[
    // Ruby LSP - executes shell command to activate Ruby environment
    (
        "rubyLsp.customRubyCommand",
        "executes shell command to activate Ruby",
    ),
];

#[derive(Debug, Clone)]
pub struct DangerousFile {
    pub path: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone)]
pub struct DangerousSetting {
    pub key: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone)]
pub struct TrustScanResult {
    pub has_auto_tasks: bool,
    pub task_labels: Vec<String>,
    pub vim_local_rc_files: Vec<String>,
    pub dangerous_files: Vec<DangerousFile>,
    pub dangerous_settings: Vec<DangerousSetting>,
    pub scan_error: Option<String>,
}

fn scan_for_vim_local_rc(workspace_root: &Path) -> Vec<String> {
    VIM_LOCAL_RC_FILES
        .iter()
        .filter(|name| workspace_root.join(name).exists())
        .map(|name| (*name).to_string())
        .collect()
}

fn scan_for_dangerous_files(workspace_path: &Path) -> Vec<DangerousFile> {
    let mut results = Vec::new();

    for (pattern, reason) in DANGEROUS_FILE_PATTERNS {
        let full_pattern = workspace_path.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();

        match glob::glob(&pattern_str) {
            Ok(paths) => {
                for entry in paths.flatten() {
                    if let Ok(relative) = entry.strip_prefix(workspace_path) {
                        results.push(DangerousFile {
                            path: relative.to_string_lossy().into_owned(),
                            reason,
                        });
                    }
                }
            }
            Err(e) => {
                warn!("Invalid glob pattern '{pattern}': {e}");
            }
        }
    }

    results
}

fn scan_for_dangerous_settings(workspace_path: &Path) -> Vec<DangerousSetting> {
    let settings_file = workspace_path.join(".vscode").join("settings.json");
    if !settings_file.exists() {
        return Vec::new();
    }

    let metadata = match fs::metadata(&settings_file) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to read settings.json metadata: {e}");
            return Vec::new();
        }
    };

    let len = metadata.len();
    if len > MAX_SETTINGS_FILE_SIZE {
        warn!("settings.json exceeds maximum size ({len} bytes > {MAX_SETTINGS_FILE_SIZE} bytes)");
        return Vec::new();
    }

    let content = match fs::read_to_string(&settings_file) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read settings.json: {e}");
            return Vec::new();
        }
    };

    parse_settings_for_dangerous_keys(&content)
}

fn parse_settings_for_dangerous_keys(content: &str) -> Vec<DangerousSetting> {
    let parsed =
        match jsonc_parser::parse_to_serde_value(content, &jsonc_parser::ParseOptions::default()) {
            Ok(Some(value)) => value,
            Ok(None) => return Vec::new(),
            Err(e) => {
                warn!("Failed to parse settings.json: {e}");
                return Vec::new();
            }
        };

    let Some(obj) = parsed.as_object() else {
        return Vec::new();
    };

    let mut results = Vec::new();

    for (key, reason) in DANGEROUS_SETTINGS_KEYS {
        if obj.contains_key(*key) {
            results.push(DangerousSetting {
                key: (*key).to_string(),
                reason,
            });
        }
    }

    results
}

pub fn scan_workspace_for_auto_tasks(workspace_path: &Path) -> TrustScanResult {
    let vim_local_rc_files = scan_for_vim_local_rc(workspace_path);
    let dangerous_files = scan_for_dangerous_files(workspace_path);
    let dangerous_settings = scan_for_dangerous_settings(workspace_path);
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
            dangerous_files,
            dangerous_settings,
            scan_error: None,
        };
    }

    let metadata = match fs::metadata(&tasks_file) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to read tasks.json metadata: {e}");
            return TrustScanResult {
                has_auto_tasks: true,
                task_labels: Vec::new(),
                vim_local_rc_files,
                dangerous_files,
                dangerous_settings,
                scan_error: Some(format!("Failed to read tasks.json: {e}")),
            };
        }
    };

    let len = metadata.len();
    if len > MAX_TASKS_FILE_SIZE {
        warn!("tasks.json exceeds maximum size ({len} bytes > {MAX_TASKS_FILE_SIZE} bytes)");
        return TrustScanResult {
            has_auto_tasks: true,
            task_labels: Vec::new(),
            vim_local_rc_files,
            dangerous_files,
            dangerous_settings,
            scan_error: Some("tasks.json exceeds maximum allowed size (1 MB)".to_string()),
        };
    }

    let content = match fs::read_to_string(&tasks_file) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read tasks.json: {e}");
            return TrustScanResult {
                has_auto_tasks: true,
                task_labels: Vec::new(),
                vim_local_rc_files,
                dangerous_files,
                dangerous_settings,
                scan_error: Some(format!("Failed to read tasks.json: {e}")),
            };
        }
    };

    let mut result = parse_tasks_json(&content);
    result.vim_local_rc_files = vim_local_rc_files;
    result.dangerous_files = dangerous_files;
    result.dangerous_settings = dangerous_settings;
    result
}

fn parse_tasks_json(content: &str) -> TrustScanResult {
    let parsed =
        match jsonc_parser::parse_to_serde_value(content, &jsonc_parser::ParseOptions::default()) {
            Ok(Some(value)) => value,
            Ok(None) => {
                return TrustScanResult {
                    has_auto_tasks: false,
                    task_labels: Vec::new(),
                    vim_local_rc_files: Vec::new(),
                    dangerous_files: Vec::new(),
                    dangerous_settings: Vec::new(),
                    scan_error: None,
                };
            }
            Err(e) => {
                warn!("Failed to parse tasks.json: {e}");
                return TrustScanResult {
                    has_auto_tasks: true,
                    task_labels: Vec::new(),
                    vim_local_rc_files: Vec::new(),
                    dangerous_files: Vec::new(),
                    dangerous_settings: Vec::new(),
                    scan_error: Some(format!("Invalid JSON in tasks.json: {e}")),
                };
            }
        };

    let Some(serde_json::Value::Array(tasks)) = parsed.get("tasks") else {
        return TrustScanResult {
            has_auto_tasks: false,
            task_labels: Vec::new(),
            vim_local_rc_files: Vec::new(),
            dangerous_files: Vec::new(),
            dangerous_settings: Vec::new(),
            scan_error: None,
        };
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
        dangerous_files: Vec::new(),
        dangerous_settings: Vec::new(),
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

    let has_risks = result.has_auto_tasks
        || !result.vim_local_rc_files.is_empty()
        || !result.dangerous_files.is_empty()
        || !result.dangerous_settings.is_empty()
        || result.scan_error.is_some();

    if has_risks {
        Some(result)
    } else {
        None
    }
}
