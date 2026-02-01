# srcuri:// Protocol Specification v1.0

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
srcuri://myrepo/src/main.rs@L42
```

When clicked, this link:
- Opens in the user's preferred editor (VS Code, IntelliJ, Neovim, etc.)
- Navigates directly to the file and line number
- Works regardless of where the file lives on the user's filesystem
- Maintains developer flow without context switching

---

## Key Protocol Goals

These goals guide all design decisions and should be considered by future spec maintainers:

1. **Easy conversion** — Converting from a filesystem path (workspace-relative or absolute) to a `srcuri://` protocol link OR a `srcuri.com/` URL should be trivial and mechanical.

2. **Brevity** — Links should be as short as possible while remaining unambiguous.

3. **Extensibility** — The protocol must support future enhancements via URL-style query parameters.

4. **Bidirectional consistency** — Converting between `srcuri://` protocol and `srcuri.com/` URLs must be consistent and lossless in both directions.

5. **URL-friendly characters** — The format must work reliably in URLs, avoiding characters or patterns that may be stripped or normalized by web servers, proxies, or tools like nginx.

6. **Simple heuristics** — The rules for converting a raw path into a srcuri link should be explainable in one sentence per path type.

7. **Avoid mangling middlemen** — We need a syntax that, in URL format, doesn't get mangled by nginx, proxies or gateways.

8. **Easy linkification** — We need to provide a format that can be linkified in terminals and web scenarios easily. The `://` pattern is recognized by most link detectors.

9. **Easy explainability/memorability** — We want to make the rules as simple and intuitive as possible, so they are easy to remember.

10. **Familiar/Elegant Look** — The protocol should feel familiar and intuitive.

11. **Ergonomic Common Case** — If a variant of the protocol is more common, or has a short-hand, that's the one we make simplest/easiest.

---

## Design Philosophy

The `srcuri://` protocol is designed with several key principles:

- **Editor Independence**: Works with any editor, no vendor lock-in
- **Portability**: Same links work for all team members regardless of OS or file system layout
- **Simplicity**: Intuitive URL format that's easy to construct and share
- **Git Integration**: First-class support for referencing specific commits, branches, and tags
- **Security**: Built-in protections against path traversal and malicious URLs

---

## Protocol Format Overview

The srcuri protocol uses standard URI format:

```
srcuri://<authority>/<path>[<line-suffix>][?<query>][#<fragment>]
```

**Line suffix** (preferred): `@L<line>[C<column>]`  
**Legacy line suffix**: `:line[:column]` (accepted for compatibility)

The **authority** field determines the link mode:
- If authority is a **reserved token** (`wks`, `rel`, `any`, `abs`, `ext`) → explicit mode
- Otherwise → **implicit workspace mode** (authority is the workspace name)

### Why This Design?

The authority-based approach provides:
- **Easy linkification** — The `srcuri://` pattern is universally recognized as a URL
- **Clean syntax** — `srcuri://myrepo/path` is cleaner than sigil-based alternatives
- **URL compatibility** — Works correctly with URL parsers and web infrastructure
- **Implicit workspace as default** — The most common case (workspace references) has the shortest syntax

---

## URL Format Specification

The protocol supports five distinct link modes, determined by the authority component.

### Quick Reference

| Mode | Authority | Format                                         | Example |
|------|-----------|------------------------------------------------|---------|
| **Workspace (implicit)** | `<workspace>` | `srcuri://<workspace>/<path>@L<line>`           | `srcuri://myrepo/src/main.rs@L42` |
| **Workspace (explicit)** | `wks` | `srcuri://wks/<workspace>/<path>@L<line>` | `srcuri://wks/myrepo/src/main.rs@L42` |
| **Relative (search)** | `rel` | `srcuri://rel/<path>@L<line>`                   | `srcuri://rel/main.rs@L42` |
| **Any (best-effort)** | `any` | `srcuri://any/<path>@L<line>`                   | `srcuri://any/src/main.rs@L42` |
| **Absolute** | `abs` | `srcuri://abs/<path>@L<line>`                   | `srcuri://abs/etc/hosts@L1` |
| **External** | `ext` | `srcuri://ext/<scheme>/<host>/<path>`           | `srcuri://ext/https/github.com/user/repo/...` |

### Reserved Authority Tokens

These authority values are **reserved** and indicate explicit modes:

- `wks` — explicit workspace mode (may be omitted; workspace name becomes authority)
- `rel` — relative path search mode (searches all workspaces)
- `any` — best-effort mode (workspace → relative → absolute)
- `abs` — absolute filesystem path mode
- `ext` — external URL mode

These cannot be used as workspace names.

### Detection Algorithm

```
Parse srcuri:// link:

1. Validate prefix is "srcuri://" (reject "srcuri:" without //)

2. Extract authority (host component)

3. Check if authority is a reserved token:
   - "wks" → Explicit Workspace Mode
   - "rel" → Relative Mode (search all workspaces)
   - "any" → Any Mode (best-effort resolution)
   - "abs" → Absolute Path Mode
   - "ext" → External URL Mode

4. If authority is NOT reserved → Implicit Workspace Mode
   (authority is treated as workspace name)

5. If query contains git parameters → Add revision behavior
   Example: srcuri://myrepo/file.rs@L10?commit=abc123
```

