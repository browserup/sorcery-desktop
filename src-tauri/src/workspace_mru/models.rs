use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceActivity {
    pub last_seen: SystemTime,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMruData {
    pub workspaces: HashMap<PathBuf, WorkspaceActivity>,
}
