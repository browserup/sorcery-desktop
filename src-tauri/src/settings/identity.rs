use super::models::{RepoIdentity, WorkspaceConfig, WorkspaceKind, WorkspaceState};
use crate::git_command_log::run_git_command;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceInspection {
    pub workspace_kind: WorkspaceKind,
    pub workspace_state: WorkspaceState,
    pub repo_identity: Option<RepoIdentity>,
}

#[must_use]
pub(crate) fn now_timestamp_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[must_use]
pub(crate) fn canonical_workspace_key_for_lookup(key: &str) -> String {
    let trimmed = key.trim();
    if cfg!(target_os = "windows") || cfg!(target_os = "macos") {
        trimmed.to_lowercase()
    } else {
        trimmed.to_string()
    }
}

#[must_use]
pub(crate) fn derive_workspace_key(workspace: &WorkspaceConfig) -> String {
    workspace.workspace_key.trim().to_string()
}

#[must_use]
pub(crate) fn normalize_remote_identity(remote: &str) -> Option<String> {
    let mut trimmed = remote.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((without_fragment, _)) = trimmed.split_once('#') {
        trimmed = without_fragment;
    }
    if let Some((without_query, _)) = trimmed.split_once('?') {
        trimmed = without_query;
    }
    trimmed = trimmed.trim_end_matches('/');

    let mut host_and_path = if let Some(stripped) = trimmed.strip_prefix("https://") {
        stripped
    } else if let Some(stripped) = trimmed.strip_prefix("http://") {
        stripped
    } else if let Some(stripped) = trimmed.strip_prefix("ssh://") {
        stripped
    } else if let Some(stripped) = trimmed.strip_prefix("git://") {
        stripped
    } else {
        trimmed
    };

    if let Some((_, without_user)) = host_and_path.rsplit_once('@') {
        host_and_path = without_user;
    }

    let (host, repo_path) = split_host_and_path(host_and_path)?;

    let host = host.trim().trim_matches('/').to_lowercase();
    if host.is_empty() {
        return None;
    }

    let mut repo_path = repo_path.trim().trim_matches('/').to_string();
    if repo_path.is_empty() {
        return None;
    }
    if repo_path.ends_with(".git") {
        repo_path.truncate(repo_path.len() - 4);
    }
    if repo_path.is_empty() {
        return None;
    }

    Some(format!("{host}/{repo_path}"))
}

fn split_host_and_path(host_and_path: &str) -> Option<(&str, &str)> {
    let slash_index = host_and_path.find('/');
    let colon_index = host_and_path.find(':');

    if let Some(colon_index) = colon_index {
        // Handle `host:path` scp-style remotes and `host:port/path` ssh URLs.
        if slash_index.is_none_or(|slash_index| colon_index < slash_index) {
            let host = &host_and_path[..colon_index];
            let remainder = &host_and_path[colon_index + 1..];
            if host.is_empty() || remainder.is_empty() {
                return None;
            }

            if let Some((port_segment, path_after_port)) = remainder.split_once('/') {
                if !port_segment.is_empty()
                    && port_segment
                        .chars()
                        .all(|character| character.is_ascii_digit())
                    && !path_after_port.is_empty()
                {
                    return Some((host, path_after_port));
                }
            }

            return Some((host, remainder));
        }
    }

    host_and_path.split_once('/')
}

#[must_use]
pub(crate) fn remote_matches_identity(remote: &str, identity: &RepoIdentity) -> bool {
    let Some(normalized) = normalize_remote_identity(remote) else {
        return false;
    };

    if identity
        .primary_remote
        .as_ref()
        .is_some_and(|p| p == &normalized)
    {
        return true;
    }

    identity
        .all_remotes
        .iter()
        .any(|candidate| candidate == &normalized)
}

#[must_use]
pub(crate) fn repo_group_key(identity: &RepoIdentity) -> Option<String> {
    if let Some(git_common_dir) = identity.git_common_dir.as_ref() {
        return Some(format!("common:{}", git_common_dir.to_string_lossy()));
    }

    identity
        .primary_remote
        .as_ref()
        .map(|remote| format!("remote:{remote}"))
}

#[must_use]
pub(crate) fn inspect_workspace(path: &Path) -> WorkspaceInspection {
    if !path.exists() {
        return WorkspaceInspection {
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Missing,
            repo_identity: None,
        };
    }

    if !path.is_dir() {
        return WorkspaceInspection {
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Unavailable,
            repo_identity: None,
        };
    }

    if !path.join(".git").exists() {
        return WorkspaceInspection {
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
        };
    }

    WorkspaceInspection {
        workspace_kind: WorkspaceKind::Git,
        workspace_state: WorkspaceState::Present,
        repo_identity: read_repo_identity(path),
    }
}

