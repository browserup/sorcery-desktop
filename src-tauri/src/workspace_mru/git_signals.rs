use git2::{Repository, Status, StatusOptions, StatusShow};
use std::cmp;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

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

pub fn latest_uncommitted_mtime(repo_path: &Path) -> Option<SystemTime> {
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

    let workdir = match repo.workdir() {
        Some(w) => w,
        None => {
            debug!("Repository is bare, no working directory");
            return None;
        }
    };

    let mut opts = StatusOptions::new();
    opts.show(StatusShow::IndexAndWorkdir)
        .include_untracked(true)
        .recurse_untracked_dirs(false)
        .exclude_submodules(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .no_refresh(false);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to get git status: {}", e);
            return None;
        }
    };

    let mut latest: Option<SystemTime> = None;

    for entry in statuses.iter() {
        let status = entry.status();
        let interesting = status.intersects(
            Status::WT_MODIFIED
                | Status::WT_NEW
                | Status::WT_DELETED
                | Status::WT_TYPECHANGE
                | Status::WT_RENAMED
                | Status::INDEX_MODIFIED
                | Status::INDEX_NEW
                | Status::INDEX_DELETED
                | Status::INDEX_TYPECHANGE
                | Status::INDEX_RENAMED,
        );

        if interesting {
            if let Some(rel) = entry.path() {
                let path = workdir.join(rel);
                if let Ok(md) = fs::metadata(&path) {
                    if let Ok(time) = md.modified() {
                        latest = Some(match latest {
                            Some(cur) => cmp::max(cur, time),
                            None => time,
                        });
                    }
                }
            }
        }
    }

    if latest.is_some() {
        debug!(
            "Latest uncommitted file time for {}: {:?}",
            repo_path.display(),
            latest
        );
    }

    latest
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

    #[test]
    fn test_head_reflog_time_nonexistent() {
        let fake_path = Path::new("/nonexistent/repo");
        let result = head_reflog_time(fake_path);
        assert!(result.is_none(), "Should return None for nonexistent repo");
    }

    #[test]
    fn test_latest_uncommitted_mtime_nonexistent() {
        let fake_path = Path::new("/nonexistent/repo");
        let result = latest_uncommitted_mtime(fake_path);
        assert!(result.is_none(), "Should return None for nonexistent repo");
    }
}
