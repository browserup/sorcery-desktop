# Sorcery

Hyperlinks for your editor (or IDE) that open on the right line. That's *Sorcery!*

# Sorcery enables:

* Share editor links with other developers that open in their editor, on the correct line
* Command-Click a file-path in your terminal--it opens in your editor/IDE
* Create a link in repo A to a file in repo B that will open in your editor when you command-click it
* Share a link to a file in a branch in slack or JIRA. When developers click it, it opens to that file with that branch checked out
* Writing a blog on how to configure your /etc/hosts? Link: srcuri://ect/hosts  will open it in the user's editor
Use it in your onboarding docs, too.


## With Sorcery Extension

* On github/gitlab? Command-click a file path, and it will open in your editor
* Command-Click an error in Datadog and be on that file/line in your editor
* Command-Click on stack trace lines in your browser in your dev env, and jump to the file/line in your editor
* Command-click on source code lines in github/gitlab and jump to the file/line

Sorcery Desktop is an open source tool, built on the open-source srcuri:// protocol

    scuri:// pronounced Sorcery, gives you a URI to source code that opens in your editor

Sorcery Desktop provides the local protocol handler component for srcuri:// links. 
It routes srcuri:// protocol links to your editor or IDE of choice.


What's it do:
* Sorcery Desktop - Makes srcuri links open in your editor, right to the file/line
* Sorcery Chrome Extension - Makes stack traces, and file paths in the browser command-clickable so they open
in your editor (via srcuri protocol).
* srcuri.com - Responds to the srcuri protocol syntax in URLs. It lets you create 


## Problem: Your co-worker shares links to source code--but they open in github, not your editor.

You just stare at the code, looking through the window. You can't
* run it
* add breakpoints
* compile it
* use your vim hotkeys
* use your LLM to analyze it

No Longer!

Sorcery uses the srcuri:// protocol to link to lines of code in *your* editor. Now, your coworker shares:

srcuri://reponame/path/to/file.js@L53

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
srcuri://myrepo/src/main.rs@L42
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
https://srcuri.com/open#src/main.rs@L42?workspace=myrepo
```

When clicked, the web page parses the URL fragment and redirects to:
```
srcuri://myrepo/src/main.rs@L42
```

Sorcery Desktop then opens your editor to that exact file and line.

**Key features:**
- Fragment-based URLs (paths never sent to server)
- Enterprise subdomain support for multi-tenant deployments
- Tenant-specific configuration via `/.well-known/srcuri.json`
- Dockerized for easy cloud deployment
- AGPL licensed

## Architecture

Sorcery Desktop is built with Tauri/Rust to keep it lightweight. It sits in the system tray while running.

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
1. Deep link clicked: srcuri://project/file.rs@L42
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
srcuri://<authority>/<path>:<line>:<column>?editor=<editor-id>
```

- `authority`: Workspace name (default) or reserved token (`wks`, `rel`, `any`, `abs`, `ext`)
- `path`: Relative path for workspace modes, search path for `rel`/`any`, or absolute path for `abs`
- `line`: Optional line number (1-indexed)
- `column`: Optional column number (1-indexed)
- `editor`: Optional editor hint (overrides preferences)

Examples:
```
srcuri://myapp/src/main.rs@L42
srcuri://any/src/main.rs@L42
srcuri://abs/etc/hosts@L1
srcuri://webapp/index.ts@L10C5?editor=cursor
srcuri://backend/api/handler.go@L100?editor=goland
```

### Opening Folders

In addition to files, srcuri:// links can open folders in most editors:

```
srcuri://myapp/src/controllers          # Open a folder within a workspace
srcuri://abs/Users/dev/projects/myapp    # Open an absolute folder path
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
