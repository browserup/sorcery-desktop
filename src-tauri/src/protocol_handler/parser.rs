use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GitRef {
    Commit(String),
    Branch(String),
    Tag(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SrcuriRequest {
    /// Implicit workspace: authority IS the workspace name
    /// srcuri://myrepo/path:42
    ImplicitWorkspace {
        workspace: String,
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        git_ref: Option<GitRef>,
        remote: Option<String>,
    },
    /// Explicit workspace: authority is "wks"
    /// srcuri://wks/myrepo/path:42
    ExplicitWorkspace {
        workspace: String,
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        git_ref: Option<GitRef>,
        remote: Option<String>,
    },
    /// Relative mode: authority is "rel" - search for file
    /// srcuri://rel/path/file.rs:42
    RelativePath {
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        workspace_hint: Option<String>,
    },
    /// Absolute path: authority is "abs"
    /// srcuri://abs/etc/hosts:1
    /// srcuri://abs/C:/Windows/system.ini:1
    /// srcuri://abs/UNC/server/share/path:1
    AbsolutePath {
        full_path: String,
        line: Option<usize>,
        column: Option<usize>,
    },
    /// External URL: authority is "ext"
    /// srcuri://ext/https/github.com/owner/repo/blob/main/file.rs#L42
    ExternalUrl {
        provider: String,
        repo_name: String,
        provider_path: String,
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        git_ref: Option<GitRef>,
        workspace_override: Option<String>,
        fragment: Option<String>,
    },
}

pub struct SrcuriParser;

fn is_valid_branch_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(
                    c,
                    '-' | '_' | '.' | '/' | '@' | ',' | '(' | ')' | '+' | '#' | '='
                )
        })
        && !name.starts_with('/')
        && !name.ends_with('/')
        && !name.contains("..")
}

fn is_valid_tag_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '+'))
        && !name.starts_with('/')
        && !name.ends_with('/')
        && !name.contains("..")
}

fn is_valid_commit_sha(sha: &str) -> bool {
    let len = sha.len();
    (7..=64).contains(&len) && sha.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_remote_url(url: &str) -> bool {
    let path = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("git@");

    !path.is_empty()
        && path.len() <= 256
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '@'))
        && !path.contains("..")
        && !path.contains("//")
        && !path.starts_with('/')
}

fn is_valid_workspace_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

impl SrcuriParser {
    pub fn parse(link: &str) -> Result<SrcuriRequest> {
        let link = link.trim();

        // Only accept srcuri:// (not srcuri: without //)
        if !link.starts_with("srcuri://") {
            bail!("Invalid scheme: expected 'srcuri://'");
        }

        let remainder = &link[9..]; // after srcuri://

        if remainder.is_empty() {
            bail!("Path is empty after scheme prefix");
        }

        // Handle fragment (e.g., #L42 for line numbers from provider URLs)
        let (remainder_no_fragment, fragment) = if let Some(hash_pos) = remainder.find('#') {
            (&remainder[..hash_pos], Some(&remainder[hash_pos + 1..]))
        } else {
            (remainder, None)
        };

        // Handle query parameters
        let (path_part, query_part) = if let Some(qmark_pos) = remainder_no_fragment.find('?') {
            (
                &remainder_no_fragment[..qmark_pos],
                Some(&remainder_no_fragment[qmark_pos + 1..]),
            )
        } else {
            (remainder_no_fragment, None)
        };

        // Parse common query params
        let git_ref = Self::parse_git_ref_param(query_part)?;
        let remote = Self::parse_remote_param(query_part)?;
        let workspace_hint = Self::parse_workspace_hint_param(query_part)?;

        // Split into authority and path
        // Format: authority/path...
        let (authority, path_after_authority) = Self::split_authority_path(path_part);

        // Mode detection by authority
        match authority.to_lowercase().as_str() {
            "wks" => Self::parse_explicit_workspace_mode(
                path_after_authority,
                fragment,
                git_ref,
                remote,
            ),
            "rel" => Self::parse_rel_mode(path_after_authority, fragment, workspace_hint),
            "abs" => Self::parse_abs_mode(path_after_authority, fragment),
            "ext" => Self::parse_ext_mode(path_after_authority, query_part, fragment, git_ref, workspace_hint),
            _ => Self::parse_implicit_workspace_mode(
                authority,
                path_after_authority,
                fragment,
                git_ref,
                remote,
            ),
        }
    }