fn read_repo_identity(workspace_path: &Path) -> Option<RepoIdentity> {
    let workspace_str = workspace_path.to_string_lossy();

    let remote_names_output = run_git_command(&workspace_str, &["remote"]).ok()?;
    if !remote_names_output.status.success() {
        return None;
    }

    let mut all_remotes = BTreeSet::new();
    let mut primary_remote: Option<String> = None;

    let remote_names = String::from_utf8_lossy(&remote_names_output.stdout);
    for remote_name in remote_names
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let get_url_output = run_git_command(&workspace_str, &["remote", "get-url", remote_name]);
        let Ok(get_url_output) = get_url_output else {
            continue;
        };
        if !get_url_output.status.success() {
            continue;
        }

        let raw_remote = String::from_utf8_lossy(&get_url_output.stdout);
        let Some(normalized) = normalize_remote_identity(raw_remote.trim()) else {
            continue;
        };

        if remote_name == "origin" {
            primary_remote = Some(normalized.clone());
        }
        all_remotes.insert(normalized);
    }

    if primary_remote.is_none() {
        primary_remote = all_remotes.iter().next().cloned();
    }

    let git_common_dir = run_git_command(&workspace_str, &["rev-parse", "--git-common-dir"])
        .ok()
        .and_then(|output| {
            if !output.status.success() {
                return None;
            }
            let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if raw.is_empty() {
                return None;
            }
            let common_dir = PathBuf::from(raw);
            let resolved = if common_dir.is_absolute() {
                common_dir
            } else {
                workspace_path.join(common_dir)
            };
            Some(normalize_path_for_identity(resolved))
        });

    let current_branch = run_git_command(&workspace_str, &["symbolic-ref", "--short", "HEAD"])
        .ok()
        .and_then(|output| {
            if !output.status.success() {
                return None;
            }
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if value.is_empty() {
                return None;
            }
            Some(value)
        });

    let head_commit = run_git_command(&workspace_str, &["rev-parse", "HEAD"])
        .ok()
        .and_then(|output| {
            if !output.status.success() {
                return None;
            }
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if value.is_empty() {
                return None;
            }
            Some(value)
        });

    if primary_remote.is_none()
        && all_remotes.is_empty()
        && git_common_dir.is_none()
        && current_branch.is_none()
        && head_commit.is_none()
    {
        return None;
    }

    Some(RepoIdentity {
        primary_remote,
        all_remotes: all_remotes.into_iter().collect(),
        git_common_dir,
        current_branch,
        head_commit,
    })
}

fn normalize_path_for_identity(path: PathBuf) -> PathBuf {
    let normalized = path.canonicalize().unwrap_or(path);

    #[cfg(target_os = "macos")]
    {
        let normalized_str = normalized.to_string_lossy();
        if normalized_str.starts_with("/private/") {
            if let Ok(stripped) = normalized.strip_prefix("/private") {
                let mut absolute = PathBuf::from("/");
                absolute.push(stripped);
                return absolute;
            }
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::{inspect_workspace, normalize_remote_identity, repo_group_key};
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn normalizes_https_ssh_and_scp_remotes() {
        assert_eq!(
            normalize_remote_identity("https://github.com/rails/rails.git"),
            Some("github.com/rails/rails".to_string())
        );
        assert_eq!(
            normalize_remote_identity("git@github.com:rails/rails.git"),
            Some("github.com/rails/rails".to_string())
        );
        assert_eq!(
            normalize_remote_identity("ssh://git@github.com/rails/rails"),
            Some("github.com/rails/rails".to_string())
        );
        assert_eq!(
            normalize_remote_identity("ssh://git@github.com:2222/rails/rails.git"),
            Some("github.com/rails/rails".to_string())
        );
    }

    #[test]
    fn normalizes_host_and_path_without_scheme() {
        assert_eq!(
            normalize_remote_identity("github.enterprise.com/team/project"),
            Some("github.enterprise.com/team/project".to_string())
        );
    }

    #[test]
    fn strips_query_fragment_and_trailing_slash() {
        assert_eq!(
            normalize_remote_identity("https://github.com/vercel/next.js.git/?foo=bar#frag"),
            Some("github.com/vercel/next.js".to_string())
        );
    }

    #[test]
    fn rejects_invalid_remote_identities() {
        assert_eq!(normalize_remote_identity(""), None);
        assert_eq!(normalize_remote_identity("just-a-folder"), None);
        assert_eq!(normalize_remote_identity("/tmp/local/repo"), None);
    }

    #[test]
    fn worktree_metadata_includes_common_dir_and_branch_hints() {
        let temp_dir = TempDir::new().expect("temp dir");
        let repo_path = temp_dir.path().join("repo");
        let feature_path = temp_dir.path().join("repo-feature");
        std::fs::create_dir_all(&repo_path).expect("create repo dir");

        run_git(&repo_path, &["init"]);
        run_git(&repo_path, &["config", "user.email", "test@example.com"]);
        run_git(&repo_path, &["config", "user.name", "Test User"]);
        std::fs::write(repo_path.join("README.md"), "hello").expect("write readme");
        run_git(&repo_path, &["add", "README.md"]);
        run_git(&repo_path, &["commit", "-m", "init"]);
        run_git(&repo_path, &["branch", "-M", "main"]);
        run_git(&repo_path, &["branch", "feature"]);

        let feature_path_string = feature_path.to_string_lossy().to_string();
        run_git(
            &repo_path,
            &["worktree", "add", &feature_path_string, "feature"],
        );

        let primary = inspect_workspace(Path::new(&repo_path))
            .repo_identity
            .expect("primary repo identity");
        let feature = inspect_workspace(Path::new(&feature_path))
            .repo_identity
            .expect("feature repo identity");

        assert_eq!(primary.current_branch.as_deref(), Some("main"));
        assert_eq!(feature.current_branch.as_deref(), Some("feature"));
        assert!(primary.head_commit.is_some());
        assert!(feature.head_commit.is_some());
        assert_eq!(primary.git_common_dir, feature.git_common_dir);
        assert_eq!(repo_group_key(&primary), repo_group_key(&feature));
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("run git");
        assert!(status.success(), "git command failed: {:?}", args);
    }
}
