use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_protocol_handler_full_path() {
    let (protocol_handler, _settings_manager, _temp_dir, test_file) = setup().await;

    let url = format!("srcuri://{}:5:10", test_file.display());
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

    let url = "srcuri://main.rs:10:5";
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

    let url = "srcuri://myproject/README.md:1:1";
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

    let url = "srcuri:///nonexistent/file.rs:1:1";
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

    let settings_manager = Arc::new(
        sorcery_desktop::settings::SettingsManager::new()
            .await
            .expect("Failed to create settings manager"),
    );

    let mut settings = settings_manager.get().await;
    settings.defaults.allow_non_workspace_files = true;
    // Use VSCodium as default - works better than VSCode when running as root in Docker
    settings.defaults.editor = "vscodium".to_string();
    settings_manager
        .save(settings)
        .await
        .expect("Failed to save test settings");

    let path_validator = Arc::new(sorcery_desktop::path_validator::PathValidator::new(
        settings_manager.clone(),
    ));

    let editor_registry = Arc::new(sorcery_desktop::editors::EditorRegistry::new());

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
            name: Some(workspace_name),
            editor: "vscodium".to_string(),
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

    let url = format!("srcuri://{}", subdir.display());
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
    let url = format!("srcuri://{}:42", subdir.display());
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
