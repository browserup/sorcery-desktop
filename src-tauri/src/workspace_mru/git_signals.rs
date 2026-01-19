use git2::Repository;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::debug;

pub fn head_reflog_time(repo_path: &Path) -> Option<SystemTime> {
    let repo = match Repository::open(repo_path) {
        Ok(r) => r,
        Err(e) => {
            debug!(
                "Failed to open Git repository at {}: {}",
                repo_path.display(),
                e
            );
            return None;
        }
    };

    let log = match repo.reflog("HEAD") {
        Ok(l) => l,
        Err(e) => {
            debug!("Failed to read HEAD reflog: {}", e);
            return None;
        }
    };

    if log.len() == 0 {
        debug!("HEAD reflog is empty");
        return None;
    }

    let entry = log.get(log.len() - 1)?;
    let when = entry.committer().when();
    let timestamp = UNIX_EPOCH + Duration::from_secs(when.seconds() as u64);

    debug!(
        "Git reflog time for {}: {:?}",
        repo_path.display(),
        timestamp
    );
    Some(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;

    fn find_git_repo() -> Option<PathBuf> {
        let mut current = env::current_dir().ok()?;
        loop {
            if current.join(".git").exists() {
                return Some(current);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    #[test]
    fn test_head_reflog_time() {
        if let Some(repo_path) = find_git_repo() {
            let result = head_reflog_time(&repo_path);
            assert!(result.is_some(), "Should find HEAD reflog time in Git repo");
        }
    }
}
