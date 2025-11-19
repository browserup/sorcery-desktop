use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Probe {
    pub from_process: Option<SystemTime>,
    pub from_reflog: Option<SystemTime>,
    pub from_uncommitted: Option<SystemTime>,
    pub from_fs: Option<SystemTime>,
    pub last_active: Option<SystemTime>,
}

impl Probe {
    pub fn compute_last_active(&mut self) {
        self.last_active = [
            self.from_process,
            self.from_reflog,
            self.from_uncommitted,
            self.from_fs,
        ]
        .into_iter()
        .flatten()
        .max();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceActivity {
    pub last_active: SystemTime,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMruData {
    pub workspaces: HashMap<PathBuf, WorkspaceActivity>,
}
