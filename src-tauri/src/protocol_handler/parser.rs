use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GitRef {
    Commit(String),
    Branch(String),
    Tag(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SrcuriRequest {
    /// Implicit workspace: authority IS the workspace name.
    /// Example: `srcuri://myrepo/path@L42`
    ImplicitWorkspace {
        workspace: String,
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        git_ref: Option<GitRef>,
        remote: Option<String>,
    },
    /// Explicit workspace: authority is "wks".
    /// Example: `srcuri://wks/myrepo/path@L42`
    ExplicitWorkspace {
        workspace: String,
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        git_ref: Option<GitRef>,
        remote: Option<String>,
    },
    /// Relative mode: authority is "rel" - search for file.
    /// Example: `srcuri://rel/path/file.rs@L42`
    RelativePath {
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        workspace_hint: Option<String>,
    },
    /// Any mode: authority is "any" - best-effort resolution.
    /// Example: `srcuri://any/path/file.rs@L42`
    AnyPath {
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        workspace_hint: Option<String>,
    },
    /// Absolute path: authority is "abs".
    /// Examples: `srcuri://abs/etc/hosts@L1`, `srcuri://abs/C:/Windows/system.ini@L1`
    AbsolutePath {
        full_path: String,
        line: Option<usize>,
        column: Option<usize>,
    },
    /// External URL: authority is "ext".
    /// Example: `srcuri://ext/https/github.com/owner/repo/blob/main/file.rs#L42`
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
    /// Ping request: used by browser extension to check if Desktop is installed.
    /// Example: `srcuri://ping`
    Ping,
    /// Hello request: sent by extension on install to register its version.
    /// Example: `srcuri://hello?version=1.0.0`
    Hello { version: Option<String> },
}

pub struct SrcuriParser;

#[derive(Debug, Error)]
pub enum SrcuriParseError {
    #[error("Invalid scheme: expected 'srcuri://'")]
    InvalidScheme,
    #[error("Path is empty after scheme prefix")]
    EmptyPath,
    #[error("Missing workspace name after 'workspace/' authority")]
    MissingWorkspaceName,
    #[error("Invalid workspace name '{0}': may only contain letters, numbers, and - _ .")]
    InvalidWorkspaceName(String),
    #[error("Invalid commit SHA '{0}': must be 7-64 hexadecimal characters")]
    InvalidCommitSha(String),
    #[error(
        "Invalid branch name '{0}': may only contain letters, numbers, and - _ . / @ , ( ) + # ="
    )]
    InvalidBranchName(String),
    #[error("Invalid tag name '{0}': may only contain letters, numbers, and - _ . / +")]
    InvalidTagName(String),
    #[error("Invalid remote URL '{0}': may only contain letters, numbers, and - _ . / : @")]
    InvalidRemoteUrl(String),
    #[error("External URL must start with https/ or http/")]
    InvalidExternalUrlScheme,
    #[error("Failed to parse external URL")]
    ExternalUrlParse {
        #[source]
        source: srcuri_core::ParseError,
    },
}

