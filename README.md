# Sorcery Desktop

## Problem: Your co-worker shares links to source code--but they open in github, not your editor.

You just stare at the code, looking through the window. You can't
* run it
* add breakpoints
* compile it
* use your vim hotkeys
* use your LLM to analyze it

No Longer!

Sorcery uses the srcuri:// protocol to link to lines of code in *your* editor. Now, your coworker shares:

srcuri://reponame/path/to/file.js:53

With one click, you're on *that line* in **your** local editor and repo.

How it works:

An editor agnostic protocol backed by Sorcery Desktop - a Free, Open Source, MIT Licensed
launcher you install locally.

* Open Source, MIT Licensed
* In Rust to keep memory usage small
* editor-agnostic deep link handler

## Why Sorcery?

Instead of:
```
https://github.com/user/myrepo/blob/main/src/main.rs#L42
```

Use:
```
srcuri://myrepo/src/main.rs:42
```

When clicked, this opens `src/main.rs` at line 42 in **your** preferred editor - whether that's VS Code, IntelliJ IDEA, Neovim, Vim, Emacs, Sublime Text, or any other supported editor.

## Features

Workspace matching. Sorcery maps your srcuri links to the matching workspace on your machine.

- **Editor-agnostic**: Works with 15+ editors out of the box
- **Cross-platform**: macOS, Windows, and Linux support
- **Smart editor detection**: Automatically finds installed editors and tracks which you use most
- **Toolbox-aware**: Seamlessly handles JetBrains Toolbox installations with auto-updates
- **Session reuse**: Reuses existing editor sessions when possible (e.g., Neovim sockets)
- **Zero configuration**: Works out of the box with sensible defaults
- **MIT licensed**: Free and open source

## Supported Editors

### Visual Studio Code Family
- Visual Studio Code
- VSCodium
- Cursor

### JetBrains IDEs
- IntelliJ IDEA (Community & Ultimate)
- RubyMine
- PyCharm
- WebStorm
- GoLand
- PhpStorm
- CLion
- Rider
- RustRover
- DataGrip
- AppCode

### Terminal Editors
- Neovim (with socket-based session reuse)
- Vim
- Emacs (via emacsclient)

### Other
- Sublime Text
- Zed

## Web Gateway (Sorcery Server)

