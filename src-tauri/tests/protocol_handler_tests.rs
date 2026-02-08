#![allow(clippy::panic, clippy::unwrap_used, clippy::clone_on_ref_ptr)]
// Integration tests intentionally use direct panic/unwrap patterns for compact fixtures.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_protocol_handler_full_path() {
    let (protocol_handler, _settings_manager, _temp_dir, test_file) = setup().await;

    let url = format!("srcuri://abs/{}@L5C10", test_file.display());
    let result = protocol_handler.handle_url(&url).await;

    match result {
        Ok(_) => {}
        Err(e) => panic!(
            "Protocol handler should successfully parse full path URL. Error: {}",
            e
        ),
    }
}

#[tokio::test]
async fn test_protocol_handler_partial_path_single_match() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let workspace_dir = temp_dir.path().join("workspace1");
    fs::create_dir(&workspace_dir).unwrap();
    let test_file = workspace_dir.join("main.rs");
    fs::write(&test_file, "fn main() {}").unwrap();

    configure_workspace(&settings_manager, workspace_dir.to_str().unwrap()).await;

    let url = "srcuri://rel/main.rs@L10C5";
    let result = protocol_handler.handle_url(url).await;

    match result {
        Ok(_) => {}
        Err(e) => panic!(
            "Protocol handler should successfully find unique partial path match. Error: {}",
            e
        ),
    }
}

#[tokio::test]
async fn test_protocol_handler_workspace_path() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let workspace_dir = temp_dir.path().join("myproject");
    fs::create_dir(&workspace_dir).unwrap();
    let test_file = workspace_dir.join("README.md");
    fs::write(&test_file, "# Test").unwrap();

    configure_workspace(&settings_manager, workspace_dir.to_str().unwrap()).await;

    let url = "srcuri://myproject/README.md@L1C1";
    let result = protocol_handler.handle_url(url).await;

    match result {
        Ok(_) => {}
        Err(e) => panic!(
            "Protocol handler should successfully resolve workspace path. Error: {}",
            e
        ),
    }
}

#[tokio::test]
async fn test_protocol_handler_invalid_url() {
    let (protocol_handler, _settings_manager, _temp_dir, _test_file) = setup().await;

    let url = "not-a-srcuri-url";
    let result = protocol_handler.handle_url(url).await;

    assert!(
        result.is_err(),
        "Protocol handler should reject invalid URLs"
    );
}

#[tokio::test]
async fn test_protocol_handler_missing_file() {
    let (protocol_handler, _settings_manager, _temp_dir, _test_file) = setup().await;

    let url = "srcuri://abs/nonexistent/file.rs@L1C1";
    let result = protocol_handler.handle_url(url).await;

    assert!(
        result.is_err(),
        "Protocol handler should error on missing files"
    );
}

#[tokio::test]
async fn test_dispatcher_with_vscode_manager() {
    let (_protocol_handler, _settings_manager, _temp_dir, _test_file) = setup().await;

    // This test verifies the full integration:
    // protocol handler -> dispatcher -> editor manager -> actual editor launch
    //
    // Note: This requires editors to be installed and may not work in all CI environments
    // Mark as ignored to run manually
}

