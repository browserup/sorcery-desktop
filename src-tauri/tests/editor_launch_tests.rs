#![cfg(any(feature = "docker-tests", target_os = "linux"))]

extern crate libc;

use sorcery_desktop::editors::{EditorRegistry, OpenOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Test helper utilities
mod test_utils {
    use super::*;

    pub fn is_root() -> bool {
        unsafe { libc::geteuid() == 0 }
    }

    pub fn is_process_running(process_name: &str) -> bool {
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(process_name)
            .output()
            .expect("Failed to execute pgrep");

        output.status.success() && !output.stdout.is_empty()
    }

    pub fn kill_process(process_name: &str) {
        let _ = Command::new("pkill").arg("-f").arg(process_name).output();
        thread::sleep(Duration::from_millis(500));
    }

    pub fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let file_path = dir.join(name);
        fs::write(&file_path, content).expect("Failed to create test file");
        file_path
    }

    pub fn wait_for_process(process_name: &str, timeout_secs: u64) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < timeout_secs {
            if is_process_running(process_name) {
                return true;
            }
            thread::sleep(Duration::from_millis(100));
        }
        false
    }
}

use test_utils::*;

fn setup() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = create_test_file(
        temp_dir.path(),
        "test.rs",
        "fn main() {\n    println!(\"Hello, world!\");\n}\n",
    );
    (temp_dir, test_file)
}

struct ProcessGuard {
    processes: Vec<&'static str>,
}

impl ProcessGuard {
    fn new(processes: &[&'static str]) -> Self {
        Self {
            processes: processes.to_vec(),
        }
    }

    fn add(&mut self, name: &'static str) {
        self.processes.push(name);
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        for process in self.processes.drain(..) {
            kill_process(process);
        }
    }
}

/// Tests that verify Sorcery Desktop's EditorManager implementations
/// correctly launch editors with the proper arguments.
mod manager_tests {
    use super::*;

    #[tokio::test]
    async fn test_vscodium_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let _guard = ProcessGuard::new(&["codium"]);

        let registry = EditorRegistry::new();
        let manager = registry.get("vscodium").expect("VSCodium manager not found");

        let options = OpenOptions {
            line: Some(5),
            column: Some(10),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "VSCodium manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("codium", 10), "VSCodium process did not start");
    }

    #[tokio::test]
    async fn test_sublime_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let _guard = ProcessGuard::new(&["sublime_text"]);

        let registry = EditorRegistry::new();
        let manager = registry.get("sublime").expect("Sublime manager not found");

        let options = OpenOptions {
            line: Some(5),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "Sublime manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("sublime_text", 10), "Sublime Text process did not start");
    }

    #[tokio::test]
    async fn test_vim_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let mut guard = ProcessGuard::new(&["vim"]);
        guard.add("xterm");

        let registry = EditorRegistry::new();
        let manager = registry.get("vim").expect("Vim manager not found");

        let options = OpenOptions {
            line: Some(5),
            terminal_preference: Some("xterm".to_string()),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "Vim manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("vim", 5), "Vim process did not start");
    }

    #[tokio::test]
    async fn test_neovim_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let mut guard = ProcessGuard::new(&["nvim"]);
        guard.add("xterm");

        let registry = EditorRegistry::new();
        let manager = registry.get("neovim").expect("Neovim manager not found");

        let options = OpenOptions {
            line: Some(5),
            terminal_preference: Some("xterm".to_string()),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "Neovim manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("nvim", 5), "Neovim process did not start");
    }

    #[tokio::test]
    async fn test_emacs_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let mut guard = ProcessGuard::new(&["emacs"]);
        guard.add("xterm");

        let registry = EditorRegistry::new();
        let manager = registry.get("emacs").expect("Emacs manager not found");

        let options = OpenOptions {
            line: Some(5),
            terminal_preference: Some("xterm".to_string()),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "Emacs manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("emacs", 5), "Emacs process did not start");
    }

    #[tokio::test]
    async fn test_nano_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let mut guard = ProcessGuard::new(&["nano"]);
        guard.add("xterm");

        let registry = EditorRegistry::new();
        let manager = registry.get("nano").expect("Nano manager not found");

        let options = OpenOptions {
            line: Some(5),
            terminal_preference: Some("xterm".to_string()),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "Nano manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("nano", 5), "Nano process did not start");
    }

