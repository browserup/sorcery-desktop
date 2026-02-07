# Sorcery Features

> **Maintenance**: When adding a new feature, update this document as a final step. Add the feature under the appropriate section and update the table of contents if adding a new section.

## Core Features

**Share code locations that open in any editor.** The srcuri:// protocol lets teams share precise code references (file + line) that open in each developer's preferred editor—VS Code, JetBrains, Vim, Emacs, or 20+ others.

| Feature | Description |
|---------|-------------|
| **Click-to-open links** | `srcuri://myproject/src/main.rs@L42` opens line 42 in your editor |
| **25+ supported editors** | VS Code, JetBrains IDEs, Vim, Neovim, Emacs, Sublime, Zed, more |
| **Workspace mapping** | Map project names to local paths for portable links |
| **Git-aware opening** | Open files at specific commits, branches, or tags |
| **Auto-clone** | Clone repositories when clicking links to repos you don't have |

---

## Table of Contents

- [srcuri:// Protocol](#srcuri-protocol)
- [Supported Editors](#supported-editors)
- [Workspaces](#workspaces)
- [Settings](#settings)
- [Git Integration](#git-integration)
- [Sorcery UI](#sorcery-ui)
- [Setup Wizard](#setup-wizard)
- [Extension Protocol](#extension-protocol)
- [Protocol Registration](#protocol-registration)
- [System Integration](#system-integration)
- [Installation](#installation)

---

## srcuri:// Protocol

The srcuri:// protocol enables editor-independent code linking. Developers can share references that open in each recipient's preferred editor.

### URL Formats

| Format | Example | Description |
|--------|---------|-------------|
| Implicit Workspace | `srcuri://myproject/src/main.rs@L42` | Authority is workspace name (recommended) |
| Explicit Workspace | `srcuri://wks/myproject/src/main.rs@L42` | Explicit workspace mode |
| Relative (Search) | `srcuri://rel/file.rs@L42` | Searches all workspaces |
| Any (Best-Effort) | `srcuri://any/file.rs@L42` | Best-effort resolution for unknown path context |
| Absolute Path | `srcuri://abs/etc/hosts@L42` | Absolute filesystem path |
| External URL | `srcuri://ext/https/github.com/user/repo/blob/main/file.rs#L42` | Git provider URL |
| Revision Path | `srcuri://myproject/file.rs@L42?commit=abc123` | Git-aware with revision |

### Location Specifiers

- **Line number**: `file.rs:42` (1-indexed)
- **Line and column**: `file.rs:42:5` (both 1-indexed)

### Query Parameters

| Parameter | Example | Description |
|-----------|---------|-------------|
| `commit` | `?commit=abc1234` | Open at specific commit |
| `branch` | `?branch=main` | Open at branch head |
| `tag` | `?tag=v1.0.0` | Open at tag |
| `remote` | `?remote=github.com/org/repo` | Clone URL if workspace not found |

### Path Matching

Relative mode (`srcuri://rel/...`) uses intelligent path resolution:

**Resolution Priority:**
1. **Workspace-in-path** (highest): If the path contains a workspace name as a segment, extract the relative path and resolve in that workspace. Example: `rel/a/b/myproject/src/main.rs` → opens `src/main.rs` in `myproject` workspace
2. **workspaceHint**: If `?workspaceHint=name` is provided, prioritize that workspace
3. **Suffix matching**: Search all workspaces for the file path
4. **MRU sorting**: Multiple matches are sorted by most recently used

**Matching Rules:**
- Case-insensitive workspace name matching
- Segment-based matching (full path segment, not substring)
- Cross-platform path normalization (Windows backslashes → forward slashes)
- Security validation: blocks path traversal, dangerous extensions

**Strict Workspace Mode:**
Workspace paths (`srcuri://myproject/...`) resolve only within the named workspace. If the workspace is not configured, an error is shown—no fallback to match mode or other workspaces.

---

## Supported Editors

Sorcery supports 25+ editors across VS Code family, JetBrains IDEs, terminal editors, and others.

### VS Code Family (5)

| Editor | ID | Notes |
|--------|----|-------|
| Visual Studio Code | `vscode` | Default editor |
| Cursor | `cursor` | AI-powered VS Code fork |
| VSCodium | `vscodium` | Open source VS Code |
| Roo Cline | `roo` | AI coding assistant |
| Windsurf | `windsurf` | AI IDE |

### JetBrains IDEs (11)

| Editor | ID | Notes |
|--------|----|-------|
| IntelliJ IDEA | `idea` | Java/Kotlin |
| WebStorm | `webstorm` | JavaScript/TypeScript |
| PyCharm | `pycharm` | Python |
| PhpStorm | `phpstorm` | PHP |
| RubyMine | `rubymine` | Ruby/Rails |
| GoLand | `goland` | Go |
| CLion | `clion` | C/C++ |
| Rider | `rider` | .NET |
| DataGrip | `datagrip` | Databases |
| Android Studio | `androidstudio` | Android development |
| Fleet | `fleet` | Polyglot IDE |

**JetBrains Features:**
- Toolbox discovery (stable and EAP channels)
- Binary caching with 5-minute TTL
- Auto-retry on version updates

### Terminal Editors (6)

| Editor | ID | Notes |
|--------|----|-------|
| Vim | `vim` | Launches in terminal |
| Neovim | `neovim` | Socket-based session reuse |
| Emacs | `emacs` | Uses emacsclient |
| Kakoune | `kakoune` | Modal editor |
| Micro | `micro` | Modern terminal editor |
| Nano | `nano` | Simple editor |

**Terminal Editor Features:**
- Configurable terminal preference
- Neovim: discovers running sessions via socket, matches workspace
- Emacs: reuses existing daemon sessions

### Other Editors (5)

| Editor | ID | Notes |
|--------|----|-------|
| Zed | `zed` | High-performance editor |
| Sublime Text | `sublime` | Cross-platform |
| Kate | `kate` | KDE editor |
| Gedit | `gedit` | GNOME editor |
| Xcode | `xcode` | macOS only |

### Editor Capabilities

- **Folder support**: Most editors (over 20) support opening folders
- **Line/column navigation**: All editors support positioning
- **Running instance detection**: Per-editor process monitoring

---

## Workspaces

Workspaces map project names to filesystem paths, enabling portable, partial-path URLs.

### Configuration

- Each workspace has: path, optional name, optional editor preference
- Paths are normalized (~ expansion, symlink resolution)
- Workspace-specific editor overrides global default

### MRU Tracking

- Persisted to `~/.config/sorcery/workspace_mru.yaml`
- Event-driven tracking: records activity when links open or user selects in chooser
- Best-effort workspace detection from editor window titles
- Effective ranking: `MAX(last_seen, folder_mtime, reflog_time)`
- On-demand git reflog checks (top 5 matches only for performance)
- Recent workspaces sorted first in chooser

### Workspace Chooser

- Shown when multiple workspaces match a partial path
- Displays workspace names, paths, and last-seen times
- Single-click selection

### Workspace Identity and Health

- Deterministic workspace keys (unique key per active mapping)
- Git and non-git workspaces are both first-class mappings
- Remote-aware disambiguation for fork/upstream collisions
- Drift detection for deleted, moved, or identity-changed workspaces
- Repair workflows: rebind path, rename key, resolve conflict, forget mapping
- Enterprise policy mode (advisory/enforced) for canonical key/remote controls
- Detailed design spec: `dev/workspace-identity-resolution-spec.md`

---

## Settings

Configuration stored in `~/.config/sorcery-desktop/settings.yaml`.

### Global Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `editor` | `vscode` | Default editor |
| `allow_non_workspace_files` | `false` | Allow absolute paths outside workspaces |
| `preferred_terminal` | `auto` | Terminal for terminal editors |
| `repo_base_dir` | `~/code` | Base directory for cloning |
| `auto_switch_clean_branches` | `true` | Auto-checkout if working tree is clean |

### Per-Workspace Settings

- Path to workspace root
- Optional display name
- Optional editor preference override

### Editor Last-Seen Tracking

- Tracks which editors have been active recently
- Persisted to `~/.config/sorcery/last_seen.yaml`
- Used for "most-recent" editor selection

---

## Git Integration

Git-aware features for working with code at specific revisions.

### Revision Operations

- Open files at specific commits, branches, or tags
- View file content at historical revisions
- Get commit metadata and timestamps

### Working Tree Status

- Clean/dirty detection
- Modified and untracked file counts
- Checkout availability checking
- WIP detection (uncommitted changes)

### Revision Dialog

- Shows current branch vs target revision
- Displays working tree status
- Indicates if checkout is available or blocked
- Lists blocking reasons (dirty tree, merge state)

### Clone Support

- Clone dialog when workspace not found
- Conflict-aware clone suggestions when workspace key and remote differ
- Repository URL from `?remote=` parameter
- Configurable clone destination
- One-click clone and open workflow

---

## Sorcery UI

Dark-themed UI components for user interactions.

### Settings Window

- Tabbed interface for configuration
- Installed editors list with status
- Workspace management (add/edit/remove)
- Workspace health banner (missing, unavailable, drifted, conflict)
- Default editor selection
- Terminal preference configuration

### Workspace Chooser Dialog

- Modal dialog for multiple workspace matches
- Shows workspace names, paths, timestamps
- Click to select and open

### Revision Handler Dialog

- Git revision selection interface
- Current vs target state comparison
- Checkout status and blocking reasons
- Working tree status display

### Clone Dialog

- Repository clone prompt
- URL and destination configuration
- Progress indication
- Preflight validation and key-collision hints

### Workspace Repair Dialog

- Shown when a mapped workspace is missing, unavailable, drifted, or conflicted
- Rebind workspace path from folder picker
- Retry link open after repair
- Forget mapping option

### Workspace Conflict Dialog

- Shown when workspace key matches but `?remote=` points to a different repository
- Lists existing mappings with state and primary remote
- Open selected existing mapping
- Continue to clone flow for requested remote

### Flash Message

- Transient notification overlay
- Shows branch switching operations
- Auto-dismisses after 2.5 seconds

---

## Setup Wizard

First-run experience that guides new users through initial configuration.

### Wizard Steps

| Step | Description |
|------|-------------|
| **Welcome** | Introduces Sorcery and explains its purpose |
| **Default Editor** | Grid of detected editors; user selects preferred one |
| **Workspaces Folder** | Auto-detected suggestions with repository counts |
| **Chrome Extension** | Prompt to install browser extension (shown if Chrome detected) |
| **Ready** | Summary of configuration and test link |

### First-Run Detection

- `setup_completed` flag in settings tracks wizard completion
- Wizard shown automatically if flag is false
- Normal operation proceeds after completion

### Browser Detection

Cross-platform detection for Chrome, Firefox, and Edge:
- macOS: Checks /Applications and ~/Applications
- Linux: Checks /usr/bin paths and snap directories
- Windows: Checks registry and standard installation paths

---

## Extension Protocol

Special protocol URLs for browser extension integration.

### Ping

```
srcuri://ping
```

Used by the browser extension to check if Sorcery is installed and running. Returns immediately—no UI shown.

### Hello

```
srcuri://hello?version=1.0.0
```

Sent by the extension on install to register its presence and version. The `version` parameter is optional.

---

## Protocol Registration

System integration for srcuri:// URL handling.

### Platform Support

| Platform | Method |
|----------|--------|
| macOS | LaunchServices API |
| Windows | MSI installer / Registry |
| Linux | xdg-mime / .desktop files |

### Registration Status

- Protocol registered check
- Executable path verification
- Current vs registered executable comparison
- Re-registration support

---

## System Integration

### Background Service

- Runs in system tray
- No visible window by default
- Protocol URLs activate, handle, then hide

### Tray Menu

- Settings: Opens configuration
- Quit: Exits application

### Process Monitoring

- 15-second polling for active editor detection
- Tracks foreground application
- Updates last-seen data per editor

### Platforms

| Platform | Features |
|----------|----------|
| macOS | NSWorkspace APIs, LaunchServices, AppleScript |
| Windows | GetForegroundWindow, Registry |
| Linux | X11/Wayland detection, XDG compliance |

---

## Installation

### Curl Installer

One-line installation for macOS and Linux:

```bash
curl -fsSL https://getsorcery.com/install.sh | sh
```

The installer:
- Detects OS (macOS/Linux) and architecture (x64/arm64)
- Fetches the latest release from GitHub
- Downloads the appropriate package (DMG, DEB, RPM, or AppImage)
- Installs to the standard location
- Launches the app
- Prints next steps for extension installation

### Platform Packages

| Platform | Format | Notes |
|----------|--------|-------|
| macOS | DMG | Universal binary (arm64 + x64) |
| Windows | MSI | x64 installer |
| Linux (Debian/Ubuntu) | DEB | apt-compatible |
| Linux (Fedora/RHEL) | RPM | dnf/yum-compatible |
| Linux (Other) | AppImage | Universal Linux binary |

### Package Managers

| Manager | Platform | Command |
|---------|----------|---------|
| Homebrew | macOS | `brew install --cask ebeland/sorcery-desktop/sorcery-desktop` |
| WinGet | Windows | `winget install ebeland.SorceryDesktop` |
| AUR | Arch Linux | `yay -S sorcery-bin` |

Package manager manifests are automatically updated on each release via GitHub Actions.