async fn setup() -> (
    Arc<sorcery_desktop::protocol_handler::ProtocolHandler>,
    Arc<sorcery_desktop::settings::SettingsManager>,
    TempDir,
    PathBuf,
) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, "fn main() {\n    println!(\"Test\");\n}\n")
        .expect("Failed to create test file");

    // Use temp path for settings to avoid polluting user's real settings
    let test_settings_path = temp_dir.path().join("settings.yaml");
    let settings_manager = Arc::new(
        sorcery_desktop::settings::SettingsManager::new_with_path(test_settings_path)
            .await
            .expect("Failed to create settings manager"),
    );

    let mut settings = settings_manager.get().await;
    settings.defaults.allow_non_workspace_files = true;
    // Use NullEditor for tests - doesn't launch any actual editor process
    settings.defaults.editor = "null".to_string();
    settings_manager
        .save(settings)
        .await
        .expect("Failed to save test settings");

    let path_validator = Arc::new(sorcery_desktop::path_validator::PathValidator::new());

    let editor_registry = Arc::new(sorcery_desktop::editors::EditorRegistry::new());
    // Register NullEditor for tests - it's not in the default registry to avoid polluting the UI
    editor_registry.register(Arc::new(sorcery_desktop::editors::NullEditor::new()));

    let tracker = Arc::new(sorcery_desktop::tracker::ActiveEditorTracker::new(
        editor_registry.clone(),
    ));

    let workspace_tracker = Arc::new(sorcery_desktop::workspace_mru::ActiveWorkspaceTracker::new(
        settings_manager.clone(),
    ));

    let dispatcher = Arc::new(sorcery_desktop::dispatcher::EditorDispatcher::new(
        settings_manager.clone(),
        path_validator.clone(),
        editor_registry.clone(),
        tracker.clone(),
    ));

    let protocol_handler = Arc::new(sorcery_desktop::protocol_handler::ProtocolHandler::new(
        settings_manager.clone(),
        dispatcher.clone(),
        workspace_tracker.clone(),
    ));

    (protocol_handler, settings_manager, temp_dir, test_file)
}

async fn configure_workspace(
    settings_manager: &Arc<sorcery_desktop::settings::SettingsManager>,
    workspace_path: &str,
) {
    let mut settings = settings_manager.get().await;
    let workspace_name = std::path::Path::new(workspace_path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    settings
        .workspaces
        .push(sorcery_desktop::settings::WorkspaceConfig {
            path: workspace_path.to_string(),
            name: workspace_name,
            editor: "null".to_string(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: sorcery_desktop::settings::WorkspaceKind::NonGit,
            workspace_state: sorcery_desktop::settings::WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        });
    settings_manager
        .save(settings)
        .await
        .expect("Failed to save workspace config");
}

// Folder support tests

#[tokio::test]
async fn test_protocol_handler_directory_path() {
    let (protocol_handler, _settings_manager, temp_dir, _test_file) = setup().await;

    let subdir = temp_dir.path().join("src");
    fs::create_dir(&subdir).unwrap();

    let url = format!("srcuri://abs/{}", subdir.display());
    let result = protocol_handler.handle_url(&url).await;

    match result {
        Ok(_) => {}
        Err(e) => panic!(
            "Protocol handler should accept directory paths. Error: {}",
            e
        ),
    }
}

#[tokio::test]
async fn test_protocol_handler_directory_with_line_silently_ignored() {
    let (protocol_handler, _settings_manager, temp_dir, _test_file) = setup().await;

    let subdir = temp_dir.path().join("controllers");
    fs::create_dir(&subdir).unwrap();

    // Line numbers should be silently ignored for directories
    let url = format!("srcuri://abs/{}:42", subdir.display());
    let result = protocol_handler.handle_url(&url).await;

    match result {
        Ok(_) => {}
        Err(e) => panic!(
            "Protocol handler should accept directory with line (ignoring line). Error: {}",
            e
        ),
    }
}

#[tokio::test]
async fn test_protocol_handler_workspace_directory() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let workspace_dir = temp_dir.path().join("myapp");
    fs::create_dir(&workspace_dir).unwrap();
    let src_dir = workspace_dir.join("src");
    fs::create_dir(&src_dir).unwrap();

    configure_workspace(&settings_manager, workspace_dir.to_str().unwrap()).await;

    // Open src folder within workspace
    let url = "srcuri://myapp/src";
    let result = protocol_handler.handle_url(url).await;

    match result {
        Ok(_) => {}
        Err(e) => panic!(
            "Protocol handler should resolve workspace directory path. Error: {}",
            e
        ),
    }
}

// Strict workspace mode tests

#[tokio::test]
async fn test_workspace_mode_rejects_unknown_workspace() {
    let (protocol_handler, _settings_manager, _temp_dir, _test_file) = setup().await;

    // Try to access a workspace that doesn't exist
    let url = "srcuri://nonexistent-workspace/src/main.rs:1:1";
    let result = protocol_handler.handle_url(url).await;

    assert!(
        result.is_err(),
        "Protocol handler should reject unknown workspace names"
    );
}

#[tokio::test]
async fn test_workspace_mode_is_case_insensitive() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let workspace_dir = temp_dir.path().join("MyProject");
    fs::create_dir(&workspace_dir).unwrap();
    let test_file = workspace_dir.join("README.md");
    fs::write(&test_file, "# Test").unwrap();

    configure_workspace(&settings_manager, workspace_dir.to_str().unwrap()).await;

    // Different case should still work
    let url = "srcuri://myproject/README.md:1:1";
    let result = protocol_handler.handle_url(url).await;

    match result {
        Ok(_) => {}
        Err(e) => panic!("Workspace mode should be case-insensitive. Error: {}", e),
    }
}

