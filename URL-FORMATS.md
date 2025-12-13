# Sorcery Protocol Format Guide

## Overview

Sorcery Desktop uses the `srcuri://` protocol (also known as the Sorcery protocol) to open files in your configured editor. This protocol enables editor-agnostic code linking that works across teams and platforms.

**Note:** The protocol uses `srcuri://` (NOT `file://`) as its URL scheme. For the complete protocol specification, visit [srcuri.com](https://srcuri.com).

## URL Format

```
srcuri://<workspace>/<path>:<line>:<column>
```

## Three Types of URLs

### 1. Absolute Path (Full File System Path)

**Format:** `srcuri:///<absolute-path>:<line>:<column>`

**Note the triple slash** `///` - this indicates an absolute file system path.

**Examples:**
```
srcuri:///etc/hosts:1
srcuri:///Users/ebeland/apps/sorcery/README.md:50
srcuri:///Users/ebeland/apps/sorcery/src-tauri/src/main.rs:100:5
srcuri:///tmp/test.txt:42:10
```

**When to use:**
- You know the exact file system path
- Sharing links within the same machine
- Testing specific files
- No workspace mapping needed

### 2. Workspace Path (Portable, Recommended)

**Format:** `srcuri://<workspace>/<path>:<line>:<column>`

**Note the double slash** `//` - followed by workspace name.

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

**Example workspace mapping:**
```json
{
  "workspaces": {
    "sorcery": "/Users/ebeland/apps/sorcery",
    "myproject": "/Users/ebeland/projects/myproject"
  }
}
```

### 3. Partial Path (Searches All Workspaces)

**Format:** `srcuri://<filename>:<line>:<column>`

**Examples:**
```
srcuri://README.md:1
srcuri://main.rs:50
srcuri://App.tsx:100
```

**When to use:**
- Quick access to common files
- File exists in only one workspace
- Don't remember the workspace name

**How it works:**
1. Searches all configured workspaces for matching files
2. If one match found, opens it immediately
3. If multiple matches found, shows chooser dialog
4. If no matches found, shows error

## Line and Column Numbers

All formats support optional line and column numbers:

```
srcuri://path                    # Just open the file
srcuri://path:50                 # Open at line 50
srcuri://path:50:10              # Open at line 50, column 10
```

**Note:** Column numbers are limited to 0-120. Higher values are ignored.

## Opening Folders/Directories

srcuri:// links can also open folders in editors that support it.

### Folder Examples

**Absolute path to folder:**
```
srcuri:///Users/alice/projects/myapp
srcuri:///home/bob/code/backend/src
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
- Must use workspace path format (not absolute path)
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

## Usage Examples

### From Browser

Paste these in your Chrome/Safari/Firefox address bar:

```
srcuri:///etc/hosts:1
srcuri://sorcery/README.md:50
```

Browser will ask permission the first time, then remember your choice.

### From HTML

```html
<a href="srcuri://sorcery/src/main.rs:100">View main.rs</a>
<a href="srcuri:///tmp/test.txt:1">Open test file</a>
```

### From JavaScript

```javascript
// Absolute path
window.location.href = "srcuri:///Users/ebeland/file.txt:50";

// Workspace path
window.location.href = "srcuri://myproject/src/app.js:100";
```

### From Command Line

```bash
# macOS
open "srcuri:///etc/hosts:1"
open "srcuri://sorcery/README.md:50"

# Linux
xdg-open "srcuri:///etc/hosts:1"
xdg-open "srcuri://sorcery/README.md:50"

# Windows
start srcuri:///C:/Users/user/file.txt:1
start srcuri://myproject/src/app.js:100
```

## Testing

Use the included test page:

```bash
open test-protocol.html
```

This includes clickable examples of all URL formats.

## Common Mistakes

### ❌ Using `file://` instead of `srcuri://`

**Wrong:**
```
file:///Users/ebeland/apps/sorcery/README.md
```

**Correct:**
```
srcuri:///Users/ebeland/apps/sorcery/README.md:1
```

### ❌ Wrong number of slashes

**Wrong:**
```
srcuri://etc/hosts:1           # This looks like workspace "etc"
srcuri:////Users/ebeland/file  # Too many slashes
```

**Correct:**
```
srcuri:///etc/hosts:1                           # Absolute path (3 slashes)
srcuri://sorcery/README.md:1                  # Workspace path (2 slashes)
```

### ❌ Using git references with absolute path

**Wrong:**
```
srcuri:///Users/ebeland/file.txt?commit=abc123
```

**Correct:**
```
srcuri://sorcery/file.txt?commit=abc123
```

## URL Parsing Rules

The parser (`src-tauri/src/protocol_handler/parser.rs`) follows these rules:

1. **Starts with `/`** → Absolute path
   ```
   srcuri:///etc/hosts:1
   ```

2. **Contains `/` after first component** → Workspace path
   ```
   srcuri://workspace/file.rs:1
   ```

3. **No `/` or only filename** → Partial path (search)
   ```
   srcuri://README.md:1
   ```

4. **Line and column extraction:**
   - Parse from right to left
   - Max 2 colons for line:column
   - Non-numeric values ignored
   - Column must be 0-120

## Best Practices

### For Teams

✅ **Use workspace paths:**
```
srcuri://myproject/src/app.js:100
```

The Sorcery protocol with workspace paths works for everyone regardless of where they cloned the repo.

❌ **Don't use absolute paths:**
```
srcuri:///Users/ebeland/projects/myproject/src/app.js:100
```

This only works on your machine.

### For Personal Use

✅ **Either format works:**
```
srcuri:///Users/ebeland/file.txt:1              # Direct
srcuri://myproject/file.txt:1                   # Workspace
```

Choose based on convenience.

### For Documentation

✅ **Include line numbers:**
```
See the bug in srcuri://myproject/src/bug.js:42
```

❌ **Don't omit line numbers:**
```
See the bug in srcuri://myproject/src/bug.js
```

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
- [ai/protocol-handler-fix.md](dev/protocol-handler-fix.md) - Protocol handler architecture
- [dev/protocol-handler.md](dev/done/protocol-handler.md) - Complete protocol handler guide (includes deep-link payload fix)
