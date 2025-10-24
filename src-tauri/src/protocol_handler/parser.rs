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
    PartialPath {
        path: String,
        line: Option<usize>,
        column: Option<usize>,
    },
    WorkspacePath {
        workspace: String,
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        remote: Option<String>,
    },
    FullPath {
        full_path: String,
        line: Option<usize>,
        column: Option<usize>,
    },
    RevisionPath {
        workspace: String,
        path: String,
        git_ref: GitRef,
        line: Option<usize>,
        column: Option<usize>,
        remote: Option<String>,
    },
    /// Provider-passthrough URL (e.g., srcuri://github.com/owner/repo/blob/main/file.rs#L42)
    /// First path segment contains a dot, indicating it's a provider hostname
    ProviderPassthrough {
        provider: String,
        repo_name: String,
        /// Entire provider path (provider + repo segments + view path + optional query)
        provider_path: String,
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        git_ref: Option<GitRef>,
        /// Explicit workspace override via ?workspace= (escape hatch for dot-containing names)
        workspace_override: Option<String>,
        /// Fragment string (without leading '#') preserved for browser fallbacks
        fragment: Option<String>,
    },
}

pub struct SrcuriParser;

impl SrcuriParser {
    pub fn parse(link: &str) -> Result<SrcuriRequest> {
        let link = link.trim();

        if !link.starts_with("srcuri://") {
            bail!("Invalid scheme: expected 'srcuri://'");
        }

        let remainder = &link[9..];
        if remainder.is_empty() {
            bail!("Path is empty after 'srcuri://'");
        }

        // Handle fragment (e.g., #L42 for line numbers from provider URLs)
        let (remainder_no_fragment, fragment) = if let Some(hash_pos) = remainder.find('#') {
            (&remainder[..hash_pos], Some(&remainder[hash_pos + 1..]))
        } else {
            (remainder, None)
        };

        let (path_part, query_part) = if let Some(qmark_pos) = remainder_no_fragment.find('?') {
            (
                &remainder_no_fragment[..qmark_pos],
                Some(&remainder_no_fragment[qmark_pos + 1..]),
            )
        } else {
            (remainder_no_fragment, None)
        };

        let git_ref = Self::parse_git_ref_param(query_part);
        let remote = Self::parse_remote_param(query_part);
        let workspace_override = Self::parse_workspace_param(query_part);

        // Check if first segment contains a dot AND has additional path segments
        // This indicates provider-passthrough (e.g., github.com/owner/repo)
        // Single segments like "README.md" are filenames, not providers
        let segments: Vec<&str> = path_part.split('/').filter(|s| !s.is_empty()).collect();
        if segments.len() >= 3 {
            if let Some(first_segment) = segments.first() {
                if first_segment.contains('.') && !first_segment.contains(':') {
                    let provider_input = if let Some(query) = query_part {
                        if query.is_empty() {
                            path_part.to_string()
                        } else {
                            format!("{}?{}", path_part, query)
                        }
                    } else {
                        path_part.to_string()
                    };
                    return Self::parse_provider_passthrough(
                        &provider_input,
                        fragment,
                        git_ref,
                        workspace_override,
                    );
                }
            }
        }

        let (file_path, line, column) = Self::parse_path_with_location(path_part)?;

        if Self::is_absolute_path(&file_path) {
            if git_ref.is_some() {
                bail!("Git reference parameters (commit=, branch=, tag=) require workspace name in path (e.g., srcuri://workspace/path/to/file?commit=abc123)");
            }

            return Ok(SrcuriRequest::FullPath {
                full_path: file_path,
                line,
                column,
            });
        }

        if let Some((workspace, relative_path)) = Self::split_workspace_path(&file_path) {
            if let Some(git_ref) = git_ref {
                return Ok(SrcuriRequest::RevisionPath {
                    workspace,
                    path: relative_path,
                    git_ref,
                    line,
                    column,
                    remote,
                });
            }

            return Ok(SrcuriRequest::WorkspacePath {
                workspace,
                path: relative_path,
                line,
                column,
                remote,
            });
        }

        if git_ref.is_some() {
            bail!("Git reference parameters (commit=, branch=, tag=) require workspace name in path (e.g., srcuri://workspace/path/to/file?commit=abc123)");
        }

        Ok(SrcuriRequest::PartialPath {
            path: file_path,
            line,
            column,
        })
    }