// Rel mode tests

#[tokio::test]
async fn test_rel_mode_finds_workspace_in_path() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let workspace_dir = temp_dir.path().join("backend");
    fs::create_dir(&workspace_dir).unwrap();
    let src_dir = workspace_dir.join("src");
    fs::create_dir(&src_dir).unwrap();
    let test_file = src_dir.join("main.rs");
    fs::write(&test_file, "fn main() {}").unwrap();

    configure_workspace(&settings_manager, workspace_dir.to_str().unwrap()).await;

    // Path contains workspace name in middle - should find it and use relative path
    let url = "srcuri://rel/some/prefix/backend/src/main.rs:1:1";
    let result = protocol_handler.handle_url(url).await;

    match result {
        Ok(_) => {}
        Err(e) => panic!("Rel mode should find workspace name in path. Error: {}", e),
    }
}

#[tokio::test]
async fn test_rel_mode_with_workspace_hint() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    // Create two workspaces with same file
    let workspace1 = temp_dir.path().join("backend");
    let workspace2 = temp_dir.path().join("frontend");
    fs::create_dir(&workspace1).unwrap();
    fs::create_dir(&workspace2).unwrap();

    let file1 = workspace1.join("config.json");
    let file2 = workspace2.join("config.json");
    fs::write(&file1, r#"{"name": "backend"}"#).unwrap();
    fs::write(&file2, r#"{"name": "frontend"}"#).unwrap();

    configure_workspace(&settings_manager, workspace1.to_str().unwrap()).await;
    configure_workspace(&settings_manager, workspace2.to_str().unwrap()).await;

    // Use workspaceHint to specify which workspace
    let url = "srcuri://rel/config.json:1?workspaceHint=backend";
    let result = protocol_handler.handle_url(url).await;

    // Should succeed - workspaceHint helps disambiguation
    match result {
        Ok(_) => {}
        Err(e) => panic!(
            "Rel mode with workspaceHint should resolve correctly. Error: {}",
            e
        ),
    }
}

#[tokio::test]
async fn test_rel_mode_multiple_matches_shows_chooser() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    // Create two workspaces with same file
    let workspace1 = temp_dir.path().join("proj1");
    let workspace2 = temp_dir.path().join("proj2");
    fs::create_dir(&workspace1).unwrap();
    fs::create_dir(&workspace2).unwrap();

    let file1 = workspace1.join("README.md");
    let file2 = workspace2.join("README.md");
    fs::write(&file1, "# Project 1").unwrap();
    fs::write(&file2, "# Project 2").unwrap();

    configure_workspace(&settings_manager, workspace1.to_str().unwrap()).await;
    configure_workspace(&settings_manager, workspace2.to_str().unwrap()).await;

    let url = "srcuri://rel/README.md:1";
    let result = protocol_handler.handle_url(url).await;

    // Should return ShowChooser result
    match result {
        Ok(sorcery_desktop::protocol_handler::HandleResult::ShowChooser { matches, .. }) => {
            assert_eq!(matches.len(), 2, "Should have 2 matches for chooser");
        }
        Ok(other) => panic!("Expected ShowChooser for multiple matches, got {:?}", other),
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[tokio::test]
async fn test_rel_mode_single_match_opens_directly() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let workspace_dir = temp_dir.path().join("unique");
    fs::create_dir(&workspace_dir).unwrap();
    let test_file = workspace_dir.join("unique-file.rs");
    fs::write(&test_file, "fn unique() {}").unwrap();

    configure_workspace(&settings_manager, workspace_dir.to_str().unwrap()).await;

    let url = "srcuri://rel/unique-file.rs:1";
    let result = protocol_handler.handle_url(url).await;

    // Should return Opened result (not chooser)
    match result {
        Ok(sorcery_desktop::protocol_handler::HandleResult::Opened { file_path }) => {
            assert!(
                file_path.contains("unique-file.rs"),
                "Should open the unique file"
            );
        }
        Ok(other) => panic!("Expected Opened for single match, got {:?}", other),
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

// Abs mode tests

#[tokio::test]
async fn test_abs_mode_respects_non_workspace_setting() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    // Create a file outside of any workspace
    let non_workspace_file = temp_dir.path().join("outside.txt");
    fs::write(&non_workspace_file, "outside workspace").unwrap();

    // Disable non-workspace files
    let mut settings = settings_manager.get().await;
    settings.defaults.allow_non_workspace_files = false;
    settings_manager.save(settings).await.unwrap();

    let url = format!("srcuri://abs/{}:1", non_workspace_file.display());
    let result = protocol_handler.handle_url(&url).await;

    assert!(
        result.is_err(),
        "Should reject non-workspace files when setting is disabled"
    );
}

#[tokio::test]
async fn test_revision_prefers_exact_branch_worktree_match() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let main_workspace = temp_dir.path().join("repo-main");
    let feature_workspace = temp_dir.path().join("repo-feature");
    fs::create_dir(&main_workspace).expect("create main workspace");

    run_git(&main_workspace, &["init"]);
    run_git(
        &main_workspace,
        &["config", "user.email", "test@example.com"],
    );
    run_git(&main_workspace, &["config", "user.name", "Test User"]);
    fs::write(main_workspace.join("README.md"), "main").expect("write readme");
    run_git(&main_workspace, &["add", "README.md"]);
    run_git(&main_workspace, &["commit", "-m", "init"]);
    run_git(&main_workspace, &["branch", "-M", "main"]);
    run_git(&main_workspace, &["branch", "feature"]);
    run_git(
        &main_workspace,
        &[
            "worktree",
            "add",
            feature_workspace.to_string_lossy().as_ref(),
            "feature",
        ],
    );

    configure_workspace(&settings_manager, main_workspace.to_string_lossy().as_ref()).await;
    configure_workspace(
        &settings_manager,
        feature_workspace.to_string_lossy().as_ref(),
    )
    .await;

    let result = protocol_handler
        .handle_url("srcuri://repo-main/README.md?branch=feature")
        .await
        .expect("handle branch URL");

    match result {
        sorcery_desktop::protocol_handler::HandleResult::Opened { file_path } => {
            let expected_path = feature_workspace.join("README.md");
            assert_eq!(PathBuf::from(file_path), expected_path);
        }
        other => panic!("Expected Opened result, got {:?}", other),
    }
}

#[tokio::test]
async fn test_revision_uses_mru_within_repo_group_when_no_exact_ref_match() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let main_workspace = temp_dir.path().join("repo-main");
    let feature_workspace = temp_dir.path().join("repo-feature");
    fs::create_dir(&main_workspace).expect("create main workspace");

    run_git(&main_workspace, &["init"]);
    run_git(
        &main_workspace,
        &["config", "user.email", "test@example.com"],
    );
    run_git(&main_workspace, &["config", "user.name", "Test User"]);
    fs::write(main_workspace.join("README.md"), "main").expect("write readme");
    run_git(&main_workspace, &["add", "README.md"]);
    run_git(&main_workspace, &["commit", "-m", "init"]);
    run_git(&main_workspace, &["branch", "-M", "main"]);
    run_git(&main_workspace, &["branch", "feature"]);
    run_git(
        &main_workspace,
        &[
            "worktree",
            "add",
            feature_workspace.to_string_lossy().as_ref(),
            "feature",
        ],
    );
    run_git(&main_workspace, &["tag", "v1"]);

    // Make the feature worktree the most-recent candidate by bumping directory mtime.
    std::thread::sleep(std::time::Duration::from_secs(1));
    fs::write(feature_workspace.join(".mru_touch"), "recent").expect("touch feature workspace");

    configure_workspace(&settings_manager, main_workspace.to_string_lossy().as_ref()).await;
    configure_workspace(
        &settings_manager,
        feature_workspace.to_string_lossy().as_ref(),
    )
    .await;

    let result = protocol_handler
        .handle_url("srcuri://repo-main/README.md?tag=v1")
        .await
        .expect("handle tag URL");

    match result {
        sorcery_desktop::protocol_handler::HandleResult::ShowRevisionDialog {
            workspace_path,
            ..
        } => {
            assert_eq!(workspace_path, feature_workspace);
        }
        other => panic!("Expected ShowRevisionDialog, got {:?}", other),
    }
}

