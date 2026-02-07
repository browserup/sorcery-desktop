mod discovery;
pub(crate) mod identity;
mod manager;
mod models;
pub(crate) mod policy;
pub mod watch;

pub use discovery::{SyncResult, WorkspaceSync};
pub use manager::SettingsManager;
#[allow(unused_imports)]
pub use models::{
    LastSeenData, PolicyMode, RepoIdentity, Settings, WorkspaceConfig, WorkspaceKind,
    WorkspacePolicyConfig, WorkspacePolicyMapping, WorkspaceState,
};
pub use policy::PolicyDecision;
pub use watch::WorkspaceWatchService;
