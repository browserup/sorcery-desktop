use super::GitRef;
use crate::git_command_log::run_git_command;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CloneError {
    #[error("Target path already exists: {0}")]
    TargetExists(PathBuf),

    #[error("Failed to create parent directory: {0}")]
    CreateDir(#[source] std::io::Error),

    #[error("Failed to execute git clone: {0}")]
    Execute(#[source] std::io::Error),

    #[error("Clone failed: {0}")]
    CommandFailed(String),

    #[error("Failed to checkout commit '{commit}': {message}")]
    CheckoutFailed { commit: String, message: String },
}
const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10MB

fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip escape sequence: ESC [ ... final_byte
            if chars.peek() == Some(&'[') {
                chars.next();
                // Skip until we hit a letter (the final byte of the sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else if c == '\r' {
            // Git uses \r for progress updates; treat as newline for our display
            if !result.ends_with('\n') && !result.is_empty() {
                result.push('\n');
            }
        } else {
            result.push(c);
        }
    }
    result.trim().to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkingTreeStatus {
    pub is_clean: bool,
    pub modified_count: usize,
    pub untracked_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitOperationState {
    pub is_blocked: bool,
    pub blocking_reason: Option<String>,
}

pub struct GitHandler;

#[allow(clippy::missing_errors_doc)]
impl GitHandler {
    pub fn validate_revision(workspace_path: &Path, rev: &str) -> Result<()> {
        if !Self::is_git_repo(workspace_path) {
            bail!(
                "Workspace is not a git repository: {}",
                workspace_path.display()
            );
        }

        let workspace_str = workspace_path.to_string_lossy();
        let output = run_git_command(&workspace_str, &["rev-parse", "--verify", rev])
            .context("Failed to execute git rev-parse")?;

        if !output.status.success() {
            bail!(
                "Invalid git revision '{}': {}",
                rev,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    pub fn get_file_at_revision(
        workspace_path: &Path,
        file_path: &str,
        rev: &str,
    ) -> Result<String> {
        if !Self::is_git_repo(workspace_path) {
            bail!(
                "Workspace is not a git repository: {}",
                workspace_path.display()
            );
        }

        let workspace_str = workspace_path.to_string_lossy();
        let output = run_git_command(&workspace_str, &["show", &format!("{rev}:{file_path}")])
            .context("Failed to execute git show")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("does not exist") || stderr.contains("exists on disk, but not in") {
                bail!("File '{file_path}' does not exist at revision '{rev}'");
            }
            bail!("Failed to get file '{file_path}' at revision '{rev}': {stderr}");
        }

        let content =
            String::from_utf8(output.stdout).context("File content is not valid UTF-8")?;

        let len = content.len();
        #[allow(clippy::cast_possible_truncation)]
        if len > MAX_FILE_SIZE_BYTES as usize {
            bail!("File '{file_path}' is too large ({len} bytes, max {MAX_FILE_SIZE_BYTES} bytes)");
        }

        Ok(content)
    }

    pub fn get_revision_info(workspace_path: &Path, rev: &str) -> Result<String> {
        if !Self::is_git_repo(workspace_path) {
            bail!(
                "Workspace is not a git repository: {}",
                workspace_path.display()
            );
        }

        let workspace_str = workspace_path.to_string_lossy();
        let output = run_git_command(
            &workspace_str,
            &["log", "-1", "--pretty=format:%h - %s (%an, %ar)", rev],
        )
        .context("Failed to execute git log")?;

        if !output.status.success() {
            bail!(
                "Failed to get revision info: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        String::from_utf8(output.stdout).context("Git log output is not valid UTF-8")
    }

    pub fn get_current_ref(workspace_path: &Path) -> Result<String> {
        if !Self::is_git_repo(workspace_path) {
            bail!(
                "Workspace is not a git repository: {}",
                workspace_path.display()
            );
        }

        let workspace_str = workspace_path.to_string_lossy();
        let output = run_git_command(&workspace_str, &["symbolic-ref", "--short", "HEAD"])
            .context("Failed to execute git symbolic-ref")?;

        if output.status.success() {
            return Ok(String::from_utf8(output.stdout)?.trim().to_string());
        }

        let output = run_git_command(&workspace_str, &["rev-parse", "--short", "HEAD"])
            .context("Failed to execute git rev-parse")?;

        if !output.status.success() {
            bail!(
                "Failed to get current ref: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    pub fn get_working_tree_status(workspace_path: &Path) -> Result<WorkingTreeStatus> {
        if !Self::is_git_repo(workspace_path) {
            bail!(
                "Workspace is not a git repository: {}",
                workspace_path.display()
            );
        }

        let workspace_str = workspace_path.to_string_lossy();
        let output = run_git_command(&workspace_str, &["status", "--porcelain"])
            .context("Failed to execute git status")?;

        if !output.status.success() {
            bail!(
                "Failed to get working tree status: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let status_output = String::from_utf8(output.stdout)?;
        let lines: Vec<&str> = status_output.lines().collect();

        let modified_count = lines.iter().filter(|line| !line.starts_with("??")).count();

        let untracked_count = lines.iter().filter(|line| line.starts_with("??")).count();

        Ok(WorkingTreeStatus {
            is_clean: lines.is_empty(),
            modified_count,
            untracked_count,
        })
    }

    pub fn check_git_operation_state(workspace_path: &Path) -> Result<GitOperationState> {
        if !Self::is_git_repo(workspace_path) {
            bail!(
                "Workspace is not a git repository: {}",
                workspace_path.display()
            );
        }

        let git_dir = workspace_path.join(".git");

        if git_dir.join("MERGE_HEAD").exists() {
            return Ok(GitOperationState {
                is_blocked: true,
                blocking_reason: Some("Merge in progress".to_string()),
            });
        }

        if git_dir.join("REBASE_HEAD").exists()
            || git_dir.join("rebase-merge").exists()
            || git_dir.join("rebase-apply").exists()
        {
            return Ok(GitOperationState {
                is_blocked: true,
                blocking_reason: Some("Rebase in progress".to_string()),
            });
        }

        if git_dir.join("CHERRY_PICK_HEAD").exists() {
            return Ok(GitOperationState {
                is_blocked: true,
                blocking_reason: Some("Cherry-pick in progress".to_string()),
            });
        }

        if git_dir.join("BISECT_LOG").exists() {
            return Ok(GitOperationState {
                is_blocked: true,
                blocking_reason: Some("Bisect in progress".to_string()),
            });
        }

        Ok(GitOperationState {
            is_blocked: false,
            blocking_reason: None,
        })
    }

    pub fn checkout_revision(workspace_path: &Path, rev: &str) -> Result<()> {
        if !Self::is_git_repo(workspace_path) {
            bail!(
                "Workspace is not a git repository: {}",
                workspace_path.display()
            );
        }

        let state = Self::check_git_operation_state(workspace_path)?;
        if state.is_blocked {
            bail!(
                "Cannot checkout: {}",
                state.blocking_reason.unwrap_or_default()
            );
        }

        let status = Self::get_working_tree_status(workspace_path)?;
        if !status.is_clean {
            bail!(
                "Cannot checkout: working tree has {} modified file(s)",
                status.modified_count
            );
        }

        let workspace_str = workspace_path.to_string_lossy();
        let output = run_git_command(&workspace_str, &["checkout", rev])
            .context("Failed to execute git checkout")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("pathspec") && stderr.contains("did not match") {
                bail!("Revision '{rev}' not found");
            }
            bail!("Failed to checkout '{rev}': {stderr}");
        }

        Ok(())
    }

    pub fn file_exists_at_revision(
        workspace_path: &Path,
        file_path: &str,
        rev: &str,
    ) -> Result<bool> {
        if !Self::is_git_repo(workspace_path) {
            bail!(
                "Workspace is not a git repository: {}",
                workspace_path.display()
            );
        }

        let workspace_str = workspace_path.to_string_lossy();
        let output = run_git_command(
            &workspace_str,
            &["cat-file", "-e", &format!("{rev}:{file_path}")],
        )
        .context("Failed to execute git cat-file")?;

        Ok(output.status.success())
    }

    #[must_use]
    pub fn find_git_root(start_path: &Path) -> Option<PathBuf> {
        let mut current = start_path;
        loop {
            if Self::is_git_repo(current) {
                return Some(current.to_path_buf());
            }
            current = current.parent()?;
        }
    }

    pub fn should_skip_revision_dialog(workspace_path: &Path, rev: &str) -> Result<bool> {
        if !Self::is_git_repo(workspace_path) {
            return Ok(false);
        }

        let current_ref = Self::get_current_ref(workspace_path)?;

        Ok(current_ref == rev || format!("origin/{current_ref}") == rev)
    }

    pub fn get_revision_dialog_state(
        workspace_path: &Path,
        file_path: &str,
        rev: &str,
    ) -> Result<(bool, Option<String>, WorkingTreeStatus)> {
        if !Self::is_git_repo(workspace_path) {
            bail!(
                "Workspace is not a git repository: {}",
                workspace_path.display()
            );
        }

        let file_exists = Self::file_exists_at_revision(workspace_path, file_path, rev)?;
        if !file_exists {
            bail!("File '{file_path}' does not exist at revision '{rev}'");
        }

        let status = Self::get_working_tree_status(workspace_path)?;
        let operation_state = Self::check_git_operation_state(workspace_path)?;

        let (checkout_available, checkout_blocked_reason) = if operation_state.is_blocked {
            (false, operation_state.blocking_reason)
        } else if !status.is_clean {
            {
                let count = status.modified_count;
                (
                    false,
                    Some(format!("{count} modified file(s) in working tree")),
                )
            }
        } else {
            (true, None)
        };

        Ok((checkout_available, checkout_blocked_reason, status))
    }

    fn is_git_repo(path: &Path) -> bool {
        path.join(".git").exists()
    }

    #[allow(dead_code)] // Public API for callers that don't need progress updates
    pub fn clone_repo(
        remote_url: &str,
        target_path: &Path,
        git_ref: Option<&GitRef>,
    ) -> Result<(), CloneError> {
        Self::clone_repo_with_progress(remote_url, target_path, git_ref, |_| {})
    }

    pub fn clone_repo_with_progress<F>(
        remote_url: &str,
        target_path: &Path,
        git_ref: Option<&GitRef>,
        mut on_output: F,
    ) -> Result<(), CloneError>
    where
        F: FnMut(&str),
    {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};

        if target_path.exists() {
            return Err(CloneError::TargetExists(target_path.to_path_buf()));
        }

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).map_err(CloneError::CreateDir)?;
        }

        // Ensure https:// prefix for git clone compatibility when needed
        let url = if remote_url.starts_with("https://")
            || remote_url.starts_with("http://")
            || remote_url.starts_with("git@")
            || remote_url.starts_with("ssh://")
            || remote_url.starts_with("file://")
            || remote_url.starts_with('/')
        {
            remote_url.to_string()
        } else {
            format!("https://{remote_url}")
        };

        let mut cmd = Command::new("git");
        cmd.arg("clone");
        cmd.arg("--progress"); // Force progress output even when not a tty

        if let Some(GitRef::Commit(_)) = git_ref {
            cmd.arg("--no-checkout");
        }

        if let Some(GitRef::Branch(name) | GitRef::Tag(name)) = git_ref {
            cmd.args(["--branch", name]);
        }

        cmd.arg(&url);
        cmd.arg(target_path);

        // Capture stderr for progress (git writes progress to stderr)
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(CloneError::Execute)?;

        // Read stderr in a streaming fashion
        // Git progress output uses \r for in-place updates, we normalize these
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let cleaned = strip_ansi_codes(&line);
                if !cleaned.is_empty() {
                    on_output(&cleaned);
                }
            }
        }

        let status = child.wait().map_err(CloneError::Execute)?;

        if !status.success() {
            return Err(CloneError::CommandFailed(
                "Clone process exited with error".to_string(),
            ));
        }

        if let Some(GitRef::Commit(commit)) = git_ref {
            tracing::info!("Checking out commit {commit} after clone");
            on_output(&format!("Checking out commit {commit}..."));
            let target_str = target_path.to_string_lossy();
            let checkout = run_git_command(&target_str, &["checkout", commit]).map_err(|e| {
                CloneError::CheckoutFailed {
                    commit: commit.clone(),
                    message: e.to_string(),
                }
            })?;
            if !checkout.status.success() {
                return Err(CloneError::CheckoutFailed {
                    commit: commit.clone(),
                    message: String::from_utf8_lossy(&checkout.stderr).trim().to_string(),
                });
            }
        }

        let target_display = target_path.display();
        tracing::info!("Cloned {url} to {target_display}");
        Ok(())
    }

    /// Get the base directory for worktrees under app config storage.
    fn get_worktrees_base_dir() -> Result<PathBuf> {
        let dir = crate::config_paths::canonical_config_dir()?.join("worktrees");
        std::fs::create_dir_all(&dir).context("Failed to create worktrees directory")?;
        Ok(dir)
    }

    /// Sanitize a branch/ref name for use as a directory name
    fn sanitize_ref_name(ref_name: &str) -> String {
        ref_name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-")
    }

    /// Resolve a ref to its commit hash
    fn resolve_commit_hash(workspace_path: &Path, rev: &str) -> Result<String> {
        let workspace_str = workspace_path.to_string_lossy();
        let output = run_git_command(&workspace_str, &["rev-parse", rev])
            .context("Failed to resolve commit hash")?;

        if !output.status.success() {
            bail!("Failed to resolve '{rev}' to commit hash");
        }

        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    /// Enforce LRU limit: keep max 3 worktrees per project, remove oldest if needed
    fn enforce_worktree_limit(workspace_path: &Path, project_dir: &Path) -> Result<()> {
        const MAX_WORKTREES: usize = 3;

        if !project_dir.exists() {
            return Ok(());
        }

        let mut entries: Vec<_> = std::fs::read_dir(project_dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                let metadata = e.metadata().ok()?;
                let mtime = metadata.modified().ok()?;
                Some((e.path(), mtime))
            })
            .collect();

        if entries.len() < MAX_WORKTREES {
            return Ok(());
        }

        // Sort by mtime, oldest first
        entries.sort_by(|a, b| a.1.cmp(&b.1));

        // Remove oldest entries until we're under the limit
        let to_remove = entries.len() - (MAX_WORKTREES - 1); // -1 to make room for new one
        for (path, _) in entries.into_iter().take(to_remove) {
            let path_display = path.display();
            tracing::info!("Removing old worktree: {path_display}");

            // Try git worktree remove first
            let workspace_str = workspace_path.to_string_lossy();
            let path_str = path.to_string_lossy();
            let output = run_git_command(
                &workspace_str,
                &["worktree", "remove", "--force", &path_str],
            );

            let removed = match output {
                Ok(out) => out.status.success(),
                Err(e) => {
                    tracing::debug!("git worktree remove failed: {e}");
                    false
                }
            };

            if !removed {
                if let Err(e) = std::fs::remove_dir_all(&path) {
                    tracing::warn!("Failed to remove worktree directory {path_display}: {e}");
                }
            }
        }

        // Prune stale worktree entries
        let workspace_str = workspace_path.to_string_lossy();
        let _ = run_git_command(&workspace_str, &["worktree", "prune"]);

        Ok(())
    }

    /// Create a worktree for the given branch/commit, or reuse existing one.
    /// Returns the path to the worktree.
    pub fn create_worktree(
        workspace_path: &Path,
        project_name: &str,
        branch_or_commit: &str,
    ) -> Result<PathBuf> {
        if !Self::is_git_repo(workspace_path) {
            bail!(
                "Workspace is not a git repository: {}",
                workspace_path.display()
            );
        }

        // Calculate worktree path
        let base_dir = Self::get_worktrees_base_dir()?;
        let safe_project = Self::sanitize_ref_name(project_name);
        let safe_ref = Self::sanitize_ref_name(branch_or_commit);
        let project_dir = base_dir.join(&safe_project);
        let worktree_path = project_dir.join(&safe_ref);

        // Check if worktree already exists and is valid
        if worktree_path.exists() && worktree_path.join(".git").exists() {
            let worktree_display = worktree_path.display();
            tracing::info!("Reusing existing worktree: {worktree_display}");
            // Touch the directory to update mtime for LRU
            let _ = std::fs::File::create(worktree_path.join(".sorcery_accessed"));
            let _ = std::fs::remove_file(worktree_path.join(".sorcery_accessed"));
            return Ok(worktree_path);
        }

        // Clean up if path exists but isn't a valid worktree
        if worktree_path.exists() {
            std::fs::remove_dir_all(&worktree_path)
                .context("Failed to clean up invalid worktree path")?;
        }

        // Enforce LRU limit before creating new worktree
        Self::enforce_worktree_limit(workspace_path, &project_dir)?;

        // Ensure project directory exists
        std::fs::create_dir_all(&project_dir)
            .context("Failed to create project worktree directory")?;

        // Try standard worktree add
        let workspace_str = workspace_path.to_string_lossy();
        let worktree_str = worktree_path.to_string_lossy();
        let output = run_git_command(
            &workspace_str,
            &["worktree", "add", &worktree_str, branch_or_commit],
        )
        .context("Failed to execute git worktree add")?;

        if output.status.success() {
            let worktree_display = worktree_path.display();
            tracing::info!("Created worktree at {worktree_display}");
            return Ok(worktree_path);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);

        // If branch is already checked out, try detached HEAD with commit hash
        if stderr.contains("already checked out") || stderr.contains("is already used") {
            tracing::info!("Branch already checked out, trying detached HEAD");

            let commit_hash = Self::resolve_commit_hash(workspace_path, branch_or_commit)?;
            let output = run_git_command(
                &workspace_str,
                &["worktree", "add", "--detach", &worktree_str, &commit_hash],
            )
            .context("Failed to execute git worktree add --detach")?;

            if output.status.success() {
                let worktree_display = worktree_path.display();
                tracing::info!("Created detached worktree at {worktree_display}");
                return Ok(worktree_path);
            }

            bail!(
                "Failed to create worktree (detached): {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        bail!("Failed to create worktree: {stderr}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_remote_repo() -> (TempDir, PathBuf, String) {
        let temp = TempDir::new().expect("temp dir");
        let origin = temp.path().join("origin.git");
        run(
            Command::new("git").args([
                "init",
                "--bare",
                origin.to_str().expect("origin path is valid UTF-8"),
            ]),
            temp.path(),
        );

        let work = temp.path().join("work");
        std::fs::create_dir(&work).expect("create work dir");
        run(Command::new("git").arg("init"), &work);
        run(
            Command::new("git").args(["config", "user.email", "test@test.com"]),
            &work,
        );
        run(
            Command::new("git").args(["config", "user.name", "Test"]),
            &work,
        );
        std::fs::write(work.join("README.md"), "hello").expect("write README");
        run(Command::new("git").args(["add", "README.md"]), &work);
        run(Command::new("git").args(["commit", "-m", "init"]), &work);
        run(Command::new("git").args(["branch", "-M", "main"]), &work);
        run(
            Command::new("git").args([
                "remote",
                "add",
                "origin",
                origin.to_str().expect("origin path is valid UTF-8"),
            ]),
            &work,
        );
        run(Command::new("git").args(["push", "origin", "main"]), &work);

        let rev = capture(Command::new("git").args(["rev-parse", "HEAD"]), &work);
        (temp, origin, rev)
    }

    fn run(cmd: &mut Command, dir: &Path) {
        let status = cmd.current_dir(dir).status().expect("status");
        assert!(status.success(), "command failed");
    }

    fn capture(cmd: &mut Command, dir: &Path) -> String {
        let output = cmd.current_dir(dir).output().expect("output");
        assert!(output.status.success(), "command failed");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn clone_repo_supports_commit_refs() {
        let (temp, origin, commit) = create_remote_repo();
        let target = temp.path().join("clone");
        GitHandler::clone_repo(
            origin.to_str().expect("origin path is valid UTF-8"),
            &target,
            Some(&GitRef::Commit(commit.clone())),
        )
        .expect("clone commit");

        let head = capture(Command::new("git").args(["rev-parse", "HEAD"]), &target);
        assert_eq!(head, commit);
    }
}