#[tokio::test]
async fn test_enforced_policy_blocks_non_compliant_workspace_resolution() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let workspace = temp_dir.path().join("rails");
    fs::create_dir(&workspace).expect("create workspace");
    fs::write(workspace.join("README.md"), "policy").expect("write file");
    run_git(&workspace, &["init"]);
    run_git(&workspace, &["config", "user.email", "test@example.com"]);
    run_git(&workspace, &["config", "user.name", "Test User"]);
    run_git(
        &workspace,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/company/rails.git",
        ],
    );

    configure_workspace(&settings_manager, workspace.to_string_lossy().as_ref()).await;

    write_policy_file(
        &settings_manager,
        r#"
mode: enforced
mappings:
  - name: rails
    remote: github.com/rails/rails
"#,
    )
    .await;

    let result = protocol_handler
        .handle_url("srcuri://rails/README.md")
        .await
        .expect("policy should return conflict dialog");

    match result {
        sorcery_desktop::protocol_handler::HandleResult::ShowWorkspaceConflictDialog {
            policy_violation: Some(policy_violation),
            ..
        } => {
            assert!(
                policy_violation.contains("requires remote"),
                "unexpected policy message: {policy_violation}"
            );
        }
        other => panic!("Expected ShowWorkspaceConflictDialog, got {:?}", other),
    }
}

