use super::{Settings, SettingsManager, WorkspaceConfig};
use anyhow::Result;
use notify::{recommended_watcher, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

pub struct WorkspaceWatchService {
    watcher: RecommendedWatcher,
    watched_roots: HashSet<PathBuf>,
}

impl WorkspaceWatchService {
    pub async fn new(
        settings_manager: Arc<SettingsManager>,
        change_tx: UnboundedSender<()>,
    ) -> Result<Self> {
        let mut watcher = recommended_watcher(move |event: notify::Result<Event>| match event {
            Ok(event) => {
                if should_trigger_reconcile(&event) {
                    let _ = change_tx.send(());
                }
            }
            Err(error) => {
                warn!("Workspace watcher error: {error}");
                let _ = change_tx.send(());
            }
        })?;

        let watch_roots = derive_watch_roots(&settings_manager.get().await);
        watch_all_roots(&mut watcher, &watch_roots);

        Ok(Self {
            watcher,
            watched_roots: watch_roots,
        })
    }

    pub fn refresh_watch_roots_for_settings(&mut self, settings: &Settings) {
        let desired_roots = derive_watch_roots(settings);

        for root in self.watched_roots.difference(&desired_roots) {
            if let Err(error) = self.watcher.unwatch(root) {
                let root_display = root.display();
                warn!("Failed to unwatch workspace path {root_display}: {error}");
            } else {
                let root_display = root.display();
                debug!("Stopped watching workspace path {root_display}");
            }
        }

        for root in desired_roots.difference(&self.watched_roots) {
            if let Err(error) = self.watcher.watch(root, RecursiveMode::Recursive) {
                let root_display = root.display();
                warn!("Failed to watch workspace path {root_display}: {error}");
            } else {
                let root_display = root.display();
                debug!("Watching workspace path {root_display}");
            }
        }

        self.watched_roots = desired_roots;
    }
}

fn should_trigger_reconcile(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_)
            | EventKind::Modify(_)
            | EventKind::Remove(_)
            | EventKind::Any
            | EventKind::Other
    )
}

fn watch_all_roots(watcher: &mut RecommendedWatcher, roots: &HashSet<PathBuf>) {
    for root in roots {
        if let Err(error) = watcher.watch(root, RecursiveMode::Recursive) {
            let root_display = root.display();
            warn!("Failed to watch workspace path {root_display}: {error}");
        } else {
            let root_display = root.display();
            debug!("Watching workspace path {root_display}");
        }
    }
}

fn derive_watch_roots(settings: &Settings) -> HashSet<PathBuf> {
    let mut roots: HashSet<PathBuf> = HashSet::new();

    if let Some(default_root) =
        expand_to_absolute_path(&settings.defaults.default_workspaces_folder)
            .as_deref()
            .and_then(resolve_watch_root)
            .map(normalize_watch_root)
    {
        roots.insert(default_root);
    }

    for workspace in &settings.workspaces {
        let Some(path_candidate) = workspace_watch_target(workspace) else {
            continue;
        };
        let Some(root) = resolve_watch_root(&path_candidate).map(normalize_watch_root) else {
            continue;
        };
        roots.insert(root);
    }

    roots
}

fn workspace_watch_target(workspace: &WorkspaceConfig) -> Option<PathBuf> {
    if let Some(path) = workspace.normalized_path.as_ref() {
        return Some(path.clone());
    }
    expand_to_absolute_path(&workspace.path)
}

fn expand_to_absolute_path(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let expanded = shellexpand::tilde(trimmed);
    let expanded_path = PathBuf::from(expanded.as_ref());
    if expanded_path.is_absolute() {
        return Some(expanded_path);
    }

    std::env::current_dir()
        .ok()
        .map(|current_dir| current_dir.join(expanded_path))
}

fn resolve_watch_root(path: &Path) -> Option<PathBuf> {
    if path.exists() {
        if path.is_dir() {
            return Some(path.to_path_buf());
        }
        return path.parent().map(Path::to_path_buf);
    }

    for ancestor in path.ancestors().skip(1) {
        if !ancestor.exists() || !ancestor.is_dir() {
            continue;
        }

        ancestor.parent()?;

        return Some(ancestor.to_path_buf());
    }

    None
}

fn normalize_watch_root(path: PathBuf) -> PathBuf {
    let canonical = path.canonicalize().unwrap_or(path);

    #[cfg(target_os = "macos")]
    {
        let canonical_str = canonical.to_string_lossy();
        if canonical_str.starts_with("/private/") {
            if let Ok(stripped) = canonical.strip_prefix("/private") {
                let mut absolute = PathBuf::from("/");
                absolute.push(stripped);
                return absolute;
            }
        }
    }

    canonical
}

#[cfg(test)]
mod tests {
    use super::{derive_watch_roots, resolve_watch_root};
    use crate::settings::{Settings, WorkspaceConfig, WorkspaceKind, WorkspaceState};
    use tempfile::TempDir;

    fn workspace_config(path: String) -> WorkspaceConfig {
        WorkspaceConfig {
            path,
            workspace_key: "workspace".to_string(),
            editor: String::new(),
            auto_discovered: false,
            trusted: false,
            workspace_kind: WorkspaceKind::NonGit,
            workspace_state: WorkspaceState::Present,
            repo_identity: None,
            last_verified_at: None,
            normalized_path: None,
        }
    }

    #[test]
    fn derives_watch_roots_from_default_folder_and_workspace_paths() {
        let root = TempDir::new().expect("temp dir");
        let default_root = root.path().join("apps");
        let workspace_path = root.path().join("apps").join("repo");
        std::fs::create_dir_all(&workspace_path).expect("create workspace path");

        let mut settings = Settings::default();
        settings.defaults.default_workspaces_folder = default_root.to_string_lossy().to_string();

        let mut workspace = workspace_config(workspace_path.to_string_lossy().to_string());
        workspace.normalized_path = Some(workspace_path.clone());
        settings.workspaces.push(workspace);

        let roots = derive_watch_roots(&settings);
        assert!(roots.contains(&default_root));
        assert!(roots.contains(&workspace_path));
    }

    #[test]
    fn resolves_missing_path_to_existing_parent_directory() {
        let root = TempDir::new().expect("temp dir");
        let existing = root.path().join("apps");
        std::fs::create_dir_all(&existing).expect("create apps folder");

        let missing_path = existing.join("myapp").join("src");
        let watch_root = resolve_watch_root(&missing_path).expect("watch root");
        assert_eq!(watch_root, existing);
    }
}
