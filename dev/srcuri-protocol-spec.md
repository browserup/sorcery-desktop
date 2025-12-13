# srcuri:// Protocol Technical Specification

**Version:** 1.0
**Status:** Stable
**Last Updated:** 2025-01-12

---

## Table of Contents

1. [Introduction](#introduction)
2. [URL Format Specification](#url-format-specification)
3. [Folder/Directory Support](#folderdirectory-support)
4. [Query Parameters](#query-parameters)
5. [URL Parsing Rules](#url-parsing-rules)
6. [Path Resolution](#path-resolution)
7. [Security Considerations](#security-considerations)
8. [Examples Library](#examples-library)

**Related Documents:**
- [Protocol Registration](done/srcuri-registration.md) - Platform-specific setup (macOS, Linux, Windows)
- [Dialogs & Error Handling](srcuri-dialogs.md) - Interactive dialogs and error messages

---

## Introduction

### The Problem

When developers share code references today, they typically share GitHub, GitLab, or other web-based repository links:

```
https://github.com/user/myrepo/blob/main/src/main.rs#L42
```

While these links are convenient for viewing code in a browser, they don't help developers actually *work* with the code. To debug, edit, or understand the context around line 42, developers must:

1. Click the link (opens in browser)
2. Note the file path and line number
3. Switch to their terminal or IDE
4. Manually navigate to the file
5. Jump to the line number

This workflow is slow, error-prone, and breaks the developer's flow.

### The Solution

The `srcuri://` (Sorcery) protocol is an editor-agnostic deep linking mechanism that enables code references to open directly in the developer's local editor:

```
srcuri://myrepo/src/main.rs:42
```

When clicked, this link:
- Opens in the user's preferred editor (VS Code, IntelliJ, Neovim, etc.)
- Navigates directly to the file and line number
- Works regardless of where the file lives on the user's filesystem
- Maintains developer flow without context switching

### Design Philosophy

The srcuri:// protocol is designed with several key principles:

- **Editor Independence**: Works with any editor, no vendor lock-in
- **Portability**: Same links work for all team members regardless of OS or file system layout
- **Simplicity**: Intuitive URL format that's easy to construct and share
- **Git Integration**: First-class support for referencing specific commits, branches, and tags
- **Security**: Built-in protections against path traversal and malicious URLs

---

## URL Format Specification

The srcuri:// protocol supports four distinct request types, each designed for specific use cases.

### 1. Absolute Path Format

**Syntax:**
```
srcuri:///<absolute-path>:<line>:<column>
```

**Description:**
Uses a full filesystem path to reference a file. Note the **triple slash** (`///`) which indicates an absolute path.

**Use Cases:**
- Local testing and development
- System configuration files
- Scripts that generate links to known absolute locations

**Examples:**

```
srcuri:///etc/hosts:1
Opens /etc/hosts at line 1

srcuri:///Users/alice/projects/myapp/src/main.rs:100:5
Opens main.rs at line 100, column 5 (macOS)

srcuri:///home/bob/code/server/app.py:42
Opens app.py at line 42 (Linux)

srcuri:///C:/Users/Carol/Dev/project/README.md:10
Opens README.md at line 10 (Windows)
```

**Platform Notes:**
- **macOS/Linux**: Paths start with `/`
- **Windows**: Paths start with drive letter (e.g., `C:/` or `C:\`)
- Both forward slashes (`/`) and backslashes (`\`) are supported on Windows

**Limitations:**
- Not portable across team members (file paths differ per machine)
- Requires knowing the exact filesystem location
- Cannot be used with query parameters (git references)

---

### 2. Workspace Path Format

**Syntax:**
```
srcuri://<workspace>/<path>:<line>:<column>
```

**Description:**
References a file relative to a named workspace. This is the **recommended format** for team collaboration. Note the **double slash** (`//`) followed by the workspace name.

**How It Works:**

1. The URL contains a workspace identifier (e.g., `myproject`)
2. Each user configures workspace mappings in their settings:
   ```json
   {
     "workspaces": {
       "myproject": "/Users/alice/code/myproject"
     }
   }
   ```
3. The relative path is appended to the workspace root
4. The file opens at the specified line and column

**Use Cases:**
- Team collaboration (same link works for everyone)
- Documentation that references code
- Code review comments
- Issue trackers and wikis
- CI/CD logs and build output

**Examples:**

```
srcuri://myproject/README.md:1
Opens README.md at line 1 in the 'myproject' workspace

srcuri://backend-api/src/handlers/auth.rs:42:10
Opens auth.rs in 'backend-api' workspace at line 42, column 10

srcuri://infra/terraform/aws/main.tf:150
Opens main.tf at line 150 in 'infra' workspace

srcuri://docs/content/guides/getting-started.md:25
Opens getting-started.md at line 25 in 'docs' workspace
```

**Workspace Naming Conventions:**

- Use lowercase alphanumeric characters
- Hyphens and underscores are allowed
- Keep names short and memorable
- Match your repository name when possible
- Examples: `myproject`, `backend-api`, `mobile-app`, `shared_utils`

**Benefits:**

- **Portability**: Links work regardless of where users store code
- **Team Consistency**: Everyone uses the same link format
- **Multi-Repository**: Each repository/project can have its own workspace
- **Clear Intent**: Workspace name provides immediate context

---

### 3. Partial Path Format

**Syntax:**
```
srcuri://<filename>:<line>:<column>
```

**Description:**
References a file by name only, without specifying its location. The protocol handler searches all configured workspaces for matching files.

**How It Works:**

1. Parse the filename from the URL
2. Search all configured workspaces for files matching the name
3. Based on matches found:
   - **Zero matches**: Show error
   - **One match**: Open the file immediately
   - **Multiple matches**: Show chooser dialog for user selection

**Use Cases:**
- Quick references to unique files (README.md, package.json)
- Prototyping and exploration
- When exact path is unknown
- Informal team communication

**Examples:**

```
srcuri://README.md:1
Searches for README.md in all workspaces, opens at line 1

srcuri://main.rs:50:5
Finds main.rs (if unique), opens at line 50, column 5

srcuri://config.yaml:10
Searches for config.yaml across all workspaces

srcuri://AuthController.java:200
Finds AuthController.java, opens at line 200
```

**Matching Behavior:**

```
Single Match:
srcuri://package.json:1
→ Opens ~/code/myapp/package.json immediately

Multiple Matches:
srcuri://main.rs:10
→ Shows chooser with:
  - ~/code/backend/src/main.rs
  - ~/code/frontend/src/main.rs
  - ~/code/tools/cli/src/main.rs

No Matches:
srcuri://nonexistent.txt:1
→ Shows error: "File not found in any configured workspace"
```

**Best Practices:**

- Use for files with unique names (README.md, Makefile)
- Avoid for common names (main.rs, index.js, utils.py)
- Consider workspace path format for better reliability
- Useful for quick, informal sharing within small teams

---

### 4. Revision Path Format

**Syntax:**
```
srcuri://<workspace>/<path>:<line>?<git-param>=<value>
```

**Description:**
References a file at a specific git revision (commit, branch, or tag). Must use workspace path format. Provides git-aware features like temporary file viewing or branch checkout.

**Supported Git Parameters:**

- `commit=<SHA>` or `sha=<SHA>` - Reference a specific commit (most precise)
- `branch=<name>` - Reference the current state of a branch
- `tag=<name>` - Reference a tagged version

**Use Cases:**
- Code review comments referencing specific commits
- Bug reports citing exact versions
- Documentation linking to stable releases
- Historical code analysis
- Cross-branch comparisons

**Examples:**

```
srcuri://myrepo/src/file.rs:23?commit=abc123def456
Opens file.rs at line 23 from commit abc123def456

srcuri://backend/api/routes.py:100?branch=feature-auth
References routes.py on the feature-auth branch

srcuri://docs/README.md:1?tag=v1.0.0
Opens README.md from the v1.0.0 tagged release

srcuri://infra/config.yml:50?sha=7f8a9b2c
References config.yml at commit 7f8a9b2c
```

**Resolution Behavior:**

When a revision path is opened, the protocol handler:

1. Validates workspace is a git repository
2. Verifies commit/branch/tag exists
3. Presents options: view in temporary file (read-only) or checkout reference

See [Dialogs & Error Handling](srcuri-dialogs.md) for dialog details and auto-switching behavior.

---

### URL Component Details

#### Line Numbers

- **Format**: Integer following the path, separated by `:`
- **Indexing**: 1-indexed (first line is line 1)
- **Range**: No upper limit (limited only by file size)
- **Optional**: Yes (omit to open file without jumping to a line)
- **Invalid Values**: Non-numeric values are ignored

**Examples:**
```
srcuri://myproject/file.rs:1       → Line 1
srcuri://myproject/file.rs:42      → Line 42
srcuri://myproject/file.rs:10000   → Line 10000
srcuri://myproject/file.rs         → No line specified (open at top)
srcuri://myproject/file.rs:abc     → Invalid, ignored (opens at top)
```

#### Column Numbers

- **Format**: Integer following the line number, separated by `:`
- **Indexing**: 1-indexed (first column is column 1)
- **Range**: 0-120 (inclusive)
- **Optional**: Yes (omit to jump to line without specific column)
- **Invalid Values**: Values > 120 are rejected; non-numeric values are ignored

**Examples:**
```
srcuri://myproject/file.rs:42:1    → Line 42, column 1
srcuri://myproject/file.rs:42:10   → Line 42, column 10
srcuri://myproject/file.rs:42:120  → Line 42, column 120 (max)
srcuri://myproject/file.rs:42:150  → Invalid, column ignored
srcuri://myproject/file.rs:42      → Line 42, no column specified
```

**Why 120 column limit?**
The 120-character line width is a common coding standard. Larger values likely indicate malformed URLs or errors.

---

## Folder/Directory Support

In addition to opening files, srcuri:// links can open folders/directories in editors that support it.

### Supported Editors

| Editor | Folder Support | Notes |
|--------|---------------|-------|
| **VS Code Family** | ✅ Yes | Opens folder in sidebar/explorer |
| Cursor | ✅ Yes | |
| VSCodium | ✅ Yes | |
| Roo Cline | ✅ Yes | |
| Windsurf | ✅ Yes | |
| **JetBrains IDEs** | ✅ Yes | Opens as project |
| IntelliJ IDEA | ✅ Yes | |
| WebStorm | ✅ Yes | |
| PyCharm | ✅ Yes | |
| PhpStorm | ✅ Yes | |
| RubyMine | ✅ Yes | |
| GoLand | ✅ Yes | |
| CLion | ✅ Yes | |
| Rider | ✅ Yes | |
| DataGrip | ✅ Yes | |
| Android Studio | ✅ Yes | |
| Fleet | ✅ Yes | |
| **Sublime Text** | ✅ Yes | Opens folder in sidebar |
| **Zed** | ✅ Yes | Full project support |
| **Xcode** | ✅ Yes | Opens xcworkspace/xcodeproj if present |
| **Vim** | ✅ Yes | Opens netrw file browser |
| **Neovim** | ✅ Yes | Opens netrw file browser |
| **Emacs** | ✅ Yes | Opens dired mode |
| Kate | ❌ No | File-only editor |
| Kakoune | ❌ No | File-only editor |
| Micro | ❌ No | File-only editor |
| Nano | ❌ No | File-only editor |

**Summary:** 22 of 26 supported editors can open folders.

### Folder URL Examples

**Absolute path to folder:**
```
srcuri:///Users/alice/projects/myapp
Opens the myapp folder

srcuri:///home/bob/code/backend/src
Opens the src folder
```

**Workspace-relative folder:**
```
srcuri://myproject/src/controllers
Opens the controllers folder within the myproject workspace

srcuri://backend/lib
Opens the lib folder within the backend workspace
```

**Partial path folder:**
```
srcuri://src
Searches for folders named 'src' in all workspaces
```

### Line/Column Behavior for Folders

Line and column numbers are **silently ignored** when opening folders:

```
srcuri://myproject/src:42:10
→ Opens /path/to/myproject/src folder (line 42, column 10 ignored)
```

This allows links to be refactored from files to folders without breaking existing URLs.

### Error Handling

If an editor doesn't support folders, a clear error is shown:

```
Editor 'Nano' does not support opening folders.
Please select a different editor or open a file instead.
```

---

## Query Parameters

Query parameters extend the base URL format to provide additional functionality, particularly for git integration.

### Git Reference Parameters

Git references allow linking to specific versions of files using git commits, branches, or tags.

#### `commit=<SHA>` or `sha=<SHA>`

References a specific git commit by its SHA hash.

**Format:**
```
srcuri://<workspace>/<path>:<line>?commit=<SHA>
srcuri://<workspace>/<path>:<line>?sha=<SHA>
```

**Examples:**
```
srcuri://myrepo/src/main.rs:42?commit=abc123def456
srcuri://myrepo/README.md:1?sha=7f8a9b2c1e5d4f3a
```

**Notes:**
- Full or short SHA supported (short must be unambiguous)
- Most precise reference type (immutable)
- Ideal for bug reports and code reviews
- Both `commit=` and `sha=` are equivalent

#### `branch=<name>`

References the current state of a git branch.

**Format:**
```
srcuri://<workspace>/<path>:<line>?branch=<name>
```

**Examples:**
```
srcuri://myrepo/src/auth.rs:100?branch=main
srcuri://myrepo/config.yml:10?branch=feature-oauth
srcuri://myrepo/README.md:1?branch=develop
```

**Notes:**
- References latest commit on the branch
- May enable auto-checkout if working tree is clean
- Useful for feature branch discussions
- Branch must exist in the repository

#### `tag=<name>`

References a git tag (typically a release version).

**Format:**
```
srcuri://<workspace>/<path>:<line>?tag=<name>
```

**Examples:**
```
srcuri://myrepo/CHANGELOG.md:1?tag=v1.0.0
srcuri://myrepo/src/api.rs:50?tag=release-2.3.1
srcuri://myrepo/docs/guide.md:10?tag=stable
```

**Notes:**
- Typically points to release versions
- Immutable reference (like commits)
- Good for documentation and stable references
- Tag must exist in the repository

### Remote Parameter (Clone-on-Demand)

Enables sharing links to repositories the recipient may not have cloned locally.

#### `remote=<url>`

Specifies where to clone the repository from if the workspace isn't found locally.

**Format:**
```
srcuri://<workspace>/<path>:<line>?remote=<git-url>
```

**Examples:**
```
srcuri://myrepo/README.md:1?remote=github.com/user/myrepo
Opens README.md, cloning if needed from github.com/user/myrepo

srcuri://lib/src/utils.rs:42?remote=gitlab.com/org/lib
Opens utils.rs, offering to clone if 'lib' workspace not configured

srcuri://api/routes.py:100?branch=main&remote=github.com/team/api
Opens routes.py at branch main, cloning first if needed
```

**Behavior:**

1. If workspace is configured locally → Open file normally (remote param ignored)
2. If workspace not found AND remote specified → Show clone dialog
3. Clone dialog shows:
   - Remote URL
   - Clone destination: `{repo_base_dir}/{workspace_name}` (e.g., `~/code/myrepo`)
   - File to open after cloning
   - Branch/ref if specified
4. On confirmation:
   - Repository is cloned to calculated path
   - Workspace mapping is automatically added to settings
   - File opens in editor
5. Future links to that workspace work without cloning

**Clone Path Calculation:**

The clone destination is determined by the `repo_base_dir` setting (default: `~/code`) combined with the workspace name:

```
srcuri://cool-project/file.rs:1?remote=github.com/user/cool-project
→ Clone to: ~/code/cool-project
→ Add workspace: "cool-project" → "~/code/cool-project"
```

**Combining with Git References:**

The remote parameter can be combined with branch, commit, or tag:

```
srcuri://repo/file.rs:1?branch=feature&remote=github.com/user/repo
→ Clone with: git clone --branch feature github.com/user/repo
```

**Notes:**
- Remote URL format: host/org/repo (without protocol prefix)
- Does not clone if workspace already exists locally
- User must confirm clone operation (not automatic)
- Workspace is registered in settings after successful clone

### Parameter Precedence

If multiple git reference parameters are present, only the **first recognized parameter** is used:

```
srcuri://myrepo/file.rs:10?commit=abc123&branch=main
→ Uses commit=abc123 (commit appears first)

srcuri://myrepo/file.rs:10?branch=main&tag=v1.0.0
→ Uses branch=main (branch appears first)
```

### Unknown Parameters

Unknown or unsupported query parameters are silently ignored:

```
srcuri://myrepo/file.rs:10?editor=vscode&theme=dark
→ Unknown parameters 'editor' and 'theme' are ignored
→ URL is treated as: srcuri://myrepo/file.rs:10
```

### Combining Parameters

Currently, only one git reference parameter is processed per URL. Multiple git references in a single URL are not supported:

```
srcuri://myrepo/file.rs:10?commit=abc123&branch=main
→ Only commit=abc123 is used, branch=main is ignored
```

---

## URL Parsing Rules

Understanding how URLs are parsed helps construct valid links and predict behavior.

### Request Type Detection

The parser determines the request type based on the path structure:

```
Detection Algorithm:

1. If path starts with "/" → Absolute Path
   Example: srcuri:///etc/hosts:1

2. Else if path contains "/" → Workspace Path
   Example: srcuri://myproject/src/main.rs:10

3. Else → Partial Path
   Example: srcuri://README.md:1

4. If query contains git parameters → Upgrade to Revision Path
   Example: srcuri://myproject/file.rs:10?commit=abc123
```

**Visual Decision Tree:**

```
                    srcuri://...
                         |
              ┌──────────┴──────────┐
              │                     │
         Has git param?         No git param
              │                     │
              v                     v
       Revision Path        ┌──────┴──────┐
                           │               │
                    Starts with "/"?   No slash
                           │               │
                           v               v
                    Absolute Path   ┌─────┴─────┐
                                   │             │
                            Contains "/"?    No slash
                                   │             │
                                   v             v
                           Workspace Path   Partial Path
```

### Line and Column Extraction

Line and column numbers are extracted using **right-to-left parsing** to handle colons in filenames.

**Algorithm:**

```
1. Split path from right using ":" as delimiter (max 3 parts)
2. Extract rightmost part:
   - If numeric and ≤ 120 → Column number
   - Otherwise → Part of filename
3. Extract middle part:
   - If numeric → Line number
   - Otherwise → Part of filename
4. Remaining part → Filename/path
```

**Examples:**

```
Input: "file.rs:42:10"
Split (right-to-left): ["file.rs", "42", "10"]
Result: path="file.rs", line=42, column=10

Input: "file.rs:42"
Split (right-to-left): ["file.rs", "42"]
Result: path="file.rs", line=42, column=None

Input: "file.rs"
Split (right-to-left): ["file.rs"]
Result: path="file.rs", line=None, column=None

Input: "file:with:colons.txt:10:5"
Split (right-to-left): ["file:with:colons.txt", "10", "5"]
Result: path="file:with:colons.txt", line=10, column=5

Input: "file.rs:42:200"
Split (right-to-left): ["file.rs", "42", "200"]
Result: path="file.rs:42:200", line=None, column=None (200 > 120)

Input: "file.rs:abc:10"
Split (right-to-left): ["file.rs", "abc", "10"]
Result: path="file.rs:abc", line=10, column=None ("abc" not numeric)
```

### Colon Handling in Filenames

Files with colons in their names are supported through right-to-left parsing:

```
srcuri://project/config:prod.yml:10
→ path="config:prod.yml", line=10

srcuri://workspace/log:2025-01-12:errors.txt:50:5
→ path="log:2025-01-12:errors.txt", line=50, column=5

srcuri:///tmp/file:a:b:c.txt:1:1
→ path="/tmp/file:a:b:c.txt", line=1, column=1
```

**Platform Note:** Windows drive letters are handled specially and don't interfere with parsing:

```
srcuri:///C:/Users/alice/file.txt:10
→ Detected as absolute path (colon at position 1)
→ path="C:/Users/alice/file.txt", line=10
```

### Edge Cases

#### Empty or Whitespace Line/Column

```
srcuri://file.rs::
→ path="file.rs", line=None, column=None

srcuri://file.rs:  :
→ path="file.rs", line=None, column=None
```

#### Negative Numbers

```
srcuri://file.rs:-1:-5
→ path="file.rs:-1:-5", line=None, column=None
(Negative numbers are invalid)
```

#### Decimal Numbers

```
srcuri://file.rs:42.5:10.2
→ path="file.rs:42.5", line=None, column=None
(Decimals are invalid, integers only)
```

#### Very Large Line Numbers

```
srcuri://file.rs:999999999:10
→ path="file.rs", line=999999999, column=10
(Large line numbers are accepted, editor handles bounds)
```

#### Column Over Limit

```
srcuri://file.rs:42:150
→ path="file.rs:42:150", line=None, column=None
(Column 150 exceeds max of 120, entire suffix rejected)
```

---

## Path Resolution

Once a URL is parsed, the protocol handler must resolve it to an actual file on the filesystem.

### Workspace Resolution

Workspace paths are resolved by looking up the workspace name in the user's configuration.

**Configuration Format:**

```json
{
  "workspaces": {
    "myproject": "/Users/alice/code/myproject",
    "backend": "/Users/alice/work/api-server",
    "docs": "/Users/alice/repos/documentation"
  }
}
```

**Resolution Process:**

```
Input: srcuri://backend/src/handlers/auth.rs:42

1. Extract workspace name: "backend"
2. Look up in configuration: "/Users/alice/work/api-server"
3. Append relative path: "/Users/alice/work/api-server/src/handlers/auth.rs"
4. Validate file exists
5. Open at line 42
```

**Error Conditions:**

```
Unknown workspace:
srcuri://unknown/file.rs:1
→ Error: "Workspace 'unknown' not found in configuration"

File not found:
srcuri://myproject/missing.rs:10
→ Error: "File not found: /Users/alice/code/myproject/missing.rs"

Path traversal attempt:
srcuri://myproject/../../../etc/passwd:1
→ Error: "Invalid path (security violation)"
```

### Partial Path Matching

Partial paths search all configured workspaces for matching files.

**Matching Algorithm:**

```
Input: srcuri://main.rs:10

1. For each configured workspace:
   a. Recursively search for files named "main.rs"
   b. Add matches to results list

2. Based on match count:
   - 0 matches: Return error
   - 1 match: Return file path for immediate opening
   - 2+ matches: Return list for user selection
```

**Match Outcomes:**

- **Single match**: Opens file immediately
- **Multiple matches**: Shows chooser dialog (see [Dialogs](srcuri-dialogs.md))
- **No matches**: Shows error with list of searched workspaces

### Absolute Path Handling

Absolute paths trigger a security confirmation dialog if the file is outside all configured workspaces. Files within a workspace open immediately without confirmation.

See [Dialogs](srcuri-dialogs.md) for the security dialog details.

### Path Normalization

All paths are normalized before resolution to prevent security issues:

**Normalization Steps:**

1. Resolve symbolic links to actual paths
2. Resolve `.` (current directory) references
3. Resolve `..` (parent directory) references
4. Convert to canonical absolute path
5. Validate resulting path is within allowed boundaries

**Examples:**

```
Input: srcuri://myproject/./src/../README.md:1
Normalized: srcuri://myproject/README.md:1

Input: srcuri://myproject/src/./handlers/./auth.rs:10
Normalized: srcuri://myproject/src/handlers/auth.rs:10

Input: srcuri://myproject/../../../etc/passwd:1
Normalized: srcuri:///etc/passwd:1
→ Path traversal detected, security dialog shown
```

---

## Security Considerations

The srcuri:// protocol includes several security measures to prevent malicious URLs.

### Path Traversal Prevention

**Attack Vector:**

```
srcuri://myproject/../../../etc/passwd:1
```

**Protection:**

1. All paths are normalized using canonical path resolution
2. Resolved paths are validated against workspace boundaries
3. Paths outside configured workspaces trigger confirmation dialogs
4. Absolute paths require explicit user approval (unless in workspace)

**Implementation:**

```
Input: srcuri://myproject/../../../etc/passwd:1

Step 1: Resolve workspace
  myproject → /Users/alice/code/myproject

Step 2: Append relative path
  /Users/alice/code/myproject/../../../etc/passwd

Step 3: Normalize
  /etc/passwd

Step 4: Validate
  /etc/passwd is NOT within /Users/alice/code/myproject
  → Trigger security warning

Step 5: User confirmation required
  [1] Open anyway (risky)
  [2] Cancel (recommended)
```

### Workspace Boundary Enforcement

Files outside configured workspaces require explicit user consent:

```
Configured workspaces:
  myproject: /Users/alice/code/myproject
  backend: /Users/alice/code/backend

Safe (auto-open):
  srcuri://myproject/src/main.rs:1
  → /Users/alice/code/myproject/src/main.rs ✓

Requires confirmation:
  srcuri:///etc/hosts:1
  → /etc/hosts (not in any workspace) ⚠

  srcuri://myproject/../other-project/file.txt:1
  → /Users/alice/code/other-project/file.txt (outside boundary) ⚠
```

### Column Number Bounds

**Attack Vector:**

```
srcuri://file.rs:1:999999999
```

**Protection:**

- Column numbers limited to 0-120 range
- Values outside range cause entire line:column suffix to be rejected
- Prevents potential buffer overflows in editors
- Aligns with reasonable line width standards

**Examples:**

```
Valid:
srcuri://file.rs:42:0    → column 0 ✓
srcuri://file.rs:42:120  → column 120 ✓

Invalid:
srcuri://file.rs:42:121  → entire suffix rejected ✗
srcuri://file.rs:42:999  → entire suffix rejected ✗
```

### Git Reference Validation

**Attack Vector:**

```
srcuri://myrepo/file.rs:1?commit=malicious-payload
```

**Protection:**

1. Git references validated against actual repository
2. Non-existent refs rejected before any operations
3. Checkout operations require clean working tree
4. User confirmation for all git operations

**Validation Process:**

```
Input: srcuri://myrepo/file.rs:1?commit=abc123

Step 1: Verify workspace is git repository
Step 2: Verify commit exists: git cat-file -e abc123^{commit}
Step 3: Check working tree status: git status --porcelain
Step 4: Present options with clear warnings
Step 5: Execute only user-approved actions
```

### Symbolic Link Handling

**Attack Vector:**

```bash
# Attacker creates symlink
ln -s /etc/passwd ~/code/myproject/innocent-file.txt

# Crafts URL
srcuri://myproject/innocent-file.txt:1
```

**Protection:**

- Symbolic links are resolved to their targets
- Resolved targets validated against workspace boundaries
- Cross-workspace symlinks trigger security warnings
- Users warned about links pointing outside workspaces

**Resolution:**

```
Input: srcuri://myproject/innocent-file.txt:1

Step 1: Resolve workspace
  /Users/alice/code/myproject/innocent-file.txt

Step 2: Resolve symlink
  → /etc/passwd

Step 3: Validate
  /etc/passwd is NOT within workspace boundary
  → Security warning triggered
```

### URL Injection

**Attack Vector:**

```html
<!-- Malicious website -->
<a href="srcuri:///etc/passwd:1">Click for free prize!</a>
```

**Protection:**

- Browser security requires user interaction (click)
- Operating system shows protocol handler confirmation (first use)
- File existence validated before opening
- Non-workspace files require explicit confirmation

**User Experience:**

```

User clicks link → Browser asks "Open with srcuri?" → User confirms
→ srcuri:// handler launches → File outside workspace detected
→ Security dialog shown → User must explicitly approve

---

## Examples Library

Practical examples for common scenarios.

### Quick Start Examples

**Basic file reference:**
```
srcuri://myproject/README.md:1
Opens README.md at line 1
```

**With line and column:**
```
srcuri://myproject/src/main.rs:42:10
Opens main.rs at line 42, column 10
```

**Partial path:**
```
srcuri://package.json:15
Opens package.json (searches all workspaces)
```

**Absolute path:**
```
srcuri:///tmp/debug.log:100
Opens debug.log at line 100
```

### Team Collaboration

**Code review comment:**
```
Found a bug here: srcuri://api-server/src/auth.rs:156

The authentication check is missing validation.
```

**Issue tracker:**
```
Bug Report #1234

Crash occurs at:
srcuri://mobile-app/lib/screens/home.dart:89:12

Stack trace shows null pointer exception.
```

**Documentation:**
```
# Installation Guide

Edit the configuration file:
srcuri://myproject/config/app.yml:25

Set the `api_key` value to your API key.
```

**Team wiki:**
```
## Architecture Overview

The main entry point is:
srcuri://backend/src/main.rs:1

Request routing happens in:
srcuri://backend/src/routes/mod.rs:50
```

### Git Workflow Integration

**Bug report with commit reference:**
```
This bug was introduced in commit abc123:
srcuri://myrepo/src/parser.rs:75?commit=abc123def

The validation logic is incorrect.
```

**Feature branch discussion:**
```
I'm working on OAuth integration:
srcuri://api/src/auth/oauth.rs:100?branch=feature-oauth

Review the implementation when you have time.
```

**Release documentation:**
```
Changes in v2.0.0:

New API endpoint added:
srcuri://server/api/v2/users.rs:1?tag=v2.0.0
```

**Historical analysis:**
```
The old implementation was:
srcuri://app/legacy/handler.js:50?commit=old-version

The new implementation is:
srcuri://app/src/handler.ts:50?branch=main
```

### CI/CD Integration

**Build log linking:**
```bash
#!/bin/bash
# Link build errors to source files

cargo build 2>&1 | while read line; do
  if [[ $line =~ error.*src/(.+):([0-9]+):([0-9]+) ]]; then
    file="${BASH_REMATCH[1]}"
    line="${BASH_REMATCH[2]}"
    col="${BASH_REMATCH[3]}"
    echo "Error: srcuri://myproject/src/${file}:${line}:${col}"
  fi
done
```

**Test failure reporting:**
```
Test failed: test_authentication

Failed assertion at:
srcuri://backend/tests/auth_tests.rs:42

Expected: 200 OK
Got: 401 Unauthorized
```

**Deployment script:**
```bash
#!/bin/bash
echo "Review configuration before deploying:"
echo "  Database: srcuri://infra/terraform/rds.tf:25"
echo "  Servers: srcuri://infra/ansible/web-servers.yml:10"
echo "  Secrets: srcuri://infra/secrets/prod.env:1"
```

### Editor Integration

**IDE quick link generation:**
```python
# Generate srcuri:// link from current cursor position
def generate_srcuri_link():
    workspace = get_current_workspace()  # e.g., "myproject"
    file_path = get_relative_path()      # e.g., "src/main.py"
    line = get_cursor_line()             # e.g., 42
    column = get_cursor_column()         # e.g., 10

    return f"srcuri://{workspace}/{file_path}:{line}:{column}"
```

**Chat application integration:**
```javascript
// Detect and linkify srcuri:// URLs in chat messages
const srcuriRegex = /srcuri:\/\/[^\s]+/g;

function linkifySrcuriUrls(text) {
  return text.replace(srcuriRegex, (url) => {
    return `<a href="${url}">${url}</a>`;
  });
}

// Usage:
const message = "Check this out: srcuri://myapp/src/bug.rs:42";
const html = linkifySrcuriUrls(message);
// Result: Check this out: <a href="srcuri://myapp/src/bug.rs:42">srcuri://myapp/src/bug.rs:42</a>
```

### Cross-Platform Examples

**macOS:**
```
srcuri:///Users/alice/code/myproject/README.md:1
srcuri://myproject/src/main.swift:50
```

**Linux:**
```
srcuri:///home/bob/projects/myapp/README.md:1
srcuri://myapp/src/main.rs:50
```

**Windows:**
```
srcuri:///C:/Users/Carol/Dev/myproject/README.md:1
srcuri://myproject/src/main.cs:50
```

**Portable (recommended):**
```
srcuri://myproject/README.md:1
Works on all platforms with proper workspace configuration
```

### Browser Extension Examples

**Convert GitHub URL to srcuri:// (concept):**
```javascript
// Example: Convert GitHub blob URL to srcuri://
function githubToSrcuri(githubUrl) {
  // Input: https://github.com/user/repo/blob/main/src/file.rs#L42
  // Output: srcuri://repo/src/file.rs:42

  const match = githubUrl.match(
    /github\.com\/[^\/]+\/([^\/]+)\/blob\/[^\/]+\/(.+)#L(\d+)/
  );

  if (match) {
    const [_, repo, path, line] = match;
    return `srcuri://${repo}/${path}:${line}`;
  }

  return null;
}
```

### Advanced Patterns

**Multiple locations (documentation):**
```markdown
The authentication flow involves three files:

1. Route handler: srcuri://api/routes/auth.rs:100
2. Service layer: srcuri://api/services/auth_service.rs:50
3. Database queries: srcuri://api/db/auth_queries.rs:25

Read them in order to understand the complete flow.

```

**Workspace-specific preferences:**
```json
{
  "workspaces": {
    "frontend": {
      "path": "/Users/alice/code/frontend",
      "default_editor": "vscode"
    },
    "backend": {
      "path": "/Users/alice/code/backend",
      "default_editor": "intellij"
    }
  }
}
```

---

## Appendix

### URL Encoding

Special characters in file paths should be URL-encoded:

```
Space: " " → "%20"
srcuri://project/my%20file.txt:1

Hash: "#" → "%23"
srcuri://project/file%23name.txt:1

Question: "?" → "%3F"
srcuri://project/what%3F.txt:1
```

## Contributing

The srcuri:// protocol is an open standard. Contributions, feedback, and alternative implementations are welcome.

**Specification Updates:**
- Propose changes via GitHub issues
- Discuss breaking changes with community
- Document rationale for changes
- Update examples and tests

---

## License

This specification is released under the MIT License.

---

## References

- **Repository**: [github.com/yourusername/sorcery-desktop](https://github.com/yourusername/sorcery-desktop)
- **Website**: [srcuri.org](https://srcuri.org)
- **Documentation**: [docs.srcuri.org](https://docs.srcuri.org)
- **Discussion Forum**: [community.srcuri.org](https://community.srcuri.org)
