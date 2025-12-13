# Sorcery Desktop Protocol Handler - Complete Guide

## Overview

Sorcery Desktop uses the custom `srcuri://` protocol to open files in configured editors. The protocol handler operates silently in the background, processing URLs from browsers, command-line tools, and other applications without showing any visible UI.

For more information on the protocol specification, visit srcuri.com.

## Protocol Format

```
srcuri://<workspace>/<path>:<line>:<column>?<query-params>
```

See [URL-FORMATS.md](../../URL-FORMATS.md) for detailed format documentation.

## Architecture

### Two Launch Paths

The protocol handler supports two distinct launch mechanisms depending on how the URL is invoked:

| Launch Method | Platforms | URL Source | Process Lifetime |
|--------------|-----------|------------|------------------|
| **Command-Line** | All | `argv[1]` | Process and exit |
| **Deep-Link Event** | macOS, iOS, Android | Tauri event system | Stay running |

### Platform-Specific Behavior

#### macOS
- **Command-line:** URL passed as `argv[1]`, app exits after processing
- **Browser:** App launches via Tauri, receives deep-link event, stays running
- **Registration:** Info.plist `CFBundleURLTypes`
- **Silent operation:** `LSUIElement` + `ActivationPolicy::Accessory` + explicit hide calls

#### Linux
- **All invocations:** URL passed as command-line argument
- **Process:** Always exits after handling
- **Registration:** `.desktop` file with `MimeType=x-scheme-handler/srcuri`
- **Handler:** `xdg-open` routes to registered desktop file

#### Windows
- **All invocations:** URL passed as command-line argument
- **Process:** Always exits after handling
- **Registration:** Registry key `HKEY_CLASSES_ROOT\srcuri`
- **Handler:** Windows shell routes to registered executable

## Protocol Handler Flows

### Flow 1: Command-Line Invocation (All Platforms)

```
User or application runs:
  $ open "srcuri:///etc/hosts:1"        # macOS
  $ xdg-open "srcuri:///etc/hosts:1"    # Linux
  $ start srcuri:///C:/file.txt:1       # Windows
    ↓
OS launches Sorcery Desktop with URL as argv[1]
    ↓
main() checks args, finds srcuri:// URL
    ↓
Calls protocol_handler.handle_url(url)
    ↓
Parses URL → determines file path, line, column
    ↓
Opens file in configured editor via EditorDispatcher
    ↓
Exits immediately (never starts Tauri GUI)
```

**Result:** ✅ Silent, fast, no UI, process exits

**Code path:** `src-tauri/src/main.rs:72-91`

### Flow 2: Browser Click (macOS Only)

```
User clicks srcuri:// link in Chrome/Safari/Firefox
    ↓
Browser asks: "Open Sorcery Desktop?" → User clicks "Allow"
    ↓
macOS launches sorcery.app (no args)
    ↓
main() finds no URL in args, starts Tauri app
    ↓
Tauri setup() runs:
  - Set ActivationPolicy::Accessory (no Dock/Cmd+Tab)
  - Hide all windows
  - Register deep-link event listener
    ↓
macOS sends URL via deep-link mechanism
    ↓
Tauri plugin fires "deep-link://new-url" event
    ↓
Event listener receives event:
  - Payload is JSON array: ["srcuri:///etc/hosts:1"]
  - IMMEDIATELY call app.hide() to prevent window activation
  - Parse JSON array, extract first URL
    ↓
Calls protocol_handler.handle_url(url)
    ↓
Parses URL → determines file path, line, column
    ↓
Opens file in configured editor via EditorDispatcher
    ↓
Calls app.hide() again (ensure app stays hidden)
    ↓
App continues running in background for future links
```

**Result:** ✅ Silent (no window, no Dock icon), app stays running

**Code path:** `src-tauri/src/main.rs:113-205`

### Flow 3: Browser Click (Linux/Windows)

```
User clicks srcuri:// link in browser
    ↓
Browser asks to open srcuri
    ↓
OS spawns new srcuri process with URL as argument
    ↓
[Same as Flow 1: Command-Line Invocation]
```

**Result:** ✅ Silent, process exits after handling

## Silent Operation Implementation

### Three-Layer Protection (macOS)

Sorcery uses three complementary mechanisms to ensure completely silent operation:

| Layer | Mechanism | File | Purpose |
|-------|-----------|------|---------|
| **OS-level** | `LSUIElement: true` | `src-tauri/Info.plist` | Never create Dock icon |
| **Event-level** | `app.hide()` on deep-link | `src-tauri/src/main.rs:135-145` | Prevent window activation |
| **Result-level** | `app.hide()` after success | `src-tauri/src/main.rs:154-163` | Ensure app stays hidden |

### LSUIElement Configuration

**File:** `src-tauri/Info.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>LSUIElement</key>
    <true/>
</dict>
</plist>
```

**Effect:**
- App never shows in Dock
- App never shows in Cmd+Tab switcher
- Still allows windows when explicitly opened (settings, chooser dialogs)
- Tauri merges this with auto-generated Info.plist during build

### ActivationPolicy::Accessory

**File:** `src-tauri/src/main.rs`

```rust
#[cfg(target_os = "macos")]
{
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}
```

**Effect:**
- Reinforces LSUIElement at application level
- Prevents app from becoming active
- Allows windows to be shown when needed

**Alternatives:**
- `Regular`: Normal app (shows in Dock) - ❌ Not suitable
- `Prohibited`: Cannot show ANY UI - ❌ Breaks settings window
- `Accessory`: Perfect balance - ✅ Current choice

### Explicit Hide Calls

**On deep-link event (immediate):**

```rust
// src-tauri/src/main.rs:135-145
app.handle().listen("deep-link://new-url", move |event| {
    // Parse payload...

    // Hide IMMEDIATELY before processing URL
    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::NSApp;
        use objc::{msg_send, sel, sel_impl};
        unsafe {
            let app = NSApp();
            let _: () = msg_send![app, hide: nil];
        }
    }

    // Now process URL...
});
```

**Purpose:** Prevents existing windows (like settings) from coming to foreground when app receives deep-link event.

**After successful file opening:**

```rust
// src-tauri/src/main.rs:154-163
Ok(protocol_handler::HandleResult::Opened) => {
    tracing::info!("File opened successfully");

    #[cfg(target_os = "macos")]
    {
        let _: () = msg_send![app, hide: nil];
    }
}
```

**Purpose:** Ensures app stays hidden even if something tries to activate it during processing.

## Deep-Link Payload Format

The Tauri deep-link plugin sends URLs as a **JSON array**, not a raw string.

### Payload Structure

When a deep-link event is received, the payload is a JSON array of URLs:

```json
["srcuri:///etc/hosts:1"]
```

### Parsing Implementation

```rust
app.handle().listen("deep-link://new-url", move |event| {
    let payload = event.payload();
    tracing::info!("Received deep link event - raw payload: {}", payload);

    // Parse the payload as a JSON array of URLs
    let urls: Vec<String> = match serde_json::from_str(payload) {
        Ok(urls) => urls,
        Err(e) => {
            tracing::error!("Failed to parse deep link payload: {}", e);
            return;
        }
    };

    if urls.is_empty() {
        tracing::warn!("Received empty URL list");
        return;
    }

    // Use the first URL (most cases will only have one)
    let url = urls[0].clone();
    tracing::info!("Processing deep link URL: {}", url);

    // Handle the URL
    match ph.handle_url(&url).await {
        Ok(protocol_handler::HandleResult::Opened) => {
            tracing::info!("File opened successfully");
        }
        // ... other cases
    }
});
```

### Why JSON Array?

