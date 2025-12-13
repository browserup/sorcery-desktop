# CLI Implementation Specification

## Executive Summary

This document specifies the implementation of a standalone CLI tool (`srcuri-cli`) that enables command-line and server-based usage of Sorcery protocol links without requiring a GUI or running process.
The CLI tool will share core logic with the existing Sorcery Desktop application through a shared library crate.

## Use Cases

### Primary Use Case: Server Error Reporting
```bash
# Application on server encounters config error
# Prints: "Config error at srcuri:///etc/myapp/config.yaml:42"
# User runs:
srcuri srcuri:///etc/myapp/config.yaml:42
# Opens in $EDITOR at line 42
```

Learn more about the Sorcery protocol at srcuri.com.

### Secondary Use Cases
- Quick file opening from terminal without launching GUI
- Integration with CLI tools (git, ripgrep, compiler errors)
- SSH sessions where GUI is unavailable
- CI/CD pipelines for opening files locally after remote failures

## Architecture Overview

### Three-Crate Structure

```
srcuri/
├── Cargo.toml                    # Workspace root
├── srcuri-core/                # Shared library [NEW]
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── parser.rs             # URL parsing (from Tauri)
│       ├── editors/              # Editor managers (from Tauri)
│       │   ├── mod.rs
│       │   ├── traits.rs
│       │   ├── vscode.rs
│       │   ├── vim.rs
│       │   ├── neovim.rs
│       │   └── ... (all editors)
│       └── settings.rs           # Settings models (from Tauri)
├── srcuri-cli/                 # CLI tool [NEW]
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── cli.rs                # Argument parsing
│       ├── editor_selector.rs    # Editor selection logic
│       └── launcher.rs           # Editor launch orchestration
└── src-tauri/                    # Sorcery Desktop app [EXISTING]
    ├── Cargo.toml                # Now depends on srcuri-core
    └── src/
        ├── main.rs
        ├── lib.rs
        ├── commands/
        ├── dispatcher/
        ├── protocol_handler/
        ├── tracker/
        └── ... (GUI-specific code)
```

### Dependency Graph

```
┌─────────────────┐
│  srcuri-cli   │ (standalone binary, no Tauri)
└────────┬────────┘
         │
         ├─────────────────┐
         │                 │
         v                 v
┌────────────────┐   ┌──────────────┐
│ srcuri-core  │   │ clap, dirs   │
└────────┬───────┘   └──────────────┘
         │
         v
┌────────────────────────────────────┐
│ anyhow, thiserror, serde_yaml, ... │
└────────────────────────────────────┘

┌─────────────────┐
│   src-tauri     │ (Sorcery Desktop app)
└────────┬────────┘
         │
         ├─────────────────┬──────────────────┐
         │                 │                  │
         v                 v                  v
┌────────────────┐   ┌──────────────┐  ┌──────────┐
│ srcuri-core  │   │ Tauri deps   │  │ cocoa... │
└────────────────┘   └──────────────┘  └──────────┘
```

## Code Extraction: Tauri → Core Library

### What Moves to `srcuri-core`

#### 1. URL Parser (`protocol_handler/parser.rs`)
**Current location:** `src-tauri/src/protocol_handler/parser.rs`
**New location:** `srcuri-core/src/parser.rs`

**Changes needed:**
- None! Already dependency-free, uses only `anyhow`
- Export `SrcuriRequest` enum and `SrcuriParser` struct
- All tests move with it

**API:**
```rust
pub enum SorceryRequest {
    PartialPath { path: String, line: Option<usize>, column: Option<usize> },
    WorkspacePath { workspace: String, path: String, line: Option<usize>, column: Option<usize> },
    FullPath { full_path: String, line: Option<usize>, column: Option<usize> },
    RevisionPath { workspace: String, path: String, git_ref: GitRef, line: Option<usize>, column: Option<usize> },
}

pub enum GitRef {
    Commit(String),
    Branch(String),
    Tag(String),
}

pub struct SorceryParser;
impl SorceryParser {
    pub fn parse(link: &str) -> Result<SrcuriRequest>;
}
```

