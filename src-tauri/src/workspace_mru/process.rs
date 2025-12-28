use std::path::Path;
use std::time::SystemTime;
use sysinfo::{ProcessRefreshKind, System, UpdateKind};
use tracing::debug;

pub fn refresh_process_snapshot(sys: &mut System) {
    let kind = ProcessRefreshKind::new().with_cwd(UpdateKind::Always);
    sys.refresh_processes_specifics(kind);
}

pub fn check_running_process(root: &Path, sys: &System) -> Option<SystemTime> {
    let canon_root = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            debug!("Failed to canonicalize workspace root: {}", root.display());
            return None;
        }
    };

    for process in sys.processes().values() {
        if let Some(cwd) = process.cwd() {
            if let Ok(canon_cwd) = cwd.canonicalize() {
                if canon_cwd.starts_with(&canon_root) {
                    debug!(
                        "Found running process in workspace {}: {} (pid: {})",
                        root.display(),
                        process.name(),
                        process.pid()
                    );
                    return Some(SystemTime::now());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_process_detection_current_dir() {
        let mut sys = System::new();
        refresh_process_snapshot(&mut sys);

        let current_dir = env::current_dir().expect("Failed to get current directory");

        let result = check_running_process(&current_dir, &sys);

        assert!(
            result.is_some(),
            "Should detect current process in current directory"
        );
    }

    #[test]
    fn test_process_detection_nonexistent_path() {
        let mut sys = System::new();
        refresh_process_snapshot(&mut sys);

        let fake_path = Path::new("/nonexistent/workspace/path");

        let result = check_running_process(fake_path, &sys);

        assert!(
            result.is_none(),
            "Should not detect process in nonexistent path"
        );
    }
}