**Visual Decision Tree:**

```
srcuri://...
    │
    ├─ Authority = "wks"? ──YES──► Explicit Workspace Mode
    │         │
    │        NO
    │         │
    ├─ Authority = "rel"? ──YES──► Relative Mode (search)
    │         │
    │        NO
    │         │
    ├─ Authority = "any"? ──YES──► Any Mode (best-effort)
    │         │
    │        NO
    │         │
    ├─ Authority = "abs"? ──YES──► Absolute Path Mode
    │         │
    │        NO
    │         │
    ├─ Authority = "ext"? ──YES──► External URL Mode
    │         │
    │        NO
    │         ▼
    │    Implicit Workspace Mode
    │    (authority = workspace name)
    │
    └─ Has git query param? ──YES──► Add Revision behavior
```

---

### 1. Workspace Mode (Implicit)

**Syntax:**
```
srcuri://<workspace>/<path>@L<line>[C<column>]
```

**Description:** References a file relative to a named workspace. The authority IS the workspace name. This is the **recommended canonical format** for team collaboration.

**How It Works:**
1. The authority component contains the workspace name (e.g., `myproject`)
2. Each user configures workspace mappings in their settings:
   ```json
   {
     "workspaces": {
       "myproject": "/Users/alice/code/myproject",
       "backend": "/home/alice/work/api-server"
     }
   }
   ```
3. The relative path is appended to the workspace root
4. The file opens at the specified line and column

**Strict Resolution:**

Workspace mode uses **strict resolution** - the specified workspace must exist in the user's configuration:
- If the workspace is not configured → **Error** (prompts user to configure)
- If workspace exists but file doesn't → **Error** (file not found)
- **No fallback** to searching other workspaces or treating as absolute path

This strict behavior ensures predictable, intentional navigation. Use Relative Mode when you want flexible cross-workspace search.

**Use Cases:**
- Team collaboration (same link works for everyone)
- Documentation that references code
- Code review comments
- Issue trackers and wikis
- CI/CD logs and build output

**Examples:**

```
srcuri://myproject/README.md@L1
Opens README.md at line 1 in the 'myproject' workspace

srcuri://backend-api/src/handlers/auth.rs@L42C10
Opens auth.rs in 'backend-api' workspace at line 42, column 10

srcuri://infra/terraform/aws/main.tf@L150
Opens main.tf at line 150 in 'infra' workspace

srcuri://docs/content/guides/getting-started.md@L25
Opens getting-started.md at line 25 in 'docs' workspace
```

**srcuri.com Equivalent:**
```
srcuri://myrepo/src/main.rs@L42  →  https://srcuri.com/myrepo/src/main.rs@L42
```

**Workspace Naming Conventions:**
- Use lowercase alphanumeric characters
- Hyphens and underscores are allowed
- Keep names short and memorable
- Match your repository name when possible
- Examples: `myproject`, `backend-api`, `mobile-app`, `shared_utils`

**Benefits:**
- **Brevity**: Shortest possible syntax for the most common case
- **Portability**: Links work regardless of where users store code
- **Team Consistency**: Everyone uses the same link format
- **Multi-Repository**: Each repository/project can have its own workspace

---

### 2. Workspace Mode (Explicit)

**Syntax:**
```
srcuri://wks/<workspace>/<path>@L<line>[C<column>]
```

**Description:** Same as implicit workspace mode, but with the explicit `wks` authority. Useful when clarity is preferred over brevity.

**Examples:**

```
srcuri://wks/myrepo/src/main.rs@L42
Opens main.rs at line 42 in workspace 'myrepo'

srcuri://wks/backend/api/routes.py@L100C5
Opens routes.py at line 100, column 5 in workspace 'backend'
```

**When to Use:**
- When you want to be unambiguous about intent
- In documentation or tooling where explicitness aids clarity
- Both implicit and explicit forms are accepted; emit implicit for brevity

---

### 3. Relative Mode

**Syntax:**
```
srcuri://rel/<path>@L<line>[C<column>][?workspaceHint=<name>]
```

**Description:** References a file by name or partial path, without specifying a workspace. The protocol handler searches all configured workspaces for matching files.

**How It Works:**
1. Parse the filename/path from the URL
2. If `workspaceHint` parameter is present, try that workspace first
3. Search all configured workspaces for files matching the path
4. Based on matches found:
   - **Zero matches**: Show error
   - **One match**: Open the file immediately
   - **Multiple matches**: Show chooser dialog for user selection

**Resolution Algorithm (recommended):**
1. Parse path and optional line/col
2. **Workspace name detection** (highest priority): If any path segment matches a configured workspace name (case-insensitive), extract the relative path from segments after the match
3. If `workspaceHint` is present, prioritize that workspace
4. Otherwise, append the path to each workspace root and check existence
5. If exactly one candidate exists, open it
6. If multiple candidates exist, prompt user to choose (sorted by MRU)