type Result<T> = std::result::Result<T, SrcuriParseError>;

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
            return Err(SrcuriParseError::InvalidScheme);
        }

        let remainder = &link[9..]; // after srcuri://

        if remainder.is_empty() {
            return Err(SrcuriParseError::EmptyPath);
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
            "ping" => Ok(SrcuriRequest::Ping),
            "hello" => {
                let version = Self::parse_version_param(query_part);
                Ok(SrcuriRequest::Hello { version })
            }
            "wks" => {
                Self::parse_explicit_workspace_mode(path_after_authority, fragment, git_ref, remote)
            }
            "rel" => Self::parse_rel_mode(path_after_authority, fragment, workspace_hint),
            "any" => Self::parse_any_mode(path_after_authority, fragment, workspace_hint),
            "abs" => Self::parse_abs_mode(path_after_authority, fragment),
            "ext" => Self::parse_ext_mode(
                path_after_authority,
                query_part,
                fragment,
                git_ref,
                workspace_hint,
            ),
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

    /// Parse implicit workspace (e.g., `srcuri://myrepo/path@L42`).
    /// Authority IS the workspace name.
    fn parse_implicit_workspace_mode(
        authority: &str,
        path_part: &str,
        fragment: Option<&str>,
        git_ref: Option<GitRef>,
        remote: Option<String>,
    ) -> Result<SrcuriRequest> {
        // Validate workspace name
        if !is_valid_workspace_name(authority) {
            return Err(SrcuriParseError::InvalidWorkspaceName(Self::safe_display(
                authority,
            )));
        }

        let (file_path, mut line, column) = Self::parse_path_with_location(path_part);

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

    /// Parse explicit workspace (e.g., `srcuri://workspace/myrepo/path@L42`).
    /// First path segment after "workspace" is the workspace name.
    fn parse_explicit_workspace_mode(
        path_part: &str,
        fragment: Option<&str>,
        git_ref: Option<GitRef>,
        remote: Option<String>,
    ) -> Result<SrcuriRequest> {
        // Split to get workspace name and relative path
        let (workspace, relative_path) = Self::split_authority_path(path_part);

        if workspace.is_empty() {
            return Err(SrcuriParseError::MissingWorkspaceName);
        }

        if !is_valid_workspace_name(workspace) {
            return Err(SrcuriParseError::InvalidWorkspaceName(Self::safe_display(
                workspace,
            )));
        }

        let (file_path, mut line, column) = Self::parse_path_with_location(relative_path);

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

    /// Parse rel mode (e.g., `srcuri://rel/path/file.rs@L42`).
    /// Searches all workspaces for matching path.
    #[allow(clippy::unnecessary_wraps)]
    fn parse_rel_mode(
        path_part: &str,
        fragment: Option<&str>,
        workspace_hint: Option<String>,
    ) -> Result<SrcuriRequest> {
        let (file_path, mut line, column) = Self::parse_path_with_location(path_part);

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

    /// Parse any mode (e.g., `srcuri://any/path/file.rs@L42`).
    /// Best-effort resolution in handler.
    #[allow(clippy::unnecessary_wraps)]
    fn parse_any_mode(
        path_part: &str,
        fragment: Option<&str>,
        workspace_hint: Option<String>,
    ) -> Result<SrcuriRequest> {
        let (file_path, mut line, column) = Self::parse_path_with_location(path_part);

        if line.is_none() {
            line = Self::parse_fragment_line(fragment);
        }

        Ok(SrcuriRequest::AnyPath {
            path: file_path,
            line,
            column,
            workspace_hint,
        })
    }

    /// Parse absolute path mode (e.g., `srcuri://abs/path`).
    /// Handles POSIX, Windows drive letters, and UNC paths.
    #[allow(clippy::unnecessary_wraps)]
    fn parse_abs_mode(path_part: &str, fragment: Option<&str>) -> Result<SrcuriRequest> {
        let (file_path, mut line, column) = Self::parse_path_with_location(path_part);

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

    /// Convert abs-mode path to filesystem path.
    /// - `/etc/hosts` → `/etc/hosts` (POSIX - add leading /)
    /// - `C:/Windows/...` → `C:/Windows/...` (Windows drive - keep as-is)
    /// - `UNC/server/share/path` → `//server/share/path` (Windows UNC)
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

    /// Parse external URL mode (e.g., `srcuri://ext/https/github.com/owner/repo/...`).
    /// Re-constructs the original URL and parses with srcuri-core.
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
            .map_err(|source| SrcuriParseError::ExternalUrlParse { source })?;

        let (fragment_line, fragment_column) = Self::parse_provider_fragment(fragment);

        // Map srcuri-core's ref_value to our GitRef enum, preserving incoming
        let git_ref = incoming_git_ref.or_else(|| target.ref_value.map(GitRef::Branch));

        Ok(SrcuriRequest::ExternalUrl {
            provider: target.remote,
            repo_name: target.repo_name,
            provider_path: path_part.to_string(),
            path: target.file_path.unwrap_or_default(),
            line: fragment_line.or_else(|| target.line.map(|l| l as usize)),
            column: fragment_column,
            git_ref,
            workspace_override,
            fragment: fragment.map(ToString::to_string),
        })
    }

    /// Reconstruct external URL from ext-mode path.
    /// Accepts two formats:
    /// - `https/github.com/owner/repo` → `https://github.com/owner/repo`
    /// - `https://github.com/owner/repo` → `https://github.com/owner/repo` (pass-through)
    fn reconstruct_external_url(path: &str) -> Result<String> {
        // Format 1: scheme already has :// (from srcuri.com)
        if let Some(rest) = path.strip_prefix("https://") {
            return Ok(format!("https://{}", rest));
        }
        if let Some(rest) = path.strip_prefix("http://") {
            return Ok(format!("http://{}", rest));
        }
        // Format 2: scheme uses / instead of :// (canonical srcuri format)
        if let Some(rest) = path.strip_prefix("https/") {
            return Ok(format!("https://{}", rest));
        }
        if let Some(rest) = path.strip_prefix("http/") {
            return Ok(format!("http://{}", rest));
        }
        Err(SrcuriParseError::InvalidExternalUrlScheme)
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
                            return Err(SrcuriParseError::InvalidCommitSha(Self::safe_display(
                                &decoded_str,
                            )));
                        }
                        return Ok(Some(GitRef::Commit(decoded_str)));
                    }
                    "branch" => {
                        if !is_valid_branch_name(&decoded_str) {
                            return Err(SrcuriParseError::InvalidBranchName(Self::safe_display(
                                &decoded_str,
                            )));
                        }
                        return Ok(Some(GitRef::Branch(decoded_str)));
                    }
                    "tag" => {
                        if !is_valid_tag_name(&decoded_str) {
                            return Err(SrcuriParseError::InvalidTagName(Self::safe_display(
                                &decoded_str,
                            )));
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
                        return Err(SrcuriParseError::InvalidRemoteUrl(Self::safe_display(
                            value,
                        )));
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
                        return Err(SrcuriParseError::InvalidWorkspaceName(Self::safe_display(
                            value,
                        )));
                    }
                    return Ok(Some(value.to_string()));
                }
            }
        }
        Ok(None)
    }

    /// Parse ?version= parameter (for hello ping)
    fn parse_version_param(query_part: Option<&str>) -> Option<String> {
        let q = query_part?;

        for pair in q.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if key == "version" && !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
        None
    }

    fn parse_path_with_location(path: &str) -> (String, Option<usize>, Option<usize>) {
        if let Some(parsed) = Self::parse_at_location(path) {
            return parsed;
        }

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

        (file_path, line, column)
    }

    fn parse_at_location(path: &str) -> Option<(String, Option<usize>, Option<usize>)> {
        let (head, tail) = Self::split_last_segment(path);
        let (marker_idx, marker_len) = Self::find_at_marker(tail)?;
        let suffix = &tail[marker_idx + marker_len..];

        let parsed = Self::parse_at_suffix(suffix)?;
        let base_tail = &tail[..marker_idx];
        let file_path = if head.is_empty() {
            base_tail.to_string()
        } else {
            format!("{}/{}", head, base_tail)
        };

        Some((file_path, parsed.line, parsed.column))
    }

    fn split_last_segment(path: &str) -> (&str, &str) {
        if let Some(idx) = path.rfind('/') {
            (&path[..idx], &path[idx + 1..])
        } else {
            ("", path)
        }
    }

    fn find_at_marker(tail: &str) -> Option<(usize, usize)> {
        let mut best_idx: Option<(usize, usize)> = None;

        if let Some(idx) = tail.rfind('@') {
            best_idx = Some((idx, 1));
        }

        let lower = tail.to_ascii_lowercase();
        if let Some(idx) = lower.rfind("%40") {
            if best_idx.map_or(true, |(best, _)| idx > best) {
                best_idx = Some((idx, 3));
            }
        }

        let (idx, len) = best_idx?;
        let suffix = &tail[idx + len..];
        if suffix.is_empty() {
            return None;
        }
        if !suffix.starts_with('L') && !suffix.starts_with('l') {
            return None;
        }

        Some((idx, len))
    }

    fn parse_at_suffix(suffix: &str) -> Option<AtSuffix> {
        let mut chars = suffix.chars();
        let first = chars.next()?;
        if first != 'L' && first != 'l' {
            return None;
        }

        let rest = chars.as_str();
        if rest.is_empty() {
            return Some(AtSuffix::empty());
        }

        let (line_opt, rem) = Self::parse_leading_number(rest);
        if let Some(line) = line_opt {
            return Self::parse_at_column(line, rem);
        }

        if let Some(rem) = rem {
            let mut rem_chars = rem.chars();
            let first_rem = rem_chars.next()?;
            if first_rem == 'C' || first_rem == 'c' {
                let col_tail = rem_chars.as_str();
                if col_tail.chars().all(|c| c.is_ascii_digit()) {
                    return Some(AtSuffix::empty());
                }
            }
        }

        None
    }

    fn parse_at_column(line: usize, rem: Option<&str>) -> Option<AtSuffix> {
        let Some(rem) = rem else {
            return Some(AtSuffix::new(line, None));
        };
        if rem.is_empty() {
            return Some(AtSuffix::new(line, None));
        }

        let mut rem_chars = rem.chars();
        let marker = rem_chars.next().unwrap_or_default();
        let col_tail = rem_chars.as_str();

        if marker == 'C' || marker == 'c' || marker == ':' {
            if col_tail.is_empty() {
                return Some(AtSuffix::new(line, None));
            }
            if !col_tail.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            if let Ok(parsed_column) = col_tail.parse::<usize>() {
                if parsed_column <= 120 {
                    return Some(AtSuffix::new(line, Some(parsed_column)));
                }
            }
            return Some(AtSuffix::new(line, None));
        }

        None
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

        matches!(bytes.get(colon_idx + 1), Some(b'\\') | Some(b'/'))
    }
}

