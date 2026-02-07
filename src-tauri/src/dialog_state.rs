use crate::protocol_handler::{GitRef, WorkspaceMatch};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub fn git_ref_display(git_ref: &GitRef) -> String {
    match git_ref {
        GitRef::Branch(value) => value.clone(),
        GitRef::Tag(value) => format!("tag {value}"),
        GitRef::Commit(value) => format!("commit {value}"),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceChooserData {
    pub matches: Vec<WorkspaceMatch>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionDialogData {
    pub workspace: String,
    pub workspace_path: String,
    pub file_path: String,
    pub full_file_path: String,
    pub rev: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub current_ref: String,
    pub is_working_tree_clean: bool,
    pub dirty_file_count: usize,
    pub checkout_available: bool,
    pub checkout_blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneDialogData {
    pub workspace_name: String,
    pub clone_path: String,
    pub remote_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_violation: Option<String>,
    pub file_path: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub git_ref: Option<String>,
    #[serde(default = "default_clone_allowed")]
    pub clone_allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clone_validation_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_workspace_key: Option<String>,
    #[serde(skip)]
    pub git_ref_kind: Option<GitRef>,
}

const fn default_clone_allowed() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRepairDialogData {
    pub workspace_key: String,
    pub workspace_path: String,
    pub workspace_state: String,
    pub file_path: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConflictCandidateData {
    pub workspace_key: String,
    pub workspace_path: String,
    pub workspace_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_remote: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConflictDialogData {
    pub workspace_key: String,
    pub requested_remote: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_violation: Option<String>,
    pub clone_path: String,
    pub file_path: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub git_ref: Option<String>,
    pub candidates: Vec<WorkspaceConflictCandidateData>,
    #[serde(skip)]
    pub git_ref_kind: Option<GitRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeFileDialogData {
    pub file_path: String,
    pub file_size_bytes: u64,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub editor_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DangerousFileData {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DangerousSettingData {
    pub key: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustDialogData {
    pub workspace_path: String,
    pub workspace_name: String,
    pub task_labels: Vec<String>,
    pub vim_local_rc_files: Vec<String>,
    pub dangerous_files: Vec<DangerousFileData>,
    pub dangerous_settings: Vec<DangerousSettingData>,
    pub scan_error: Option<String>,
    pub pending_file_path: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub editor_hint: Option<String>,
}

pub struct DialogState {
    workspace_chooser: Mutex<Option<WorkspaceChooserData>>,
    revision_dialog: Mutex<Option<RevisionDialogData>>,
    clone_dialog: Mutex<Option<CloneDialogData>>,
    workspace_repair_dialog: Mutex<Option<WorkspaceRepairDialogData>>,
    workspace_conflict_dialog: Mutex<Option<WorkspaceConflictDialogData>>,
    large_file_dialog: Mutex<Option<LargeFileDialogData>>,
    trust_dialog: Mutex<Option<TrustDialogData>>,
}

impl DialogState {
    #[allow(clippy::missing_const_for_fn)] // Mutex::new is not const
    pub fn new() -> Self {
        Self {
            workspace_chooser: Mutex::new(None),
            revision_dialog: Mutex::new(None),
            clone_dialog: Mutex::new(None),
            workspace_repair_dialog: Mutex::new(None),
            workspace_conflict_dialog: Mutex::new(None),
            large_file_dialog: Mutex::new(None),
            trust_dialog: Mutex::new(None),
        }
    }

    pub fn set_workspace_chooser(&self, data: WorkspaceChooserData) {
        *self.workspace_chooser.lock() = Some(data);
    }

    pub fn take_workspace_chooser(&self) -> Option<WorkspaceChooserData> {
        self.workspace_chooser.lock().take()
    }

    pub fn set_revision_dialog(&self, data: RevisionDialogData) {
        *self.revision_dialog.lock() = Some(data);
    }

    pub fn take_revision_dialog(&self) -> Option<RevisionDialogData> {
        self.revision_dialog.lock().take()
    }

    pub fn set_clone_dialog(&self, data: CloneDialogData) {
        *self.clone_dialog.lock() = Some(data);
    }

    pub fn take_clone_dialog(&self) -> Option<CloneDialogData> {
        self.clone_dialog.lock().take()
    }

    pub fn peek_clone_dialog(&self) -> Option<CloneDialogData> {
        self.clone_dialog.lock().clone()
    }

    pub fn update_clone_path(&self, new_path: &str) -> bool {
        let mut guard = self.clone_dialog.lock();
        if let Some(ref mut data) = *guard {
            data.clone_path = new_path.to_string();
            true
        } else {
            false
        }
    }

    pub fn set_workspace_repair_dialog(&self, data: WorkspaceRepairDialogData) {
        *self.workspace_repair_dialog.lock() = Some(data);
    }

    pub fn take_workspace_repair_dialog(&self) -> Option<WorkspaceRepairDialogData> {
        self.workspace_repair_dialog.lock().take()
    }

    pub fn set_workspace_conflict_dialog(&self, data: WorkspaceConflictDialogData) {
        *self.workspace_conflict_dialog.lock() = Some(data);
    }

    pub fn take_workspace_conflict_dialog(&self) -> Option<WorkspaceConflictDialogData> {
        self.workspace_conflict_dialog.lock().take()
    }

    pub fn peek_workspace_conflict_dialog(&self) -> Option<WorkspaceConflictDialogData> {
        self.workspace_conflict_dialog.lock().clone()
    }

    pub fn set_large_file_dialog(&self, data: LargeFileDialogData) {
        *self.large_file_dialog.lock() = Some(data);
    }

    pub fn take_large_file_dialog(&self) -> Option<LargeFileDialogData> {
        self.large_file_dialog.lock().take()
    }

    pub fn set_trust_dialog(&self, data: TrustDialogData) {
        *self.trust_dialog.lock() = Some(data);
    }

    pub fn take_trust_dialog(&self) -> Option<TrustDialogData> {
        self.trust_dialog.lock().take()
    }
}

impl Default for DialogState {
    fn default() -> Self {
        Self::new()
    }
}
