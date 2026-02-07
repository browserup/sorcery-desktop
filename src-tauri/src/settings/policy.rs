use super::identity;
use super::models::{
    PolicyMode, RepoIdentity, WorkspaceConfig, WorkspaceKind, WorkspacePolicyConfig,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allowed,
    AdvisoryViolation(PolicyViolation),
    EnforcedViolation(PolicyViolation),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PolicyViolation {
    #[error("Workspace key '{workspace_key}' is not allowed by policy")]
    WorkspaceKeyNotAllowed { workspace_key: String },

    #[error(
        "Workspace '{workspace_key}' requires remote '{expected_remote}', but found '{actual_remote}'"
    )]
    WorkspaceRemoteMismatch {
        workspace_key: String,
        expected_remote: String,
        actual_remote: String,
    },

    #[error("Workspace '{workspace_key}' requires git remote '{expected_remote}'")]
    WorkspaceRemoteRequired {
        workspace_key: String,
        expected_remote: String,
    },

    #[error("Workspace '{workspace_key}' requires a remote, but none was provided")]
    RemoteRequiredForWorkspace { workspace_key: String },

    #[error("Remote '{remote}' is invalid for policy validation")]
    InvalidRemote { remote: String },

    #[error("Remote host '{host}' is not allowed by policy")]
    RemoteHostNotAllowed { host: String },

    #[error("Remote org '{org}' is not allowed by policy")]
    RemoteOrgNotAllowed { org: String },

    #[error("Clone target '{target_path}' is outside allowed clone roots")]
    CloneRootNotAllowed { target_path: String },
}

#[derive(Debug, Error)]
pub enum PolicyBuildError {
    #[error("Policy mapping has an empty workspace key")]
    EmptyWorkspaceKey,

    #[error("Duplicate policy mapping for workspace key '{workspace_key}'")]
    DuplicateWorkspaceKey { workspace_key: String },

    #[error("Policy mapping for workspace '{workspace_key}' has invalid remote '{remote}'")]
    InvalidMappingRemote {
        workspace_key: String,
        remote: String,
    },

    #[error("Allowed clone root '{root}' must be absolute")]
    NonAbsoluteCloneRoot { root: String },

    #[error("Invalid allowed remote host '{host}'")]
    InvalidRemoteHost { host: String },

    #[error("Invalid allowed remote org '{org}'")]
    InvalidRemoteOrg { org: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PolicyMappingRule {
    expected_remote: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePolicy {
    mode: PolicyMode,
    mappings: HashMap<String, PolicyMappingRule>,
    allowed_clone_roots: Vec<PathBuf>,
    allowed_remote_hosts: HashSet<String>,
    allowed_remote_orgs: HashSet<String>,
}

impl WorkspacePolicy {
    pub fn from_config(config: WorkspacePolicyConfig) -> Result<Self, PolicyBuildError> {
        let mut mappings = HashMap::new();

        for mapping in config.mappings {
            let key = identity::canonical_workspace_key_for_lookup(mapping.workspace_key.trim());
            if key.is_empty() {
                return Err(PolicyBuildError::EmptyWorkspaceKey);
            }

            let expected_remote = mapping
                .remote
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    identity::normalize_remote_identity(value).ok_or_else(|| {
                        PolicyBuildError::InvalidMappingRemote {
                            workspace_key: key.clone(),
                            remote: value.to_string(),
                        }
                    })
                })
                .transpose()?;

            if mappings
                .insert(key.clone(), PolicyMappingRule { expected_remote })
                .is_some()
            {
                return Err(PolicyBuildError::DuplicateWorkspaceKey { workspace_key: key });
            }
        }

        let mut allowed_clone_roots = Vec::new();
        for root in config
            .allowed_clone_roots
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            let normalized = normalize_policy_root(&root)?;
            allowed_clone_roots.push(normalized);
        }

        let mut allowed_remote_hosts = HashSet::new();
        for host in config
            .allowed_remote_hosts
            .into_iter()
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty())
        {
            if host.contains('/') || host.contains(':') {
                return Err(PolicyBuildError::InvalidRemoteHost { host });
            }
            allowed_remote_hosts.insert(host);
        }

