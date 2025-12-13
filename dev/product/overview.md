# Sorcery Desktop: Porting from Node.js to Tauri

> **Historical Note**: This document describes the initial port from the original Node.js implementation to Tauri. The port is now complete. This is kept for reference.

## Project Goal
- Tracks active editors across platforms
- Responds to custom URL protocol links (srcuri://)
- Manages editor dispatching based on workspace configuration
- Provides occasional UI dialogs when needed

## Component Inventory

### 2. Active Editor Tracker (src/core/active-editor-tracker.js)
- Polls every 10-15 seconds to detect frontmost editor
- Platform-specific detection:
  - macOS: osascript queries
  - Windows: PowerShell window title queries
  - Linux: xdotool/wmctrl window queries
- Persisted last-seen data to config file
- Tracks timestamps for each editor
- Identifies specific editors (VSCode, IntelliJ IDEs, Vim, Neovim, etc.)

### 3. User Settings (src/core/user-settings.js)
- YAML-based configuration
- Schema:
  - workspaces: Array of workspace configurations
  - defaults: Default editor preferences
- Each repo config maps path � editor

### 4. Editor Dispatcher (src/core/editor-dispatcher.js)
- Routes file open requests to correct editor
- Path validation and sanitization
- Workspace matching
- Supports "most-recent" editor selection
- Integrates with ActiveEditorTracker

### 5. Repo/Path Matcher (src/core/repo-path-matcher.js)
- Verifies paths exist
- Matches paths to configured workspace roots
- Security validation (no path traversal)

### 6. Editor Managers (src/editors/*.js)
Base class: EditorManager
- Abstract interface: open(), getRunningInstances()

Implementations:
- **VSCodeManager** - VSCode, Cursor, VSCodium, Roo, Windsurf
- **JetBrainsManager** - IntelliJ, RubyMine, PyCharm, GoLand, WebStorm, PhpStorm, Rider, CLion, RustRover, DataGrip, AppCode
- **NeovimManager** - Neovim
- **VimManager** - Vim
- **EmacsManager** - Emacs
- **XcodeManager** - Xcode
- **EclipseManager** - Eclipse
- **ZedManager** - Zed
- **SublimeTextManager** - Sublime Text
- **VisualStudioManager** - Visual Studio
- **NotepadPlusPlusManager** - Notepad++

Each manager handles:
- Detecting editor installations (including Toolbox for JetBrains)
- Opening files with optional line numbers
- Process detection

### 7. Settings UI (public/settings.html)
- Web-based settings interface
- Configure workspaces
- Set default editors
- Manage repo � editor mappings

### 8. Testbed UIs
- **editor-testbed.html** - Test editor detection
- **pagelink-testbed.html** - Test path detection

## Port Strategy: Node � Tauri

### Architecture Changes

**Current (Node.js):**
- HTTP server on localhost:52788
- Browser extension/external processes hit HTTP endpoints
- Polling-based active editor detection
- HTML files served from /public

**Target (Tauri):**
- Sorcery Desktop registered as URL protocol handler (srcuri://)
- Deep linking plugin handles srcuri:// URLs (Sorcery protocol)
- Backend Rust code for:
  - Active editor tracking
  - Settings management
  - Editor dispatching
  - Editor managers
- Minimal frontend (Svelte/React/Vue) for:
  - Settings UI (Tauri window)
  - Confirmation dialogs
  - Testbed UIs
- Background app with system tray icon

## Port Steps & Documentation

Each component will get its own detailed implementation guide in ./ai/:

### Step 1: Project Setup
**File:** `1-tauri-setup.md`
- [ ] Initialize Tauri project
- [ ] Configure deep linking plugin
- [ ] Register srcuri:// protocol
- [ ] Set up build system (Rust + frontend)

### Step 2: Active Editor Tracker
**File:** `2-active-editor-tracker.md`
- [ ] Port detection logic to Rust
- [ ] macOS: Use osascript via std::process::Command
- [ ] Windows: PowerShell queries
- [ ] Linux: xdotool/wmctrl
- [ ] JSON persistence to config dir
- [ ] Background polling task

### Step 3: Settings Management
**File:** `3-settings-management.md`
- [ ] Define settings schema (YAML or TOML)
- [ ] Rust struct with serde
- [ ] Load/save to platform config directory
- [ ] Tauri command interface for frontend

### Step 4: Path Validation & Repo Matching
**File:** `4-path-validation.md`
- [ ] Path sanitization
- [ ] Workspace matching logic
- [ ] Security checks
- [ ] Path normalization

### Step 5: Editor Manager Trait
**File:** `5-editor-manager-trait.md`
- [ ] Define Rust trait for EditorManager
- [ ] Common interface: open(), detect_instances()
- [ ] Platform-specific utilities
- [ ] Process spawning abstraction

### Step 6: VSCode Family Managers
**File:** `6-vscode-managers.md`
- [ ] VSCode, Cursor, VSCodium, Roo, Windsurf
- [ ] CLI invocation with --goto
- [ ] Process detection
- [ ] App bundle handling (macOS)

### Step 7: JetBrains Manager
**File:** `7-jetbrains-manager.md`
- [ ] Detection across all JetBrains IDEs
- [ ] Toolbox installation support
- [ ] Standard installation paths
- [ ] Line number support
- [ ] Platform-specific launching

### Step 8: Terminal Editor Managers
**File:** `8-terminal-editors.md`
- [ ] Neovim manager
- [ ] Vim manager
- [ ] Emacs manager
- [ ] Terminal detection challenges

### Step 9: Other Editor Managers
**File:** `9-other-editors.md`
- [ ] Xcode
- [ ] Eclipse
- [ ] Visual Studio
- [ ] Zed
- [ ] Sublime Text
- [ ] Notepad++

### Step 10: Editor Dispatcher
**File:** `10-editor-dispatcher.md`
- [ ] Route requests to correct manager
- [ ] Workspace � editor mapping
- [ ] Most-recent editor logic
- [ ] Error handling

### Step 11: Deep Link Handler
**File:** `11-deep-link-handler.md`
- [ ] Parse srcuri:// URLs
- [ ] Extract file path, line number, editor
- [ ] Validate and dispatch
- [ ] URL scheme format design

### Step 12: Settings UI
**File:** `12-settings-ui.md`
- [ ] Tauri window for settings
- [ ] Workspace configuration UI
- [ ] Editor selection
- [ ] Save/load via Tauri commands

### Step 13: Testbed UIs
**File:** `13-testbed-uis.md`
- [ ] Editor detection testbed
- [ ] Path detection testbed
- [ ] Debug/diagnostic tools

### Step 14: Background App & System Tray
**File:** `14-background-app.md`
- [ ] App lifecycle management
- [ ] System tray icon
- [ ] Auto-start on login
- [ ] Graceful shutdown


## Dependencies (Rust)

Expected crates:
- **tauri** - Application framework
- **serde** / **serde_json** / **serde_yaml** - Serialization
- **tokio** - Async runtime
- **dirs** - Platform config directories
- **url** - URL parsing
- Platform-specific:
  - macOS: **core-foundation**, **cocoa**
  - Windows: **winapi**, **windows**
  - Linux: Process execution for xdotool

## Migration Notes

### What We're NOT Porting
- Browser extension (content.js, background.js, manifest.json)
- Detection engine for web page path extraction
- HTTP server endpoints (replaced by deep linking)

### Key Differences
- No CORS needed (no HTTP server)
- Deep linking instead of localhost HTTP
- Native Rust performance for editor tracking
- Tauri's native dialog APIs instead of web UI (where appropriate)
- Platform-specific config directories

## Success Criteria

The port is complete when:
1. srcuri:// links (Sorcery protocol) open files in correct editor
2. Active editor tracking works across platforms
3. Settings UI allows workspace configuration
4. All major editors supported
5. Background app runs transparently
6. Auto-launches on system startup
7. System tray integration

## Next Steps

1. Review this overview
2. Create detailed implementation docs for each step
3. Start with Step 1 (Tauri setup) once approved