    /// Parse provider-passthrough URL like srcuri://github.com/owner/repo/blob/main/file.rs#L42
    /// Uses srcuri-core for comprehensive provider URL parsing
    fn parse_provider_passthrough(
        path: &str,
        fragment: Option<&str>,
        incoming_git_ref: Option<GitRef>,
        workspace_override: Option<String>,
    ) -> Result<SrcuriRequest> {
        // Build full URL with fragment for srcuri-core parsing
        let full_url = if let Some(frag) = fragment {
            format!("{}#{}", path, frag)
        } else {
            path.to_string()
        };

        // Use srcuri-core for comprehensive provider URL parsing
        let target = srcuri_core::parse_remote_url(&full_url)
            .map_err(|e| anyhow::anyhow!("Failed to parse provider URL: {}", e))?;

        let (fragment_line, fragment_column) = Self::parse_provider_fragment(fragment);

        // Map srcuri-core's ref_value to our GitRef enum, preserving incoming
        let git_ref =
            incoming_git_ref.or_else(|| target.ref_value.map(|value| GitRef::Branch(value)));

        Ok(SrcuriRequest::ProviderPassthrough {
            provider: target.remote,
            repo_name: target.repo_name,
            provider_path: path.to_string(),
            path: target.file_path.unwrap_or_default(),
            line: fragment_line.or_else(|| target.line.map(|l| l as usize)),
            column: fragment_column,
            git_ref,
            workspace_override,
            fragment: fragment.map(|f| f.to_string()),
        })
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

    fn parse_git_ref_param(query_part: Option<&str>) -> Option<GitRef> {
        query_part.and_then(|q| {
            for pair in q.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    match key {
                        "commit" | "sha" => return Some(GitRef::Commit(value.to_string())),
                        "branch" => return Some(GitRef::Branch(value.to_string())),
                        "tag" => return Some(GitRef::Tag(value.to_string())),
                        _ => {}
                    }
                }
            }
            None
        })
    }

    fn parse_remote_param(query_part: Option<&str>) -> Option<String> {
        query_part.and_then(|q| {
            for pair in q.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    if key == "remote" && !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
            None
        })
    }

    /// Parse ?workspace= parameter (escape hatch for dot-containing workspace names)
    fn parse_workspace_param(query_part: Option<&str>) -> Option<String> {
        query_part.and_then(|q| {
            for pair in q.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    if key == "workspace" && !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
            None
        })
    }

    fn parse_path_with_location(path: &str) -> Result<(String, Option<usize>, Option<usize>)> {
        let mut parts: Vec<&str> = path.rsplitn(3, ':').collect();
        parts.reverse();

        match parts.len() {
            1 => {
                // No colons in path
                Ok((path.to_string(), None, None))
            }
            2 => {
                // One colon: file.txt:LINE
                if let Ok(line) = parts[1].parse::<usize>() {
                    Ok((parts[0].to_string(), Some(line), None))
                } else {
                    // Malformed line number - use filename without colon suffix
                    Ok((parts[0].to_string(), None, None))
                }
            }
            3 => {
                // Two colons: file.txt:LINE:COL
                if let (Ok(line), Ok(column)) =
                    (parts[1].parse::<usize>(), parts[2].parse::<usize>())
                {
                    if column <= 120 {
                        Ok((parts[0].to_string(), Some(line), Some(column)))
                    } else {
                        // Column out of range - keep line, drop column
                        Ok((parts[0].to_string(), Some(line), None))
                    }
                } else if let Ok(line) = parts[1].parse::<usize>() {
                    // Valid line, malformed column - keep line, drop column
                    Ok((parts[0].to_string(), Some(line), None))
                } else {
                    // Malformed line - use filename without colon suffix
                    Ok((parts[0].to_string(), None, None))
                }
            }
            _ => {
                // More than 2 colons - use first part as filename, ignore rest
                Ok((parts[0].to_string(), None, None))
            }
        }
    }

    fn is_absolute_path(path: &str) -> bool {
        path.starts_with('/') || (path.len() > 2 && path.chars().nth(1) == Some(':'))
    }

    fn split_workspace_path(path: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_path_simple() {
        let request = SrcuriParser::parse("srcuri://README.md").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "README.md".to_string(),
                line: None,
                column: None,
            }
        );
    }

    #[test]
    fn test_partial_path_with_line() {
        let request = SrcuriParser::parse("srcuri://README.md:25").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "README.md".to_string(),
                line: Some(25),
                column: None,
            }
        );
    }

    #[test]
    fn test_partial_path_with_line_and_column() {
        let request = SrcuriParser::parse("srcuri://README.md:25:10").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "README.md".to_string(),
                line: Some(25),
                column: Some(10),
            }
        );
    }

    #[test]
    fn test_workspace_path_simple() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::WorkspacePath {
                workspace: "myproject".to_string(),
                path: "README.md".to_string(),
                line: None,
                column: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_workspace_path_with_line() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md:25").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::WorkspacePath {
                workspace: "myproject".to_string(),
                path: "README.md".to_string(),
                line: Some(25),
                column: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_workspace_path_nested() {
        let request = SrcuriParser::parse("srcuri://myproject/src/main.rs:42").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::WorkspacePath {
                workspace: "myproject".to_string(),
                path: "src/main.rs".to_string(),
                line: Some(42),
                column: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_workspace_path_with_line_and_column() {
        let request = SrcuriParser::parse("srcuri://myproject/src/main.rs:42:7").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::WorkspacePath {
                workspace: "myproject".to_string(),
                path: "src/main.rs".to_string(),
                line: Some(42),
                column: Some(7),
                remote: None,
            }
        );
    }

    #[test]
    fn test_absolute_path_unix() {
        let request =
            SrcuriParser::parse("srcuri:///Users/ebeland/apps/myproject/README.md:10").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::FullPath {
                full_path: "/Users/ebeland/apps/myproject/README.md".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_absolute_path_server() {
        let request =
            SrcuriParser::parse("srcuri:///devsrv1/deploy/current/myrepo/apps/user.rb:23").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::FullPath {
                full_path: "/devsrv1/deploy/current/myrepo/apps/user.rb".to_string(),
                line: Some(23),
                column: None,
            }
        );
    }

    #[test]
    fn test_absolute_path_with_column() {
        let request = SrcuriParser::parse("srcuri:///Users/ebeland/file.txt:10:5").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::FullPath {
                full_path: "/Users/ebeland/file.txt".to_string(),
                line: Some(10),
                column: Some(5),
            }
        );
    }

    #[test]
    fn test_commit_param() {
        let request =
            SrcuriParser::parse("srcuri://myrepo/src/file.rs:23?commit=abc123def").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RevisionPath {
                workspace: "myrepo".to_string(),
                path: "src/file.rs".to_string(),
                git_ref: GitRef::Commit("abc123def".to_string()),
                line: Some(23),
                column: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_sha_param_alias() {
        let request = SrcuriParser::parse("srcuri://myrepo/src/file.rs:23?sha=abc123def").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RevisionPath {
                workspace: "myrepo".to_string(),
                path: "src/file.rs".to_string(),
                git_ref: GitRef::Commit("abc123def".to_string()),
                line: Some(23),
                column: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_branch_param() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md:1?branch=main").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RevisionPath {
                workspace: "myproject".to_string(),
                path: "README.md".to_string(),
                git_ref: GitRef::Branch("main".to_string()),
                line: Some(1),
                column: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_tag_param() {
        let request = SrcuriParser::parse("srcuri://myrepo/src/file.rs:10?tag=v1.0.0").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RevisionPath {
                workspace: "myrepo".to_string(),
                path: "src/file.rs".to_string(),
                git_ref: GitRef::Tag("v1.0.0".to_string()),
                line: Some(10),
                column: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_commit_without_workspace_fails() {
        let result = SrcuriParser::parse("srcuri://file.rs?commit=abc123");
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_with_absolute_path_fails() {
        let result = SrcuriParser::parse("srcuri:///Users/file.rs?commit=abc123");
        assert!(result.is_err());
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
    fn test_unknown_query_params_ignored() {
        let request = SrcuriParser::parse("srcuri://file.rs?foo=bar&baz=qux").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.rs".to_string(),
                line: None,
                column: None,
            }
        );
    }

    #[test]
    fn test_colon_in_path_without_number() {
        // Security: When suffix after colon is non-numeric, strip it from filename
        let request = SrcuriParser::parse("srcuri://file:name.txt").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file".to_string(),
                line: None,
                column: None,
            }
        );
    }

    #[test]
    fn test_column_over_120_ignored() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:150").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_column_at_boundary_120_accepted() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:120").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: Some(120),
            }
        );
    }

    #[test]
    fn test_column_at_boundary_121_rejected() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:121").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_column_zero_accepted() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:0").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: Some(0),
            }
        );
    }

    #[test]
    fn test_column_one_accepted() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:1").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: Some(1),
            }
        );
    }

    #[test]
    fn test_non_numeric_column_ignored() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:abc").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_non_numeric_line_with_numeric_column_treated_as_filename() {
        // Security: When line is non-numeric, strip the malformed suffix from filename
        let request = SrcuriParser::parse("srcuri://file.txt:abc:10").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: None,
                column: None,
            }
        );
    }

    #[test]
    fn test_negative_column_ignored() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:-5").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_float_column_ignored() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:5.5").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_empty_column_ignored() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_whitespace_column_ignored() {
        let request = SrcuriParser::parse("srcuri://file.txt:10: ").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_very_large_column_ignored() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:999999").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_column_with_leading_zeros() {
        let request = SrcuriParser::parse("srcuri://file.txt:10:005").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file.txt".to_string(),
                line: Some(10),
                column: Some(5),
            }
        );
    }

    #[test]
    fn test_multiple_colons_in_filename() {
        let request = SrcuriParser::parse("srcuri://file:with:colons.txt:10:5").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::PartialPath {
                path: "file:with:colons.txt".to_string(),
                line: Some(10),
                column: Some(5),
            }
        );
    }

    #[test]
    fn test_workspace_with_column_boundary() {
        let request = SrcuriParser::parse("srcuri://workspace/file.txt:10:120").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::WorkspacePath {
                workspace: "workspace".to_string(),
                path: "file.txt".to_string(),
                line: Some(10),
                column: Some(120),
                remote: None,
            }
        );
    }

    #[test]
    fn test_absolute_path_with_malformed_column() {
        let request = SrcuriParser::parse("srcuri:///home/user/file.txt:10:xyz").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::FullPath {
                full_path: "/home/user/file.txt".to_string(),
                line: Some(10),
                column: None,
            }
        );
    }

    #[test]
    fn test_commit_with_column() {
        let request =
            SrcuriParser::parse("srcuri://workspace/file.txt:10:5?commit=abc123").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RevisionPath {
                workspace: "workspace".to_string(),
                path: "file.txt".to_string(),
                git_ref: GitRef::Commit("abc123".to_string()),
                line: Some(10),
                column: Some(5),
                remote: None,
            }
        );
    }

    #[test]
    fn test_commit_with_invalid_column() {
        let request =
            SrcuriParser::parse("srcuri://workspace/file.txt:10:999?commit=abc123").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RevisionPath {
                workspace: "workspace".to_string(),
                path: "file.txt".to_string(),
                git_ref: GitRef::Commit("abc123".to_string()),
                line: Some(10),
                column: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_remote_param() {
        let request = SrcuriParser::parse(
            "srcuri://myproject/src/main.rs:42?remote=github.com/user/myproject",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::WorkspacePath {
                workspace: "myproject".to_string(),
                path: "src/main.rs".to_string(),
                line: Some(42),
                column: None,
                remote: Some("github.com/user/myproject".to_string()),
            }
        );
    }

    #[test]
    fn test_remote_with_branch() {
        let request = SrcuriParser::parse(
            "srcuri://myproject/src/main.rs:42?branch=main&remote=github.com/user/myproject",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::RevisionPath {
                workspace: "myproject".to_string(),
                path: "src/main.rs".to_string(),
                git_ref: GitRef::Branch("main".to_string()),
                line: Some(42),
                column: None,
                remote: Some("github.com/user/myproject".to_string()),
            }
        );
    }

    // Provider-passthrough tests

    #[test]
    fn test_provider_passthrough_github() {
        let request =
            SrcuriParser::parse("srcuri://github.com/owner/repo/blob/main/src/lib.rs#L42").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ProviderPassthrough {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "github.com/owner/repo/blob/main/src/lib.rs".to_string(),
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
    fn test_provider_passthrough_github_no_file() {
        let request = SrcuriParser::parse("srcuri://github.com/owner/repo").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ProviderPassthrough {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "github.com/owner/repo".to_string(),
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
    fn test_provider_passthrough_gitlab() {
        // GitLab: gitlab.com/group/project/-/blob/main/file.py
        let request =
            SrcuriParser::parse("srcuri://gitlab.com/group/project/-/blob/main/file.py#L10")
                .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ProviderPassthrough {
                provider: "gitlab.com/group/project".to_string(),
                repo_name: "project".to_string(),
                provider_path: "gitlab.com/group/project/-/blob/main/file.py".to_string(),
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
    fn test_provider_passthrough_line_range() {
        let request =
            SrcuriParser::parse("srcuri://github.com/owner/repo/blob/main/file.rs#L10-L20")
                .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ProviderPassthrough {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "github.com/owner/repo/blob/main/file.rs".to_string(),
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
    fn test_provider_passthrough_bitbucket_lines() {
        let request =
            SrcuriParser::parse("srcuri://bitbucket.org/workspace/repo/src/main/file.txt#lines-5")
                .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ProviderPassthrough {
                provider: "bitbucket.org/workspace/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "bitbucket.org/workspace/repo/src/main/file.txt".to_string(),
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
    fn test_provider_passthrough_selfhosted_gitlab() {
        // Self-hosted GitLab: gitlab.mycompany.com/team/project/-/blob/main/app.py
        let request = SrcuriParser::parse(
            "srcuri://gitlab.mycompany.com/team/project/-/blob/main/app.py#L15",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ProviderPassthrough {
                provider: "gitlab.mycompany.com/team/project".to_string(),
                repo_name: "project".to_string(),
                provider_path: "gitlab.mycompany.com/team/project/-/blob/main/app.py".to_string(),
                path: "app.py".to_string(),
                line: Some(15),
                column: None,
                git_ref: Some(GitRef::Branch("main".to_string())),
                workspace_override: None,
                fragment: Some("L15".to_string()),
            }
        );
    }

    #[test]
    fn test_provider_passthrough_with_workspace_override() {
        // Escape hatch: ?workspace= allows using a dot-containing workspace name
        // Note: query (?workspace=) must come before fragment (#L42) per URL spec
        let request = SrcuriParser::parse(
            "srcuri://github.com/owner/repo/blob/main/file.rs?workspace=my.custom.workspace#L42",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ProviderPassthrough {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path:
                    "github.com/owner/repo/blob/main/file.rs?workspace=my.custom.workspace"
                        .to_string(),
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
    fn test_provider_passthrough_not_triggered_for_workspace() {
        // Workspace names without dots should NOT trigger provider-passthrough
        let request = SrcuriParser::parse("srcuri://myproject/src/lib.rs:42").unwrap();
        assert_eq!(
            request,
            SrcuriRequest::WorkspacePath {
                workspace: "myproject".to_string(),
                path: "src/lib.rs".to_string(),
                line: Some(42),
                column: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_provider_url_minimum_segments() {
        // With fewer than 3 path segments after a dot-containing first segment,
        // it's not treated as provider-passthrough (would be handled as workspace path).
        // Only provider/owner/repo (3 segments) triggers provider mode.
        let request = SrcuriParser::parse("srcuri://github.com/owner").unwrap();
        // This parses as workspace path since only 2 segments
        assert_eq!(
            request,
            SrcuriRequest::WorkspacePath {
                workspace: "github.com".to_string(),
                path: "owner".to_string(),
                line: None,
                column: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_provider_fragment_with_column() {
        let request =
            SrcuriParser::parse("srcuri://github.com/owner/repo/blob/main/src/lib.rs#L15C9")
                .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ProviderPassthrough {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "github.com/owner/repo/blob/main/src/lib.rs".to_string(),
                path: "src/lib.rs".to_string(),
                line: Some(15),
                column: Some(9),
                git_ref: Some(GitRef::Branch("main".to_string())),
                workspace_override: None,
                fragment: Some("L15C9".to_string()),
            }
        );
    }

    #[test]
    fn test_provider_passthrough_preserves_query() {
        let request = SrcuriParser::parse(
            "srcuri://dev.azure.com/org/project/_git/repo?path=/src/index.ts&version=GBmain",
        )
        .unwrap();
        assert_eq!(
            request,
            SrcuriRequest::ProviderPassthrough {
                provider: "dev.azure.com/org/project/_git/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path:
                    "dev.azure.com/org/project/_git/repo?path=/src/index.ts&version=GBmain"
                        .to_string(),
                path: "src/index.ts".to_string(),
                line: None,
                column: None,
                git_ref: Some(GitRef::Branch("main".to_string())),
                workspace_override: None,
                fragment: None,
            }
        );
    }
}