#### 2. Editor Trait & Managers (`editors/`)
**Current location:** `src-tauri/src/editors/`
**New location:** `srcuri-core/src/editors/`

**Files to move:**
- `traits.rs` → Core trait definitions
- `vscode.rs` → VS Code family (code, cursor, vscodium, roo, windsurf)
- `jetbrains.rs` → JetBrains IDEs (requires simplification)
- `terminal/*.rs` → All terminal editors (vim, nvim, emacs, etc.)
- `others.rs` → Zed, Sublime (Xcode stays in Tauri - GUI only)
- `kate.rs` → Kate editor

**Changes needed:**

1. **Simplify JetBrains Manager:**
   - Remove Toolbox scanning (GUI-specific, heavyweight)
   - CLI mode: Only check standard paths and `which`
   - Document that CLI mode won't find Toolbox installations

2. **Simplify Neovim Socket Detection:**
   - Current: Recursive search of `/tmp` for sockets
   - CLI mode: Check `$NVIM` env var only (set by terminal)
   - Fallback: Launch new instance

3. **Remove Platform-Specific GUI Code:**
   - `get_running_instances()` → Return empty vec in CLI mode
   - macOS: Remove NSWorkspace detection
   - Keep binary finding logic

4. **Trait Simplification:**
```rust
// Core trait (minimal, CLI-compatible)
pub trait EditorManager: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    async fn find_binary(&self) -> Option<PathBuf>;
    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()>;
}

// Tauri-specific extension (in src-tauri)
pub trait EditorManagerExt: EditorManager {
    async fn get_running_instances(&self) -> EditorResult<Vec<EditorInstance>>;
}
```

**Dependencies to add to core:**
- `async-trait` (already in Tauri)
- `thiserror` (already in Tauri)
- Keep platform-specific deps minimal

#### 3. Settings Models (`settings/models.rs`)
**Current location:** `src-tauri/src/settings/models.rs`
**New location:** `srcuri-core/src/settings.rs`

**What to move:**
- `Settings` struct
- `DefaultEditorConfig` struct
- `WorkspaceConfig` struct
- Defaults (for optional CLI usage)

**What stays in Tauri:**
- `SettingsManager` (file I/O, caching, Arc<RwLock>)
- `LastSeenData` (active editor tracking - GUI only)

**API:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub defaults: DefaultEditorConfig,
    pub workspaces: Vec<WorkspaceConfig>,
}

// CLI will read YAML directly using serde_yaml
// Tauri will use SettingsManager wrapper
```

### What Stays in `src-tauri`

**GUI-Specific Components:**
- `commands/` - Tauri commands (frontend IPC)
- `tracker/` - Active editor tracking (NSWorkspace polling)
- `dispatcher/` - Orchestration with workspace matching
- `protocol_handler/mod.rs` - Protocol handling (deep-link plugin)
- `protocol_handler/matcher.rs` - Workspace matching
- `protocol_handler/git.rs` - Git reference support (commit/branch/tag)
- `path_validator/` - Security validation (optional for CLI)
- `git_command_log/` - Logging infrastructure
- `main.rs` - Tauri app initialization
- Platform-specific UI code (system tray, windows)

## CLI Tool Design

### Command-Line Interface

```bash
# Format
srcuri [OPTIONS] <PATH>

# PATH accepts:
# - srcuri:// URLs:    srcuri:///etc/hosts:22
# - Absolute paths:       /etc/hosts:22
# - Paths with ~ :        ~/code/project/main.rs:100:5

# OPTIONS:
#   -e, --editor <EDITOR>     Override editor (vim, nvim, code, etc.)
#   -h, --help                Print help
#   -V, --version             Print version
```

### Examples

```bash
# Use $EDITOR (if set)
export EDITOR=nvim
srcuri /etc/hosts:22