#[derive(Debug)]
struct AtSuffix {
    line: Option<usize>,
    column: Option<usize>,
}

impl AtSuffix {
    fn new(line: usize, column: Option<usize>) -> Self {
        Self {
            line: Some(line),
            column,
        }
    }

    fn empty() -> Self {
        Self {
            line: None,
            column: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Implicit workspace tests

    #[test]
    fn test_implicit_workspace_simple() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://myproject/README.md:25").expect("parse URL");
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
    fn test_implicit_workspace_with_at_line() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md@L25").expect("parse URL");
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
    fn test_implicit_workspace_with_at_line_lowercase() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md@l25").expect("parse URL");
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
    fn test_implicit_workspace_with_at_line_and_column() {
        let request =
            SrcuriParser::parse("srcuri://myproject/README.md@L25C7").expect("parse URL");
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "README.md".to_string(),
                line: Some(25),
                column: Some(7),
                git_ref: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_implicit_workspace_with_at_line_and_column_colon() {
        let request =
            SrcuriParser::parse("srcuri://myproject/README.md@L25:7").expect("parse URL");
        assert_eq!(
            request,
            SrcuriRequest::ImplicitWorkspace {
                workspace: "myproject".to_string(),
                path: "README.md".to_string(),
                line: Some(25),
                column: Some(7),
                git_ref: None,
                remote: None,
            }
        );
    }

    #[test]
    fn test_implicit_workspace_with_at_empty_line() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md@L").expect("parse URL");
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
    fn test_implicit_workspace_with_at_empty_line_and_column_marker() {
        let request = SrcuriParser::parse("srcuri://myproject/README.md@LC").expect("parse URL");
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
    fn test_implicit_workspace_with_encoded_at_line() {
        let request =
            SrcuriParser::parse("srcuri://myproject/README.md%40L25").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://myproject/src/main.rs:42").expect("parse URL");
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
        let request =
            SrcuriParser::parse("srcuri://myproject/src/main.rs:42:7").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://myrepo/src/file.rs:23?commit=abc123def")
            .expect("parse URL");
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
        .expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://wks/myrepo/src/main.rs:42").expect("parse URL");
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
            SrcuriParser::parse("srcuri://wks/myrepo/file.rs:10?branch=main").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://rel/README.md").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://rel/README.md:25").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://rel/src/utils.py:10?workspaceHint=backend")
            .expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://rel/src/lib/utils.py:10").expect("parse URL");
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

    // Any mode tests

    #[test]
    fn test_any_mode_simple() {
        let request = SrcuriParser::parse("srcuri://any/README.md").expect("parse URL");
        assert_eq!(
            request,
            SrcuriRequest::AnyPath {
                path: "README.md".to_string(),
                line: None,
                column: None,
                workspace_hint: None,
            }
        );
    }

    #[test]
    fn test_any_mode_with_line() {
        let request = SrcuriParser::parse("srcuri://any/src/lib.rs:10").expect("parse URL");
        assert_eq!(
            request,
            SrcuriRequest::AnyPath {
                path: "src/lib.rs".to_string(),
                line: Some(10),
                column: None,
                workspace_hint: None,
            }
        );
    }

    #[test]
    fn test_any_mode_with_workspace_hint() {
        let request = SrcuriParser::parse("srcuri://any/src/utils.py:10?workspaceHint=backend")
            .expect("parse URL");
        assert_eq!(
            request,
            SrcuriRequest::AnyPath {
                path: "src/utils.py".to_string(),
                line: Some(10),
                column: None,
                workspace_hint: Some("backend".to_string()),
            }
        );
    }

    // Absolute path mode tests

    #[test]
    fn test_abs_mode_posix() {
        let request = SrcuriParser::parse("srcuri://abs/etc/hosts:1").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://abs/Users/alice/code/myproject/README.md:50")
            .expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://abs/C:/Users/Carol/Dev/project/README.md:10")
            .expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://abs/UNC/server/share/docs/readme.txt:5")
            .expect("parse URL");
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
        let request =
            SrcuriParser::parse("srcuri://abs/home/user/file.txt:10:5").expect("parse URL");
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
        let request =
            SrcuriParser::parse("srcuri://abs//private/var/folders/test.rs:1").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://abs/tmp/test.rs:42").expect("parse URL");
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
        .expect("parse URL");
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
        let request =
            SrcuriParser::parse("srcuri://ext/https/github.com/owner/repo").expect("parse URL");
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
        .expect("parse URL");
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
        .expect("parse URL");
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
        .expect("parse URL");
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
        .expect("parse URL");
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
        .expect("parse URL");
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

    #[test]
    fn test_ext_mode_srcuri_com_format() {
        // srcuri.com generates URLs with https:// instead of https/
        let request = SrcuriParser::parse("srcuri://ext/https://github.com/fcsonline/drill")
            .expect("parse URL");
        assert_eq!(
            request,
            SrcuriRequest::ExternalUrl {
                provider: "github.com/fcsonline/drill".to_string(),
                repo_name: "drill".to_string(),
                provider_path: "https://github.com/fcsonline/drill".to_string(),
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
    fn test_ext_mode_srcuri_com_format_with_file() {
        let request = SrcuriParser::parse(
            "srcuri://ext/https://github.com/owner/repo/blob/main/src/lib.rs#L42",
        )
        .expect("parse URL");
        assert_eq!(
            request,
            SrcuriRequest::ExternalUrl {
                provider: "github.com/owner/repo".to_string(),
                repo_name: "repo".to_string(),
                provider_path: "https://github.com/owner/repo/blob/main/src/lib.rs".to_string(),
                path: "src/lib.rs".to_string(),
                line: Some(42),
                column: None,
                git_ref: Some(GitRef::Branch("main".to_string())),
                workspace_override: None,
                fragment: Some("L42".to_string()),
            }
        );
    }

    // Column boundary tests

    #[test]
    fn test_column_at_boundary_120_accepted() {
        let request = SrcuriParser::parse("srcuri://myproject/file.txt:10:120").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://myproject/file.txt:10:121").expect("parse URL");
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
        let request =
            SrcuriParser::parse("srcuri://myproject/README.md:1?branch=main").expect("parse URL");
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
        let request =
            SrcuriParser::parse("srcuri://myrepo/src/file.rs:10?tag=v1.0.0").expect("parse URL");
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
        let request =
            SrcuriParser::parse("srcuri://myrepo/src/file.rs:23?sha=abc123def").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://myrepo/file.rs:1?branch=feature%2Fc%2B%2B")
            .expect("parse URL");
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
        let request =
            SrcuriParser::parse("srcuri://myrepo/file.rs:1?branch=%23pr470").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://myproject/src/main.rs#100").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://myproject/file.txt:42#99").expect("parse URL");
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
        let request = SrcuriParser::parse("srcuri://myproject/src/main.rs:").expect("parse URL");
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
        let request =
            SrcuriParser::parse("srcuri://abs/Users/alice/file.txt:").expect("parse URL");
        assert_eq!(
            request,
            SrcuriRequest::AbsolutePath {
                full_path: "/Users/alice/file.txt".to_string(),
                line: None,
                column: None,
            }
        );
    }

    #[test]
    fn test_trailing_colon_rel_mode() {
        let request = SrcuriParser::parse("srcuri://rel/README.md:").expect("parse URL");
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
    fn test_trailing_colon_any_mode() {
        let request = SrcuriParser::parse("srcuri://any/README.md:").expect("parse URL");
        assert_eq!(
            request,
            SrcuriRequest::AnyPath {
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
        let request =
            SrcuriParser::parse("srcuri://myproject/file.rs?foo=bar&baz=qux").expect("parse URL");
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
        let request = result.expect("parse URL");
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
        let result = SrcuriParser::parse("srcuri://wks/myrepo/file.rs").expect("parse URL");
        assert!(matches!(result, SrcuriRequest::ExplicitWorkspace { .. }));

        let result = SrcuriParser::parse("srcuri://rel/file.rs").expect("parse URL");
        assert!(matches!(result, SrcuriRequest::RelativePath { .. }));

        let result = SrcuriParser::parse("srcuri://any/file.rs").expect("parse URL");
        assert!(matches!(result, SrcuriRequest::AnyPath { .. }));

        let result = SrcuriParser::parse("srcuri://abs/etc/hosts").expect("parse URL");
        assert!(matches!(result, SrcuriRequest::AbsolutePath { .. }));

        let result = SrcuriParser::parse("srcuri://ext/https/github.com/owner/repo");
        if let Ok(req) = result {
            assert!(!matches!(req, SrcuriRequest::ImplicitWorkspace { .. }));
        }
    }

    #[test]
    fn test_reserved_authorities_case_insensitive() {
        let result = SrcuriParser::parse("srcuri://REL/file.rs:1").expect("parse URL");
        assert!(matches!(result, SrcuriRequest::RelativePath { .. }));

        let result = SrcuriParser::parse("srcuri://ANY/file.rs:1").expect("parse URL");
        assert!(matches!(result, SrcuriRequest::AnyPath { .. }));

        let result = SrcuriParser::parse("srcuri://ABS/etc/hosts").expect("parse URL");
        assert!(matches!(result, SrcuriRequest::AbsolutePath { .. }));

        let result = SrcuriParser::parse("srcuri://WKS/repo/file.rs").expect("parse URL");
        assert!(matches!(result, SrcuriRequest::ExplicitWorkspace { .. }));
    }

    #[test]
    fn test_ping_request() {
        let result = SrcuriParser::parse("srcuri://ping").expect("parse URL");
        assert_eq!(result, SrcuriRequest::Ping);
    }

    #[test]
    fn test_ping_request_case_insensitive() {
        let result = SrcuriParser::parse("srcuri://PING").expect("parse URL");
        assert_eq!(result, SrcuriRequest::Ping);

        let result = SrcuriParser::parse("srcuri://Ping").expect("parse URL");
        assert_eq!(result, SrcuriRequest::Ping);
    }

    #[test]
    fn test_hello_request_without_version() {
        let result = SrcuriParser::parse("srcuri://hello").expect("parse URL");
        assert_eq!(result, SrcuriRequest::Hello { version: None });
    }

    #[test]
    fn test_hello_request_with_version() {
        let result = SrcuriParser::parse("srcuri://hello?version=1.0.0").expect("parse URL");
        assert_eq!(
            result,
            SrcuriRequest::Hello {
                version: Some("1.0.0".to_string())
            }
        );
    }

    #[test]
    fn test_hello_request_case_insensitive() {
        let result = SrcuriParser::parse("srcuri://HELLO?version=2.0").expect("parse URL");
        assert_eq!(
            result,
            SrcuriRequest::Hello {
                version: Some("2.0".to_string())
            }
        );
    }
}
