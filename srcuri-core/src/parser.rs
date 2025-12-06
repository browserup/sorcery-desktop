use crate::types::{ParseError, Provider, SrcuriTarget};
use url::Url;

/// Parse a remote URL in various formats:
/// - Full URL: `https://github.com/owner/repo/blob/main/file.rs#L42`
/// - Path-style: `github.com/owner/repo/blob/main/file.rs:42`
/// - With https:// in path: `https://github.com/owner/repo/...`
pub fn parse_remote_url(remote_url: &str) -> Result<SrcuriTarget, ParseError> {
    // Extract line number from :N suffix if present (for path-style URLs)
    let (url_part, path_line) = extract_path_line_suffix(remote_url);

    // Normalize to full URL
    let normalized = normalize_to_url(url_part);

    let url = Url::parse(&normalized)
        .map_err(|e| ParseError::new(format!("Invalid URL: {}", e), remote_url))?;

    let provider = detect_provider(&url)
        .ok_or_else(|| ParseError::new("Unrecognized repository provider", remote_url))?;

    let mut target = match provider {
        Provider::GitHub => parse_github(&url),
        Provider::GitLab => parse_gitlab(&url),
        Provider::Bitbucket => parse_bitbucket(&url),
        Provider::Gitea | Provider::Codeberg => parse_gitea(&url, provider),
        Provider::AzureDevOps => parse_azure(&url),
    }?;

    // Override line with path-extracted line if present and no line from fragment
    if target.line.is_none() {
        target.line = path_line;
    }

    Ok(target)
}

/// Extract :N line suffix from end of path-style URL
/// Returns (url_without_suffix, optional_line)
pub fn extract_path_line_suffix(input: &str) -> (&str, Option<u32>) {
    // Look for :N at the end (but not :// which is protocol)
    if let Some(colon_pos) = input.rfind(':') {
        // Make sure it's not the :// in https://
        if colon_pos > 0 && !input[..colon_pos].ends_with('/') {
            let after_colon = &input[colon_pos + 1..];
            // Check if it's a number (possibly followed by fragment)
            let num_part = after_colon.split('#').next().unwrap_or(after_colon);
            if let Ok(line) = num_part.parse::<u32>() {
                return (&input[..colon_pos], Some(line));
            }
        }
    }
    (input, None)
}

/// Normalize various URL formats to a full https:// URL
fn normalize_to_url(input: &str) -> String {
    let trimmed = input.trim_start_matches('/');

    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed)
    }
}

pub fn detect_provider(url: &Url) -> Option<Provider> {
    let host = url.host_str()?;
    let path = url.path();

    // GitHub.dev (VS Code in browser) - same patterns as github.com
    if host == "github.dev" {
        return Some(Provider::GitHub);
    }

    // codespaces.new domain (shorthand for creating codespaces)
    if host == "codespaces.new" {
        return Some(Provider::GitHub);
    }

    // GitHub Codespaces URLs on github.com
    if host == "github.com" && path.starts_with("/codespaces/") {
        return Some(Provider::GitHub);
    }

    // GitLab Web IDE pattern
    if path.starts_with("/-/ide/") {
        return Some(Provider::GitLab);
    }

    // Pattern-based detection (supports self-hosted)
    if path.contains("/-/blob/")
        || path.contains("/-/tree/")
        || path.contains("/-/blame/")
        || path.contains("/-/raw/")
    {
        return Some(Provider::GitLab);
    }

    if path.contains("/src/branch/") || path.contains("/src/tag/") || path.contains("/src/commit/")
    {
        if host == "codeberg.org" {
            return Some(Provider::Codeberg);
        }
        return Some(Provider::Gitea);
    }

    if path.contains("/_git/") {
        return Some(Provider::AzureDevOps);
    }

    if path.contains("/blob/")
        || path.contains("/tree/")
        || path.contains("/blame/")
        || path.contains("/raw/")
    {
        return Some(Provider::GitHub);
    }

    // Host-based detection (for repo-only URLs)
    if host == "github.com" || host.ends_with(".github.com") {
        return Some(Provider::GitHub);
    }
    if host == "gitlab.com" || host.contains("gitlab") {
        return Some(Provider::GitLab);
    }
    if host == "bitbucket.org" {
        return Some(Provider::Bitbucket);
    }
    if host == "gitea.com" {
        return Some(Provider::Gitea);
    }
    if host == "codeberg.org" {
        return Some(Provider::Codeberg);
    }
    if host == "dev.azure.com" || host.ends_with(".visualstudio.com") {
        return Some(Provider::AzureDevOps);
    }

    None
}