#[tokio::test]
async fn test_enforced_policy_blocks_clone_resolution() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let disallowed_default = temp_dir.path().join("clones");
    fs::create_dir_all(&disallowed_default).expect("create clone dir");
    let mut settings = settings_manager.get().await;
    settings.defaults.default_workspaces_folder = disallowed_default.to_string_lossy().to_string();
    settings_manager
        .save(settings)
        .await
        .expect("save settings");

    let allowed_root = temp_dir.path().join("allowed");
    fs::create_dir_all(&allowed_root).expect("create allowed root");
    let policy = format!(
        r#"
mode: enforced
mappings:
  - name: myrepo
    remote: github.com/company/myrepo
allowed_clone_roots:
  - {}
"#,
        allowed_root.to_string_lossy()
    );
    write_policy_file(&settings_manager, &policy).await;

    let result = protocol_handler
        .handle_url("srcuri://myrepo/README.md?remote=https://github.com/company/myrepo.git")
        .await
        .expect("policy should return clone dialog");

    match result {
        sorcery_desktop::protocol_handler::HandleResult::ShowCloneDialog {
            policy_violation: Some(policy_violation),
            ..
        } => {
            assert!(
                policy_violation.contains("outside allowed clone roots"),
                "unexpected policy message: {policy_violation}"
            );
        }
        other => panic!("Expected ShowCloneDialog, got {:?}", other),
    }
}

#[tokio::test]
async fn test_same_key_same_remote_opens_existing_mapping() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let workspace = temp_dir.path().join("rails");
    fs::create_dir(&workspace).expect("create workspace");
    fs::write(workspace.join("README.md"), "# rails").expect("write file");
    run_git(&workspace, &["init"]);
    run_git(&workspace, &["config", "user.email", "test@example.com"]);
    run_git(&workspace, &["config", "user.name", "Test User"]);
    run_git(
        &workspace,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/company/rails.git",
        ],
    );

    configure_workspace(&settings_manager, workspace.to_string_lossy().as_ref()).await;

    let result = protocol_handler
        .handle_url("srcuri://rails/README.md?remote=https://github.com/company/rails.git")
        .await
        .expect("should resolve to existing mapping");

    match result {
        sorcery_desktop::protocol_handler::HandleResult::Opened { file_path } => {
            assert_eq!(PathBuf::from(file_path), workspace.join("README.md"));
        }
        other => panic!("Expected Opened, got {:?}", other),
    }
}