The Sorcery Server (available separately at [github.com/ebeland/sorcery-server](https://github.com/ebeland/sorcery-server)) provides a web gateway that enables srcuri links to work in contexts where custom protocols face limitations (Jira, Slack, web browsers).

**How it works:**
```
https://srcuri.com/open#src/main.rs:42?workspace=myrepo
```

When clicked, the web page parses the URL fragment and redirects to:
```
srcuri://myrepo/src/main.rs:42
```

Sorcery Desktop then opens your editor to that exact file and line.

**Key features:**
- Fragment-based URLs (paths never sent to server)
- Enterprise subdomain support for multi-tenant deployments
- Tenant-specific configuration via `/.well-known/srcuri.json`
- Dockerized for easy cloud deployment
- AGPL licensed

## Architecture

Sorcery Desktop is built with Tauri 2.0, combining a Rust backend with a web-based UI for configuration and testing.

### Core Components

```
src-tauri/src/
├── main.rs              # Application entry point, Tauri setup
├── settings/            # Settings persistence and management
│   ├── mod.rs           # SettingsManager
│   └── models.rs        # Settings data structures
├── path_validator/      # Path security and normalization
│   └── mod.rs           # PathValidator
├── editors/             # Editor integrations
│   ├── mod.rs           # EditorRegistry
│   ├── traits.rs        # EditorManager trait
│   ├── vscode.rs        # VS Code family managers
│   ├── jetbrains.rs     # JetBrains IDE manager
│   ├── terminal.rs      # Vim, Neovim, Emacs managers
│   ├── sublime.rs       # Sublime Text manager
│   └── zed.rs           # Zed manager
├── tracker/             # Active editor detection
│   ├── mod.rs           # ActiveEditorTracker
│   └── detector.rs      # OS-specific frontmost app detection
├── dispatcher/          # Request routing and execution
│   └── mod.rs           # EditorDispatcher
└── commands.rs          # Tauri command handlers
```

### Component Responsibilities

#### **SettingsManager** (`settings/`)
- Persists user preferences to JSON
- Manages editor preferences per workspace
- Tracks last-seen editors and timestamps
- Thread-safe with RwLock for concurrent access

#### **PathValidator** (`path_validator/`)
- Sanitizes and normalizes file paths
- Validates path existence
- Prevents path traversal attacks
- Handles symlinks and relative paths

#### **EditorRegistry** (`editors/`)
- Central registry of all editor managers
- Provides lookup by editor ID
- Discovers installed editors on-demand
- Caches binary locations with TTL

#### **EditorManager Trait** (`editors/traits.rs`)
Each editor implements the `EditorManager` trait:
```rust
#[async_trait]
pub trait EditorManager: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    async fn find_binary(&self) -> Option<PathBuf>;
    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()>;
    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>>;
}
```

Key features per editor type:

**VS Code Family** (`vscode.rs`):
- Detects VS Code, VSCodium, and Cursor
- Uses CLI flags: `--goto <file>:<line>:<column>`
- Reuses existing windows by default

**JetBrains IDEs** (`jetbrains.rs`):
- Unified manager for all JetBrains products
- Toolbox-aware with mtime-based version selection
- Handles both standalone and Toolbox installations
- Auto-retry on launch failure with cache invalidation
- Platform-specific launching:
  - macOS: `open -n -a <app> --args --line <num> <file>`
  - Windows: `cmd.exe /c start "" <exe> --line <num> <file>`
  - Linux: Direct execution with detached process
- 5-minute binary cache TTL

**Terminal Editors** (`terminal.rs`):
- **Neovim**: Socket discovery and reuse via `nvim --server`
  - Recursively searches `/tmp` and `$TMPDIR` for sockets
  - Matches socket to workspace via `getcwd()`
  - Falls back to new terminal window if no socket found
- **Vim**: Opens in Terminal.app via AppleScript
- **Emacs**: Uses `emacsclient` for session reuse

#### **ActiveEditorTracker** (`tracker/`)
- Polls every 10 seconds to detect frontmost application
- Uses platform-specific APIs:
  - macOS: NSWorkspace
  - Windows: GetForegroundWindow
  - Linux: X11/Wayland detection
- Maps process names to editor IDs
- Updates last-seen timestamps

#### **EditorDispatcher** (`dispatcher/`)
- Routes `open()` requests to appropriate editor
- Validates paths before opening
- Determines editor selection:
  1. Explicit editor hint from request
  2. Workspace-specific preference
  3. Most recently used editor
  4. First available editor
- Handles errors and provides user feedback

### Data Flow

```
1. Deep link clicked: srcuri://project/file.rs:42
   ↓
2. OS routes to sorcery application
   ↓
3. Dispatcher.open(file, line, column, hint)
   ↓
4. PathValidator.validate(file)
   ↓
5. EditorDispatcher.determine_editor(workspace, hint)
   ↓
6. EditorManager.open(file, OpenOptions)
   ↓
7. Platform-specific launch command
   ↓
8. Editor opens file at specified location
```

### JetBrains Implementation Deep Dive

The JetBrains manager implements sophisticated discovery and launching:

**Binary Discovery** (macOS example):
1. Check cache (5-minute TTL)
2. Look for standalone `.app` in `/Applications` and `~/Applications`
3. Search Toolbox installations:
   - `~/Library/Application Support/JetBrains/Toolbox/apps/<product>/`
   - Check `ch-0` (stable) then `ch-1` (EAP)
   - Sort versions by modification time (newest first)
   - Return full `.app` path (not internal CLI script)
4. Heuristic fallback: search all Toolbox products for matching `.app`
5. Cache result (or null) for 5 minutes

**Launch Strategy**:
- macOS uses `open -n -a` to force new instance (required for argument passing)
- Without `-n`, macOS activates existing instance and ignores arguments
- Arguments passed as: `--line <num> <file>` (not `<file>:<line>`)

**Auto-retry on Failure**:
```rust
let result = spawn_editor(binary, args);
if result.is_err() {
    cache.invalidate();
    if let Some(new_binary) = find_binary() {
        return spawn_editor(new_binary, args);
    }
}
```

This handles Toolbox updates seamlessly - if the cached binary is deleted, we rediscover it.

### Neovim Socket Discovery

Neovim integration uses Unix domain sockets for IPC:

1. **Socket Discovery**: Recursively search `/tmp` and `$TMPDIR` up to 2 levels deep
   - Example: `$TMPDIR/nvim.user/aKHN7l/nvim.79673.0`
   - Filters for socket file type using `FileTypeExt::is_socket()`

2. **Workspace Matching**: For each socket, query current directory:
   ```rust
   nvim --server <socket> --remote-expr "getcwd()"
   ```
   Match target file path against nvim's cwd to find best session.

3. **File Opening**: Send commands via remote protocol:
   ```rust
   nvim --server <socket> --remote-send ":{line}<CR>:e {file}<CR>"
   ```
   Path escaping: backslashes → `\\`, spaces → `\ `

4. **Fallback**: If no socket found, spawn new instance in Terminal.app

## Building from Source

### Quick Start (Development)

```bash
# macOS - Build, install to /Applications, and register protocol
./install-local.sh

# For faster iterations (just updates the binary, no full rebuild)
./install-dev.sh

# Manual steps
cd src-tauri
cargo build                    # Build debug version
cargo tauri build --debug      # Create app bundle
```

The install scripts will:
1. Build the application
2. Kill any running instances
3. Copy to `/Applications/Sorcery Desktop.app`
4. Register the `srcuri://` protocol handler

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed development instructions.

### Prerequisites
- Rust 1.70+
- Node.js 18+
- Platform-specific:
  - macOS: Xcode Command Line Tools
  - Windows: Visual Studio Build Tools
  - Linux: webkit2gtk, libayatana-appindicator

### Build

```bash
# Install dependencies
cd src-tauri
cargo build --release

# Development mode with hot reload
npm install
npm run tauri dev

# Production build
npm run tauri build
```

### Testing

The application includes a built-in testbed UI for testing editor integrations:

1. Run `cargo run` or `npm run tauri dev`
2. Testbed window opens automatically
3. Select editor and test file opening
4. View debug output in terminal

## Configuration

Settings are stored in:
- macOS: `~/Library/Application Support/sorcery-desktop/settings.yaml`
- Windows: `%APPDATA%\sorcery-desktop\settings.yaml`
- Linux: `~/.config/sorcery-desktop/settings.yaml`

Example settings:
```yaml
defaults:
  editor: vscode
  allow_non_workspace_files: false
  preferred_terminal: auto
  repo_base_dir: ~/code
  auto_switch_clean_branches: true

workspaces:
  - path: ~/code/rust-project
    name: rust-project
    editor: idea
  - path: ~/code/web-project
    name: web-project
    editor: cursor
```

## Deep Link Format (srcuri protocol)

The srcuri protocol (also known as the "Sorcery protocol") uses this format:

```
srcuri://<workspace>/<path>:<line>:<column>?editor=<editor-id>
```

- `workspace`: Logical workspace name (maps to filesystem path)
- `path`: Relative path within workspace
- `line`: Optional line number (1-indexed)
- `column`: Optional column number (1-indexed)
- `editor`: Optional editor hint (overrides preferences)

Examples:
```
srcuri://myapp/src/main.rs:42
srcuri://webapp/index.ts:10:5?editor=cursor
srcuri://backend/api/handler.go:100?editor=goland
```

### Opening Folders

In addition to files, srcuri:// links can open folders in most editors:

```
srcuri://myapp/src/controllers           # Open a folder within a workspace
srcuri:///Users/dev/projects/myapp       # Open an absolute folder path
```

Most editors (22 of 26) support opening folders. Line/column numbers are silently ignored for folders.

For the full protocol specification, see [srcuri.com](https://srcuri.com)

## License

MIT License - see [LICENSE](MIT-LICENSE) for details.

## Contributing

Contributions welcome! Areas of interest:
- Additional editor integrations
- Windows/Linux testing and fixes
- Deep link protocol enhancements
- UI/UX improvements

## Links

- **Website**: [srcuri.com](https://srcuri.com)
- **Protocol Spec**: [srcuri.com](https://srcuri.com)
- **Server**: [github.com/ebeland/sorcery-server](https://github.com/ebeland/sorcery-server)
- **Chrome Extension**: [github.com/ebeland/sorcery-chrome](https://github.com/ebeland/sorcery-chrome)

## Credits

Built from the ground up in Rust with Tauri for better performance, maintainability, and cross-platform support.
