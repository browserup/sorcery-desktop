use std::fs;
use std::path::Path;
use std::time::SystemTime;
use tracing::debug;

const ALLOW_DIRS: [&str; 9] = [
    "src", "app", "lib", "packages", "test", "spec", "include", "bin", "scripts",
];

fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

pub fn fs_recent_mtime(root: &Path, max_entries: usize) -> Option<SystemTime> {
    let mut best: Option<SystemTime> = None;

    if let Some(t) = mtime(root) {
        best = Some(best.map_or(t, |b| b.max(t)));
    }

    for dir_name in &ALLOW_DIRS {
        if let Some(t) = mtime(&root.join(dir_name)) {
            best = Some(best.map_or(t, |b| b.max(t)));
        }
    }

    let mut seen = 0usize;

    {
        if let Ok(rd) = fs::read_dir(root) {
            for entry in rd.flatten() {
                if seen >= max_entries {
                    break;
                }
                seen += 1;
                if let Ok(md) = entry.metadata() {
                    if let Ok(t) = md.modified() {
                        best = Some(best.map_or(t, |b| b.max(t)));
                    }
                }
            }
        }
    }

    for dir_name in &ALLOW_DIRS {
        if seen >= max_entries {
            break;
        }
        let path = root.join(dir_name);
        if let Ok(rd) = fs::read_dir(&path) {
            for entry in rd.flatten() {
                if seen >= max_entries {
                    break;
                }
                seen += 1;
                if let Ok(md) = entry.metadata() {
                    if let Ok(t) = md.modified() {
                        best = Some(best.map_or(t, |b| b.max(t)));
                    }
                }
            }
        }
    }

    debug!(
        "Filesystem signal for {} (scanned {} entries): {:?}",
        root.display(),
        seen,
        best
    );

    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_fs_recent_mtime_current_dir() {
        let current_dir = env::current_dir().expect("Failed to get current directory");
        let result = fs_recent_mtime(&current_dir, 400);
        assert!(result.is_some(), "Should find files in current directory");
    }

    #[test]
    fn test_fs_recent_mtime_temp_dir() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        let result = fs_recent_mtime(temp_path, 400);
        assert!(
            result.is_some(),
            "Should get mtime for empty temp directory"
        );

        fs::create_dir_all(temp_path.join("src")).expect("Failed to create src dir");
        std::fs::write(temp_path.join("src/test.txt"), "test").expect("Failed to write file");

        let result = fs_recent_mtime(temp_path, 400);
        assert!(result.is_some(), "Should find mtime after creating file");
    }

    #[test]
    fn test_fs_recent_mtime_nonexistent() {
        let fake_path = Path::new("/nonexistent/workspace");
        let result = fs_recent_mtime(fake_path, 400);
        assert!(result.is_none(), "Should return None for nonexistent path");
    }

    #[test]
    fn test_fs_recent_mtime_respects_max_entries() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        for i in 0..500 {
            std::fs::write(temp_path.join(format!("file{}.txt", i)), "test")
                .expect("Failed to write file");
        }

        let result = fs_recent_mtime(temp_path, 100);
        assert!(result.is_some(), "Should complete even with many files");
    }
}