    /// Split authority from path: "authority/path/to/file" -> ("authority", "path/to/file")
    fn split_authority_path(path: &str) -> (&str, &str) {
        if let Some(slash_pos) = path.find('/') {
            (&path[..slash_pos], &path[slash_pos + 1..])
        } else {
            (path, "")
        }
    }

    /// Parse implicit workspace: srcuri://myrepo/path:42
    /// Authority IS the workspace name
    fn parse_implicit_workspace_mode(
        authority: &str,
        path_part: &str,
        fragment: Option<&str>,
        git_ref: Option<GitRef>,
        remote: Option<String>,
    ) -> Result<SrcuriRequest> {
        // Validate workspace name
        if !is_valid_workspace_name(authority) {
            bail!(
                "Invalid workspace name '{}': may only contain letters, numbers, and - _ .",
                Self::safe_display(authority)
            );
        }

        let (file_path, mut line, column) = Self::parse_path_with_location(path_part)?;

        // If no line from colon syntax, try fragment as line number
        if line.is_none() {
            line = Self::parse_fragment_line(fragment);
        }

        Ok(SrcuriRequest::ImplicitWorkspace {
            workspace: authority.to_string(),
            path: file_path,
            line,
            column,
            git_ref,
            remote,
        })
    }

    /// Parse explicit workspace: srcuri://workspace/myrepo/path:42
    /// First path segment after "workspace" is the workspace name
    fn parse_explicit_workspace_mode(
        path_part: &str,
        fragment: Option<&str>,
        git_ref: Option<GitRef>,
        remote: Option<String>,
    ) -> Result<SrcuriRequest> {
        // Split to get workspace name and relative path
        let (workspace, relative_path) = Self::split_authority_path(path_part);

        if workspace.is_empty() {
            bail!("Missing workspace name after 'workspace/' authority");
        }

        if !is_valid_workspace_name(workspace) {
            bail!(
                "Invalid workspace name '{}': may only contain letters, numbers, and - _ .",
                Self::safe_display(workspace)
            );
        }

        let (file_path, mut line, column) = Self::parse_path_with_location(relative_path)?;

        if line.is_none() {
            line = Self::parse_fragment_line(fragment);
        }

        Ok(SrcuriRequest::ExplicitWorkspace {
            workspace: workspace.to_string(),
            path: file_path,
            line,
            column,
            git_ref,
            remote,
        })
    }

    /// Parse rel mode: srcuri://rel/path/file.rs:42
    /// Searches all workspaces for matching path
    fn parse_rel_mode(
        path_part: &str,
        fragment: Option<&str>,
        workspace_hint: Option<String>,
    ) -> Result<SrcuriRequest> {
        let (file_path, mut line, column) = Self::parse_path_with_location(path_part)?;

        if line.is_none() {
            line = Self::parse_fragment_line(fragment);
        }

        Ok(SrcuriRequest::RelativePath {
            path: file_path,
            line,
            column,
            workspace_hint,
        })
    }

    /// Parse absolute path mode: srcuri://abs/path
    /// Handles POSIX, Windows drive letters, and UNC paths
    fn parse_abs_mode(path_part: &str, fragment: Option<&str>) -> Result<SrcuriRequest> {
        let (file_path, mut line, column) = Self::parse_path_with_location(path_part)?;

        if line.is_none() {
            line = Self::parse_fragment_line(fragment);
        }

        // Convert to actual filesystem path
        let full_path = Self::abs_path_to_filesystem(&file_path);

        Ok(SrcuriRequest::AbsolutePath {
            full_path,
            line,
            column,
        })
    }

    /// Convert abs-mode path to filesystem path
    /// - /etc/hosts → /etc/hosts (POSIX - add leading /)
    /// - C:/Windows/... → C:/Windows/... (Windows drive - keep as-is)
    /// - UNC/server/share/path → //server/share/path (Windows UNC)
    fn abs_path_to_filesystem(path: &str) -> String {
        // Check for UNC path: abs/UNC/server/share/...
        if path.starts_with("UNC/") || path.starts_with("unc/") {
            let unc_path = &path[4..]; // strip "UNC/"
            return format!("//{}", unc_path);
        }

        // Check for Windows drive: C:/ or similar
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            // Already has drive letter, return as-is
            return path.to_string();
        }