# Override editor
srcuri --editor vim /var/log/app.log:100

# From srcuri URL
srcuri srcuri:///home/user/config.yaml:15:8

# With tilde expansion
srcuri ~/project/src/main.rs:42

# From error message
cargo build 2>&1 | grep error | while read line; do
    # Parse file:line from compiler output
    srcuri "$file:$line"
done
```

### Implementation: `srcuri-cli/src/main.rs`

```rust
use clap::Parser;
use srcuri_core::{SrcuriParser, SorceryRequest, editors::*};
use anyhow::{Result, bail, Context};
use std::path::{Path, PathBuf};
use std::env;

#[derive(Parser)]
#[command(name = "srcuri")]
#[command(version, about = "Open files in your preferred editor", long_about = None)]
struct Cli {
    /// File path or srcuri:// URL with optional :line:col
    /// Examples: /etc/hosts:22, srcuri:///path/to/file:10:5
    path: String,

    /// Editor to use (overrides $EDITOR and config file)
    /// Examples: vim, nvim, code, cursor, idea
    #[arg(short, long)]
    editor: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Parse input (URL or plain path)
    let location = parse_cli_input(&cli.path)?;

    // Select editor
    let editor_id = select_editor(cli.editor)?;

    // Get editor manager
    let registry = EditorRegistry::new();
    let manager = registry.get(&editor_id)
        .ok_or_else(|| anyhow::anyhow!("Editor '{}' not found", editor_id))?;

    // Verify editor installed
    if !manager.is_installed().await {
        bail!("Editor '{}' is not installed", manager.display_name());
    }

    // Open file
    let options = OpenOptions {
        line: location.line,
        column: location.column,
        new_window: false,
        terminal_preference: None,
    };

    manager.open(&location.path, &options).await
        .context(format!("Failed to open {} in {}", location.path.display(), manager.display_name()))?;

    eprintln!("Opened {} in {}", location.path.display(), manager.display_name());
    Ok(())
}

struct FileLocation {
    path: PathBuf,
    line: Option<usize>,
    column: Option<usize>,
}

fn parse_cli_input(input: &str) -> Result<FileLocation> {
    // Try parsing as srcuri:// URL first
    if input.starts_with("srcuri://") {
        let request = SorceryParser::parse(input)?;

        return match request {
            SorceryRequest::FullPath { full_path, line, column } => {
                let path = expand_path(&full_path)?;
                Ok(FileLocation { path, line, column })
            }
            SorceryRequest::PartialPath { .. } => {
                bail!("CLI mode requires full path. Use: srcuri:///full/path or /full/path")
            }
            SorceryRequest::WorkspacePath { .. } => {
                bail!("CLI mode does not support workspace paths. Use full path: srcuri:///full/path")
            }
            SorceryRequest::RevisionPath { .. } => {
                bail!("CLI mode does not support git references. Use Tauri app for git reference support.")
            }
        };
    }

    // Otherwise parse as plain path with optional :line:col
    parse_plain_path(input)
}

fn parse_plain_path(input: &str) -> Result<FileLocation> {
    // Split on last colons for line/column
    let parts: Vec<&str> = input.rsplitn(3, ':').collect();

    let (path_str, line, column) = match parts.len() {
        1 => (parts[0], None, None),
        2 => {
            let line = parts[0].parse::<usize>().ok();
            (parts[1], line, None)
        }
        3 => {
            let column = parts[0].parse::<usize>().ok();
            let line = parts[1].parse::<usize>().ok();
            (parts[2], line, column)
        }
        _ => (input, None, None),
    };

    let path = expand_path(path_str)?;

    // Verify file exists
    if !path.exists() {
        bail!("File does not exist: {}", path.display());
    }

    if !path.is_file() {
        bail!("Path is not a file: {}", path.display());
    }

    Ok(FileLocation { path, line, column })
}

