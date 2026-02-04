use std::fmt::{self, Write};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SrcuriTarget {
    pub remote: String,
    pub repo_name: String,
    pub ref_value: Option<String>,
    pub file_path: Option<String>,
    pub line: Option<u32>,
    pub is_absolute: bool,
}

impl SrcuriTarget {
    #[must_use]
    pub fn to_mirror_url(&self) -> String {
        let mut url = format!("/{}", self.repo_name);

        if let Some(ref path) = self.file_path {
            url.push('/');
            url.push_str(path);
        }

        if let Some(line) = self.line {
            write!(url, "@L{line}").unwrap();
        }

        let mut query_parts = Vec::new();
        if let Some(ref branch) = self.ref_value {
            query_parts.push(format!("branch={branch}"));
        }
        // Include https:// prefix for git clone compatibility
        query_parts.push(format!("remote=https://{}", self.remote));

        if !query_parts.is_empty() {
            url.push('?');
            url.push_str(&query_parts.join("&"));
        }

        url
    }

    /// Construct a URL to view this file on the remote provider (GitHub, GitLab, etc.)
    /// Returns None if there's no remote or no file path to view.
    #[must_use]
    pub fn to_view_url(&self) -> Option<String> {
        if self.remote.is_empty() || self.file_path.is_none() {
            return None;
        }

        let file_path = self.file_path.as_ref()?;
        let branch = self.ref_value.as_deref().unwrap_or("main");
        let remote_lower = self.remote.to_lowercase();

        // Determine provider and construct appropriate URL
        let base_url = if remote_lower.contains("github.com") {
            // GitHub: https://github.com/owner/repo/blob/branch/path#L42
            format!("https://{}/blob/{}/{}", self.remote, branch, file_path)
        } else if remote_lower.contains("gitlab") {
            // GitLab: https://gitlab.com/owner/repo/-/blob/branch/path#L42
            format!("https://{}/-/blob/{}/{}", self.remote, branch, file_path)
        } else if remote_lower.contains("bitbucket") {
            // Bitbucket: https://bitbucket.org/owner/repo/src/branch/path#lines-42
            format!("https://{}/src/{}/{}", self.remote, branch, file_path)
        } else if remote_lower.contains("codeberg.org") {
            // Codeberg (Gitea-based): https://codeberg.org/owner/repo/src/branch/main/path#L42
            format!(
                "https://{}/src/branch/{}/{}",
                self.remote, branch, file_path
            )
        } else if remote_lower.contains("dev.azure.com")
            || remote_lower.contains("visualstudio.com")
        {
            // Azure DevOps: complex URL structure, return basic for now
            format!("https://{}", self.remote)
        } else {
            // Generic Gitea/other: use Gitea-style URL
            format!(
                "https://{}/src/branch/{}/{}",
                self.remote, branch, file_path
            )
        };

        // Append line number fragment
        let url = if let Some(line) = self.line {
            if remote_lower.contains("bitbucket") {
                format!("{base_url}#lines-{line}")
            } else {
                format!("{base_url}#L{line}")
            }
        } else {
            base_url
        };

        Some(url)
    }

    /// Get a human-readable name for the remote provider
    #[must_use]
    pub fn provider_name(&self) -> &'static str {
        let remote_lower = self.remote.to_lowercase();
        if remote_lower.contains("github.com") {
            "GitHub"
        } else if remote_lower.contains("gitlab") {
            "GitLab"
        } else if remote_lower.contains("bitbucket") {
            "Bitbucket"
        } else if remote_lower.contains("codeberg.org") {
            "Codeberg"
        } else if remote_lower.contains("dev.azure.com")
            || remote_lower.contains("visualstudio.com")
        {
            "Azure DevOps"
        } else {
            "Remote"
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    GitHub,
    GitLab,
    Bitbucket,
    Gitea,
    Codeberg,
    AzureDevOps,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub original_url: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.message, self.original_url)
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    pub fn new(message: impl Into<String>, original_url: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            original_url: original_url.into(),
        }
    }
}