From [Tauri deep-link plugin documentation](https://v2.tauri.app/plugin/deep-linking/):
> The open URL event is triggered with a list of URLs that were requested to be compatible with the macOS API for deep links, but in most cases your app will only receive a single URL.

macOS can theoretically send multiple URLs in one event, so the plugin uses an array format for consistency with the platform API. In practice, srcuri always receives a single URL per event.

## Protocol Registration

### macOS

**Registration Method:** `CFBundleURLTypes` in Info.plist

**File:** Auto-generated during build, merged with `src-tauri/Info.plist`

**Key entries:**
```xml
<key>CFBundleURLTypes</key>
<array>
    <dict>
        <key>CFBundleURLSchemes</key>
        <array>
            <string>srcuri</string>
        </array>
        <key>CFBundleURLName</key>
        <string>com.srcuri.app</string>
        <key>CFBundleTypeRole</key>
        <string>Editor</string>
    </dict>
</array>
```

**Registration Process:**
1. App bundle includes CFBundleURLTypes in Info.plist
2. When app is copied to `/Applications/`, macOS reads Info.plist
3. LaunchServices automatically registers the protocol
4. User can verify: `defaults read /Applications/sorcery.app/Contents/Info CFBundleURLTypes`

**When It Happens:**
- ✅ Automatically on first app launch
- ✅ Automatically when app is copied to `/Applications/`
- ✅ No manual registration needed

**Development:**
- Run `make install` to copy bundle to `/Applications/` and register
- LaunchServices updates database automatically

### Linux

**Registration Method:** `.desktop` file + `xdg-mime`

**File:** `/usr/share/applications/srcuri.desktop` or `~/.local/share/applications/srcuri.desktop`

**Content:**
```ini
[Desktop Entry]
Version=1.0
Type=Application
Name=Sorcery
Comment=Editor-agnostic deep link handler
Exec=/usr/bin/srcuri %u
Icon=srcuri
Terminal=false
Categories=Development;Utility;
MimeType=x-scheme-handler/srcuri;
StartupWMClass=srcuri
```

**Registration Process:**
1. Install `.desktop` file to applications directory
2. Update desktop database: `update-desktop-database ~/.local/share/applications`
3. Register handler: `xdg-mime default srcuri.desktop x-scheme-handler/srcuri`

**When It Happens:**
- ✅ DEB/RPM package installer creates `.desktop` file
- ✅ App auto-registers on first run (via `protocol_registration` module)
- ⚠️ Fallback: Manual `xdg-mime` command if auto-registration fails

**Verification:**
```bash
xdg-mime query default x-scheme-handler/srcuri
# Should output: srcuri.desktop
```

**Auto-Registration Code:**

File: `src-tauri/src/protocol_registration/mod.rs`

```rust
#[cfg(target_os = "linux")]
{
    if !protocol_registration::ProtocolRegistration::is_registered() {
        tracing::info!("Protocol handler not registered, attempting registration...");
        protocol_registration::ProtocolRegistration::register()?;
    }
}
```

This runs on every app launch and silently registers the protocol if needed.

### Windows

**Registration Method:** Registry keys

**Location:** `HKEY_CLASSES_ROOT\srcuri`

**Structure:**
```
HKEY_CLASSES_ROOT\srcuri
    (Default) = "URL:Sorcery Protocol"
    URL Protocol = ""
    DefaultIcon
        (Default) = "C:\Program Files\Sorcery\srcuri.exe,0"
    shell
        open
            command
                (Default) = "C:\Program Files\Sorcery\srcuri.exe" "%1"
```

**Registration Process:**
1. MSI installer creates registry keys during installation
2. Windows associates `srcuri://` URLs with executable
3. When URL is opened, Windows launches executable with URL as `%1`

**When It Happens:**
- ✅ Automatically during MSI installation
- ⚠️ Development: Manual registry import needed

**Development Registration:**

Create `register-protocol.reg`:
```reg
Windows Registry Editor Version 5.00

[HKEY_CLASSES_ROOT\srcuri]
@="URL:Sorcery Protocol"
"URL Protocol"=""

[HKEY_CLASSES_ROOT\srcuri\shell\open\command]
@="\"C:\\Program Files\\Sorcery\\srcuri.exe\" \"%1\""
```

Import: `reg import register-protocol.reg`

**Verification:**
```powershell
reg query HKEY_CLASSES_ROOT\srcuri\shell\open\command
```

## URL Processing Pipeline

### 1. URL Parsing

**File:** `src-tauri/src/protocol_handler/parser.rs`

```rust
pub fn parse(link: &str) -> Result<SrcuriRequest>
```

**Input:** `srcuri://workspace/path/file.rs:42:10`

**Output:** `SrcuriRequest` enum variant

**Variants:**
- `PartialPath`: Just filename (searches all workspaces)
- `WorkspacePath`: Workspace + relative path
- `FullPath`: Absolute file system path
- `RevisionPath`: Workspace + path + git reference (commit/branch/tag)

### 2. Path Resolution

**File:** `src-tauri/src/protocol_handler/matcher.rs`

```rust
pub async fn find_partial_matches(&self, path: &str) -> Result<Vec<WorkspaceMatch>>
```

**Process:**
1. Load workspace configuration from settings
2. Search configured workspace directories
3. Match file path against workspace contents
4. Return all matching file paths with metadata

### 3. Path Validation

**File:** `src-tauri/src/path_validator/mod.rs`

```rust
pub fn validate(&self, path: &str) -> Result<PathBuf>
```

**Security checks:**
- Prevent path traversal (`../../../etc/passwd`)
- Normalize paths (resolve `..`, `.`, symlinks)
- Verify file exists
- Ensure path is within configured workspaces

### 4. Editor Selection

**File:** `src-tauri/src/dispatcher/mod.rs`

```rust
pub async fn open(&self, path: &str, line: Option<usize>, column: Option<usize>, ...) -> Result<()>
```

**Priority:**
1. Explicit editor hint from URL query param
2. Workspace-specific preference from settings
3. Most recently used editor (from active tracking)
4. First available editor from registry

### 5. Editor Invocation

**File:** `src-tauri/src/editors/*.rs`

```rust
async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()>
```

**Platform-specific launching:**
- **macOS:** `open -a`, AppleScript, or direct process spawn
- **Linux:** Direct process spawn with detached process
- **Windows:** `cmd.exe /c start` or direct spawn

**Arguments vary by editor:**
- VS Code: `code --goto file.rs:42:10`
- IntelliJ: `idea --line 42 file.rs`
- Neovim: `nvim --server socket --remote-send ':42<CR>:e file.rs<CR>'`

## Edge Cases and Special Scenarios

### Scenario 1: Settings Window Open

**Problem:** Settings window comes to foreground when protocol link is clicked

**Solution:** Immediate hide call on deep-link event

```rust
app.handle().listen("deep-link://new-url", move |event| {
    // Hide IMMEDIATELY before any processing
    #[cfg(target_os = "macos")]
    { let _: () = msg_send![NSApp(), hide: nil]; }

    // Process URL...
});
```

**Result:** Settings window stays in background, file opens silently

### Scenario 2: Multiple Workspaces Match

**Problem:** `srcuri://README.md:1` matches multiple workspace directories

**Solution:** Show chooser dialog

```rust
Ok(HandleResult::ShowChooser { matches, line, column }) => {
    // Create chooser window
    WebviewWindowBuilder::new(app, "workspace-chooser", ...)
        .build()?;
}
```

**User Experience:**
1. Dialog appears with list of matching workspaces
2. User selects one
3. File opens in chosen workspace
4. Choice is remembered for future (most recent)

### Scenario 3: Git References

**Problem:** `srcuri://project/file.rs:1?commit=abc123` wants specific version

**Solution:** Show git reference dialog with options

```rust
Ok(HandleResult::ShowRevisionDialog { workspace, file_path, rev, ... }) => {
    // Create git reference handler window
    WebviewWindowBuilder::new(app, "revision-handler", ...)
        .build()?;
}
```

**Supported parameters:**
- `?commit=abc123` or `?sha=abc123` - Open file at specific commit
- `?branch=main` - Open file at branch (shows message if behind)
- `?tag=v1.0.0` - Open file at tagged version

**Options presented:**
- View in temporary file (always available)
- Checkout the reference (only if working tree is clean)
- Cancel

### Scenario 4: First Browser Launch

**Problem:** Browser shows permission dialog on first `srcuri://` link click

**Solution:** This is expected browser security behavior

**User Experience:**
1. Click link in Chrome
2. Dialog: "Open srcuri?" with "Open srcuri" and "Cancel" buttons
3. Optional checkbox: "Always open these types of links in the associated app"
4. Click "Open srcuri"
5. File opens, future links open without dialog (if checkbox was checked)

**This happens once per browser, not once per user.**

## Performance Characteristics

### Command-Line Launch
- **Startup time:** ~50-100ms (Rust binary, no GUI)
- **Processing time:** ~10-50ms (parse URL, resolve path, launch editor)
- **Total time:** ~100-200ms from command to editor open
- **Memory usage:** ~5-10 MB peak, process exits immediately

### Browser Launch (First Time)
- **Startup time:** ~1-2 seconds (Tauri GUI initialization)
- **Processing time:** ~10-50ms (parse URL, resolve path, launch editor)
- **Memory usage:** ~50-100 MB (Tauri + WebView)
- **Stays running:** Yes, for instant future link handling

### Browser Launch (Subsequent)
- **Startup time:** 0ms (already running)
- **Processing time:** ~10-50ms
- **Memory usage:** +0 MB (already allocated)
- **Total time:** ~10-100ms from link click to editor open

### Editor Launch Time
- **VS Code:** ~500ms-1s (if not already running)
- **IntelliJ IDEA:** ~2-5s (if not already running, JVM startup)
- **Neovim (socket):** ~10-50ms (reuses existing session)
- **Vim (new):** ~100-500ms (Terminal.app + Vim startup)

**Total user experience:** Click link → See editor with file open in 100ms-5s depending on editor and whether it's already running.

## Debugging and Troubleshooting

### Enable Logging

**macOS/Linux:**
```bash
RUST_LOG=debug /Applications/sorcery.app/Contents/MacOS/srcuri
```

**Windows:**
```powershell
$env:RUST_LOG="debug"
.\srcuri.exe
```

### Check Protocol Registration

**macOS:**
```bash
defaults read /Applications/sorcery.app/Contents/Info CFBundleURLTypes
defaults read /Applications/sorcery.app/Contents/Info LSUIElement
```

**Linux:**
```bash
xdg-mime query default x-scheme-handler/srcuri
cat ~/.local/share/applications/srcuri.desktop
```

**Windows:**
```powershell
reg query HKEY_CLASSES_ROOT\srcuri\shell\open\command
```

### Test Protocol Handler

**Create test HTML:**
```html
<a href="srcuri:///etc/hosts:1">Test Link</a>
```

**Or use command line:**
```bash
# macOS
open "srcuri:///etc/hosts:1"

# Linux
xdg-open "srcuri:///etc/hosts:1"

# Windows
start srcuri:///C:/Windows/System32/drivers/etc/hosts:1
```

### Common Issues

**Issue:** Dock icon appears

**Solution:**
- Verify LSUIElement is set: `defaults read /Applications/sorcery.app/Contents/Info LSUIElement`
- Should return `1`
- If not, rebuild: `make install`

**Issue:** Window comes to foreground

**Solution:**
- Check that hide() calls are present in code
- Verify ActivationPolicy::Accessory is set
- Look for `Hide IMMEDIATELY before processing URL` in logs

**Issue:** "Failed to parse srcuri URL"

**Solution:**
- Check URL format matches spec
- Verify deep-link payload parsing (should parse JSON array)
- Look for payload in logs: `"deep-link event - raw payload"`

**Issue:** File doesn't open

**Solution:**
- Check editor is installed and configured
- Verify path exists
- Check workspace configuration
- Look for error in logs

## Testing

### Automated Tests

**Unit Tests:**
```bash
cd src-tauri
cargo test
```

**Integration Tests:**
```bash
cd tests/docker
./run-protocol-tests.sh
```

### Manual Test Suite

**Test 1: Command-line (no Dock icon)**
```bash
pkill -9 srcuri
open "srcuri:///etc/hosts:1"
# Verify: File opens, no Dock icon, process exits
ps aux | grep srcuri | grep -v grep  # Should be empty
```

**Test 2: Browser (first time)**
```bash
pkill -9 srcuri
# Click link in test-protocol.html
# Verify: Permission dialog, file opens, no Dock icon, process stays running
ps aux | grep srcuri | grep -v grep  # Should show one process
```

**Test 3: Browser (subsequent)**
```bash
# Don't kill process
# Click another link
# Verify: Instant open, no Dock icon
```

**Test 4: Settings window doesn't activate**
```bash
# Open settings window manually
# Click protocol link in browser
# Verify: File opens, settings window stays in background
```

**Test 5: Multiple rapid clicks**
```bash
# Click test link 5 times quickly
# Verify: All 5 files open, no flickering, no errors
```

## Related Documentation

- [URL-FORMATS.md](../../URL-FORMATS.md) - URL format specification
- [dev/installers.md](../installers.md) - Installation and distribution
- [dev/protocol-registration-improvements.md](protocol-registration.md) - Auto-registration implementation
- [DEVELOPMENT.md](../../DEVELOPMENT.md) - Development workflow
- [README.md](../../README.md) - Project overview

## Summary

Sorcery Desktop's protocol handler is designed for **completely silent operation**:

1. **No visible UI** when handling protocol links
2. **No Dock icon** (macOS) or taskbar presence
3. **Fast processing** (<200ms typical)
4. **Cross-platform** support with platform-specific optimizations
5. **Automatic registration** on all platforms
6. **Secure path validation** prevents directory traversal
7. **Smart editor selection** based on workspace and usage
8. **Session reuse** for terminal editors (Neovim)
9. **Git integration** for viewing files at specific commits, branches, or tags

For the complete protocol specification, visit srcuri.com.

The implementation uses three-layer protection (LSUIElement + ActivationPolicy + explicit hide) to ensure zero visual interference while maintaining the ability to show UI when explicitly needed (settings, choosers, dialogs).