        let mut allowed_remote_orgs = HashSet::new();
        for org in config
            .allowed_remote_orgs
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            let normalized = normalize_remote_org_scope(&org)
                .ok_or_else(|| PolicyBuildError::InvalidRemoteOrg { org: org.clone() })?;
            allowed_remote_orgs.insert(normalized);
        }

        Ok(Self {
            mode: config.mode,
            mappings,
            allowed_clone_roots,
            allowed_remote_hosts,
            allowed_remote_orgs,
        })
    }

    #[must_use]
    pub fn evaluate_workspace(&self, workspace: &WorkspaceConfig) -> PolicyDecision {
        self.policy_decision(self.workspace_violation(workspace))
    }

    #[must_use]
    pub fn evaluate_clone_request(
        &self,
        workspace_key: &str,
        remote: Option<&str>,
        target_path: &Path,
    ) -> PolicyDecision {
        self.policy_decision(self.clone_violation(workspace_key, remote, target_path))
    }

    fn policy_decision(&self, violation: Option<PolicyViolation>) -> PolicyDecision {
        match violation {
            None => PolicyDecision::Allowed,
            Some(violation) => match self.mode {
                PolicyMode::Advisory => PolicyDecision::AdvisoryViolation(violation),
                PolicyMode::Enforced => PolicyDecision::EnforcedViolation(violation),
            },
        }
    }

    fn workspace_violation(&self, workspace: &WorkspaceConfig) -> Option<PolicyViolation> {
        let workspace_key = identity::canonical_workspace_key_for_lookup(
            &identity::derive_workspace_key(workspace),
        );

        if !self.mappings.is_empty() {
            let Some(rule) = self.mappings.get(&workspace_key) else {
                return Some(PolicyViolation::WorkspaceKeyNotAllowed { workspace_key });
            };

            if let Some(expected_remote) = rule.expected_remote.as_deref() {
                let Some(repo_identity) = workspace.repo_identity.as_ref() else {
                    return Some(PolicyViolation::WorkspaceRemoteRequired {
                        workspace_key,
                        expected_remote: expected_remote.to_string(),
                    });
                };

                if !identity::remote_matches_identity(expected_remote, repo_identity) {
                    let actual_remote = repo_identity
                        .primary_remote
                        .clone()
                        .or_else(|| repo_identity.all_remotes.first().cloned())
                        .unwrap_or_else(|| "none".to_string());

                    return Some(PolicyViolation::WorkspaceRemoteMismatch {
                        workspace_key,
                        expected_remote: expected_remote.to_string(),
                        actual_remote,
                    });
                }
            }
        }

        if self.allowed_remote_hosts.is_empty() && self.allowed_remote_orgs.is_empty() {
            return None;
        }

        let Some(repo_identity) = workspace.repo_identity.as_ref() else {
            if workspace.workspace_kind == WorkspaceKind::Git {
                return Some(PolicyViolation::RemoteRequiredForWorkspace { workspace_key });
            }
            return None;
        };

        self.identity_remote_policy_violation(&workspace_key, repo_identity)
    }

    fn clone_violation(
        &self,
        workspace_key: &str,
        remote: Option<&str>,
        target_path: &Path,
    ) -> Option<PolicyViolation> {
        let canonical_key = identity::canonical_workspace_key_for_lookup(workspace_key);
        let normalized_remote = match remote.map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) => match identity::normalize_remote_identity(value) {
                Some(normalized) => Some(normalized),
                None => {
                    return Some(PolicyViolation::InvalidRemote {
                        remote: value.to_string(),
                    })
                }
            },
            None => None,
        };

        if !self.mappings.is_empty() {
            let Some(rule) = self.mappings.get(&canonical_key) else {
                return Some(PolicyViolation::WorkspaceKeyNotAllowed {
                    workspace_key: canonical_key,
                });
            };

            if let Some(expected_remote) = rule.expected_remote.as_deref() {
                let Some(actual_remote) = normalized_remote.as_deref() else {
                    return Some(PolicyViolation::WorkspaceRemoteRequired {
                        workspace_key: canonical_key,
                        expected_remote: expected_remote.to_string(),
                    });
                };

                if actual_remote != expected_remote {
                    return Some(PolicyViolation::WorkspaceRemoteMismatch {
                        workspace_key: canonical_key,
                        expected_remote: expected_remote.to_string(),
                        actual_remote: actual_remote.to_string(),
                    });
                }
            }
        }

        if !self.allowed_clone_roots.is_empty() {
            let normalized_target = normalize_target_path(target_path);
            let allowed = self
                .allowed_clone_roots
                .iter()
                .any(|root| normalized_target.starts_with(root));
            if !allowed {
                return Some(PolicyViolation::CloneRootNotAllowed {
                    target_path: normalized_target.to_string_lossy().to_string(),
                });
            }
        }

        if self.allowed_remote_hosts.is_empty() && self.allowed_remote_orgs.is_empty() {
            return None;
        }

        let Some(remote) = normalized_remote.as_deref() else {
            return Some(PolicyViolation::RemoteRequiredForWorkspace {
                workspace_key: canonical_key,
            });
        };

        self.remote_policy_violation(remote)
    }

    fn identity_remote_policy_violation(
        &self,
        workspace_key: &str,
        identity: &RepoIdentity,
    ) -> Option<PolicyViolation> {
        let mut remotes = Vec::new();
        if let Some(primary) = identity.primary_remote.as_deref() {
            remotes.push(primary);
        }
        remotes.extend(identity.all_remotes.iter().map(String::as_str));

        if remotes.is_empty() {
            return Some(PolicyViolation::RemoteRequiredForWorkspace {
                workspace_key: workspace_key.to_string(),
            });
        }

        let mut first_violation = None;
        for remote in remotes {
            if let Some(violation) = self.remote_policy_violation(remote) {
                if first_violation.is_none() {
                    first_violation = Some(violation);
                }
                continue;
            }
            return None;
        }

        first_violation
    }

    fn remote_policy_violation(&self, remote: &str) -> Option<PolicyViolation> {
        let host = remote_host(remote)?;

        if !self.allowed_remote_hosts.is_empty() && !self.allowed_remote_hosts.contains(host) {
            return Some(PolicyViolation::RemoteHostNotAllowed {
                host: host.to_string(),
            });
        }

        if self.allowed_remote_orgs.is_empty() {
            return None;
        }

        let Some(org) = remote_org(remote) else {
            return Some(PolicyViolation::RemoteOrgNotAllowed {
                org: "unknown".to_string(),
            });
        };

        if !self.allowed_remote_orgs.contains(&org) {
            return Some(PolicyViolation::RemoteOrgNotAllowed { org });
        }

        None
    }
}