fn expand_path(path_str: &str) -> Result<PathBuf> {
    let expanded = shellexpand::tilde(path_str);
    let path = PathBuf::from(expanded.as_ref());

    // Must be absolute
    if !path.is_absolute() {
        bail!("CLI mode requires absolute paths. Got: {}", path_str);
    }

    Ok(path)
}

fn select_editor(explicit: Option<String>) -> Result<String> {
    // 1. Explicit flag
    if let Some(editor) = explicit {
        return Ok(editor);
    }

    // 2. $EDITOR environment variable
    if let Ok(editor) = env::var("EDITOR") {
        // Extract binary name from path (e.g., /usr/bin/vim -> vim)
        let editor_name = PathBuf::from(&editor)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&editor);

        return Ok(map_editor_name(editor_name));
    }

    // 3. Check for settings file (optional compatibility with Tauri app)
    if let Some(config_dir) = dirs::config_dir() {
        let settings_path = config_dir.join("sorcery-desktop").join("settings.yaml");
        if settings_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&settings_path) {
                if let Ok(settings) = serde_yaml::from_str::<srcuri_core::Settings>(&contents) {
                    return Ok(settings.defaults.editor);
                }
            }
        }
    }

    // 4. Auto-detect installed editors
    detect_installed_editor().await
}

fn map_editor_name(name: &str) -> String {
    // Map common $EDITOR values to srcuri editor IDs
    match name {
        "vi" => "vim".to_string(),
        "emacs" | "emacsclient" => "emacs".to_string(),
        "nano" => "nano".to_string(),
        _ => name.to_string(),
    }
}

async fn detect_installed_editor() -> Result<String> {
    let registry = EditorRegistry::new();

    // Check editors in priority order
    let priority = vec![
        "nvim", "vim", "code", "nano", "emacs", "cursor",
        "zed", "sublime", "vi"
    ];

    for editor_id in priority {
        if let Some(manager) = registry.get(editor_id) {
            if manager.is_installed().await {
                return Ok(editor_id.to_string());
            }
        }
    }

    bail!("No editor found. Set $EDITOR or install vim/nano/nvim")
}
```

### Implementation: `srcuri-cli/Cargo.toml`

```toml
[package]
name = "srcuri-cli"
version = "0.1.0"
description = "Standalone CLI for opening files via srcuri:// links"
authors = ["Eric Beland"]
license = "MIT"
edition = "2021"

[[bin]]
name = "srcuri"
path = "src/main.rs"

[dependencies]
srcuri-core = { path = "../srcuri-core" }
clap = { version = "4.5", features = ["derive"] }
tokio = { version = "1.40", features = ["rt-multi-thread", "process"] }
anyhow = "1.0"
dirs = "5.0"
shellexpand = "3.1"
serde_yaml = "0.9"