**Workspace Name in Path:**

When the search path contains a configured workspace name as a path segment, Sorcery extracts the relative path and resolves it within that workspace. This enables cross-platform path matching.

```
Configuration:
  myproject → /home/alice/code/myproject

Input:
  srcuri://rel/D:/Code/myproject/src/main.rs@L42

Detection:
  - Path segments: ["D:", "Code", "myproject", "src", "main.rs"]
  - "myproject" matches workspace name at index 2
  - Relative path: "src/main.rs" (segments after match)

Resolution:
  → /home/alice/code/myproject/src/main.rs (line 42)
```

This matching is:
- **Case-insensitive**: "MyProject" matches "myproject" workspace
- **Segment-based**: "myproject" in "myproject2" does NOT match (must be exact segment)
- **First-occurrence**: If workspace name appears multiple times, first match wins
- **Higher priority**: Workspace-in-path matches rank above simple suffix matches

**Use Cases:**
- Quick references to unique files (`README.md`, `package.json`)
- Prototyping and exploration
- When exact workspace is unknown
- Informal team communication

**Examples:**

```
srcuri://rel/README.md@L1
Searches for README.md in all workspaces, opens at line 1

srcuri://rel/main.rs@L50C5
Finds main.rs (if unique), opens at line 50, column 5

srcuri://rel/src/utils.py@L10
Searches for src/utils.py path in all workspaces

srcuri://rel/AuthController.java@L200?workspaceHint=backend
Searches for AuthController.java, preferring the 'backend' workspace
```

**srcuri.com Equivalent:**
```
srcuri://rel/README.md@L1  →  https://srcuri.com/rel/README.md@L1
```

**Matching Behavior:**

```
Single Match:
srcuri://rel/package.json@L1
→ Opens ~/code/myapp/package.json immediately

Multiple Matches:
srcuri://rel/main.rs@L10
→ Shows chooser with:
  - ~/code/backend/src/main.rs
  - ~/code/frontend/src/main.rs
  - ~/code/tools/cli/src/main.rs

No Matches:
srcuri://rel/nonexistent.txt@L1
→ Shows error: "File not found in any configured workspace"
```

**Query Parameters (rel/any mode):**
- `workspaceHint` (string, optional): preferred workspace name to try first

**Best Practices:**
- Use for files with unique names (`README.md`, `Makefile`, `Cargo.toml`)
- Avoid for common names (`main.rs`, `index.js`, `utils.py`)
- Consider implicit workspace format for better reliability
- Useful for quick, informal sharing within small teams

---

### 4. Any Mode (Best-Effort)

**Syntax:**
```
srcuri://any/<path>@L<line>[C<column>][?workspaceHint=<name>]
```

**Description:** Best-effort resolution for sources that do not know whether a path is workspace-relative, search-relative, or absolute. Intended for terminals and log output. Do not emit this format when a more specific mode is known.

**Resolution Order (applicable steps only):**
1. **Workspace (hinted):** If the path is relative and `workspaceHint` is present, resolve as workspace mode using that workspace and the provided path. Missing workspace or file → error.
2. **Workspace (leading segment):** If the path is relative and the first segment matches a configured workspace name (case-insensitive), resolve as workspace mode using the remainder of the path. Missing file → error.
3. **Relative:** If the path is relative, resolve using Relative Mode search (with chooser on multiple matches). No matches → error.
4. **Absolute:** If the path is absolute (POSIX `/`, Windows drive `C:/`, UNC `//`, or `~`), resolve as Absolute Path Mode.

**Notes:**
- `any` never attempts `ext` resolution.
- If a higher-priority step is applicable but fails, resolution stops and returns an error.

---

### 5. Absolute Path Mode

**Syntax:**
```
srcuri://abs/<path-without-leading-slash>@L<line>[C<column>]
```

**Description:** Uses a full filesystem path to reference a file. The `abs` authority indicates an absolute path. For POSIX paths, a leading `/` is **implied**.

**Path Reconstruction:**
- **POSIX**: `abs/etc/hosts` → `/etc/hosts`
- **Windows drive**: `abs/C:/Windows/...` → `C:/Windows/...`
- **UNC path**: `abs/UNC/server/share/path` → `//server/share/path`

**Use Cases:**
- Local testing and development
- System configuration files
- Scripts that generate links to known absolute locations

**Examples:**

```
srcuri://abs/etc/hosts@L1
Opens /etc/hosts at line 1

srcuri://abs/Users/alice/projects/myapp/src/main.rs@L100C5
Opens /Users/alice/projects/myapp/src/main.rs at line 100, column 5 (macOS)

srcuri://abs/home/bob/code/server/app.py@L42
Opens /home/bob/code/server/app.py at line 42 (Linux)

srcuri://abs/C:/Users/Carol/Dev/project/README.md@L10
Opens C:/Users/Carol/Dev/project/README.md at line 10 (Windows)

srcuri://abs/UNC/fileserver/share/docs/readme.txt@L5
Opens //fileserver/share/docs/readme.txt at line 5 (Windows UNC)
```

