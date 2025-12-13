# Sorcery Desktop Features

> **Maintenance**: When adding a new feature, update this document as a final step. Add the feature under the appropriate section and update the table of contents if adding a new section.

## Core Features

**Share code locations that open in any editor.** The srcuri:// protocol lets teams share precise code references (file + line) that open in each developer's preferred editor—VS Code, JetBrains, Vim, Emacs, or 20+ others.

| Feature | Description |
|---------|-------------|
| **Click-to-open links** | `srcuri://myproject/src/main.rs:42` opens line 42 in your editor |
| **26 supported editors** | VS Code, JetBrains IDEs, Vim, Neovim, Emacs, Sublime, Zed, more |
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
- [Protocol Registration](#protocol-registration)
- [System Integration](#system-integration)

---

## srcuri:// Protocol

The srcuri:// protocol enables editor-independent code linking. Developers can share references that open in each recipient's preferred editor.

### URL Formats

| Format | Example | Description |
|--------|---------|-------------|
| Partial Path | `srcuri://file.rs:42` | Searches all workspaces |
| Workspace Path | `srcuri://myproject/src/main.rs:42` | Direct workspace reference |
| Full Path | `srcuri:///absolute/path/file.rs:42` | Absolute filesystem path |
| Revision Path | `srcuri://myproject/file.rs:42?commit=abc123` | Git-aware with revision |

### Location Specifiers

- **Line number**: `file.rs:42` (1-indexed)
- **Line and column**: `file.rs:42:5` (both 1-indexed)

### Query Parameters

| Parameter | Example | Description |
|-----------|---------|-------------|
| `commit` | `?commit=abc1234` | Open at specific commit |
| `branch` | `?branch=main` | Open at branch head |
| `tag` | `?tag=v1.0.0` | Open at tag |
| `remote` | `?remote=https://github.com/org/repo.git` | Clone URL if workspace not found |

### Path Matching

- Case-insensitive workspace name matching
- Searches across all configured workspaces
- MRU-sorted results when multiple matches exist
- Security validation: blocks path traversal, dangerous extensions

---

## Supported Editors

Sorcery supports 26 editors across VS Code family, JetBrains IDEs, terminal editors, and others.

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

### Other Editors (4)

| Editor | ID | Notes |
|--------|----|-------|
| Zed | `zed` | High-performance editor |
| Sublime Text | `sublime` | Cross-platform |
| Kate | `kate` | KDE editor |
| Xcode | `xcode` | macOS only |

### Editor Capabilities

- **Folder support**: 22 of 26 editors support opening folders
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
- 20-second polling for activity detection
- Tracks last-seen timestamps
- Recent workspaces sorted first in chooser

### Workspace Chooser

- Shown when multiple workspaces match a partial path
- Displays workspace names, paths, and last-seen times
- Single-click selection

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

### Flash Message

- Transient notification overlay
- Shows branch switching operations
- Auto-dismisses after 2.5 seconds

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

- 10-second polling for active editor detection
- Tracks foreground application
- Updates last-seen data per editor

### Platforms

| Platform | Features |
|----------|----------|
| macOS | NSWorkspace APIs, LaunchServices, AppleScript |
| Windows | GetForegroundWindow, Registry |
| Linux | X11/Wayland detection, XDG compliance |