[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
strip = true        # Strip symbols
```

## Core Library Design

### `srcuri-core/Cargo.toml`

```toml
[package]
name = "srcuri-core"
version = "0.1.0"
description = "Core parsing and editor management for srcuri"
authors = ["Eric Beland"]
license = "MIT"
edition = "2021"

[lib]
name = "srcuri_core"
path = "src/lib.rs"

[dependencies]
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
async-trait = "0.1"
tokio = { version = "1.40", features = ["process"] }
tracing = "0.1"

[target.'cfg(target_os = "macos")'.dependencies]
# Minimal - only for binary detection if needed

[target.'cfg(target_os = "windows")'.dependencies]
# Minimal - only for binary detection if needed

[target.'cfg(target_os = "linux")'.dependencies]
# Minimal - only for binary detection if needed
```

### `srcuri-core/src/lib.rs`

```rust
pub mod parser;
pub mod editors;
pub mod settings;

pub use parser::{SrcuriParser, SorceryRequest};
pub use editors::{EditorManager, OpenOptions, EditorRegistry, EditorResult, EditorError};
pub use settings::{Settings, DefaultEditorConfig, WorkspaceConfig};
```

## Workspace Root Configuration

### `Cargo.toml` (workspace root)

```toml
[workspace]
members = [
    "srcuri-core",
    "srcuri-cli",
    "src-tauri",
]
resolver = "2"

[workspace.dependencies]
# Shared dependencies
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
tokio = { version = "1.40", features = ["full"] }
async-trait = "0.1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
dirs = "5.0"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### Updated `src-tauri/Cargo.toml`

```toml
[package]
name = "srcuri"
version = "0.1.0"
# ... (rest unchanged)

[dependencies]
srcuri-core = { path = "../srcuri-core" }

# Tauri-specific deps
tauri = { version = "2.0.0", features = ["tray-icon", "protocol-asset"] }
tauri-plugin-deep-link = "2.0.0"
# ... (rest unchanged, remove duplicates now in workspace)

# Use workspace versions
anyhow = { workspace = true }
serde = { workspace = true }
# ... etc
```

## Editor Selection Logic

### CLI Priority (Simplified)

```
1. --editor flag
2. $EDITOR environment variable
3. ~/.config/sorcery-desktop/settings.yaml (if exists)
4. Auto-detect (nvim → vim → code → nano → emacs)
5. Error: No editor found
```

### Tauri Priority (Full-Featured)

```
1. Explicit editor parameter (from command)
2. "most-recent" hint + ActiveEditorTracker
3. Workspace-specific editor (from settings)
4. Default editor (from settings)
5. Auto-detect
```

## Editor Support Matrix

| Editor | CLI Support | Tauri Support | Notes |
|--------|-------------|---------------|-------|
| VS Code family (code, cursor, etc.) | ✅ Full | ✅ Full | CLI via `--goto` |
| JetBrains (IDEA, WebStorm, etc.) | ⚠️ Partial | ✅ Full | CLI: no Toolbox detection |
| Vim | ✅ Full | ✅ Full | Launch in terminal |
| Neovim | ⚠️ Limited | ✅ Full | CLI: no socket reuse, $NVIM only |
| Emacs | ✅ Full | ✅ Full | CLI via `+line` flag |
| Nano | ✅ Full | ✅ Full | CLI via `+line` flag |
| Zed | ✅ Full | ✅ Full | CLI via `zed` command |
| Sublime | ✅ Full | ✅ Full | CLI via `subl` |
| Xcode | ❌ None | ✅ Full | GUI only (macOS) |
| Kate | ✅ Full | ✅ Full | CLI via `kate` |

## Binary Size Targets

| Binary | Target Size | Dependencies |
|--------|-------------|--------------|
| `srcuri-cli` | 2-5 MB | Core + clap + tokio (minimal) |
| `srcuri` (Sorcery Desktop) | 40-60 MB | Core + Tauri + WebView |

## Testing Strategy

### Unit Tests

```rust
// srcuri-core/src/parser.rs
#[cfg(test)]
mod tests {
    // All existing parser tests move here
}

// srcuri-core/src/editors/vscode.rs
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_find_binary() {
        // Test binary detection
    }
}
```

### Integration Tests

```rust
// srcuri-cli/tests/integration_test.rs
#[tokio::test]
async fn test_cli_open_file() {
    let output = Command::new("cargo")
        .args(&["run", "--", "/tmp/test.txt:10"])
        .env("EDITOR", "echo") // Mock editor
        .output()
        .unwrap();

    assert!(output.status.success());
}

// Test URL parsing
#[tokio::test]
async fn test_cli_parse_srcuri_url() {
    let output = Command::new("cargo")
        .args(&["run", "--", "srcuri:///tmp/test.txt:10"])
        .output()
        .unwrap();

    assert!(output.status.success());
}

// Test error cases
#[tokio::test]
async fn test_cli_rejects_partial_path() {
    let output = Command::new("cargo")
        .args(&["run", "--", "srcuri://README.md"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("requires full path"));
}
```

## Installation & Distribution

### Cargo Install

```bash
# Install CLI only
cargo install --path srcuri-cli

# Install from git
cargo install --git https://github.com/user/srcuri srcuri-cli
```

### Pre-built Binaries

```bash
# GitHub Actions workflow
name: Release

on:
  release:
    types: [created]

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build CLI
        run: cargo build --release -p srcuri-cli

      - name: Upload binary
        uses: actions/upload-release-asset@v1
        with:
          asset_path: target/release/srcuri
          asset_name: srcuri-${{ runner.os }}-${{ runner.arch }}
```

### Shell Integration

```bash
# Add to ~/.bashrc or ~/.zshrc
alias he='srcuri'

# Function for quick editing
edit() {
    srcuri "$@"
}
```

## Migration Path

### Phase 1: Extract Core (Week 1)
1. Create `srcuri-core/` crate structure
2. Move `parser.rs` (no changes needed)
3. Move editor `traits.rs` (no changes needed)
4. Move `settings/models.rs` (no changes needed)
5. Run tests, ensure Tauri app still compiles

### Phase 2: Extract Editor Managers (Week 1-2)
1. Move `vscode.rs` (no changes)
2. Move `terminal/*.rs` (simplify Neovim socket detection)
3. Move `jetbrains.rs` (remove Toolbox scanning)
4. Move `others.rs` (exclude Xcode)
5. Update `src-tauri` imports
6. Run all tests

### Phase 3: Build CLI (Week 2)
1. Create `srcuri-cli/` crate
2. Implement `main.rs` with clap parsing
3. Implement editor selection logic
4. Implement path validation
5. Write integration tests
6. Document CLI usage

### Phase 4: Testing & Polish (Week 3)
1. Test on Linux server environments
2. Test with various $EDITOR settings
3. Optimize binary size
4. Add error message improvements
5. Write documentation
6. Create installation scripts

### Phase 5: Release (Week 3)
1. Set up GitHub Actions for releases
2. Build binaries for all platforms
3. Update README with CLI instructions
4. Announce CLI tool availability

## Documentation Requirements

### README Updates

Add section:

```markdown
## CLI Tool (Server Usage)

For command-line and server environments, use `srcuri-cli`:

### Installation

```bash
cargo install srcuri-cli
```

### Usage

```bash
# Open file at specific line
srcuri /etc/hosts:22

# Use specific editor
srcuri --editor vim /var/log/app.log:100

# From srcuri URL
srcuri srcuri:///home/user/config.yaml:15
```

### Configuration

Set your preferred editor:

```bash
export EDITOR=nvim
```

Or use the `--editor` flag to override.
```

### New Doc: `docs/CLI.md`

Comprehensive CLI documentation:
- Installation methods
- Usage examples
- Editor detection logic
- Integration with tools (git, ripgrep)
- Server deployment patterns
- Troubleshooting

## Error Messages

CLI should provide clear, actionable errors:

```bash
# No editor found
Error: No editor found. Set $EDITOR or install vim/nano/nvim
Examples:
  export EDITOR=nvim
  srcuri --editor vim /path/to/file

# Partial path rejected
Error: CLI mode requires full path. Got: README.md
Use: srcuri:///full/path or /full/path

# File not found
Error: File does not exist: /etc/missing.conf

# Workspace path rejected
Error: CLI mode does not support workspace paths
Use full path: srcuri:///home/user/project/file.rs
Or use the Tauri app for workspace support
```

## Performance Considerations

### Startup Time

Target: < 100ms total (input → editor launch)

```
Parse arguments:     ~5ms   (clap)
Parse URL/path:      ~1ms   (parser)
Detect editor:       ~20ms  (if auto-detect, can cache)
Find binary:         ~10ms  (filesystem checks)
Launch editor:       ~50ms  (process spawn)
------------------------------------
Total:              ~86ms
```

### Binary Size Optimization

```toml
[profile.release]
opt-level = "z"     # Optimize for size (not speed)
lto = true          # Link-time optimization
codegen-units = 1   # Slower build, smaller binary
strip = true        # Remove debug symbols
panic = "abort"     # Smaller panic handler
```

Expected size: 2-3 MB (vs Tauri's 40-60 MB)

## Security Considerations

### Path Validation

CLI is more permissive than GUI (local trust):

```rust
// Tauri: Strict validation (untrusted input from protocol)
// - Block path traversal
// - Block suspicious characters
// - Block executables
// - Require workspace membership

// CLI: Minimal validation (trusted local user)
// - Allow any path user has permission to
// - Verify file exists
// - Expand ~ and environment variables
// - No workspace requirement
```

### Environment Variable Trust

CLI trusts `$EDITOR` without validation:
- User controls their environment
- Shell escaping handled by Rust `Command`
- No injection risk (not using shell=true)

## Compatibility Matrix

### Rust Version

Minimum: 1.70 (for workspace dependencies)

### OS Support

| OS | CLI | Tauri | Notes |
|----|-----|-------|-------|
| macOS 11+ | ✅ | ✅ | Full support |
| Ubuntu 20.04+ | ✅ | ✅ | Full support |
| Debian 11+ | ✅ | ✅ | Full support |
| RHEL 8+ | ✅ | ⚠️ | CLI only on servers |
| Windows 10+ | ✅ | ✅ | Full support |

### Architecture

| Arch | Support | Notes |
|------|---------|-------|
| x86_64 | ✅ | Primary target |
| aarch64 | ✅ | Apple Silicon, ARM servers |
| armv7 | ⚠️ | May work, not tested |

## Success Metrics

### CLI Adoption
- Downloads per month
- GitHub stars/forks
- Integration examples from community

### Performance
- Binary size < 5 MB
- Startup time < 100ms
- Memory usage < 10 MB

### Reliability
- No crashes on invalid input
- Clear error messages
- 100% test coverage for core parsing

## Future Enhancements

### V1.1: Enhanced Features
- `--list-editors` flag (show available editors)
- `--check` flag (verify file exists without opening)
- Config file generation (`srcuri --init`)

### V1.2: Remote Support
- `--remote` flag for SSH file editing
- Integration with `scp`/`rsync` patterns
- Remote → local path mapping

### V2.0: Protocol Handler
- Register CLI as protocol handler (macOS/Linux)
- Compete with or complement Tauri app
- User choice: GUI or CLI for protocol

## Open Questions

1. **Should CLI read Tauri's settings file by default?**
   - Pro: Consistency between CLI and GUI
   - Con: Adds dependency on specific config location
   - Decision: Optional fallback, don't require it

2. **Should we publish separate crates to crates.io?**
   - `srcuri-core` → Maybe (if useful to others)
   - `srcuri-cli` → Yes (for `cargo install`)
   - Decision: Publish CLI, keep core internal for now

3. **Should CLI support workspace paths eventually?**
   - Would require reading `settings.yaml`
   - Increases complexity
   - Decision: No, keep CLI simple. Use Tauri for workspaces.

4. **Should we combine binaries (`srcuri --gui` vs `srcuri <path>`)?**
   - Pro: Single installation
   - Con: Larger binary, more complexity
   - Decision: Keep separate for now, can merge later

## Conclusion

This implementation provides a lightweight, server-friendly CLI tool that shares core logic with the existing Sorcery Desktop application while maintaining separation of concerns. The three-crate structure (core, CLI, Tauri) enables code reuse without coupling unrelated features, and provides users with choice: full-featured GUI or minimal CLI.

For more information about the Sorcery protocol, visit srcuri.com.

Key benefits:
- ✅ Works on servers without GUI
- ✅ Fast startup (< 100ms)
- ✅ Small binary (< 5 MB)
- ✅ Respects $EDITOR
- ✅ Shares code with Tauri app
- ✅ No breaking changes to existing app

Implementation time: 2-3 weeks for full release with tests and documentation.
