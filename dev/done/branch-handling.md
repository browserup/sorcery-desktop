I want to make the app handle settings that are not the current branch. My principles for this are:

We should offer the user a choice. I'd like to make the design clean, and spare. The UI should come from our
local installation.

When a link with commit, branch, sha, or tag is passed, we should check if we can switch to it safely (no uncommitted changes)
We should display a dialog, with several button choices. Chosing(clicking) one should close the dialog and take the action.
We should put a small bit of text to describe each action next to it.

The user can:

* switch to the branch automatically, if they have no uncommited changes. This should require a setting. In this case, we should
  flash a message that we are switching to the branch.

If the user has uncommitted changes, we should provide a UI choice, with a few UI buttons:

One for:

[Go to Branch] (disabled until they clean up)
* Your branch (name) has uncommitted changes. Clean up to check out.

When it’s disabled show why inline (like a red “2 files modified: ...”)

Poll every second, and if it is rectified, enable.




[Checkout in Git WorkTree] (non-destructive)

We create the worktree. We should use a default worktree directory in our install directory. We should offer a setting to let the     
user change the default. We should default to using this, but let the user change the path.


[Ignore SHA]
Ignore the selected Branch/SHA/Tag and go to the file in your current branch.

[Cancel]

I also want to use this same dialog for when a link points to an unmapped file. If the user has said to
allow unmapped URLs in the settings, they should be presented with the dialog showing them what file will
be opened, and be able to Open or Cancel. We just shouldn't open it automatically.

If the link is stale (missing in current version), we should indicate that.

If there are conflicts, the most-specific thing wins.

If there are multiple matching workspaces, we should also offer a "choose workspace" dialog.
That dialog should precede this dialog, which is more workspace specific.

What do we do if git checkout <branch> fails (branch doesn’t exist locally, remote missing, detached HEAD, shallow clone)? We should fall back to “Worktree” automatically, or surface the error in the dialog.

If the repo is bare or not actually Git, we should sshow “This workspace doesn’t support branch switching”

If we can’t create the worktree (permissions, path exists), we show a second dialog with the error so the user knows, but we
give up from that point.



---
Feature Specification: Sorcery Smart Link Handler
Product: Sorcery Feature: srcuri:// Deep Link Handling with Intelligent Git Context Switching Version: 1.0

1. Overview
   Sorcery acts as a system-wide URI handler for srcuri:// links. Its primary goal is to open a specific file at a specific line number in the user's preferred editor.

The Problem: The user's local repository may be on a different branch or have uncommitted changes compared to the link target. The Solution: Sorcery acts as a gatekeeper. It prioritizes user state preservation. It will never force a stash or overwrite data. It uses Git Worktrees to handle "dirty" states non-destructively.

2. URL Schema
   The application must parse URIs with the following format:

Plaintext

srcuri://<absolute_path_to_repo>/<relative_file_path>[:<line_number>][?branch=<target_branch>]
Parameters:

repo_path: Local absolute path to the Git repository root.

file_path: Path to the file relative to the repo root.

line_number: (Optional) Line to focus.

target_branch: (Optional) The Git branch name (or commit hash) the file should be viewed from.

3. Configuration & Constants
   Worktree Storage Location: ~/.sorcery/worktrees/

Note: Ensure this directory structure exists.

Max Concurrent Worktrees: 3 (LRU Strategy).

Dependency Policy: "Lightweight Mode" (Do NOT automate npm install or similar).

Editor Launch Command: The existing logic used by Sorcery to open files in external editors (VS Code, Vim, etc.).

4. Logic Flow (The Algorithm)
   When a srcuri event is received, execute the following logic sequence:

Step 1: Validation & Diagnosis
Run the following git commands in repo_path:

Get Current Branch: git rev-parse --abbrev-ref HEAD

Check Dirty State: git status --porcelain (If output length > 0, state is DIRTY).

Check Existing Worktrees: git worktree list --porcelain

Step 2: Decision Tree
Scenario A: Exact Match

Condition: Current Branch == target_branch.

Action: Launch Editor immediately at repo_path/file_path.

Scenario B: Existing Worktree Match

Condition: The git worktree list shows a path managed by Sorcery that is already on target_branch.

Action: Launch Editor immediately at <existing_worktree_path>/file_path.

Scenario C: Clean State Switch (The "Easy" Switch)

Condition: Current Branch != target_branch AND State is CLEAN.

Action: Display Dialog 1 (Switch Branch).

If User Confirms: Execute git checkout <target_branch> in repo_path. Launch Editor.

If User Cancels: Abort.

Scenario D: Dirty State (The "Worktree" Fallback)

Condition: Current Branch != target_branch AND State is DIRTY.

Action: Display Dialog 2 (Create Worktree).

If User Confirms: Proceed to Section 5: Worktree Creation Logic.

If User Cancels: Abort.

5. Worktree Creation Logic
   If the workflow reaches this step, we are creating a new temporary environment to avoid stashing user changes.

Enforce LRU Cap:

Scan ~/.sorcery/worktrees/<project_hash>/.

If count >= Max Concurrent Worktrees (3), identify the folder with the oldest last_accessed timestamp (or directory creation time).

Delete that folder and run git worktree prune in the main repo.

Determine Path:

Generate path: ~/.sorcery/worktrees/<project_name_safe>/<target_branch_safe>/