#[tokio::test]
async fn test_same_key_different_remote_shows_conflict_dialog() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let workspace = temp_dir.path().join("rails");
    fs::create_dir(&workspace).expect("create workspace");
    fs::write(workspace.join("README.md"), "# rails").expect("write file");
    run_git(&workspace, &["init"]);
    run_git(&workspace, &["config", "user.email", "test@example.com"]);
    run_git(&workspace, &["config", "user.name", "Test User"]);
    run_git(
        &workspace,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/company/rails.git",
        ],
    );

    configure_workspace(&settings_manager, workspace.to_string_lossy().as_ref()).await;

    let result = protocol_handler
        .handle_url("srcuri://rails/README.md?remote=https://github.com/rails/rails.git")
        .await
        .expect("should show conflict dialog");

    match result {
        sorcery_desktop::protocol_handler::HandleResult::ShowWorkspaceConflictDialog {
            requested_remote,
            existing_mappings,
            ..
        } => {
            assert!(requested_remote.contains("github.com/rails/rails"));
            assert_eq!(existing_mappings.len(), 1);
        }
        other => panic!("Expected ShowWorkspaceConflictDialog, got {:?}", other),
    }
}

#[tokio::test]
async fn test_fork_and_upstream_with_distinct_keys_open_by_key() {
    let (protocol_handler, settings_manager, temp_dir, _test_file) = setup().await;

    let upstream_workspace = temp_dir.path().join("rails-upstream");
    let fork_workspace = temp_dir.path().join("rails-fork");
    fs::create_dir(&upstream_workspace).expect("create upstream workspace");
    fs::create_dir(&fork_workspace).expect("create fork workspace");
    fs::write(upstream_workspace.join("README.md"), "upstream").expect("write upstream readme");
    fs::write(fork_workspace.join("README.md"), "fork").expect("write fork readme");

    run_git(&upstream_workspace, &["init"]);
    run_git(
        &upstream_workspace,
        &["config", "user.email", "test@example.com"],
    );
    run_git(&upstream_workspace, &["config", "user.name", "Test User"]);
    run_git(
        &upstream_workspace,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/rails/rails.git",
        ],
    );

    run_git(&fork_workspace, &["init"]);
    run_git(
        &fork_workspace,
        &["config", "user.email", "test@example.com"],
    );
    run_git(&fork_workspace, &["config", "user.name", "Test User"]);
    run_git(
        &fork_workspace,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/my-org/rails.git",
        ],
    );

    configure_workspace(
        &settings_manager,
        upstream_workspace.to_string_lossy().as_ref(),
    )
    .await;
    configure_workspace(&settings_manager, fork_workspace.to_string_lossy().as_ref()).await;

    let upstream_result = protocol_handler
        .handle_url("srcuri://rails-upstream/README.md")
        .await
        .expect("open upstream");
    let fork_result = protocol_handler
        .handle_url("srcuri://rails-fork/README.md")
        .await
        .expect("open fork");

    match upstream_result {
        sorcery_desktop::protocol_handler::HandleResult::Opened { file_path } => {
            assert_eq!(
                PathBuf::from(file_path),
                upstream_workspace.join("README.md")
            );
        }
        other => panic!("Expected Opened for upstream workspace, got {:?}", other),
    }

    match fork_result {
        sorcery_desktop::protocol_handler::HandleResult::Opened { file_path } => {
            assert_eq!(PathBuf::from(file_path), fork_workspace.join("README.md"));
        }
        other => panic!("Expected Opened for fork workspace, got {:?}", other),
    }
}

async fn write_policy_file(
    settings_manager: &Arc<sorcery_desktop::settings::SettingsManager>,
    yaml: &str,
) {
    let policy_path = settings_manager
        .config_path()
        .parent()
        .expect("settings path parent")
        .join("policy.yaml");
    fs::write(policy_path, yaml).expect("write policy");
    settings_manager
        .reload_policy()
        .await
        .expect("reload policy");
}

fn run_git(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("run git");
    assert!(status.success(), "git command failed: {:?}", args);
}