    #[tokio::test]
    async fn test_micro_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let mut guard = ProcessGuard::new(&["micro"]);
        guard.add("xterm");

        let registry = EditorRegistry::new();
        let manager = registry.get("micro").expect("Micro manager not found");

        let options = OpenOptions {
            line: Some(5),
            terminal_preference: Some("xterm".to_string()),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "Micro manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("micro", 5), "Micro process did not start");
    }

    #[tokio::test]
    async fn test_kate_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let _guard = ProcessGuard::new(&["kate"]);

        let registry = EditorRegistry::new();
        let manager = registry.get("kate").expect("Kate manager not found");

        let options = OpenOptions {
            line: Some(5),
            column: Some(10),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "Kate manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("kate", 10), "Kate process did not start");
    }

    #[tokio::test]
    async fn test_intellij_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let _guard = ProcessGuard::new(&["idea"]);

        let registry = EditorRegistry::new();
        let manager = registry.get("idea").expect("IntelliJ IDEA manager not found");

        let options = OpenOptions {
            line: Some(5),
            column: Some(10),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "IntelliJ manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("idea", 15), "IntelliJ IDEA process did not start");
    }

    #[tokio::test]
    async fn test_pycharm_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let _guard = ProcessGuard::new(&["pycharm"]);

        let registry = EditorRegistry::new();
        let manager = registry.get("pycharm").expect("PyCharm manager not found");

        let options = OpenOptions {
            line: Some(5),
            column: Some(10),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "PyCharm manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("pycharm", 15), "PyCharm process did not start");
    }

    #[tokio::test]
    async fn test_gedit_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let _guard = ProcessGuard::new(&["gedit"]);

        let registry = EditorRegistry::new();
        let manager = registry.get("gedit").expect("Gedit manager not found");

        let options = OpenOptions {
            line: Some(5),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "Gedit manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("gedit", 10), "Gedit process did not start");
    }

    #[tokio::test]
    async fn test_kakoune_manager_opens_file() {
        let (_temp_dir, test_file) = setup();
        let mut guard = ProcessGuard::new(&["kak"]);
        guard.add("xterm");

        let registry = EditorRegistry::new();
        let manager = registry.get("kakoune").expect("Kakoune manager not found");

        let options = OpenOptions {
            line: Some(5),
            terminal_preference: Some("xterm".to_string()),
            ..Default::default()
        };

        let result = manager.open(&test_file, &options).await;
        assert!(result.is_ok(), "Kakoune manager failed to open file: {:?}", result.err());
        assert!(wait_for_process("kak", 5), "Kakoune process did not start");
    }
}

/// Tests that verify EditorManagers correctly detect installed editors
mod detection_tests {
    use super::*;

    #[tokio::test]
    async fn test_installed_editors_are_detected() {
        let registry = EditorRegistry::new();

        let expected_installed = vec![
            "vscodium",
            "sublime",
            "vim",
            "neovim",
            "emacs",
            "nano",
            "micro",
            "kate",
            "idea",
            "pycharm",
            "gedit",
            "kakoune",
        ];

        for editor_id in expected_installed {
            let manager = registry.get(editor_id).expect(&format!("{} manager not found", editor_id));
            assert!(
                manager.is_installed().await,
                "{} should be detected as installed",
                editor_id
            );
        }
    }

    #[tokio::test]
    async fn test_find_binary_returns_valid_path() {
        let registry = EditorRegistry::new();

        let editors_to_check = vec![
            ("vscodium", "codium"),
            ("sublime", "subl"),
            ("vim", "vim"),
            ("neovim", "nvim"),
            ("kate", "kate"),
        ];

        for (editor_id, expected_name) in editors_to_check {
            let manager = registry.get(editor_id).expect(&format!("{} manager not found", editor_id));
            let binary = manager.find_binary().await;
            assert!(binary.is_some(), "{} binary should be found", editor_id);
            let path = binary.unwrap();
            assert!(path.exists(), "{} binary path should exist: {:?}", editor_id, path);
            assert!(
                path.to_string_lossy().contains(expected_name),
                "{} binary path should contain '{}': {:?}",
                editor_id,
                expected_name,
                path
            );
        }
    }
}
