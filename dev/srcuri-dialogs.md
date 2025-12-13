# srcuri:// Interactive Dialogs & Error Handling

This document describes the user-facing dialogs and error messages shown by the srcuri:// protocol handler.

---

## Chooser Dialogs

### Multiple File Matches (Partial Path)

When a partial path matches multiple files across workspaces, the user must choose which file to open.

**Trigger:** `srcuri://main.rs:50` finds multiple files named `main.rs`

**Dialog:**

```
┌────────────────────────────────────────────────┐
│ Multiple files found. Choose one:              │
│                                                 │
│ [1] frontend/src/main.rs                       │
│ [2] backend/src/main.rs                        │
│ [3] tools/cli/src/main.rs                      │
│                                                 │
│ [0] Cancel                                      │
└────────────────────────────────────────────────┘
```

**Behavior:**
- Files are listed with workspace-relative paths for clarity
- Selection opens the file at the specified line/column
- Cancel aborts the operation

---

### Revision Path Options

When opening a file at a specific git revision (commit, branch, or tag), the user chooses how to access it.

**Trigger:** `srcuri://myapp/src/main.rs:42?commit=abc123`

**Dialog:**

```
┌─────────────────────────────────────────────────┐
│ Open file from commit abc123?                   │
│                                                  │
│ Options:                                         │
│  [1] View in temporary file (read-only)         │
│  [2] Checkout commit abc123 (working tree clean)│
│  [3] Cancel                                      │
└─────────────────────────────────────────────────┘
```

**Options Explained:**

1. **View in temporary file** - Always available. Extracts the file content at that revision to a temp file and opens it read-only. Safe, no changes to working tree.

2. **Checkout reference** - Only available if working tree is clean (no uncommitted changes). Switches the repository to the specified commit/branch/tag.

3. **Cancel** - Abort the operation.

---

### Auto-Switching Behavior

In certain scenarios, the protocol handler may automatically switch branches without showing a dialog:

**Conditions (all must be true):**
- Working tree is clean (no uncommitted changes)
- Target is a branch (not commit or tag)
- Branch exists locally
- User has enabled auto-switch in settings

When auto-switch is enabled and conditions are met, the handler silently checks out the branch and opens the file.

---

## Security Dialogs

### File Outside Workspace

When an absolute path references a file outside all configured workspaces, confirmation is required.

**Trigger:** `srcuri:///etc/passwd:1` (not in any workspace)

**Dialog:**

```
┌────────────────────────────────────────────────┐
│ Open file outside configured workspaces?       │
│                                                 │
│ Path: /etc/passwd                              │
│                                                 │
│ This file is not in any of your configured    │
│ workspaces. Opening files outside workspaces   │
│ may be a security risk.                        │
│                                                 │
│ [1] Open anyway                                │
│ [2] Cancel                                     │
└────────────────────────────────────────────────┘
```

**When dialog is NOT shown:**
- File path is within a configured workspace (even if accessed via absolute path)

---

## Error Messages

### File Not Found

**Cause:** Referenced file doesn't exist at resolved path

**Example:** `srcuri://myproject/missing.rs:10`

```
┌────────────────────────────────────────────────┐
│ Error: File Not Found                          │
│                                                 │
│ Could not find file:                           │
│   myproject/missing.rs                         │
│                                                 │
│ Resolved to:                                   │
│   /Users/alice/code/myproject/missing.rs       │
│                                                 │
│ Please verify:                                 │
│  • The file path is correct                   │
│  • The file exists in your workspace          │
│  • You have read permissions                  │
└────────────────────────────────────────────────┘
```

---

### Unknown Workspace

**Cause:** Workspace name not found in configuration

**Example:** `srcuri://unknown-project/file.rs:10`

```
┌────────────────────────────────────────────────┐
│ Error: Unknown Workspace                       │
│                                                 │
│ Workspace 'unknown-project' is not configured. │
│                                                 │
│ Configured workspaces:                         │
│  • myproject                                   │
│  • backend                                     │
│  • docs                                        │
│                                                 │
│ To add this workspace:                         │
│  1. Open srcuri:// settings                    │
│  2. Add workspace mapping                     │
│  3. Try link again                            │
└────────────────────────────────────────────────┘
```

---

### No Matches for Partial Path

**Cause:** Partial path doesn't match any files in workspaces

**Example:** `srcuri://nonexistent.txt:1`

