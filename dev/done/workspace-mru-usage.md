# Workspace MRU Usage Guide

## Overview

The Workspace MRU (Most Recently Used) tracker helps intelligently select the correct workspace when multiple workspaces contain a matching file path. This is critical for path normalization, where we need to convert partial paths or full paths into the actual local file the user wants to edit.

---

## Architecture

### Components

- **ActiveWorkspaceTracker** - Background service that polls workspaces every 20 seconds
- **WorkspaceMruData** - Persistent cache of workspace activity timestamps
- **PathMatcher** - Uses MRU data to sort workspace matches by recency

### Data Flow

```
Workspaces (from settings)
    ↓
ActiveWorkspaceTracker (polls every 20s)
    ↓
Probes each workspace:
  - Running processes in workspace?
  - Git reflog activity?
  - Uncommitted changes?
  - Filesystem changes?
    ↓
Computes last_active = max(all signals)
    ↓
Saves to workspace_mru.yaml
    ↓
PathMatcher queries MRU data
    ↓
Sorts matches by last_active (most recent first)
```

---

## Getting the Most Recent Workspace

### Step 1: Access the Workspace Tracker

The `ActiveWorkspaceTracker` is managed by Tauri and available throughout the application:

```rust
use crate::workspace_mru::ActiveWorkspaceTracker;
use std::sync::Arc;

// In a Tauri command handler or service:
let workspace_tracker: Arc<ActiveWorkspaceTracker> = /* from dependency injection */;
```

### Step 2: Query for a Specific Workspace's Last Active Time

```rust
use std::path::PathBuf;
use std::time::SystemTime;

let workspace_path = PathBuf::from("/Users/bob/src/myproject");
let last_active: Option<SystemTime> = workspace_tracker
    .get_workspace_last_active(&workspace_path)
    .await;

if let Some(time) = last_active {
    println!("Workspace last active: {:?}", time);
} else {
    println!("No activity data for this workspace");
}
```

### Step 3: Get All Workspace MRU Data

```rust
use crate::workspace_mru::WorkspaceMruData;

let mru_data: WorkspaceMruData = workspace_tracker.get_mru_data().await;

// Iterate through all tracked workspaces
for (workspace_path, activity) in &mru_data.workspaces {
    println!("{}: {:?}", workspace_path.display(), activity.last_active);
}
```

### Step 4: Sort Multiple Matches by MRU

When you have multiple workspace matches for a path, use the `PathMatcher`:

```rust
use crate::protocol_handler::matcher::{PathMatcher, WorkspaceMatch};

let matcher = PathMatcher::new(settings_manager, workspace_tracker);

// Find all workspaces that contain this partial path
let mut matches: Vec<WorkspaceMatch> = matcher
    .find_partial_matches("app/models/user.rb")
    .await?;

// Sort by most recently used workspace first
matcher.sort_by_recent_usage(&mut matches).await;

// First match is now the most recently active workspace
if let Some(best_match) = matches.first() {
    println!("Opening: {}", best_match.full_file_path.display());
    println!("From workspace: {}", best_match.workspace_name);
}
```

---

## Integration with Path Normalization

### Use Case: Partial Path Resolution

When a user clicks a path fragment like `app/models/account.rb`, the system needs to find which workspace contains this file.

**Problem:** Multiple workspaces might have `app/models/account.rb`

**Solution:** Use MRU to pick the workspace the user was most recently working in

```rust
async fn resolve_partial_path(
    path: &str,
    matcher: &PathMatcher,
) -> Result<PathBuf> {
    let mut matches = matcher.find_partial_matches(path).await?;

    if matches.is_empty() {
        bail!("File '{}' not found in any workspace", path);
    }

    if matches.len() == 1 {
        // Only one match, use it
        return Ok(matches[0].full_file_path.clone());
    }

    // Multiple matches - sort by MRU
    matcher.sort_by_recent_usage(&mut matches).await;

    // Return the most recently active workspace's match
    Ok(matches[0].full_file_path.clone())
}
```

