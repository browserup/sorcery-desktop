#![allow(clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;

fn read_workspace_file(relative: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let base = manifest_dir.parent().expect("workspace root");
    fs::read_to_string(base.join(relative)).expect("read file")
}

#[test]
fn workspace_chooser_escapes_untrusted_fields_and_uses_bound_handlers() {
    let html = read_workspace_file("public/workspace-chooser.html");

    assert!(html.contains("function escapeHtml(value)"));
    assert!(html.contains("const workspaceName = escapeHtml(match.workspace_name);"));
    assert!(html.contains("const workspacePath = escapeHtml(match.workspace_path);"));
    assert!(html.contains("bindEvents();"));
    assert!(!html.contains("onclick=\""));
    assert!(!html.contains("onchange=\""));
}

#[test]
fn workspace_conflict_escapes_untrusted_fields_and_uses_bound_handlers() {
    let html = read_workspace_file("public/workspace-conflict.html");

    assert!(html.contains("function escapeHtml(value)"));
    assert!(html.contains("const name = escapeHtml(candidate.name);"));
    assert!(html.contains("const workspacePath = escapeHtml(candidate.workspace_path);"));
    assert!(html.contains(
        "const primaryRemote = escapeHtml(candidate.primary_remote || 'remote: not available');"
    ));
    assert!(html.contains("bindEvents();"));
    assert!(!html.contains("onclick=\""));
    assert!(!html.contains("onchange=\""));
}

#[test]
fn settings_page_escapes_workspace_editor_and_history_data() {
    let html = read_workspace_file("public/settings.html");

    assert!(html.contains("function escapeHtml(value)"));
    assert!(html.contains("const safeEditorId = escapeHtml(editor.editor_id);"));
    assert!(html.contains("const safeStdout = escapeHtml(entry.stdout);"));
    assert!(html.contains("const safeStderr = escapeHtml(entry.stderr);"));
    assert!(html.contains("Error loading data: ${escapeHtml(err)}"));
    assert!(!html.contains("onclick=\""));
    assert!(!html.contains("onchange=\""));
    assert!(!html.contains("oninput=\""));
    assert!(!html.contains("onblur=\""));
    assert!(html.contains("function bindClickHandlers()"));
}

#[test]
fn tauri_capabilities_no_longer_use_global_wildcard_windows() {
    let conf = read_workspace_file("src-tauri/tauri.conf.json");

    assert!(
        !conf.contains("\"windows\": [\"*\"]"),
        "capability windows wildcard reintroduced"
    );
    assert!(conf.contains("\"windows\": ["));
    assert!(conf.contains("\"settings\""));
    assert!(conf.contains("\"workspace-chooser\""));
    assert!(conf.contains("\"workspace-conflict\""));
}