```
┌────────────────────────────────────────────────┐
│ Error: No Matching Files                       │
│                                                 │
│ No file named 'nonexistent.txt' found in any   │
│ configured workspace.                          │
│                                                 │
│ Searched workspaces:                           │
│  • myproject (/Users/alice/code/myproject)    │
│  • backend (/Users/alice/code/backend)        │
│  • docs (/Users/alice/code/docs)              │
│                                                 │
│ Suggestions:                                   │
│  • Check the filename spelling                │
│  • Use workspace path format instead          │
│  • Ensure file exists in a workspace          │
└────────────────────────────────────────────────┘
```

---

### Invalid Git Reference

**Cause:** Git commit/branch/tag doesn't exist in repository

**Example:** `srcuri://myrepo/file.rs:10?commit=invalid123`

```
┌────────────────────────────────────────────────┐
│ Error: Invalid Git Reference                   │
│                                                 │
│ Could not find commit 'invalid123' in          │
│ repository at:                                 │
│   /Users/alice/code/myrepo                     │
│                                                 │
│ This may mean:                                 │
│  • The commit doesn't exist                   │
│  • The commit isn't fetched locally           │
│  • The SHA is incorrect                       │
│                                                 │
│ Try:                                           │
│  • git fetch --all                            │
│  • Verify commit SHA                          │
│  • Check if commit is on a remote branch      │
└────────────────────────────────────────────────┘
```

---

### Working Tree Dirty

**Cause:** Attempted checkout with uncommitted changes

**Example:** User selected "Checkout branch" for `srcuri://myrepo/file.rs:10?branch=feature`

```
┌────────────────────────────────────────────────┐
│ Error: Cannot Checkout (Uncommitted Changes)   │
│                                                 │
│ Your working tree has uncommitted changes.     │
│ Cannot checkout 'feature' branch.              │
│                                                 │
│ Uncommitted changes:                           │
│  M src/main.rs                                 │
│  M README.md                                   │
│  ?? new-file.txt                               │
│                                                 │
│ Options:                                       │
│  1. Commit your changes: git commit -am "..."  │
│  2. Stash your changes: git stash             │
│  3. View file in temporary location (safe)    │
└────────────────────────────────────────────────┘
```

---

### Not a Git Repository

**Cause:** Git reference used with non-git workspace

**Example:** `srcuri://myproject/file.rs:10?commit=abc123`

```
┌────────────────────────────────────────────────┐
│ Error: Not a Git Repository                    │
│                                                 │
│ Workspace 'myproject' is not a git repository. │
│ Git references require a git repository.       │
│                                                 │
│ Workspace path:                                │
│   /Users/alice/code/myproject                  │
│                                                 │
│ To fix:                                        │
│  • Use URL without git reference              │
│  • Initialize git: git init                   │
│  • Clone repository instead of copying files  │
└────────────────────────────────────────────────┘
```

---

### Malformed URL

**Cause:** URL doesn't follow srcuri:// protocol syntax

**Example:** `srcuri:/malformed-url`

```
┌────────────────────────────────────────────────┐
│ Error: Malformed URL                           │
│                                                 │
│ The URL does not follow srcuri:// protocol      │
│ syntax:                                        │
│   srcuri:/malformed-url                        │
│                                                 │
│ Expected formats:                              │
│  • srcuri://workspace/path:line:col            │
│  • srcuri:///absolute/path:line:col           │
│  • srcuri://filename:line:col                  │
│                                                 │
│ See documentation for details:                 │
│  https://srcuri.org/docs/url-format            │
└────────────────────────────────────────────────┘
```

---

### Permission Denied

**Cause:** User lacks permissions to read file

**Example:** `srcuri:///etc/shadow:1`

```
┌────────────────────────────────────────────────┐
│ Error: Permission Denied                       │
│                                                 │
│ Cannot read file:                              │
│   /etc/shadow                                  │
│                                                 │
│ You do not have permission to access this file.│
│                                                 │
│ This may require:                              │
│  • Root/administrator privileges              │
│  • File permission changes                    │
│  • Ownership changes                          │
│                                                 │
│ Use 'ls -l /etc/shadow' to view permissions.  │
└────────────────────────────────────────────────┘
```

---

### Path Traversal Detected

**Cause:** URL attempts to escape workspace boundaries via `..`

**Example:** `srcuri://myproject/../../../etc/passwd:1`

```
┌────────────────────────────────────────────────┐
│ Security Warning: Path Traversal Detected      │
│                                                 │
│ The URL attempts to access a file outside      │
│ the workspace boundary:                        │
│                                                 │
│ Requested: myproject/../../../etc/passwd       │
│ Resolved:  /etc/passwd                         │
│ Workspace: /Users/alice/code/myproject         │
│                                                 │
│ This may be a malicious link.                  │
│                                                 │
│ [1] Open anyway (not recommended)              │
│ [2] Cancel                                     │
└────────────────────────────────────────────────┘
```