### Use Case: Full Path Alignment

When given a server path like `/var/www/browserup/current/app/models/user.rb`, we need to:
1. Extract the workspace name (`browserup`)
2. Strip the deployment path prefix
3. Find matching workspaces
4. Prefer the most recently used one

```rust
async fn normalize_server_path(
    full_path: &str,
    matcher: &PathMatcher,
) -> Result<PathBuf> {
    // Find workspaces that match this path
    let mut matches = matcher.find_full_path_matches(full_path).await?;

    if matches.is_empty() {
        bail!("No workspace found for path: {}", full_path);
    }

    // Sort by MRU - most active workspace first
    matcher.sort_by_recent_usage(&mut matches).await;

    // Use the most recently active match
    Ok(matches[0].full_file_path.clone())
}
```

---

## How the Tracker Works

### Background Polling (20-second cycle)

1. **Load workspaces** from settings
2. **Refresh process list** (using `sysinfo`)
3. **For each workspace**, collect signals:
   - Check if any running process has CWD in workspace → `now()`
   - Query Git HEAD reflog → timestamp of last checkout/commit/rebase
   - Run `git status` and get mtimes of uncommitted files
   - Scan filesystem: root + common dirs (`src/`, `app/`, etc.) up to 400 files, depth ≤ 2
4. **Compute** `last_active = max(all signal timestamps)`
5. **Save** results to `~/.config/sorcery-desktop/workspace_mru.yaml`
6. **Sleep** 20 seconds (naturally adapts if probing takes longer)

### Signal Priority (by strength)

1. **Process presence** - If a process is running in the workspace, assume it's active NOW
2. **Git reflog** - Captures actions even if files are old (e.g., checking out an old commit)
3. **Uncommitted changes** - Recent edits not yet committed
4. **Filesystem mtimes** - Catches non-Git activity or when Git is unavailable

**All signals are equal** - the highest timestamp wins (no fallbacks, no special casing)

---

## Data Persistence

### Location
```
~/.config/sorcery-desktop/workspace_mru.yaml
```

### Format
```yaml
workspaces:
  /Users/bob/src/project1:
    last_active: !SystemTime { secs_since_epoch: 1729395742, nanos_since_epoch: 123456789 }
  /Users/bob/src/project2:
    last_active: !SystemTime { secs_since_epoch: 1729392142, nanos_since_epoch: 987654321 }
```

### Lifecycle
- **Loaded** on application startup
- **Updated** every 20 seconds after polling cycle
- **Persisted** after each update

---

## API Reference

### ActiveWorkspaceTracker

```rust
impl ActiveWorkspaceTracker {
    /// Create a new tracker instance
    pub fn new(settings_manager: Arc<SettingsManager>) -> Self;

    /// Load persisted MRU data from disk
    pub async fn load(&self) -> Result<()>;

    /// Start the background polling loop (runs indefinitely)
    pub async fn start_polling(self: Arc<Self>);

    /// Get the last active time for a specific workspace
    pub async fn get_workspace_last_active(&self, workspace_path: &PathBuf) -> Option<SystemTime>;

    /// Get all MRU data (for debugging/testing)
    pub async fn get_mru_data(&self) -> WorkspaceMruData;
}
```

### PathMatcher

```rust
impl PathMatcher {
    /// Create a new path matcher
    pub fn new(
        settings_manager: Arc<SettingsManager>,
        workspace_tracker: Arc<ActiveWorkspaceTracker>
    ) -> Self;

    /// Find all workspaces containing this partial path
    pub async fn find_partial_matches(&self, partial_path: &str) -> Result<Vec<WorkspaceMatch>>;

    /// Find workspace by name and relative path
    pub async fn find_workspace_path(&self, workspace_name: &str, relative_path: &str) -> Result<PathBuf>;

    /// Find workspaces matching a full server path
    pub async fn find_full_path_matches(&self, full_path: &str) -> Result<Vec<WorkspaceMatch>>;

    /// Sort matches by most recently used workspace first
    pub async fn sort_by_recent_usage(&self, matches: &mut Vec<WorkspaceMatch>);
}
```