fn normalize_policy_root(raw_root: &str) -> Result<PathBuf, PolicyBuildError> {
    let expanded = shellexpand::tilde(raw_root);
    let expanded_path = PathBuf::from(expanded.as_ref());
    if !expanded_path.is_absolute() {
        return Err(PolicyBuildError::NonAbsoluteCloneRoot {
            root: raw_root.to_string(),
        });
    }

    Ok(normalize_path_for_policy(expanded_path))
}

fn normalize_target_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return normalize_path_for_policy(path.to_path_buf());
    }

    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    normalize_path_for_policy(current_dir.join(path))
}

fn normalize_path_for_policy(path: PathBuf) -> PathBuf {
    let normalized = if path.exists() {
        path.canonicalize().unwrap_or(path)
    } else {
        path
    };

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

fn normalize_remote_org_scope(value: &str) -> Option<String> {
    let normalized = identity::normalize_remote_identity(value)?;
    remote_org(&normalized)
}

fn remote_host(remote: &str) -> Option<&str> {
    remote.split('/').next()
}

fn remote_org(remote: &str) -> Option<String> {
    let mut parts = remote.split('/');
    let host = parts.next()?;
    let org = parts.next()?;
    Some(format!("{host}/{org}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::WorkspaceState;

    fn workspace(path: &Path, key: &str, remote: Option<&str>) -> WorkspaceConfig {
        WorkspaceConfig {
            path: path.to_string_lossy().to_string(),
            workspace_key: key.to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::Git,
            workspace_state: WorkspaceState::Present,
            repo_identity: remote.map(|value| RepoIdentity {
                primary_remote: Some(value.to_string()),
                all_remotes: vec![value.to_string()],
                git_common_dir: None,
                current_branch: None,
                head_commit: None,
            }),
            last_verified_at: None,
            normalized_path: Some(path.to_path_buf()),
        }
    }

    #[test]
    fn policy_rejects_duplicate_workspace_keys() {
        let config = WorkspacePolicyConfig {
            mode: PolicyMode::Enforced,
            mappings: vec![
                super::super::models::WorkspacePolicyMapping {
                    workspace_key: "rails".to_string(),
                    remote: Some("github.com/rails/rails".to_string()),
                },
                super::super::models::WorkspacePolicyMapping {
                    workspace_key: "Rails".to_string(),
                    remote: Some("github.com/company/rails".to_string()),
                },
            ],
            allowed_clone_roots: Vec::new(),
            allowed_remote_hosts: Vec::new(),
            allowed_remote_orgs: Vec::new(),
        };

        let error = WorkspacePolicy::from_config(config).expect_err("duplicate key must fail");
        assert!(matches!(
            error,
            PolicyBuildError::DuplicateWorkspaceKey { workspace_key } if workspace_key == "rails"
        ));
    }

    #[test]
    fn enforced_policy_blocks_workspace_remote_mismatch() {
        let config = WorkspacePolicyConfig {
            mode: PolicyMode::Enforced,
            mappings: vec![super::super::models::WorkspacePolicyMapping {
                workspace_key: "rails".to_string(),
                remote: Some("github.com/rails/rails".to_string()),
            }],
            allowed_clone_roots: Vec::new(),
            allowed_remote_hosts: Vec::new(),
            allowed_remote_orgs: Vec::new(),
        };
        let policy = WorkspacePolicy::from_config(config).expect("policy");

        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let workspace = workspace(temp_dir.path(), "rails", Some("github.com/company/rails"));

        let decision = policy.evaluate_workspace(&workspace);
        assert!(matches!(
            decision,
            PolicyDecision::EnforcedViolation(PolicyViolation::WorkspaceRemoteMismatch { .. })
        ));
    }

    #[test]
    fn advisory_policy_warns_without_blocking() {
        let config = WorkspacePolicyConfig {
            mode: PolicyMode::Advisory,
            mappings: vec![super::super::models::WorkspacePolicyMapping {
                workspace_key: "rails".to_string(),
                remote: Some("github.com/rails/rails".to_string()),
            }],
            allowed_clone_roots: Vec::new(),
            allowed_remote_hosts: Vec::new(),
            allowed_remote_orgs: Vec::new(),
        };
        let policy = WorkspacePolicy::from_config(config).expect("policy");

        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let workspace = workspace(temp_dir.path(), "rails", Some("github.com/company/rails"));

        let decision = policy.evaluate_workspace(&workspace);
        assert!(matches!(
            decision,
            PolicyDecision::AdvisoryViolation(PolicyViolation::WorkspaceRemoteMismatch { .. })
        ));
    }

    #[test]
    fn enforced_policy_blocks_clone_outside_allowed_root() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let allowed_root = temp_dir.path().join("apps");
        std::fs::create_dir_all(&allowed_root).expect("create root");

        let config = WorkspacePolicyConfig {
            mode: PolicyMode::Enforced,
            mappings: Vec::new(),
            allowed_clone_roots: vec![allowed_root.to_string_lossy().to_string()],
            allowed_remote_hosts: Vec::new(),
            allowed_remote_orgs: Vec::new(),
        };
        let policy = WorkspacePolicy::from_config(config).expect("policy");

        let outside_path = temp_dir.path().join("other").join("repo");
        let decision =
            policy.evaluate_clone_request("repo", Some("github.com/company/repo"), &outside_path);

        assert!(matches!(
            decision,
            PolicyDecision::EnforcedViolation(PolicyViolation::CloneRootNotAllowed { .. })
        ));
    }

    #[test]
    fn clone_remote_host_allowlist_is_enforced() {
        let config = WorkspacePolicyConfig {
            mode: PolicyMode::Enforced,
            mappings: Vec::new(),
            allowed_clone_roots: Vec::new(),
            allowed_remote_hosts: vec!["github.com".to_string()],
            allowed_remote_orgs: Vec::new(),
        };
        let policy = WorkspacePolicy::from_config(config).expect("policy");

        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let target = temp_dir.path().join("repo");
        let decision =
            policy.evaluate_clone_request("repo", Some("gitlab.com/company/repo"), &target);

        assert!(matches!(
            decision,
            PolicyDecision::EnforcedViolation(PolicyViolation::RemoteHostNotAllowed { host })
                if host == "gitlab.com"
        ));
    }
}