fn parse_github(url: &Url) -> Result<SrcuriTarget, ParseError> {
    let host = url.host_str().unwrap_or("github.com");
    let path = url.path();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Handle codespaces.new domain: https://codespaces.new/owner/repo?params
    if host == "codespaces.new" {
        if segments.len() >= 2 {
            let owner = segments[0];
            let repo = segments[1];
            let remote = format!("github.com/{}/{}", owner, repo);
            return Ok(SrcuriTarget {
                remote,
                repo_name: repo.to_string(),
                ref_value: None,
                file_path: None,
                line: None,
                is_absolute: false,
            });
        }
        return Err(ParseError::new(
            "codespaces.new URL must have owner and repo",
            url.as_str(),
        ));
    }

    // Handle Codespaces URLs on github.com: /codespaces/new/:owner/:repo/...
    if segments.first() == Some(&"codespaces") {
        // /codespaces/new with no repo is the landing page - error
        if segments.len() < 4 || segments.get(1) != Some(&"new") {
            return Err(ParseError::new(
                "Codespaces URL must have owner and repo",
                url.as_str(),
            ));
        }
        let owner = segments[2];
        let repo = segments[3];
        let remote = format!("github.com/{}/{}", owner, repo);
        return Ok(SrcuriTarget {
            remote,
            repo_name: repo.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    if segments.len() < 2 {
        return Err(ParseError::new(
            "GitHub URL must have owner and repo",
            url.as_str(),
        ));
    }

    let owner = segments[0];
    let repo = segments[1];
    // For github.dev, keep github.dev as the host; otherwise use the actual host
    let remote = format!("{}/{}/{}", host, owner, repo);

    // Repo-only URL
    if segments.len() == 2 {
        return Ok(SrcuriTarget {
            remote,
            repo_name: repo.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    // Check for blob/tree/blame/raw/pull pattern
    let view_type = segments.get(2).copied();
    if !matches!(
        view_type,
        Some("blob") | Some("tree") | Some("blame") | Some("raw") | Some("pull")
    ) {
        // Unknown pattern, treat as repo-only
        return Ok(SrcuriTarget {
            remote,
            repo_name: repo.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    // For pull requests, just return repo-only (no file context)
    if view_type == Some("pull") {
        return Ok(SrcuriTarget {
            remote,
            repo_name: repo.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    let ref_value = segments.get(3).map(|s| s.to_string());
    let file_path = if segments.len() > 4 {
        Some(segments[4..].join("/"))
    } else {
        None
    };
    let line = extract_github_line(url.fragment());

    Ok(SrcuriTarget {
        remote,
        repo_name: repo.to_string(),
        ref_value,
        file_path,
        line,
        is_absolute: false,
    })
}

fn parse_gitlab(url: &Url) -> Result<SrcuriTarget, ParseError> {
    let host = url.host_str().unwrap_or("gitlab.com");
    let path = url.path();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Handle Web IDE URLs: /-/ide/project/:group/:project/...
    if segments.len() >= 5 && segments[0] == "-" && segments[1] == "ide" && segments[2] == "project"
    {
        let group = segments[3];
        let project = segments[4];
        let remote = format!("{}/{}/{}", host, group, project);

        // Check for edit/:ref/... pattern
        if segments.len() >= 7 && segments[5] == "edit" {
            let ref_value = Some(segments[6].to_string());

            // Determine file path - several patterns possible:
            // 1. edit/:ref/-/:path (standard with -/ separator)
            // 2. edit/:ref/-/ (trailing slash, no file)
            // 3. edit/:ref/:path (no -/ separator, file directly after ref)
            // 4. edit/:ref (no file at all)
            let file_path = if segments.len() >= 9 && segments[7] == "-" {
                // Pattern 1: edit/:ref/-/:path
                let path = segments[8..].join("/");
                if path.is_empty() {
                    None
                } else {
                    Some(path)
                }
            } else if segments.len() == 8 && segments[7] == "-" {
                // Pattern 2: edit/:ref/-/ (no file after separator)
                None
            } else if segments.len() > 7 && segments[7] != "-" {
                // Pattern 3: edit/:ref/:path (no -/ separator)
                Some(segments[7..].join("/"))
            } else {
                // Pattern 4: edit/:ref (no file)
                None
            };

            return Ok(SrcuriTarget {
                remote,
                repo_name: project.to_string(),
                ref_value,
                file_path,
                line: extract_github_line(url.fragment()),
                is_absolute: false,
            });
        }

        // edit/:ref with no further segments (e.g., edit/master)
        if segments.len() == 7 && segments[5] == "edit" {
            return Ok(SrcuriTarget {
                remote,
                repo_name: project.to_string(),
                ref_value: Some(segments[6].to_string()),
                file_path: None,
                line: None,
                is_absolute: false,
            });
        }

        // Other IDE patterns (merge_requests, etc.) - repo only
        return Ok(SrcuriTarget {
            remote,
            repo_name: project.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    if segments.len() < 2 {
        return Err(ParseError::new(
            "GitLab URL must have group and project",
            url.as_str(),
        ));
    }

    let group = segments[0];
    let project = segments[1];
    let remote = format!("{}/{}/{}", host, group, project);

    // Repo-only URL
    if segments.len() == 2 {
        return Ok(SrcuriTarget {
            remote,
            repo_name: project.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    // Look for /-/ pattern
    let dash_pos = segments.iter().position(|&s| s == "-");
    if dash_pos.is_none() {
        // No /-/ pattern, treat as repo-only
        return Ok(SrcuriTarget {
            remote,
            repo_name: project.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    let dash_idx = dash_pos.unwrap();
    let view_type = segments.get(dash_idx + 1).copied();
    if !matches!(
        view_type,
        Some("blob") | Some("tree") | Some("blame") | Some("raw")
    ) {
        return Ok(SrcuriTarget {
            remote,
            repo_name: project.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    let ref_value = segments.get(dash_idx + 2).map(|s| s.to_string());
    let file_path = if segments.len() > dash_idx + 3 {
        Some(segments[dash_idx + 3..].join("/"))
    } else {
        None
    };
    let line = extract_github_line(url.fragment());

    Ok(SrcuriTarget {
        remote,
        repo_name: project.to_string(),
        ref_value,
        file_path,
        line,
        is_absolute: false,
    })
}

fn parse_bitbucket(url: &Url) -> Result<SrcuriTarget, ParseError> {
    let host = url.host_str().unwrap_or("bitbucket.org");
    let path = url.path();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if segments.len() < 2 {
        return Err(ParseError::new(
            "Bitbucket URL must have workspace and repo",
            url.as_str(),
        ));
    }

    let workspace = segments[0];
    let repo = segments[1];
    let remote = format!("{}/{}/{}", host, workspace, repo);

    // Repo-only URL
    if segments.len() == 2 {
        return Ok(SrcuriTarget {
            remote,
            repo_name: repo.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    // Check for /src/ pattern
    if segments.get(2) != Some(&"src") {
        return Ok(SrcuriTarget {
            remote,
            repo_name: repo.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    let ref_value = segments.get(3).map(|s| s.to_string());
    let file_path = if segments.len() > 4 {
        Some(segments[4..].join("/"))
    } else {
        None
    };
    let line = extract_bitbucket_line(url.fragment());

    Ok(SrcuriTarget {
        remote,
        repo_name: repo.to_string(),
        ref_value,
        file_path,
        line,
        is_absolute: false,
    })
}

fn parse_gitea(url: &Url, provider: Provider) -> Result<SrcuriTarget, ParseError> {
    let host = url.host_str().unwrap_or(match provider {
        Provider::Codeberg => "codeberg.org",
        _ => "gitea.com",
    });
    let path = url.path();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if segments.len() < 2 {
        return Err(ParseError::new(
            "Gitea URL must have owner and repo",
            url.as_str(),
        ));
    }

    let owner = segments[0];
    let repo = segments[1];
    let remote = format!("{}/{}/{}", host, owner, repo);

    // Repo-only URL
    if segments.len() == 2 {
        return Ok(SrcuriTarget {
            remote,
            repo_name: repo.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    // Check for /src/(branch|tag|commit)/ pattern
    if segments.get(2) != Some(&"src") {
        return Ok(SrcuriTarget {
            remote,
            repo_name: repo.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    let ref_type = segments.get(3).copied();
    if !matches!(ref_type, Some("branch") | Some("tag") | Some("commit")) {
        return Ok(SrcuriTarget {
            remote,
            repo_name: repo.to_string(),
            ref_value: None,
            file_path: None,
            line: None,
            is_absolute: false,
        });
    }

    let ref_value = segments.get(4).map(|s| s.to_string());
    let file_path = if segments.len() > 5 {
        Some(segments[5..].join("/"))
    } else {
        None
    };
    let line = extract_github_line(url.fragment());

    Ok(SrcuriTarget {
        remote,
        repo_name: repo.to_string(),
        ref_value,
        file_path,
        line,
        is_absolute: false,
    })
}

fn parse_azure(url: &Url) -> Result<SrcuriTarget, ParseError> {
    let host = url.host_str().unwrap_or("dev.azure.com");
    let path = url.path();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Find _git position
    let git_pos = segments.iter().position(|&s| s == "_git");
    if git_pos.is_none() {
        return Err(ParseError::new(
            "Azure DevOps URL must contain /_git/",
            url.as_str(),
        ));
    }

    let git_idx = git_pos.unwrap();
    let repo = segments.get(git_idx + 1).ok_or_else(|| {
        ParseError::new("Azure DevOps URL must have repo after /_git/", url.as_str())
    })?;

    // Build remote: either org/project/_git/repo or org/_git/repo
    let remote = segments[..=git_idx + 1].join("/");
    let remote = format!("{}/{}", host, remote);

    // Extract query params
    let mut file_path = None;
    let mut ref_value = None;
    let mut line = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "path" => {
                let p = value.trim_start_matches('/');
                if !p.is_empty() {
                    file_path = Some(p.to_string());
                }
            }
            "version" => {
                // Strip GB/GT/GC prefix
                if value.len() >= 2 {
                    ref_value = Some(value[2..].to_string());
                }
            }
            "line" => {
                line = value.parse().ok();
            }
            _ => {}
        }
    }

    Ok(SrcuriTarget {
        remote,
        repo_name: repo.to_string(),
        ref_value,
        file_path,
        line,
        is_absolute: false,
    })
}

fn extract_github_line(fragment: Option<&str>) -> Option<u32> {
    let fragment = fragment?;
    if !fragment.starts_with('L') {
        return None;
    }
    let rest = &fragment[1..];
    let num_str = rest.split('-').next()?;
    let num_str = num_str.trim_start_matches('L');
    num_str.parse().ok()
}

fn extract_bitbucket_line(fragment: Option<&str>) -> Option<u32> {
    let fragment = fragment?;
    if !fragment.starts_with("lines-") {
        return None;
    }
    let rest = &fragment[6..]; // e.g., "5", "5:10", or "10-20"
                               // Try colon separator first (lines-5:10), then dash (lines-10-20)
    let num_str = if rest.contains(':') {
        rest.split(':').next()?
    } else {
        rest.split('-').next().unwrap_or(rest)
    };
    num_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== GitHub Tests ====================

    #[test]
    fn github_repo_only() {
        let result = parse_remote_url("https://github.com/owner/repo").unwrap();
        assert_eq!(result.remote, "github.com/owner/repo");
        assert_eq!(result.repo_name, "repo");
        assert_eq!(result.ref_value, None);
        assert_eq!(result.file_path, None);
        assert_eq!(result.line, None);
    }

    #[test]
    fn github_blob_with_branch() {
        let result =
            parse_remote_url("https://github.com/owner/repo/blob/main/src/lib.rs").unwrap();
        assert_eq!(result.remote, "github.com/owner/repo");
        assert_eq!(result.repo_name, "repo");
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("src/lib.rs".to_string()));
        assert_eq!(result.line, None);
    }

    #[test]
    fn github_blob_with_sha() {
        let result =
            parse_remote_url("https://github.com/owner/repo/blob/abc123def456/file.rs").unwrap();
        assert_eq!(result.ref_value, Some("abc123def456".to_string()));
    }

    #[test]
    fn github_blob_with_line() {
        let result =
            parse_remote_url("https://github.com/owner/repo/blob/main/src/lib.rs#L42").unwrap();
        assert_eq!(result.file_path, Some("src/lib.rs".to_string()));
        assert_eq!(result.line, Some(42));
    }

    #[test]
    fn github_blob_with_line_range() {
        let result =
            parse_remote_url("https://github.com/owner/repo/blob/main/file.rs#L10-L20").unwrap();
        assert_eq!(result.line, Some(10)); // Takes first line only
    }

    #[test]
    fn github_tree_directory() {
        let result =
            parse_remote_url("https://github.com/owner/repo/tree/main/src/components").unwrap();
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("src/components".to_string()));
    }

    #[test]
    fn github_tree_root() {
        let result = parse_remote_url("https://github.com/owner/repo/tree/develop").unwrap();
        assert_eq!(result.ref_value, Some("develop".to_string()));
        assert_eq!(result.file_path, None);
    }

    #[test]
    fn github_blame() {
        let result =
            parse_remote_url("https://github.com/owner/repo/blame/main/src/main.rs#L100").unwrap();
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("src/main.rs".to_string()));
        assert_eq!(result.line, Some(100));
    }

    #[test]
    fn github_raw() {
        let result = parse_remote_url("https://github.com/owner/repo/raw/main/README.md").unwrap();
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("README.md".to_string()));
    }

    #[test]
    fn github_nested_path() {
        let result =
            parse_remote_url("https://github.com/owner/repo/blob/main/src/a/b/c/d.rs#L5").unwrap();
        assert_eq!(result.file_path, Some("src/a/b/c/d.rs".to_string()));
        assert_eq!(result.line, Some(5));
    }

    // ==================== GitLab Tests ====================

    #[test]
    fn gitlab_repo_only() {
        let result = parse_remote_url("https://gitlab.com/group/project").unwrap();
        assert_eq!(result.remote, "gitlab.com/group/project");
        assert_eq!(result.repo_name, "project");
        assert_eq!(result.ref_value, None);
    }

    #[test]
    fn gitlab_blob_with_branch() {
        let result =
            parse_remote_url("https://gitlab.com/group/project/-/blob/master/lib/file.rb").unwrap();
        assert_eq!(result.remote, "gitlab.com/group/project");
        assert_eq!(result.ref_value, Some("master".to_string()));
        assert_eq!(result.file_path, Some("lib/file.rb".to_string()));
    }

    #[test]
    fn gitlab_blob_with_line() {
        let result =
            parse_remote_url("https://gitlab.com/group/project/-/blob/master/file.rb#L12").unwrap();
        assert_eq!(result.line, Some(12));
    }

    #[test]
    fn gitlab_tree() {
        let result =
            parse_remote_url("https://gitlab.com/group/project/-/tree/develop/src").unwrap();
        assert_eq!(result.ref_value, Some("develop".to_string()));
        assert_eq!(result.file_path, Some("src".to_string()));
    }

    #[test]
    fn gitlab_blame() {
        let result =
            parse_remote_url("https://gitlab.com/group/project/-/blame/main/config.yml#L50")
                .unwrap();
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.line, Some(50));
    }

    #[test]
    fn gitlab_raw() {
        let result =
            parse_remote_url("https://gitlab.com/group/project/-/raw/main/script.sh").unwrap();
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("script.sh".to_string()));
    }

    #[test]
    fn gitlab_selfhosted() {
        let result =
            parse_remote_url("https://gitlab.mycompany.com/team/app/-/blob/develop/main.py#L10")
                .unwrap();
        assert_eq!(result.remote, "gitlab.mycompany.com/team/app");
        assert_eq!(result.ref_value, Some("develop".to_string()));
        assert_eq!(result.line, Some(10));
    }

    #[test]
    fn gitlab_selfhosted_detected_by_pattern() {
        // Even without "gitlab" in hostname, detected by /-/blob/ pattern
        let result =
            parse_remote_url("https://code.internal.io/team/app/-/blob/main/file.py").unwrap();
        assert_eq!(result.remote, "code.internal.io/team/app");
    }

    // ==================== Bitbucket Tests ====================

    #[test]
    fn bitbucket_repo_only() {
        let result = parse_remote_url("https://bitbucket.org/workspace/repo").unwrap();
        assert_eq!(result.remote, "bitbucket.org/workspace/repo");
        assert_eq!(result.repo_name, "repo");
        assert_eq!(result.ref_value, None);
    }

    #[test]
    fn bitbucket_src_with_branch() {
        let result =
            parse_remote_url("https://bitbucket.org/workspace/repo/src/master/README.md").unwrap();
        assert_eq!(result.ref_value, Some("master".to_string()));
        assert_eq!(result.file_path, Some("README.md".to_string()));
    }

    #[test]
    fn bitbucket_line_single() {
        let result =
            parse_remote_url("https://bitbucket.org/workspace/repo/src/master/file.py#lines-5")
                .unwrap();
        assert_eq!(result.line, Some(5));
    }

    #[test]
    fn bitbucket_line_range_colon() {
        let result =
            parse_remote_url("https://bitbucket.org/workspace/repo/src/master/file.py#lines-5:10")
                .unwrap();
        assert_eq!(result.line, Some(5)); // Takes first line only
    }

    #[test]
    fn bitbucket_nested_path() {
        let result = parse_remote_url(
            "https://bitbucket.org/workspace/repo/src/develop/src/main/java/App.java#lines-100",
        )
        .unwrap();
        assert_eq!(result.file_path, Some("src/main/java/App.java".to_string()));
        assert_eq!(result.line, Some(100));
    }

    // ==================== Gitea Tests ====================

    #[test]
    fn gitea_repo_only() {
        let result = parse_remote_url("https://gitea.com/org/repo").unwrap();
        assert_eq!(result.remote, "gitea.com/org/repo");
        assert_eq!(result.repo_name, "repo");
        assert_eq!(result.ref_value, None);
    }

    #[test]
    fn gitea_src_branch() {
        let result =
            parse_remote_url("https://gitea.com/org/repo/src/branch/main/cmd/main.go").unwrap();
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("cmd/main.go".to_string()));
    }

    #[test]
    fn gitea_src_tag() {
        let result = parse_remote_url("https://gitea.com/org/repo/src/tag/v1.0.0/file.go").unwrap();
        assert_eq!(result.ref_value, Some("v1.0.0".to_string()));
    }

    #[test]
    fn gitea_src_commit() {
        let result =
            parse_remote_url("https://gitea.com/org/repo/src/commit/abc123/file.go").unwrap();
        assert_eq!(result.ref_value, Some("abc123".to_string()));
    }

    #[test]
    fn gitea_with_line() {
        let result =
            parse_remote_url("https://gitea.com/org/repo/src/branch/main/file.go#L24").unwrap();
        assert_eq!(result.line, Some(24));
    }

    #[test]
    fn gitea_selfhosted() {
        let result = parse_remote_url(
            "https://git.mycompany.com/team/project/src/branch/develop/app.go#L15",
        )
        .unwrap();
        assert_eq!(result.remote, "git.mycompany.com/team/project");
        assert_eq!(result.ref_value, Some("develop".to_string()));
        assert_eq!(result.line, Some(15));
    }

    // ==================== Codeberg Tests ====================

    #[test]
    fn codeberg_repo_only() {
        let result = parse_remote_url("https://codeberg.org/user/repo").unwrap();
        assert_eq!(result.remote, "codeberg.org/user/repo");
        assert_eq!(result.repo_name, "repo");
    }

    #[test]
    fn codeberg_src_branch() {
        let result =
            parse_remote_url("https://codeberg.org/user/repo/src/branch/main/file.go#L10").unwrap();
        assert_eq!(result.remote, "codeberg.org/user/repo");
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.line, Some(10));
    }

    #[test]
    fn codeberg_src_tag() {
        let result =
            parse_remote_url("https://codeberg.org/user/repo/src/tag/v2.0/README.md").unwrap();
        assert_eq!(result.ref_value, Some("v2.0".to_string()));
    }

    // ==================== Azure DevOps Tests ====================

    #[test]
    fn azure_long_form_repo_only() {
        let result = parse_remote_url("https://dev.azure.com/org/project/_git/repo").unwrap();
        assert_eq!(result.remote, "dev.azure.com/org/project/_git/repo");
        assert_eq!(result.repo_name, "repo");
        assert_eq!(result.ref_value, None);
    }

    #[test]
    fn azure_short_form_repo_only() {
        let result = parse_remote_url("https://dev.azure.com/org/_git/repo").unwrap();
        assert_eq!(result.remote, "dev.azure.com/org/_git/repo");
        assert_eq!(result.repo_name, "repo");
    }

    #[test]
    fn azure_with_path_and_branch() {
        let result = parse_remote_url(
            "https://dev.azure.com/org/project/_git/repo?path=/src/index.ts&version=GBmain",
        )
        .unwrap();
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("src/index.ts".to_string()));
    }

    #[test]
    fn azure_with_line() {
        let result = parse_remote_url(
            "https://dev.azure.com/org/project/_git/repo?path=/file.ts&version=GBmain&line=12",
        )
        .unwrap();
        assert_eq!(result.line, Some(12));
    }

    #[test]
    fn azure_version_branch_prefix() {
        let result =
            parse_remote_url("https://dev.azure.com/org/_git/repo?version=GBfeature/my-branch")
                .unwrap();
        assert_eq!(result.ref_value, Some("feature/my-branch".to_string()));
    }

    #[test]
    fn azure_version_tag_prefix() {
        let result =
            parse_remote_url("https://dev.azure.com/org/_git/repo?version=GTv1.0.0").unwrap();
        assert_eq!(result.ref_value, Some("v1.0.0".to_string()));
    }

    #[test]
    fn azure_version_commit_prefix() {
        let result =
            parse_remote_url("https://dev.azure.com/org/_git/repo?version=GCabc123def").unwrap();
        assert_eq!(result.ref_value, Some("abc123def".to_string()));
    }

    #[test]
    fn azure_full_example() {
        let result = parse_remote_url("https://dev.azure.com/org/project/_git/repo?path=/src/components/App.tsx&version=GBmain&line=42").unwrap();
        assert_eq!(result.remote, "dev.azure.com/org/project/_git/repo");
        assert_eq!(result.repo_name, "repo");
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("src/components/App.tsx".to_string()));
        assert_eq!(result.line, Some(42));
    }

    // ==================== Path-Based URL Tests ====================

    #[test]
    fn path_style_github_no_https() {
        let result = parse_remote_url("github.com/owner/repo/blob/main/file.rs").unwrap();
        assert_eq!(result.remote, "github.com/owner/repo");
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("file.rs".to_string()));
    }

    #[test]
    fn path_style_with_colon_line() {
        let result = parse_remote_url("github.com/owner/repo/blob/main/file.rs:42").unwrap();
        assert_eq!(result.line, Some(42));
        assert_eq!(result.file_path, Some("file.rs".to_string()));
    }

    #[test]
    fn path_style_with_leading_slash() {
        let result = parse_remote_url("/github.com/owner/repo/blob/main/file.rs:42").unwrap();
        assert_eq!(result.remote, "github.com/owner/repo");
        assert_eq!(result.line, Some(42));
    }

    // ==================== GitHub.dev Tests ====================

    #[test]
    fn github_dev_blob() {
        let result =
            parse_remote_url("https://github.dev/ericbeland/enhanced_errors/blob/main/Gemfile")
                .unwrap();
        assert_eq!(result.remote, "github.dev/ericbeland/enhanced_errors");
        assert_eq!(result.repo_name, "enhanced_errors");
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("Gemfile".to_string()));
    }

    #[test]
    fn github_dev_with_line() {
        let result =
            parse_remote_url("https://github.dev/owner/repo/blob/main/file.rs#L42").unwrap();
        assert_eq!(result.remote, "github.dev/owner/repo");
        assert_eq!(result.line, Some(42));
    }

    // ==================== GitHub Codespaces Tests ====================

    #[test]
    fn codespaces_new_basic() {
        let result = parse_remote_url("https://codespaces.new/OWNER/REPO").unwrap();
        assert_eq!(result.remote, "github.com/OWNER/REPO");
        assert_eq!(result.repo_name, "REPO");
    }

    #[test]
    fn github_codespaces_simple() {
        let result = parse_remote_url("https://github.com/codespaces/new/owner/repo").unwrap();
        assert_eq!(result.remote, "github.com/owner/repo");
        assert_eq!(result.repo_name, "repo");
    }

    // ==================== GitLab Web IDE Tests ====================

    #[test]
    fn gitlab_web_ide_edit_file() {
        let result =
            parse_remote_url("https://gitlab.com/-/ide/project/paynearme/juno/edit/main/-/Gemfile")
                .unwrap();
        assert_eq!(result.remote, "gitlab.com/paynearme/juno");
        assert_eq!(result.repo_name, "juno");
        assert_eq!(result.ref_value, Some("main".to_string()));
        assert_eq!(result.file_path, Some("Gemfile".to_string()));
    }

    #[test]
    fn gitlab_web_ide_edit_nested_path() {
        let result = parse_remote_url(
            "https://gitlab.com/-/ide/project/group/project/edit/develop/-/src/lib/file.rb",
        )
        .unwrap();
        assert_eq!(result.remote, "gitlab.com/group/project");
        assert_eq!(result.ref_value, Some("develop".to_string()));
        assert_eq!(result.file_path, Some("src/lib/file.rb".to_string()));
    }

    // ==================== Provider Detection ====================

    #[test]
    fn detect_github_by_host() {
        let url = Url::parse("https://github.com/owner/repo").unwrap();
        assert_eq!(detect_provider(&url), Some(Provider::GitHub));
    }

    #[test]
    fn detect_gitlab_by_pattern() {
        let url = Url::parse("https://code.company.io/team/proj/-/blob/main/f.py").unwrap();
        assert_eq!(detect_provider(&url), Some(Provider::GitLab));
    }

    #[test]
    fn detect_bitbucket_by_host() {
        let url = Url::parse("https://bitbucket.org/workspace/repo").unwrap();
        assert_eq!(detect_provider(&url), Some(Provider::Bitbucket));
    }

    #[test]
    fn detect_gitea_by_pattern() {
        let url = Url::parse("https://git.company.io/org/repo/src/branch/main/f.go").unwrap();
        assert_eq!(detect_provider(&url), Some(Provider::Gitea));
    }

    #[test]
    fn detect_codeberg_by_host() {
        let url = Url::parse("https://codeberg.org/user/repo/src/branch/main/f.go").unwrap();
        assert_eq!(detect_provider(&url), Some(Provider::Codeberg));
    }

    #[test]
    fn detect_azure_by_host() {
        let url = Url::parse("https://dev.azure.com/org/project/_git/repo").unwrap();
        assert_eq!(detect_provider(&url), Some(Provider::AzureDevOps));
    }

    // ==================== Error Cases ====================

    #[test]
    fn error_invalid_url() {
        let result = parse_remote_url("not-a-valid-url");
        assert!(result.is_err());
    }

    #[test]
    fn error_unknown_provider() {
        let result = parse_remote_url("https://unknown-host.com/owner/repo");
        assert!(result.is_err());
    }

    #[test]
    fn error_github_no_repo() {
        let result = parse_remote_url("https://github.com/owner");
        assert!(result.is_err());
    }
}