### WorkspaceMatch

```rust
pub struct WorkspaceMatch {
    pub workspace_name: String,       // Display name
    pub workspace_path: PathBuf,      // Root path on disk
    pub full_file_path: PathBuf,      // Complete path to the file
    pub last_active: Option<SystemTime>, // When workspace was last active
}
```

---

## Common Patterns

### Pattern 1: Simple MRU Selection

```rust
let mut matches = matcher.find_partial_matches("README.md").await?;
matcher.sort_by_recent_usage(&mut matches).await;
let best = matches.first().unwrap();
```

### Pattern 2: MRU with User Override

```rust
let mut matches = matcher.find_partial_matches(path).await?;
matcher.sort_by_recent_usage(&mut matches).await;

if matches.len() > 1 {
    // Show user a chooser, but pre-select the first (MRU) option
    show_workspace_chooser(matches, default_index: 0).await
} else {
    open_file(&matches[0].full_file_path).await
}
```

### Pattern 3: Debugging Activity

```rust
let mru_data = workspace_tracker.get_mru_data().await;
let mut workspaces: Vec<_> = mru_data.workspaces.iter().collect();
workspaces.sort_by_key(|(_, activity)| std::cmp::Reverse(activity.last_active));

for (path, activity) in workspaces {
    println!("{}: {:?}", path.display(), activity.last_active);
}
```

---

## Configuration

### Hardcoded Limits (No User Configuration)

- **Polling interval:** 20 seconds
- **Filesystem scan cap:** 400 entries maximum
- **Filesystem scan depth:** ≤ 2 levels
- **Directory allowlist:** `["src", "app", "lib", "packages", "test", "spec", "include", "bin", "scripts"]`

These values are intentionally not configurable to keep the system simple and predictable.

---

## Error Handling

### Git Errors

When Git operations fail (repository not found, not a Git repo, libgit2 errors):
- Git signals return `None`
- Filesystem and process signals still work
- No error is raised to the user
- System continues normally with available signals

### Missing Workspaces

When a workspace path doesn't exist or is inaccessible:
- Warning logged: `"Failed to probe workspace: ..."`
- Workspace skipped in current cycle
- Other workspaces continue to be tracked
- MRU data for missing workspace remains until next successful probe

### Permission Errors

When filesystem access is denied:
- That specific signal returns `None`
- Other signals (Git, process) may still succeed
- Partial data is better than no data

---

## Testing Considerations

### Unit Tests

Each signal type has tests:
- `test_process_detection_current_dir()` - Detects current process
- `test_head_reflog_time()` - Reads Git reflog
- `test_fs_recent_mtime_temp_dir()` - Scans filesystem
- `test_probe_workspace_current_dir()` - Combines all signals

### Integration Testing

To test MRU behavior:
1. Create multiple workspace configs pointing to test directories
2. Touch files in one workspace
3. Wait 20+ seconds for polling cycle
4. Query MRU data and verify correct workspace is most recent

### Manual Testing

```bash
# Watch the MRU data update
watch -n 2 cat ~/.config/sorcery-desktop/workspace_mru.yaml

# Check logs for polling activity
tail -f ~/.config/sorcery-desktop/logs/sorcery.log | grep -i workspace
```

---

## Future Enhancements (Out of Scope)

These are **not** implemented but could be added later:

- **Tie-breaking heuristics** - When two workspaces have identical timestamps
- **User-specified weights** - Prefer certain signal types over others
- **Configurable polling interval** - Let users adjust the 20s default
- **Manual workspace pinning** - Force a workspace to always be preferred
- **Activity history** - Track more than just last_active (trending, frequency)

---

## See Also

- `dev/path-normalization.md` - Overview of path normalization scenarios
- `dev/workspace-mru-spec.md` - Full implementation specification
- `src-tauri/src/workspace_mru/` - Implementation source code