**srcuri.com Equivalent:**
```
srcuri://abs/etc/hosts@L1  →  https://srcuri.com/abs/etc/hosts@L1
```

**Windows Path Handling:**

Windows drive paths are represented directly after `abs/`:
```
srcuri://abs/C:/Windows/System32/drivers/etc/hosts@L21
srcuri://abs/D:/repo/project/src/main.rs@L10
```

UNC paths use an explicit `UNC/` marker:
```
srcuri://abs/UNC/server/share/path/to/file.txt@L5
```

**Limitations:**
- Not portable across team members (file paths differ per machine)
- Requires knowing the exact filesystem location
- Cannot be used with git reference query parameters

---

### 6. External URL Mode

**Syntax:**
```
srcuri://ext/<scheme>/<host>/<path...>[?<upstream-query>][#<upstream-fragment>]
```

**Description:** Encodes a remote URL (e.g., GitHub, GitLab) structurally within the srcuri format. The `scheme` and `host` are separate path segments, avoiding embedded `://` in the payload.

**URL Reconstruction:**
```
srcuri://ext/https/github.com/owner/repo/blob/main/file.rs
→ https://github.com/owner/repo/blob/main/file.rs
```

**Critical Rule:** In `ext` mode, the `?query` and `#fragment` belong to the **upstream URL**, not to srcuri options.

**Use Cases:**
- Sharing links that work with or without Sorcery installed
- Converting provider URLs for editor-aware opening
- Fork disambiguation through canonical repository identity

**Examples:**

```
srcuri://ext/https/github.com/user/repo/blob/main/src/lib.rs#L42
→ https://github.com/user/repo/blob/main/src/lib.rs#L42

srcuri://ext/https/gitlab.com/org/project/-/blob/develop/README.md#L10
→ https://gitlab.com/org/project/-/blob/develop/README.md#L10

srcuri://ext/https/bitbucket.org/team/repo/src/main/file.py?at=v1.0#lines-5
→ https://bitbucket.org/team/repo/src/main/file.py?at=v1.0#lines-5
```

**srcuri.com Equivalent:**
```
srcuri://ext/https/github.com/user/repo/blob/main/file.rs#L42
→ https://srcuri.com/ext/https/github.com/user/repo/blob/main/file.rs#L42
```

**Resolution Behavior:**
When an external URL is opened, the resolver may:
- Look up local workspaces for matching repository (by git remote)
- Open the file locally if a match is found
- Open the upstream URL in browser if no local match
- Route through an interstitial UI offering "open in editor" options

See the [External URL Specification](srcuri-provider-passthrough-v1.md) for complete resolution semantics.

---

### 7. Revision Path (Query Extension)

**Syntax:**
```
srcuri://<workspace>/<path>@L<line>?<git-param>=<value>
```

**Description:** References a file at a specific git revision (commit, branch, or tag). Applies to workspace mode links. Provides git-aware features like temporary file viewing or branch checkout.

**Supported Git Parameters:**
- `commit=<SHA>` or `sha=<SHA>` — Reference a specific commit (most precise)
- `branch=<name>` — Reference the current state of a branch
- `tag=<name>` — Reference a tagged version

**Use Cases:**
- Code review comments referencing specific commits
- Bug reports citing exact versions
- Documentation linking to stable releases
- Historical code analysis
- Cross-branch comparisons

**Examples:**

```
srcuri://myrepo/src/file.rs@L23?commit=abc123def456
Opens file.rs at line 23 from commit abc123def456

srcuri://backend/api/routes.py@L100?branch=feature-auth
References routes.py on the feature-auth branch

srcuri://docs/README.md@L1?tag=v1.0.0
Opens README.md from the v1.0.0 tagged release

srcuri://infra/config.yml@L50?sha=7f8a9b2c
References config.yml at commit 7f8a9b2c
```

**Resolution Behavior:**
When a revision path is opened, the protocol handler:
1. Validates workspace is a git repository
2. Verifies commit/branch/tag exists
3. Presents options: view in temporary file (read-only) or checkout reference

---

## URL Component Details

### Line Numbers

- **Formats**:
  - `@Lline` (preferred for URL stability)
  - `@L` (empty line marker; treated as no line)
  - `:line` (legacy)
- **Case**: `@L` is case-insensitive on input; emit uppercase `@L` in new links
- **Indexing**: 1-indexed (first line is line 1)
- **Range**: No upper limit (limited only by file size)
- **Optional**: Yes (omit to open file without jumping to a line)
- **Invalid Values**: Non-numeric values are ignored
- **Empty line marker**: `@L` (and `@l`) is accepted and means "no line specified". This enables formulaic link construction without conditional logic (e.g., terminal linkifiers).
- **Encoding**: `%40L` is accepted when `@` is URL-encoded.

**Examples:**
```
srcuri://myproject/file.rs@L1        → Line 1
srcuri://myproject/file.rs@L42       → Line 42
srcuri://myproject/file.rs@L10000    → Line 10000
srcuri://myproject/file.rs@l42       → Line 42
srcuri://myproject/file.rs%40L42     → Line 42 (encoded @)
srcuri://myproject/file.rs@L         → No line specified (open at top)
srcuri://myproject/file.rs           → No line specified (open at top)
srcuri://myproject/file.rs@Labc      → Invalid, ignored (opens at top)
```