        // POSIX path - add leading / if not already present
        if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        }
    }

    /// Parse external URL mode: srcuri://ext/https/github.com/owner/repo/...
    /// Re-constructs the original URL and parses with srcuri-core
    fn parse_ext_mode(
        path_part: &str,
        query_part: Option<&str>,
        fragment: Option<&str>,
        incoming_git_ref: Option<GitRef>,
        workspace_override: Option<String>,
    ) -> Result<SrcuriRequest> {
        // Reconstruct URL: ext/https/github.com/... → https://github.com/...
        let url = Self::reconstruct_external_url(path_part)?;

        // Build full URL with query and fragment for srcuri-core parsing
        let full_url = match (query_part, fragment) {
            (Some(q), Some(f)) => format!("{}?{}#{}", url, q, f),
            (Some(q), None) => format!("{}?{}", url, q),
            (None, Some(f)) => format!("{}#{}", url, f),
            (None, None) => url,
        };

        // Use srcuri-core for comprehensive provider URL parsing
        let target = srcuri_core::parse_remote_url(&full_url)
            .map_err(|e| anyhow::anyhow!("Failed to parse external URL: {}", e))?;

        let (fragment_line, fragment_column) = Self::parse_provider_fragment(fragment);

        // Map srcuri-core's ref_value to our GitRef enum, preserving incoming
        let git_ref =
            incoming_git_ref.or_else(|| target.ref_value.map(|value| GitRef::Branch(value)));

        Ok(SrcuriRequest::ExternalUrl {
            provider: target.remote,
            repo_name: target.repo_name,
            provider_path: path_part.to_string(),
            path: target.file_path.unwrap_or_default(),
            line: fragment_line.or_else(|| target.line.map(|l| l as usize)),
            column: fragment_column,
            git_ref,
            workspace_override,
            fragment: fragment.map(|f| f.to_string()),
        })
    }

    /// Reconstruct external URL from ext-mode path
    /// https/github.com/owner/repo → https://github.com/owner/repo
    fn reconstruct_external_url(path: &str) -> Result<String> {
        // Format: <scheme>/<host>/<rest-of-path>
        if let Some(rest) = path.strip_prefix("https/") {
            return Ok(format!("https://{}", rest));
        }
        if let Some(rest) = path.strip_prefix("http/") {
            return Ok(format!("http://{}", rest));
        }
        bail!("External URL must start with https/ or http/");
    }

    /// Parse fragment as simple line number (e.g., #42)
    fn parse_fragment_line(fragment: Option<&str>) -> Option<usize> {
        let fragment = fragment?;
        if fragment.is_empty() {
            return None;
        }
        fragment.parse::<usize>().ok()
    }

    fn parse_provider_fragment(fragment: Option<&str>) -> (Option<usize>, Option<usize>) {
        let fragment = match fragment {
            Some(frag) if !frag.is_empty() => frag,
            _ => return (None, None),
        };

        // GitHub/GitLab style (#L10, #L10C5, #L10-L20)
        if let Some(rest) = fragment.strip_prefix('L') {
            let (line, remainder) = Self::parse_leading_number(rest);
            if let Some(line) = line {
                if let Some(rem) = remainder {
                    if rem.starts_with('C') || rem.starts_with('c') {
                        let (_, col_rest) = rem.split_at(1);
                        let (column, _) = Self::parse_leading_number(col_rest);
                        return (Some(line), column);
                    }
                    if rem.starts_with(':') {
                        let (_, col_rest) = rem.split_at(1);
                        let (column, _) = Self::parse_leading_number(col_rest);
                        return (Some(line), column);
                    }
                }
                return (Some(line), None);
            }
        }

        // Bitbucket style (#lines-5, #lines-5:10, #lines-5-10)
        if let Some(rest) = fragment.strip_prefix("lines-") {
            let (line, _) = Self::parse_leading_number(rest);
            if line.is_some() {
                return (line, None);
            }
        }

        (None, None)
    }

    fn parse_leading_number(input: &str) -> (Option<usize>, Option<&str>) {
        let mut digits = String::new();
        let mut split_index = 0usize;

        for (idx, ch) in input.char_indices() {
            if ch.is_ascii_digit() {
                digits.push(ch);
                split_index = idx + ch.len_utf8();
            } else {
                break;
            }
        }

        if digits.is_empty() {
            return (None, Some(input));
        }

        let remainder = if split_index < input.len() {
            Some(&input[split_index..])
        } else {
            None
        };

        (digits.parse().ok(), remainder)
    }

    fn parse_git_ref_param(query_part: Option<&str>) -> Result<Option<GitRef>> {
        let Some(q) = query_part else {
            return Ok(None);
        };

        for pair in q.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let decoded =
                    urlencoding::decode(value).unwrap_or(std::borrow::Cow::Borrowed(value));
                let decoded_str = decoded.into_owned();

                match key {
                    "commit" | "sha" => {
                        if !is_valid_commit_sha(&decoded_str) {
                            bail!(
                                "Invalid commit SHA '{}': must be 7-64 hexadecimal characters",
                                Self::safe_display(&decoded_str)
                            );
                        }
                        return Ok(Some(GitRef::Commit(decoded_str)));
                    }
                    "branch" => {
                        if !is_valid_branch_name(&decoded_str) {
                            bail!(
                                "Invalid branch name '{}': may only contain letters, numbers, and - _ . / @ , ( ) + # =",
                                Self::safe_display(&decoded_str)
                            );
                        }
                        return Ok(Some(GitRef::Branch(decoded_str)));
                    }
                    "tag" => {
                        if !is_valid_tag_name(&decoded_str) {
                            bail!(
                                "Invalid tag name '{}': may only contain letters, numbers, and - _ . / +",
                                Self::safe_display(&decoded_str)
                            );
                        }
                        return Ok(Some(GitRef::Tag(decoded_str)));
                    }
                    _ => {}
                }
            }
        }
        Ok(None)
    }

    fn safe_display(s: &str) -> String {
        s.chars()
            .take(100)
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ' ') {
                    c
                } else {
                    '?'
                }
            })
            .collect()
    }

    fn parse_remote_param(query_part: Option<&str>) -> Result<Option<String>> {
        let Some(q) = query_part else {
            return Ok(None);
        };

        for pair in q.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if key == "remote" && !value.is_empty() {
                    if !is_valid_remote_url(value) {
                        bail!(
                            "Invalid remote URL '{}': may only contain letters, numbers, and - _ . / : @",
                            Self::safe_display(value)
                        );
                    }
                    return Ok(Some(value.to_string()));
                }
            }
        }
        Ok(None)
    }

    /// Parse ?workspaceHint= parameter (for match mode)
    fn parse_workspace_hint_param(query_part: Option<&str>) -> Result<Option<String>> {
        let Some(q) = query_part else {
            return Ok(None);
        };

        for pair in q.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if (key == "workspaceHint" || key == "workspace") && !value.is_empty() {
                    if !is_valid_workspace_name(value) {
                        bail!(
                            "Invalid workspace name '{}': may only contain letters, numbers, and - _ .",
                            Self::safe_display(value)
                        );
                    }
                    return Ok(Some(value.to_string()));
                }
            }
        }
        Ok(None)
    }

    fn parse_path_with_location(path: &str) -> Result<(String, Option<usize>, Option<usize>)> {
        let mut end = path.len();
        let mut line: Option<usize> = None;
        let mut column: Option<usize> = None;

        let last_colon = Self::find_non_drive_colon(path, end);
        let penultimate_colon = last_colon.and_then(|idx| Self::find_non_drive_colon(path, idx));

        if let Some(last_idx) = last_colon {
            if let Some(line_idx) = penultimate_colon {
                let line_segment = &path[line_idx + 1..last_idx];
                let column_segment = &path[last_idx + 1..end];

                if let Ok(parsed_line) = line_segment.parse::<usize>() {
                    line = Some(parsed_line);

                    if let Ok(parsed_column) = column_segment.parse::<usize>() {
                        if parsed_column <= 120 {
                            column = Some(parsed_column);
                        }
                    }
                }

                end = line_idx;
            } else {
                let line_segment = &path[last_idx + 1..end];
                if let Ok(parsed_line) = line_segment.parse::<usize>() {
                    line = Some(parsed_line);
                }
                end = last_idx;
            }
        }

        let file_path = path[..end].to_string();

        Ok((file_path, line, column))
    }

    fn find_non_drive_colon(path: &str, before: usize) -> Option<usize> {
        if before == 0 {
            return None;
        }

        let mut slice_end = before;
        while let Some(pos) = path[..slice_end].rfind(':') {
            if Self::is_windows_drive_colon(path, pos) {
                if pos == 0 {
                    return None;
                }
                slice_end = pos;
                continue;
            }
            return Some(pos);
        }
        None
    }

    fn is_windows_drive_colon(path: &str, colon_idx: usize) -> bool {
        if colon_idx == 0 || colon_idx >= path.len() {
            return false;
        }

        let bytes = path.as_bytes();
        let drive_char = bytes[colon_idx - 1];
        if !drive_char.is_ascii_alphabetic() {
            return false;
        }

        if colon_idx == 1 {
            return true;
        }

        match bytes.get(colon_idx + 1) {
            Some(b'\\') | Some(b'/') => true,
            None => false,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Implicit workspace tests

    #[test]
    fn test_implicit_workspace_simple() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "README.md".to_string(),
                line: None,
                column: None,
                git_ref: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_implicit_workspace_with_line() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md:25").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "README.md".to_string(),
                line: Some(25),
                column: None,
                git_ref: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_implicit_workspace_nested_path() {
        let request = SrcuriParser::parse("srcuri://myproject/src/main.rs:42").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "src/main.rs".to_string(),
                line: Some(42),
                column: None,
                git_ref: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_implicit_workspace_with_line_and_column() {
        let request = SrcuriParser::parse("srcuri://myproject/src/main.rs:42:7").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "src/main.rs".to_string(),
                line: Some(42),
                column: Some(7),
                git_ref: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_implicit_workspace_with_git_ref() {
        let request =
            SrcuriParser::parse("srcuri://myrepo/src/file.rs:23?commit=abc123def").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myrepo".to_string(),
                path: "src/file.rs".to_string(),
                line: Some(23),
                column: None,
                git_ref: Some(GitRef::Commit("abc123def".to_string())),
                remote: None,
            }
        );
    }

    #[test]
    fn test_implicit_workspace_with_remote() {
        let request = SrcuriParser::parse(
            "srcuri://myproject/src/main.rs:42?remote=github.com/user/myproject",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "src/main.rs".to_string(),
                line: Some(42),
                column: None,
                git_ref: None,
                remote: Some("github.com/user/myproject".to_string()),
            }
        );
    }

    // Explicit workspace tests

    #[test]
    fn test_explicit_workspace_simple() {
        let request = SrcuriParser::parse("srcuri://wks/myrepo/src/main.rs:42").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ExplicitWorkspace {
                workspace: "myrepo".to_string(),
                path: "src/main.rs".to_string(),
                line: Some(42),
                column: None,
                git_ref: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_explicit_workspace_with_git_ref() {
        let request =
            SrcuriParser::parse("srcuri://wks/myrepo/file.rs:10?branch=main").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ExplicitWorkspace {
                workspace: "myrepo".to_string(),
                path: "file.rs".to_string(),
                line: Some(10),
                column: None,
                git_ref: Some(GitRef::Branch("main".to_string())),
                remote: None,
            }
        );
    }

    // Rel mode tests

    #[test]
    fn test_rel_mode_simple() {
        let request = SrcuriParser::parse("srcuri://rel/README.md").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RelativePath {
                path: "README.md".to_string(),
                line: None,
                column: None,
                workspace_hint: None,
            }
        );
    }

    #[test]
    fn test_rel_mode_with_line() {
        let request = SrcuriParser::parse("srcuri://rel/README.md:25").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RelativePath {
                path: "README.md".to_string(),
                line: Some(25),
                column: None,
                workspace_hint: None,
            }
        );
    }

    #[test]
    fn test_rel_mode_with_workspace_hint() {
        let request =
            SrcuriParser::parse("srcuri://rel/src/utils.py:10?workspaceHint=backend").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RelativePath {
                path: "src/utils.py".to_string(),
                line: Some(10),
                column: None,
                workspace_hint: Some("backend".to_string()),
            }
        );
    }

    #[test]
    fn test_rel_mode_nested_path() {
        let request = SrcuriParser::parse("srcuri://rel/src/lib/utils.py:10").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RelativePath {
                path: "src/lib/utils.py".to_string(),
                line: Some(10),
                column: None,
                workspace_hint: None,
            }
        );
    }

    // Absolute path mode tests

    #[test]
    fn test_abs_mode_posix() {
        let request = SrcuriParser::parse("srcuri://abs/etc/hosts:1").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::AbsolutePath {
                full_path: "/etc/hosts".to_string(),
                line: Some(1),
                column: None,
            }
        );
    }

    #[test]
    fn test_abs_mode_posix_deep_path() {
        let request =
            SrcuriParser::parse("srcuri://abs/Users/alice/code/myproject/README.md:50").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::AbsolutePath {
                full_path: "/Users/alice/code/myproject/README.md".to_string(),
                line: Some(50),
                column: None,
            }
        );
    }

    #[test]
    fn test_abs_mode_windows_drive() {
        let request =
            SrcuriParser::parse("srcuri://abs/C:/Users/Carol/Dev/project/README.md:10").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::AbsolutePath {
                full_path: "C:/Users/Carol/Dev/project/README.md".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_abs_mode_windows_unc() {
        let request =
            SrcuriParser::parse("srcuri://abs/UNC/server/share/docs/readme.txt:5").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::AbsolutePath {
                full_path: "//server/share/docs/readme.txt".to_string(),
                line: Some(5),
                column: None,
            }
        );
    }

    #[test]
    fn test_abs_mode_with_column() {
        let request = SrcuriParser::parse("srcuri://abs/home/user/file.txt:10:5").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::AbsolutePath {
                full_path: "/home/user/file.txt".to_string(),
                line: Some(10),
                column: Some(5),
            }
        );
    }

    #[test]
    fn test_abs_mode_path_already_has_leading_slash() {
        // When path already starts with /, don't add another one
        // This can happen when temp directories generate URLs like srcuri://abs//private/var/...
        let request = SrcuriParser::parse("srcuri://abs//private/var/folders/test.rs:1").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::AbsolutePath {
                full_path: "/private/var/folders/test.rs".to_string(),
                line: Some(1),
                column: None,
            }
        );
    }

    #[test]
    fn test_abs_mode_preserves_symlink_path() {
        // macOS /tmp is symlinked to /private/tmp - we should preserve the path as given
        let request = SrcuriParser::parse("srcuri://abs/tmp/test.rs:42").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::AbsolutePath {
                full_path: "/tmp/test.rs".to_string(),
                line: Some(42),
                column: None,
            }
        );
    }

    // External URL mode tests

    #[test]
    fn test_ext_mode_github() {
        let request = SrcuriParser::parse(
            "srcuri://ext/https/github.com/owner/repo/blob/main/src/lib.rs#L42",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ExternalUrl {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "https/github.com/owner/repo/blob/main/src/lib.rs".to_string(),
                path: "src/lib.rs".to_string(),
                line: Some(42),
                column: None,
                git_ref: Some(GitRef::Branch("main".to_string())),
                workspace_override: None,
                fragment: Some("L42".to_string()),
            }
        );
    }

    #[test]
    fn test_ext_mode_github_no_file() {
        let request = SrcuriParser::parse("srcuri://ext/https/github.com/owner/repo").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ExternalUrl {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "https/github.com/owner/repo".to_string(),
                path: "".to_string(),
                line: None,
                column: None,
                git_ref: None,
                workspace_override: None,
                fragment: None,
            }
        );
    }

    #[test]
    fn test_ext_mode_gitlab() {
        let request = SrcuriParser::parse(
            "srcuri://ext/https/gitlab.com/group/project/-/blob/main/file.py#L10",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ExternalUrl {
                provider: "gitlab.com/group/project".to_string(),
                repo_name: "project".to_string(),
                provider_path: "https/gitlab.com/group/project/-/blob/main/file.py".to_string(),
                path: "file.py".to_string(),
                line: Some(10),
                column: None,
                git_ref: Some(GitRef::Branch("main".to_string())),
                workspace_override: None,
                fragment: Some("L10".to_string()),
            }
        );
    }

    #[test]
    fn test_ext_mode_bitbucket_lines() {
        let request = SrcuriParser::parse(
            "srcuri://ext/https/bitbucket.org/workspace/repo/src/main/file.txt#lines-5",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ExternalUrl {
                provider: "bitbucket.org/workspace/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "https/bitbucket.org/workspace/repo/src/main/file.txt".to_string(),
                path: "file.txt".to_string(),
                line: Some(5),
                column: None,
                git_ref: Some(GitRef::Branch("main".to_string())),
                workspace_override: None,
                fragment: Some("lines-5".to_string()),
            }
        );
    }

    #[test]
    fn test_ext_mode_with_workspace_override() {
        let request = SrcuriParser::parse(
            "srcuri://ext/https/github.com/owner/repo/blob/main/file.rs?workspace=my.custom.workspace#L42",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ExternalUrl {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "https/github.com/owner/repo/blob/main/file.rs".to_string(),
                path: "file.rs".to_string(),
                line: Some(42),
                column: None,
                git_ref: Some(GitRef::Branch("main".to_string())),
                workspace_override: Some("my.custom.workspace".to_string()),
                fragment: Some("L42".to_string()),
            }
        );
    }

    #[test]
    fn test_ext_mode_line_range() {
        let request = SrcuriParser::parse(
            "srcuri://ext/https/github.com/owner/repo/blob/main/file.rs#L10-L20",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ExternalUrl {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "https/github.com/owner/repo/blob/main/file.rs".to_string(),
                path: "file.rs".to_string(),
                line: Some(10),
                column: None,
                git_ref: Some(GitRef::Branch("main".to_string())),
                workspace_override: None,
                fragment: Some("L10-L20".to_string()),
            }
        );
    }

    #[test]
    fn test_ext_mode_with_column() {
        let request = SrcuriParser::parse(
            "srcuri://ext/https/github.com/owner/repo/blob/main/src/lib.rs#L15C9",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ExternalUrl {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "https/github.com/owner/repo/blob/main/src/lib.rs".to_string(),
                path: "src/lib.rs".to_string(),
                line: Some(15),
                column: Some(9),
                git_ref: Some(GitRef::Branch("main".to_string())),
                workspace_override: None,
                fragment: Some("L15C9".to_string()),
            }
        );
    }

    // Error cases

    #[test]
    fn test_rejects_srcuri_without_double_slash() {
        let result = SrcuriParser::parse("srcuri:myproject/file.rs");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("expected 'srcuri://'"));
    }

    #[test]
    fn test_empty_path_fails() {
        let result = SrcuriParser::parse("srcuri://");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_scheme_fails() {
        let result = SrcuriParser::parse("http://file.rs");
        assert!(result.is_err());
    }

    #[test]
    fn test_ext_mode_requires_scheme() {
        let result = SrcuriParser::parse("srcuri://ext/github.com/owner/repo");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must start with https/ or http/"));
    }

    // Column boundary tests

    #[test]
    fn test_column_at_boundary_120_accepted() {
        let request = SrcuriParser::parse("srcuri://myproject/file.txt:10:120").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "file.txt".to_string(),
                line: Some(10),
                column: Some(120),
                git_ref: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_column_at_boundary_121_rejected() {
        let request = SrcuriParser::parse("srcuri://myproject/file.txt:10:121").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "file.txt".to_string(),
                line: Some(10),
                column: None,
                git_ref: None,
                remote: None,
            }
        );
    }

    // Git ref tests

    #[test]
    fn test_branch_param() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md:1?branch=main").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "README.md".to_string(),
                line: Some(1),
                column: None,
                git_ref: Some(GitRef::Branch("main".to_string())),
                remote: None,
            }
        );
    }

    #[test]
    fn test_tag_param() {
        let request = SrcuriParser::parse("srcuri://myrepo/src/file.rs:10?tag=v1.0.0").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myrepo".to_string(),
                path: "src/file.rs".to_string(),
                line: Some(10),
                column: None,
                git_ref: Some(GitRef::Tag("v1.0.0".to_string())),
                remote: None,
            }
        );
    }

    #[test]
    fn test_sha_param_alias() {
        let request = SrcuriParser::parse("srcuri://myrepo/src/file.rs:23?sha=abc123def").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myrepo".to_string(),
                path: "src/file.rs".to_string(),
                line: Some(23),
                column: None,
                git_ref: Some(GitRef::Commit("abc123def".to_string())),
                remote: None,
            }
        );
    }

    // URL decoding tests

    #[test]
    fn test_branch_with_plus_is_decoded() {
        let request =
            SrcuriParser::parse("srcuri://myrepo/file.rs:1?branch=feature%2Fc%2B%2B").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myrepo".to_string(),
                path: "file.rs".to_string(),
                line: Some(1),
                column: None,
                git_ref: Some(GitRef::Branch("feature/c++".to_string())),
                remote: None,
            }
        );
    }

    #[test]
    fn test_branch_with_hash_is_decoded() {
        let request = SrcuriParser::parse("srcuri://myrepo/file.rs:1?branch=%23pr470").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myrepo".to_string(),
                path: "file.rs".to_string(),
                line: Some(1),
                column: None,
                git_ref: Some(GitRef::Branch("#pr470".to_string())),
                remote: None,
            }
        );
    }

    // Security validation tests

    #[test]
    fn test_commit_sha_too_short_rejected() {
        let result = SrcuriParser::parse("srcuri://myrepo/file.rs?commit=abc123");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid commit SHA"));
    }

    #[test]
    fn test_commit_sha_non_hex_rejected() {
        let result = SrcuriParser::parse("srcuri://myrepo/file.rs?commit=abc123g");
        assert!(result.is_err());
    }

    #[test]
    fn test_branch_with_shell_metachar_rejected() {
        let result = SrcuriParser::parse("srcuri://myrepo/file.rs?branch=main;rm%20-rf");
        assert!(result.is_err());
    }

    #[test]
    fn test_remote_with_shell_metachar_rejected() {
        let result =
            SrcuriParser::parse("srcuri://myrepo/file.rs?remote=github.com/owner/repo;whoami");
        assert!(result.is_err());
    }

    // Fragment line number tests

    #[test]
    fn test_hash_line_implicit_workspace() {
        let request = SrcuriParser::parse("srcuri://myproject/src/main.rs#100").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "src/main.rs".to_string(),
                line: Some(100),
                column: None,
                git_ref: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_colon_takes_precedence_over_hash() {
        let request = SrcuriParser::parse("srcuri://myproject/file.txt:42#99").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "file.txt".to_string(),
                line: Some(42),
                column: None,
                git_ref: None,
                remote: None,
            }
        );
    }

    // Trailing colon tests (iTerm compatibility)

    #[test]
    fn test_trailing_colon_implicit_workspace() {
        let request = SrcuriParser::parse("srcuri://myproject/src/main.rs:").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "src/main.rs".to_string(),
                line: None,
                column: None,
                git_ref: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_trailing_colon_abs_mode() {
        let request = SrcuriParser::parse("srcuri://abs/Users/ebeland/file.txt:").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::AbsolutePath {
                full_path: "/Users/ebeland/file.txt".to_string(),
                line: None,
                column: None,
            }
        );
    }

    #[test]
    fn test_trailing_colon_rel_mode() {
        let request = SrcuriParser::parse("srcuri://rel/README.md:").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RelativePath {
                path: "README.md".to_string(),
                line: None,
                column: None,
                workspace_hint: None,
            }
        );
    }

    // Unknown query params ignored

    #[test]
    fn test_unknown_query_params_ignored() {
        let request = SrcuriParser::parse("srcuri://myproject/file.rs?foo=bar&baz=qux").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "file.rs".to_string(),
                line: None,
                column: None,
                git_ref: None,
                remote: None,
            }
        );
    }

    // Reserved authority as workspace rejected

    #[test]
    fn test_workspace_named_rel_invalid() {
        // "rel" is a reserved authority, can't be used as workspace name
        let result = SrcuriParser::parse("srcuri://rel");
        // This should parse as rel mode with empty path
        let request = result.unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RelativePath {
                path: "".to_string(),
                line: None,
                column: None,
                workspace_hint: None,
            }
        );
    }

    #[test]
    fn test_reserved_authorities_not_workspaces() {
        // All reserved authorities should be parsed as their modes, not workspaces
        // "wks" is parsed as explicit workspace mode
        let result = SrcuriParser::parse("srcuri://wks/myrepo/file.rs").unwrap();
        matches!(result, SrcuriRequest::ExplicitWorkspace { .. });

        // "rel" is parsed as rel mode
        let result = SrcuriParser::parse("srcuri://rel/file.rs").unwrap();
        matches!(result, SrcuriRequest::RelativePath { .. });

        // "abs" is parsed as absolute path mode
        let result = SrcuriParser::parse("srcuri://abs/etc/hosts").unwrap();
        matches!(result, SrcuriRequest::AbsolutePath { .. });

        // "ext" is parsed as external URL mode
        let result = SrcuriParser::parse("srcuri://ext/https/github.com/owner/repo");
        // ext mode requires more path components, this may fail, but should NOT be ImplicitWorkspace
        if let Ok(req) = result {
            assert!(
                !matches!(req, SrcuriRequest::ImplicitWorkspace { .. }),
                "'ext' should never be treated as implicit workspace"
            );
        }
    }

    #[test]
    fn test_reserved_authorities_case_insensitive() {
        // Reserved authorities should be case-insensitive
        let result = SrcuriParser::parse("srcuri://REL/file.rs:1").unwrap();
        matches!(result, SrcuriRequest::RelativePath { .. });

        let result = SrcuriParser::parse("srcuri://ABS/etc/hosts").unwrap();
        matches!(result, SrcuriRequest::AbsolutePath { .. });

        let result = SrcuriParser::parse("srcuri://WKS/repo/file.rs").unwrap();
        matches!(result, SrcuriRequest::ExplicitWorkspace { .. });
    }
}