Execute Creation (With Fallback):

Attempt 1 (Standard): git worktree add <new_path> <target_branch>

Error Handling: If standard attempt fails with "already checked out" (Git locking error):

Resolve the generic Commit Hash of <target_branch>.

Attempt 2 (Detached): git worktree add --detach <new_path> <commit_hash>

Launch:

Launch Editor at <new_path>/file_path.

Notification:

Show Toast: "Opened in Read-Only Mode. Dependencies not installed."

6. User Interface (Dialogs)
   Dialog 1: Switch Branch (Clean State)

Title: Switch Branch? Body: You are currently on [Current Branch]. This file is on [Target Branch]. Since your working directory is clean, we can switch branches automatically. Buttons: [Switch & Open] [Cancel]

Dialog 2: Create Worktree (Dirty State)

Title: Unsaved Changes Detected Body: You have unsaved changes on [Current Branch]. To view [Target Branch] without disrupting your work, Sorcery can open this file in a separate, temporary window (Git Worktree). Buttons: [Open in New Window] [Cancel]

7. Edge Case Handling
   Branch Does Not Exist Locally:

If target_branch is not found locally, attempt git fetch origin <target_branch> before running decision logic.

Config Files:

Do not copy .env or untracked configuration files to the new worktree. The environment must remain strict/clean to prevent accidental database writes or secret leaks.

8. Git Command Reference
   Check Branch: git rev-parse --abbrev-ref HEAD

Check Dirty: git status --porcelain

List Worktrees: git worktree list

Prune Stale Entries: git worktree prune

Add Worktree: git worktree add [path] [branch]

Add Detached: git worktree add --detach [path] [commit-hash]

---

## Implementation Status

### Implemented Features

**URL Parsing (`src-tauri/src/protocol_handler/parser.rs`)**
- Parses `?branch=`, `?commit=`, `?sha=`, `?tag=` for git references
- Parses `?remote=` for clone-on-demand (see below)
- Line and column support: `file.rs:42:7`

**Branch/Revision Handling (`src-tauri/src/protocol_handler/mod.rs`)**
- Auto-switch when working tree is clean (controlled by `auto_switch_clean_branches` setting)
- Shows revision dialog when dirty or when auto-switch is disabled
- Dialog polls working tree status every second

**Revision Dialog (`public/revision-handler.html`)**
- "Switch & Open" - checkout branch and open file (enabled when clean)
- "Fetch & Switch" - fetch from origin, then checkout (for remote branches not yet fetched)
- "Open in Worktree" - create git worktree for non-destructive access
- "Ignore & Open" - open file on current branch, ignoring requested ref

**Worktree Management (`src-tauri/src/protocol_handler/git.rs`, `src-tauri/src/commands/mod.rs`)**
- `create_worktree_and_open` command wired to revision dialog button
- Creates worktrees in `~/.sorcery/worktrees/<project>/<branch>/`
- LRU enforcement: max 3 worktrees per project, oldest removed automatically
- Reuses existing worktrees when available
- Detached HEAD fallback when branch is already checked out elsewhere
- Prunes stale worktree entries

**Settings (`src-tauri/src/settings/models.rs`, `public/settings.html`)**
- `auto_switch_clean_branches` - toggle auto-switch behavior (default: true)
- `repo_base_dir` - base directory for cloning repos (default: `~/code`)

---

## Clone-on-Demand Feature

### Overview

When a `srcuri://` link includes a `?remote=` parameter and the workspace isn't found locally, Sorcery offers to clone the repository automatically.

### URL Format

```
srcuri://workspace/path/to/file.rs:42?remote=github.com/user/repo
srcuri://myproject/src/main.rs?branch=feature&remote=github.com/user/myproject
```

The `remote` parameter specifies where to clone from if the workspace doesn't exist locally.

### Flow

1. User clicks link with `?remote=` parameter
2. Workspace lookup fails (not configured)
3. Clone dialog appears showing:
   - Remote URL
   - Clone destination (based on `repo_base_dir` setting)
   - File to open after clone
   - Branch/ref if specified
4. User confirms → repository is cloned
5. New workspace mapping is automatically added to settings
6. File opens in editor
7. Future links to this workspace work without cloning

### Implementation

| Component | File | Purpose |
|-----------|------|---------|
| URL Parser | `parser.rs` | Extracts `remote` param, adds to `WorkspacePath` and `RevisionPath` |
| Settings | `models.rs` | `repo_base_dir` setting for clone destination |
| Handler | `protocol_handler/mod.rs` | Returns `ShowCloneDialog` when workspace not found but remote available |
| Git | `git.rs` | `clone_repo()` function with optional branch |
| Commands | `commands/mod.rs` | `clone_and_open()` clones, adds workspace, opens file |
| Dialog | `clone-dialog.html` | Confirmation UI with loading state |
| Main | `main.rs` | Wires up dialog and commands |

### Settings UI

The "Repository Cloning" section in Settings allows users to configure the default clone directory (`repo_base_dir`).

### Example Usage

Share a link that works even if the recipient hasn't cloned the repo:

```
srcuri://sorcery-desktop/src/main.rs:50?remote=github.com/user/sorcery-desktop
```

If they don't have `sorcery-desktop` configured, they'll be prompted to clone it to `~/code/sorcery-desktop`.