### Column Numbers

- **Formats**:
  - `C<column>` following `@L<line>` (preferred)
  - `:<column>` following `@L<line>` or legacy `:line`
- **Case**: `C` is case-insensitive on input; emit uppercase `C` in new links
- **Indexing**: 1-indexed (first column is column 1)
- **Range**: 0-120 (values above 120 ignore the column)
- **Optional**: Yes (requires line number if specified)

**Examples:**
```
srcuri://myproject/file.rs@L42C10   → Line 42, column 10
srcuri://myproject/file.rs@L42@L10   → Line 42, column 10
srcuri://myproject/file.rs@L42C120  → Line 42, column 120 (max valid)
srcuri://myproject/file.rs@L42C121  → Line 42, column ignored
```

### Line/Column Parsing Rules

Line and column numbers are extracted from the **final path segment** using right-to-left parsing:

1. Let `tail` be the final path segment (after the last `/`) excluding any query/fragment
2. If `tail` ends with `@L<digits>` (or `%40L<digits>`) then parse `line = digits`
3. If `tail` ends with `@L<digits>C<digits>` (or `%40L<digits>C<digits>`) then parse `line`, `col`
4. If `tail` ends with `@L<digits>:<digits>` (or `%40L<digits>:<digits>`) then parse `line`, `col`
5. If `tail` ends with `@L`, `@LC`, `%40L`, or `%40LC` then treat as "no line specified" and remove the suffix
6. Otherwise, if `tail` ends with `:<digits>` then parse `line = digits`
7. If it ends with `:<digits>:<digits>` then parse `line`, `col`
8. Remove the parsed suffix from the path

**Windows Note:** Drive letters like `C:` appear in earlier segments (e.g., `abs/C:/Windows/...`). The line/col parser applies only to the final segment, so `C:` does not conflict.

**Examples:**
```
Input: "file.rs:42:10"
Split: ["file.rs", "42", "10"]
Result: path="file.rs", line=42, column=10

Input: "file.rs:42"
Split: ["file.rs", "42"]
Result: path="file.rs", line=42, column=None

Input: "file.rs@L42"
Split: ["file.rs", "L42"]
Result: path="file.rs", line=42, column=None

Input: "file.rs@L42C10"
Split: ["file.rs", "L42C10"]
Result: path="file.rs", line=42, column=10

Input: "file.rs@L42:10"
Split: ["file.rs", "L42:10"]
Result: path="file.rs", line=42, column=10

Input: "file.rs@L"
Split: ["file.rs", "L"]
Result: path="file.rs", line=None, column=None

Input: "file:with:colons.txt:10:5"
Split: ["file:with:colons.txt", "10", "5"]
Result: path="file:with:colons.txt", line=10, column=5

Input: "file.rs:42:200"
Split: ["file.rs", "42", "200"]
Result: path="file.rs:42:200" (200 > 120, suffix rejected)
```

### Folders

The protocol supports opening folders, not just files:

**Workspace-relative folder:**
```
srcuri://myproject/src/controllers
Opens the controllers folder within the myproject workspace
```

**Absolute path folder:**
```
srcuri://abs/Users/alice/projects/myapp
Opens the myapp folder
```

Line and column numbers are **silently ignored** when opening folders:
```
srcuri://myproject/src@L42C10
→ Opens /path/to/myproject/src folder (line 42, column 10 ignored)
```

---

## Windows Path Handling

Windows paths require special consideration due to drive letters and backslash conventions.

### Backslash Rule

