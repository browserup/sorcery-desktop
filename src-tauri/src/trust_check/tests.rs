use super::*;
use tempfile::TempDir;

fn create_test_workspace() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

fn write_tasks_json(workspace: &TempDir, content: &str) {
    let vscode_dir = workspace.path().join(".vscode");
    std::fs::create_dir_all(&vscode_dir).expect("Failed to create .vscode dir");
    std::fs::write(vscode_dir.join("tasks.json"), content).expect("Failed to write tasks.json");
}

#[test]
fn no_vscode_tasks_json_returns_no_risk() {
    let workspace = create_test_workspace();
    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(!result.has_auto_tasks);
    assert!(result.task_labels.is_empty());
    assert!(result.vim_local_rc_files.is_empty());
    assert!(result.scan_error.is_none());
}

#[test]
fn empty_tasks_array_returns_no_risk() {
    let workspace = create_test_workspace();
    write_tasks_json(
        &workspace,
        r#"{
            "version": "2.0.0",
            "tasks": []
        }"#,
    );

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(!result.has_auto_tasks);
    assert!(result.task_labels.is_empty());
    assert!(result.scan_error.is_none());
}

#[test]
fn task_without_run_on_returns_no_risk() {
    let workspace = create_test_workspace();
    write_tasks_json(
        &workspace,
        r#"{
            "version": "2.0.0",
            "tasks": [
                {
                    "label": "Build",
                    "type": "shell",
                    "command": "npm run build"
                }
            ]
        }"#,
    );

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(!result.has_auto_tasks);
    assert!(result.task_labels.is_empty());
    assert!(result.scan_error.is_none());
}

#[test]
fn task_with_folder_open_detected() {
    let workspace = create_test_workspace();
    write_tasks_json(
        &workspace,
        r#"{
            "version": "2.0.0",
            "tasks": [
                {
                    "label": "Auto Build",
                    "type": "shell",
                    "command": "npm run build",
                    "runOptions": {
                        "runOn": "folderOpen"
                    }
                }
            ]
        }"#,
    );

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(result.has_auto_tasks);
    assert_eq!(result.task_labels.len(), 1);
    assert_eq!(result.task_labels[0], "Auto Build");
    assert!(result.scan_error.is_none());
}

#[test]
fn multiple_auto_tasks_all_collected() {
    let workspace = create_test_workspace();
    write_tasks_json(
        &workspace,
        r#"{
            "version": "2.0.0",
            "tasks": [
                {
                    "label": "Task One",
                    "type": "shell",
                    "command": "echo one",
                    "runOptions": {
                        "runOn": "folderOpen"
                    }
                },
                {
                    "label": "Normal Task",
                    "type": "shell",
                    "command": "echo normal"
                },
                {
                    "label": "Task Two",
                    "type": "shell",
                    "command": "echo two",
                    "runOptions": {
                        "runOn": "folderOpen"
                    }
                }
            ]
        }"#,
    );

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(result.has_auto_tasks);
    assert_eq!(result.task_labels.len(), 2);
    assert!(result.task_labels.contains(&"Task One".to_string()));
    assert!(result.task_labels.contains(&"Task Two".to_string()));
    assert!(result.scan_error.is_none());
}

#[test]
fn jsonc_comments_handled() {
    let workspace = create_test_workspace();
    write_tasks_json(
        &workspace,
        r#"{
            // This is a comment
            "version": "2.0.0",
            "tasks": [
                {
                    "label": "Auto Task",
                    /* block comment */
                    "type": "shell",
                    "command": "echo test",
                    "runOptions": {
                        "runOn": "folderOpen" // inline comment
                    }
                }
            ]
        }"#,
    );

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(result.has_auto_tasks);
    assert_eq!(result.task_labels.len(), 1);
    assert_eq!(result.task_labels[0], "Auto Task");
    assert!(result.scan_error.is_none());
}

#[test]
fn invalid_json_treated_as_risky_with_error() {
    let workspace = create_test_workspace();
    write_tasks_json(&workspace, "{ this is not valid json }");

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(result.has_auto_tasks);
    assert!(result.task_labels.is_empty());
    assert!(result.scan_error.is_some());
    assert!(result.scan_error.unwrap().contains("Invalid JSON"));
}

#[test]
fn trusted_workspace_skips_check() {
    let workspace = create_test_workspace();
    write_tasks_json(
        &workspace,
        r#"{
            "version": "2.0.0",
            "tasks": [
                {
                    "label": "Auto Task",
                    "type": "shell",
                    "command": "echo test",
                    "runOptions": {
                        "runOn": "folderOpen"
                    }
                }
            ]
        }"#,
    );

    let result = needs_trust_check(workspace.path(), true);
    assert!(result.is_none());
}

