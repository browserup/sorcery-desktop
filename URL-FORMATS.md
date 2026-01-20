# Sorcery Protocol Format Guide

## Overview

Sorcery Desktop uses the `srcuri://` protocol (also known as the Sorcery protocol) to open files in your configured editor. This protocol enables editor-agnostic code linking that works across teams and platforms.

**Note:** The protocol uses `srcuri://` (NOT `file://`) as its URL scheme. For the complete protocol specification, visit [srcuri.com](https://srcuri.com).

## URL Format

```
srcuri://<authority>/<path>:<line>:<column>
```

The **authority** determines how the link is interpreted:
- A **workspace name** (most common) — opens file relative to that workspace
- A **reserved token** (`wks`, `rel`, `any`, `abs`, `ext`) — explicit mode

## Five Link Modes

### 1. Workspace Mode (Implicit) — Recommended

**Format:** `srcuri://<workspace>/<path>:<line>:<column>`

The authority IS the workspace name. This is the shortest and most common format.

**Examples:**
```
srcuri://sorcery/README.md:1
srcuri://sorcery/src-tauri/src/main.rs:50
srcuri://myproject/src/App.tsx:100:5
```

**When to use:**
- Sharing links with team members
- Different developers have different file system paths
- Cross-platform sharing (macOS, Windows, Linux)
- Best practice for team collaboration

**How it works:**
1. You configure workspace mappings in settings
2. Each developer maps `sorcery` to their local clone path
3. URLs work for everyone regardless of where they cloned the repo

**Strict behavior:** If the workspace is not configured, Sorcery shows an error. It does NOT search other workspaces or fall back to absolute paths. Use Match Mode for flexible cross-workspace search.

**Example workspace mapping:**
```json
{
  "workspaces": {
    "sorcery": "/Users/ebeland/apps/sorcery",
    "myproject": "/Users/ebeland/projects/myproject"
  }
}
```

### 2. Workspace Mode (Explicit)

**Format:** `srcuri://wks/<workspace>/<path>:<line>:<column>`

Uses the `wks` reserved authority for explicit clarity.

**Examples:**
```
srcuri://wks/sorcery/README.md:1
srcuri://wks/myproject/src/App.tsx:100:5
```

**When to use:**
- When you want maximum clarity about intent
- Both implicit and explicit forms work identically

### 3. Relative Mode (Search All Workspaces)

**Format:** `srcuri://rel/<path>:<line>:<column>`

Uses the `rel` reserved authority to search all workspaces.

**Examples:**
```
srcuri://rel/README.md:1
srcuri://rel/main.rs:50
srcuri://rel/App.tsx:100
srcuri://rel/src/utils.py:10?workspaceHint=backend
```

**When to use:**
- Quick access to common files
- File exists in only one workspace
- Don't remember the workspace name

**How it works:**
1. **Workspace name detection**: If the path contains a workspace name as a segment (case-insensitive), Sorcery extracts the relative path and resolves it in that workspace
2. If `?workspaceHint=<name>` is provided, that workspace is prioritized
3. Otherwise, searches all configured workspaces for matching files
4. If one match found, opens it immediately
5. If multiple matches found, shows chooser dialog (sorted by most recently used)
6. If no matches found, shows error

**Cross-platform path matching:**
```
# Path from Windows that contains workspace name
srcuri://rel/D:/Code/myproject/src/main.rs:42

# Sorcery detects "myproject" in the path and extracts "src/main.rs"
# Opens: ~/code/myproject/src/main.rs (your local path)
```

### 4. Any Mode (Best-Effort)

**Format:** `srcuri://any/<path>:<line>:<column>`

Best-effort resolution when the source does not know whether the path is workspace-relative, search-relative, or absolute.

**Examples:**
```
srcuri://any/src/main.rs:42
srcuri://any/Users/ebeland/apps/sorcery/README.md:50
```

**Resolution order:** workspace, relative, absolute (applicable steps only).

### 5. Absolute Path Mode

**Format:** `srcuri://abs/<path>:<line>:<column>`

Uses the `abs` reserved authority for absolute filesystem paths.

**Examples:**
```
srcuri://abs/etc/hosts:1
srcuri://abs/Users/ebeland/apps/sorcery/README.md:50
srcuri://abs/Users/ebeland/apps/sorcery/src-tauri/src/main.rs:100:5
srcuri://abs/tmp/test.txt:42:10
```

**Note:** For POSIX paths, the leading `/` is implied. `abs/etc/hosts` → `/etc/hosts`

**Windows paths:**
```
srcuri://abs/C:/Users/user/file.txt:1
srcuri://abs/UNC/server/share/file.txt:5
```

**When to use:**
- You know the exact file system path
- Sharing links within the same machine
- Testing specific files
- No workspace mapping needed

### 6. External Mode (Provider URLs)

**Format:** `srcuri://ext/<scheme>/<host>/<path>#<fragment>`

Uses the `ext` reserved authority to embed provider URLs (GitHub, GitLab, etc.).

**Examples:**
```
srcuri://ext/https/github.com/owner/repo/blob/main/src/lib.rs#L42
srcuri://ext/https/gitlab.com/org/project/-/blob/main/README.md#L10
```

**When to use:**
- Sharing links that work with or without Sorcery installed
- Converting GitHub/GitLab URLs to open in your editor

**How it works:**
1. If you have a matching local workspace (by git remote), opens locally
2. Otherwise, opens srcuri.com interstitial with fallback options

## Line and Column Numbers

All formats support optional line and column numbers:

```
srcuri://myproject/path                    # Just open the file
srcuri://myproject/path:50                 # Open at line 50
srcuri://myproject/path:50:10              # Open at line 50, column 10
```

**Note:** Column numbers are limited to 0-120. Higher values are ignored.

## Opening Folders/Directories

srcuri:// links can also open folders in editors that support it.

### Folder Examples

**Absolute path to folder:**
```
srcuri://abs/Users/alice/projects/myapp
srcuri://abs/home/bob/code/backend/src
```

**Workspace-relative folder:**
```
srcuri://myproject/src/controllers
srcuri://backend/lib
```

### Editor Folder Support

Most editors (22 of 26) support opening folders:

**Supported:** VS Code, Cursor, VSCodium, all JetBrains IDEs, Sublime Text, Zed, Xcode, Vim, Neovim, Emacs

**Not Supported:** Kate, Kakoune, Micro, Nano (file-only editors)

### Line/Column on Folders

Line and column numbers are **silently ignored** when opening folders:

```
srcuri://myproject/src:42:10
→ Opens the src folder (line 42, column 10 are ignored)
```

This allows links to be refactored from files to folders without breaking.

## Query Parameters

### Git References

Open a file at a specific git reference (commit, branch, or tag):

#### Commit SHA

```
srcuri://sorcery/README.md:1?commit=abc123def
srcuri://sorcery/src/main.rs:50?sha=abc123def
```

- Most precise - immutable reference to exact code state
- `sha=` is an alias for `commit=`
- Can use short or full SHA

#### Branch

```
srcuri://myproject/src/app.js:10?branch=main
srcuri://myproject/lib/utils.rs:25?branch=feature-auth
```

- Opens file at your current local branch state
- If your branch is behind, shows helpful message
- Useful for current development references

#### Tag

```
srcuri://myproject/file.txt:10?tag=v1.0.0
srcuri://myproject/CHANGELOG.md:1?tag=release-2024
```

- Immutable reference to tagged version
- Clear error if tag doesn't exist locally

**Requirements:**
- Must use workspace mode (not absolute path)
- Workspace must be a git repository
- Shows dialog with options to:
  - View in temporary file
  - Checkout the reference (if working tree is clean)

### Remote (Clone-on-Demand)

```
srcuri://myproject/src/app.js:10?remote=github.com/user/myproject
srcuri://myproject/README.md:1?branch=main&remote=github.com/user/myproject
```

- Enables sharing links to repos the recipient may not have cloned
- If workspace isn't found locally, offers to clone from the remote URL
- Clone destination: `{repo_base_dir}/{workspace_name}` (default: `~/code/myproject`)
- Automatically adds workspace to settings after cloning
- Can be combined with `?branch=`, `?commit=`, or `?tag=`

**Example workflow:**
1. Developer A shares: `srcuri://cool-lib/src/utils.rs:42?remote=github.com/org/cool-lib`
2. Developer B (who doesn't have cool-lib) clicks the link
3. Clone dialog appears: "Clone github.com/org/cool-lib to ~/code/cool-lib?"
4. Developer B confirms → repo is cloned, workspace is added, file opens
5. Future links to `srcuri://cool-lib/...` work directly

### Workspace Hint (Relative/Any Mode)

```
srcuri://rel/lib/utils.rs:10?workspaceHint=backend
```

- Used in rel or any mode to prefer a specific workspace when multiple matches exist

## Usage Examples

### From Browser

Paste these in your Chrome/Safari/Firefox address bar:

```
srcuri://abs/etc/hosts:1
srcuri://sorcery/README.md:50
```

Browser will ask permission the first time, then remember your choice.

### From HTML

```html
<a href="srcuri://sorcery/src/main.rs:100">View main.rs</a>
<a href="srcuri://abs/tmp/test.txt:1">Open test file</a>
```

### From JavaScript

```javascript
// Absolute path
window.location.href = "srcuri://abs/Users/ebeland/file.txt:50";

// Workspace path
window.location.href = "srcuri://myproject/src/app.js:100";
```

### From Command Line

```bash
# macOS
open "srcuri://abs/etc/hosts:1"
open "srcuri://sorcery/README.md:50"

# Linux
xdg-open "srcuri://abs/etc/hosts:1"
xdg-open "srcuri://sorcery/README.md:50"

# Windows
start srcuri://abs/C:/Users/user/file.txt:1
start srcuri://myproject/src/app.js:100
```

## Testing

Use the included test page:

```bash
open test-protocol.html
```

This includes clickable examples of all URL formats.

## Common Mistakes

### Using `file://` instead of `srcuri://`

**Wrong:**
```
file:///Users/ebeland/apps/sorcery/README.md
```

**Correct:**
```
srcuri://abs/Users/ebeland/apps/sorcery/README.md:1
```

### Confusing authorities

**Wrong:**
```
srcuri://etc/hosts:1           # This is workspace "etc", not /etc/hosts
```

**Correct:**
```
srcuri://abs/etc/hosts:1       # Absolute path mode
```

### Using git references with absolute path

**Wrong:**
```
srcuri://abs/Users/ebeland/file.txt?commit=abc123
```

**Correct:**
```
srcuri://sorcery/file.txt?commit=abc123
```

## URL Parsing Rules

The parser (`src-tauri/src/protocol_handler/parser.rs`) follows these rules:

1. **Authority = "wks"** → Explicit workspace mode
   ```
   srcuri://wks/myproject/file.rs:1
   ```

2. **Authority = "rel"** → Relative/search mode
   ```
   srcuri://rel/README.md:1
   ```

3. **Authority = "abs"** → Absolute path mode
   ```
   srcuri://abs/etc/hosts:1
   ```

4. **Authority = "any"** → Any mode (best-effort)
   ```
   srcuri://any/src/main.rs:42
   ```

5. **Authority = "ext"** → External URL mode
   ```
   srcuri://ext/https/github.com/user/repo/blob/main/file.rs#L42
   ```

6. **Authority = anything else** → Implicit workspace mode
   ```
   srcuri://myproject/file.rs:1
   ```

6. **Line and column extraction:**
   - Parse from right to left on final path segment
   - Max 2 colons for line:column
   - Non-numeric values ignored
   - Column must be 0-120

## Best Practices

### For Teams

**Use implicit workspace paths:**
```
srcuri://myproject/src/app.js:100
```

The Sorcery protocol with workspace paths works for everyone regardless of where they cloned the repo.

**Don't use absolute paths:**
```
srcuri://abs/Users/ebeland/projects/myproject/src/app.js:100
```

This only works on your machine.

### For Personal Use

**Either format works:**
```
srcuri://abs/Users/ebeland/file.txt:1          # Direct
srcuri://myproject/file.txt:1                  # Workspace
```

Choose based on convenience.

### For Documentation

**Include line numbers:**
```
See the bug in srcuri://myproject/src/bug.js:42
```

**Don't omit line numbers:**
```
See the bug in srcuri://myproject/src/bug.js
```

## Reserved Authority Tokens

These words cannot be used as workspace names:

- `wks` — explicit workspace mode
- `rel` — relative/search mode
- `any` — best-effort mode
- `abs` — absolute path mode
- `ext` — external URL mode

## Platform Differences

Sorcery Desktop registers the `srcuri://` protocol handler on all platforms:

### macOS
- Protocol handler registered via `Info.plist`
- Works from browser, Terminal, and other apps
- No additional configuration needed after install

### Linux
- Protocol handler registered via `.desktop` file
- Auto-registers on first launch
- Fallback: `xdg-mime default srcuri.desktop x-scheme-handler/srcuri`

### Windows
- Protocol handler registered via Registry
- Auto-registers via MSI installer
- Development: Manual registry import needed

## See Also

- [DEVELOPMENT.md](DEVELOPMENT.md) - Development workflow
- [README.md](README.md) - Project overview
- [dev/srcuri-protocol-spec-v1.md](dev/srcuri-protocol-spec-v1.md) - Complete protocol specification