**Parsers** MUST accept backslashes (`\`) in paths and treat them as equivalent to forward slashes (`/`).

**Emitters** MUST always output forward slashes (`/`), never backslashes.

This ensures:
- Windows users can paste native paths and have them work
- Generated links are consistent and URL-safe across platforms
- No escaping issues in URLs, JSON, or other contexts

### Drive Letter Handling

Windows absolute paths include a drive letter followed by a colon. In `abs` mode:

```
srcuri://abs/C:/Users/Carol/Dev/project/README.md@L10
srcuri://abs/D:/repos/myapp/src/main.rs@L42
```

The drive letter colon is distinguished from line/column colons by:
1. Position: Drive letter appears at position 0 of the path (after `abs/`)
2. Pattern: Single letter followed by `:` followed by `/` or `\`

### UNC Path Handling

Windows UNC paths (network shares) use the `UNC/` marker:

```
srcuri://abs/UNC/server/share/path/to/file.txt@L5
→ //server/share/path/to/file.txt
```

### Cross-Platform Workspace Paths

Workspace paths are inherently cross-platform since they don't include filesystem-specific roots:

```
srcuri://myproject/src/main.rs@L42
```

This works identically on Windows, macOS, and Linux—each user's workspace configuration maps `myproject` to their local path.

---

## Query Parameters

Query parameters extend the base URL format to provide additional functionality.

### Git Reference Parameters

#### `commit=<SHA>` or `sha=<SHA>`

References a specific git commit by its SHA hash.

```
srcuri://myrepo/src/main.rs@L42?commit=abc123def456
srcuri://myrepo/README.md@L1?sha=7f8a9b2c1e5d4f3a
```

**Notes:**
- Full or short SHA supported (short must be unambiguous)
- Most precise reference type (immutable)
- Ideal for bug reports and code reviews
- Both `commit=` and `sha=` are equivalent

#### `branch=<name>`

References the current state of a git branch.

```
srcuri://myrepo/src/auth.rs@L100?branch=main
srcuri://myrepo/config.yml@L10?branch=feature-oauth
```

**URL Encoding for Special Characters:**

Branch names containing URL-special characters are automatically encoded/decoded:

| Character | Problem | Encoded | Example |
|-----------|---------|---------|---------|
| `+` | Means space in URLs | `%2B` | `c++` → `?branch=c%2B%2B` |
| `#` | Fragment delimiter | `%23` | `#pr470` → `?branch=%23pr470` |
| `=` | Key/value separator | `%3D` | `fix=memory` → `?branch=fix%3Dmemory` |

#### `tag=<name>`

References a git tag (typically a release version).

```
srcuri://myrepo/CHANGELOG.md@L1?tag=v1.0.0
srcuri://myrepo/src/api.rs@L50?tag=release-2.3.1
```

### Remote Parameter (Clone-on-Demand)

#### `remote=<url>`

Enables sharing links to repositories the recipient may not have cloned locally.

```
srcuri://myrepo/README.md@L1?remote=github.com/user/myrepo
srcuri://lib/src/utils.rs@L42?remote=gitlab.com/org/lib
srcuri://api/routes.py@L100?branch=main&remote=github.com/team/api
```

**Behavior:**
1. If workspace is configured locally → Open file normally (remote param ignored)
2. If workspace not found AND remote specified → Show clone dialog
3. Clone dialog shows:
   - Remote URL
   - Clone destination: `{repo_base_dir}/{workspace_name}`
   - File to open after cloning
   - Branch/ref if specified
4. On confirmation:
   - Repository is cloned to calculated path
   - Workspace mapping is automatically added to settings
   - File opens in editor

**Notes:**
- Remote URL format: `host/org/repo` (without protocol prefix)
- Does not clone if workspace already exists locally
- User must confirm clone operation (not automatic)

### Workspace Hint Parameter

#### `workspaceHint=<name>`

Used in rel or any mode to prefer a specific workspace when multiple matches exist.

```
srcuri://rel/lib/utils.rs@L10?workspaceHint=backend
→ Searches for "lib/utils.rs", preferring matches in "backend" workspace
```

### Parameter Precedence

If multiple git reference parameters are present, only the **first recognized parameter** is used:

```
srcuri://myrepo/file.rs@L10?commit=abc123&branch=main
→ Uses commit=abc123 (commit appears first)

srcuri://myrepo/file.rs@L10?branch=main&tag=v1.0.0
→ Uses branch=main (branch appears first)
```

### Unknown Parameters

Unknown or unsupported query parameters are silently ignored:

```
srcuri://myrepo/file.rs@L10?editor=vscode&theme=dark
→ Unknown parameters 'editor' and 'theme' are ignored
→ URL is treated as: srcuri://myrepo/file.rs@L10
```

---

## URL ↔ Protocol Conversion

### Protocol → URL

Replace leading `srcuri://` with `https://srcuri.com/`. Preserve the remainder verbatim (path, query, fragment).

```
srcuri://myrepo/src/main.rs@L42
→ https://srcuri.com/myrepo/src/main.rs@L42

srcuri://rel/README.md@L1
→ https://srcuri.com/rel/README.md@L1

srcuri://abs/etc/hosts@L1
→ https://srcuri.com/abs/etc/hosts@L1

srcuri://myrepo/file.rs@L10?commit=abc123&remote=github.com/user/myrepo
→ https://srcuri.com/myrepo/file.rs@L10?commit=abc123&remote=github.com/user/myrepo
```

### URL → Protocol

Replace leading `https://srcuri.com/` with `srcuri://`. Preserve the remainder verbatim.

```
https://srcuri.com/myrepo/src/main.rs@L42
→ srcuri://myrepo/src/main.rs@L42
```

**Note:** This spec intentionally avoids semantics that require `//` in the URL path, to reduce mangling by intermediaries.

---

## Normalization (Canonicalization)

Implementations SHOULD normalize inputs into a canonical form before storage/logging:

1. **Mode canonicalization**
   - If authority is not a reserved token, treat as implicit workspace mode

2. **Percent-decoding**
   - Decode percent-encoded octets in path segments *only as needed* for filesystem matching
   - Preserve reserved delimiters (`/`, `?`, `#`) as structural characters

3. **No dot-segment meaning**
   - Implementations MUST NOT assign special semantics to `.` or `..` segments in URL form
   - (They may already be normalized away by intermediaries)

4. **Workspace explicitness**
   - Prefer emitting implicit workspace canonical form:
     - `srcuri://myrepo/path` rather than `srcuri://wks/myrepo/path`
   - Accept both on input

5. **Path normalization**
   - Convert backslashes to forward slashes
   - Remove redundant slashes

---

## Path Resolution

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
Input: srcuri://backend/src/handlers/auth.rs@L42

1. Extract workspace name from authority: "backend"
2. Look up in configuration: "/Users/alice/work/api-server"
3. Append relative path: "/Users/alice/work/api-server/src/handlers/auth.rs"
4. Validate file exists
5. Open at line 42
```

**Error Conditions:**
```
Unknown workspace:
srcuri://unknown/file.rs@L1
→ Error: "Workspace 'unknown' not found in configuration"

File not found:
srcuri://myproject/missing.rs@L10
→ Error: "File not found: /Users/alice/code/myproject/missing.rs"

Path traversal attempt:
srcuri://myproject/../../../etc/passwd@L1
→ Error: "Invalid path (security violation)"
```

### Relative Mode Resolution

Relative mode searches all configured workspaces for matching files.

**Matching Algorithm:**
```
Input: srcuri://rel/main.rs@L10

1. If workspaceHint provided, search that workspace first

2. For each configured workspace:
   a. Recursively search for files matching path
   b. Add matches to results list

3. Prefer matches where workspace name appears in path

4. Based on match count:
   - 0 matches: Return error
   - 1 match: Return file path for immediate opening
   - 2+ matches: Return list for user selection
```

---

## Security Considerations

### Path Traversal Prevention

**Attack Vector:**
```
srcuri://myproject/../../../etc/passwd@L1
```

**Protection:**
1. All paths are normalized using canonical path resolution
2. Resolved paths are validated against workspace boundaries
3. Paths outside configured workspaces trigger confirmation dialogs
4. Absolute paths require explicit user approval (unless in workspace)

### Workspace Boundary Enforcement

Files outside configured workspaces require explicit user consent:

```
Configured workspaces:
  myproject: /Users/alice/code/myproject
  backend: /Users/alice/code/backend

Safe (auto-open):
  srcuri://myproject/src/main.rs@L1
  → /Users/alice/code/myproject/src/main.rs ✓

Requires confirmation:
  srcuri://abs/etc/hosts@L1
  → /etc/hosts (not in any workspace) ⚠
```

### Path Sanitization Rules

Before normalization, the handler rejects paths containing:
- Control characters
- `#`, quotes, angle brackets, braces, pipes
- Wildcards (`*`, `?` in glob context)
- Command separators (`;`, `&`)
- Shell substitution characters (`` ` ``, `$`)
- Executable extensions (`.exe`, `.sh`, `.app`, etc.)

Permitted special characters:
- Parentheses `()` and square brackets `[]` (common in macOS folder names)
- Leading `~` (expanded to home directory; any other `~` is rejected)

### Column Number Bounds

Column numbers are limited to 0-120 range to prevent potential issues in editors.

### Git Reference Validation

Git references are validated against the actual repository before any operations:
1. Verify workspace is a git repository
2. Verify commit/branch/tag exists
3. Check working tree status for checkout operations
4. Require user confirmation for all git operations

### Symbolic Link Handling

Symbolic links are resolved to their targets and validated against workspace boundaries. Cross-workspace symlinks trigger security warnings.

---

## Error Handling

Resolvers MUST produce structured errors for:
- Unknown mode token
- Missing required segments (e.g., `wks` mode missing workspace name)
- Unknown/unmapped workspace (workspace mode)
- No matches found (rel mode)
- Multiple matches without a deterministic policy (rel mode)
- Invalid absolute path encoding (abs mode)
- Invalid upstream encoding (ext mode)

Recommended error object:
- `code` (string, stable)
- `message` (human-readable)
- `details` (map; optional)

---

## Test Vectors

### Workspace (implicit)
**Input:**
```
srcuri://sorcery-desktop/app-core/src/lib.rs@L33
```

**Parsed:**
- mode: `workspace` (implicit)
- workspace: `sorcery-desktop`
- relpath: `app-core/src/lib.rs`
- line: `33`

### Workspace (explicit)
**Input:**
```
srcuri://wks/myrepo/path/file.rs@L22
```

**Parsed:**
- mode: `workspace` (explicit)
- workspace: `myrepo`
- relpath: `path/file.rs`
- line: `22`

### Relative
**Input:**
```
srcuri://rel/config/routes.rb
```

**Parsed:**
- mode: `rel`
- path: `config/routes.rb`

**Input:**
```
srcuri://rel/app-core/src/lib.rs@L33?workspaceHint=sorcery-desktop
```

**Parsed:**
- mode: `rel`
- path: `app-core/src/lib.rs`
- line: `33`
- workspaceHint: `sorcery-desktop`

### Any (best-effort)
**Input:**
```
srcuri://any/src-tauri/src/main.rs@L12
```

**Parsed:**
- mode: `any`
- path: `src-tauri/src/main.rs`
- line: `12`

### Absolute (POSIX)
**Input:**
```
srcuri://abs/etc/hosts@L21
```

**Parsed:**
- mode: `abs`
- absPath: `/etc/hosts`
- line: `21`

### Absolute (Windows drive)
**Input:**
```
srcuri://abs/C:/Windows/System32/drivers/etc/hosts@L21
```

**Parsed:**
- mode: `abs`
- absPath: `C:/Windows/System32/drivers/etc/hosts`
- line: `21`

### Absolute (Windows UNC)
**Input:**
```
srcuri://abs/UNC/server/share/docs/file.txt@L5
```

**Parsed:**
- mode: `abs`
- absPath: `//server/share/docs/file.txt`
- line: `5`

### External (upstream)
**Input:**
```
srcuri://ext/https/github.com/user/repo/blob/main/file.rs?plain=1#L12
```

**Parsed:**
- mode: `ext`
- upstream: `https://github.com/user/repo/blob/main/file.rs?plain=1#L12`

### Revision (workspace with git params)
**Input:**
```
srcuri://myrepo/src/lib.rs@L42?commit=abc123def
```

**Parsed:**
- mode: `workspace` (implicit)
- workspace: `myrepo`
- relpath: `src/lib.rs`
- line: `42`
- git_ref: `commit:abc123def`

---

## Practical Examples

### Quick Start

```
Basic file reference:
srcuri://myproject/README.md@L1

With line and column:
srcuri://myproject/src/main.rs@L42C10

Relative mode (search):
srcuri://rel/package.json@L15

Absolute path:
srcuri://abs/tmp/debug.log@L100
```

### Team Collaboration

**Code review comment:**
```
Found a bug here: srcuri://api-server/src/auth.rs@L156
The authentication check is missing validation.
```

**Issue tracker:**
```
Bug Report #1234
Crash occurs at: srcuri://mobile-app/lib/screens/home.dart@L89C12
Stack trace shows null pointer exception.
```

**Documentation:**
```markdown
# Installation Guide

Edit the configuration file: srcuri://myproject/config/app.yml@L25

Set the `api_key` value to your API key.
```

### Git Workflow Integration

**Referencing a specific commit:**
```
The bug was introduced in: srcuri://backend/src/db.rs@L42?commit=abc123def
```

**Linking to a release:**
```
See the migration guide: srcuri://docs/MIGRATION.md@L1?tag=v1.0.0
```

**Feature branch reference:**
```
Check out the new auth flow: srcuri://api/routes/auth.py@L10?branch=feature-oauth
```

**Sharing with clone support:**
```
srcuri://cool-lib/src/lib.rs@L1?remote=github.com/user/cool-lib
```

### Cross-Platform Examples

**macOS:**
```
srcuri://abs/Users/alice/code/myproject/README.md@L1
srcuri://myproject/src/main.swift@L50
```

**Linux:**
```
srcuri://abs/home/bob/projects/myapp/README.md@L1
srcuri://myapp/src/main.rs@L50
```

**Windows:**
```
srcuri://abs/C:/Users/Carol/Dev/myproject/README.md@L1
srcuri://myproject/src/main.cs@L50
```

**Portable (recommended):**
```
srcuri://myproject/README.md@L1
Works on all platforms with proper workspace configuration
```

---

## Implementer Checklist

- [ ] Parse URI into scheme/authority/path/query/fragment
- [ ] Validate scheme is `srcuri://` (reject `srcuri:` without `//`)
- [ ] Determine mode:
  - [ ] If authority in {wks, rel, any, abs, ext} → explicit mode
  - [ ] Else → implicit workspace mode (authority is workspace name)
- [ ] For each mode, extract required fields
- [ ] Parse `:line[:col]` suffix from final path segment
- [ ] Apply resolution semantics (workspace map, rel search, abs open, ext open)
- [ ] Support URL↔protocol conversion by prefix swap
- [ ] Emit canonical form (implicit workspace) for output
- [ ] Validate security constraints (path traversal, boundaries)
- [ ] Handle git query parameters when present

---

## Extensions

The core srcuri protocol can be extended with additional capabilities. Extensions are defined in separate specifications and are optional—the core protocol is fully functional without them.

### External URL Mode (Provider Integration)

The `ext` mode enables direct conversion from git hosting provider URLs (GitHub, GitLab, Bitbucket, etc.), enabling:

- **Viral sharing** — Share links that open in Sorcery for users who have it installed
- **Fallback viewing** — Users without Sorcery can still view code via the provider's web interface
- **Fork disambiguation** — Links carry the canonical repository identity
- **Easy conversion** — Transform any provider URL by restructuring the scheme/host

**Example:**
```
Original:    https://github.com/user/myrepo/blob/main/src/lib.rs#L42
Protocol:    srcuri://ext/https/github.com/user/myrepo/blob/main/src/lib.rs#L42
Web:         https://srcuri.com/ext/https/github.com/user/myrepo/blob/main/src/lib.rs#L42
```

See the [External URL Specification](srcuri-provider-passthrough-v1.md) for complete details on provider resolution semantics.
