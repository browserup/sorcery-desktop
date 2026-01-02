use super::fs_signal;
use super::git_signals;
use super::models::Probe;
use super::process;
use std::path::Path;
use sysinfo::System;
use tracing::debug;

const MAX_FS_ENTRIES: usize = 400;

pub async fn probe_workspace(workspace_path: &Path, sys: &System) -> Probe {
    let mut probe = Probe::default();

    probe.from_process = process::check_running_process(workspace_path, sys);

    let workspace = workspace_path.to_path_buf();
    let (reflog, uncommitted, fs_time) = tokio::task::spawn_blocking(move || {
        let reflog = git_signals::head_reflog_time(&workspace);
        let uncommitted = git_signals::latest_uncommitted_mtime(&workspace);
        let fs_time = fs_signal::fs_recent_mtime(&workspace, MAX_FS_ENTRIES);
        (reflog, uncommitted, fs_time)
    })
    .await
    .unwrap_or((None, None, None));

    probe.from_reflog = reflog;
    probe.from_uncommitted = uncommitted;
    probe.from_fs = fs_time;

    probe.compute_last_active();

    debug!(
        "Workspace probe for {}: process={:?}, reflog={:?}, uncommitted={:?}, fs={:?}, last_active={:?}",
        workspace_path.display(),
        probe.from_process,
        probe.from_reflog,
        probe.from_uncommitted,
        probe.from_fs,
        probe.last_active
    );

    probe
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_probe_workspace_current_dir() {
        let mut sys = sysinfo::System::new_all();
        process::refresh_process_snapshot(&mut sys);

        let current_dir = env::current_dir().expect("Failed to get current directory");

        let probe = probe_workspace(&current_dir, &sys).await;

        assert!(
            probe.last_active.is_some(),
            "Should have a last_active time"
        );
        assert!(
            probe.from_process.is_some() || probe.from_fs.is_some(),
            "Should detect at least process or filesystem signal"
        );
    }

    #[tokio::test]
    async fn test_probe_workspace_nonexistent() {
        let mut sys = sysinfo::System::new_all();
        process::refresh_process_snapshot(&mut sys);

        let fake_path = std::path::Path::new("/nonexistent/workspace");

        let probe = probe_workspace(fake_path, &sys).await;

        assert!(
            probe.last_active.is_none(),
            "Should have no activity for nonexistent path"
        );
        assert!(probe.from_process.is_none());
        assert!(probe.from_reflog.is_none());
        assert!(probe.from_uncommitted.is_none());
        assert!(probe.from_fs.is_none());
    }
}