#[test]
fn untrusted_workspace_with_risk_triggers_dialog() {
    let workspace = create_test_workspace();
    write_tasks_json(
        &workspace,
        r#"{
            "version": "2.0.0",
            "tasks": [
                {
                    "label": "Auto Task",
                    "type": "shell",
                    "command": "echo test",
                    "runOptions": {
                        "runOn": "folderOpen"
                    }
                }
            ]
        }"#,
    );

    let result = needs_trust_check(workspace.path(), false);
    assert!(result.is_some());

    let scan_result = result.unwrap();
    assert!(scan_result.has_auto_tasks);
    assert_eq!(scan_result.task_labels.len(), 1);
}

#[test]
fn untrusted_workspace_without_risk_returns_none() {
    let workspace = create_test_workspace();
    write_tasks_json(
        &workspace,
        r#"{
            "version": "2.0.0",
            "tasks": [
                {
                    "label": "Normal Task",
                    "type": "shell",
                    "command": "echo test"
                }
            ]
        }"#,
    );

    let result = needs_trust_check(workspace.path(), false);
    assert!(result.is_none());
}

#[test]
fn task_without_label_uses_unnamed_placeholder() {
    let workspace = create_test_workspace();
    write_tasks_json(
        &workspace,
        r#"{
            "version": "2.0.0",
            "tasks": [
                {
                    "type": "shell",
                    "command": "echo test",
                    "runOptions": {
                        "runOn": "folderOpen"
                    }
                }
            ]
        }"#,
    );

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(result.has_auto_tasks);
    assert_eq!(result.task_labels.len(), 1);
    assert_eq!(result.task_labels[0], "(unnamed task)");
}

#[test]
fn run_on_default_ignored() {
    let workspace = create_test_workspace();
    write_tasks_json(
        &workspace,
        r#"{
            "version": "2.0.0",
            "tasks": [
                {
                    "label": "On Default Task",
                    "type": "shell",
                    "command": "echo test",
                    "runOptions": {
                        "runOn": "default"
                    }
                }
            ]
        }"#,
    );

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(!result.has_auto_tasks);
    assert!(result.task_labels.is_empty());
}

#[test]
fn no_vim_rc_files_returns_empty_list() {
    let workspace = create_test_workspace();
    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(result.vim_local_rc_files.is_empty());
}

#[test]
fn exrc_present_detected() {
    let workspace = create_test_workspace();
    std::fs::write(workspace.path().join(".exrc"), "set nocompatible").expect("write .exrc");

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(result.vim_local_rc_files.contains(&".exrc".to_string()));
}

#[test]
fn vimrc_present_detected() {
    let workspace = create_test_workspace();
    std::fs::write(workspace.path().join(".vimrc"), "set number").expect("write .vimrc");

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(result.vim_local_rc_files.contains(&".vimrc".to_string()));
}

#[test]
fn gvimrc_present_detected() {
    let workspace = create_test_workspace();
    std::fs::write(workspace.path().join(".gvimrc"), "set guifont=Monaco:h12")
        .expect("write .gvimrc");

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert!(result.vim_local_rc_files.contains(&".gvimrc".to_string()));
}

#[test]
fn multiple_vim_rc_files_all_collected() {
    let workspace = create_test_workspace();
    std::fs::write(workspace.path().join(".exrc"), "set nocompatible").expect("write .exrc");
    std::fs::write(workspace.path().join(".vimrc"), "set number").expect("write .vimrc");
    std::fs::write(workspace.path().join(".gvimrc"), "set guifont=Monaco:h12")
        .expect("write .gvimrc");

    let result = scan_workspace_for_auto_tasks(workspace.path());

    assert_eq!(result.vim_local_rc_files.len(), 3);
    assert!(result.vim_local_rc_files.contains(&".exrc".to_string()));
    assert!(result.vim_local_rc_files.contains(&".vimrc".to_string()));
    assert!(result.vim_local_rc_files.contains(&".gvimrc".to_string()));
}

#[test]
fn trusted_workspace_skips_vim_check() {
    let workspace = create_test_workspace();
    std::fs::write(workspace.path().join(".vimrc"), "set number").expect("write .vimrc");

    let result = needs_trust_check(workspace.path(), true);
    assert!(result.is_none());
}

#[test]
fn vim_only_triggers_dialog() {
    let workspace = create_test_workspace();
    std::fs::write(workspace.path().join(".exrc"), "set nocompatible").expect("write .exrc");

    let result = needs_trust_check(workspace.path(), false);
    assert!(result.is_some());

    let scan_result = result.unwrap();
    assert!(!scan_result.has_auto_tasks);
    assert!(scan_result.task_labels.is_empty());
    assert!(!scan_result.vim_local_rc_files.is_empty());
}
