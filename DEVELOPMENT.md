# Development Guide

## Quick Start

### macOS Development Workflow

The fastest way to get started:

```bash
# Build, install, and test in one command
make dev
```

Or step by step:

```bash
# 1. Build debug version
make build

# 2. Install to /Applications and register protocol
make install

# 3. Test the protocol handler
make test-protocol
```

### Why You Need to Install for Development

**The protocol handler only works when the app is installed in `/Applications/`.**

During development:
- Each build creates a bundle at `src-tauri/target/debug/bundle/macos/Sorcery Desktop.app`
- macOS only recognizes protocol handlers from `/Applications/` or `~/Applications/`
- You must copy the app to `/Applications/` and register it with LaunchServices

This is different from production, where:
- Users install via DMG (drag to /Applications)
- Protocol registration happens automatically on first launch

## Development Commands

### Build Commands

```bash
# Debug build (faster, includes logging)
make build
cargo tauri build --debug

# Release build (optimized, smaller)
make build-release
cargo tauri build
```

### Installation Commands

```bash
# Build and install
make install

# Install existing build (no rebuild)
make install-quick
./scripts/quick-install-macos.sh

# Full install script (builds + installs)
./install-dev.sh
```

### Testing Commands

```bash
# Test protocol handler from command line
make test-protocol
open "srcuri:///etc/hosts:1"

# Test from browser
# Paste this in Chrome/Safari/Firefox address bar:
# srcuri:///etc/hosts:1
```

## Troubleshooting

### Protocol Handler Not Working

**Symptom:** Clicking `srcuri://` links in browser does nothing or shows an error.

**Solution:**
```bash
# 1. Check if app is installed
ls -la /Applications/Sorcery\ Desktop.app

# 2. Re-register with LaunchServices
make install-quick

# 3. Force rebuild LaunchServices database
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -kill -r -domain local -domain system -domain user

# 4. Re-register app
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/Sorcery\ Desktop.app

# 5. Test
open "srcuri:///etc/hosts:1"
```

### App Won't Launch

**Symptom:** App crashes immediately or shows Gatekeeper warning.

**Solution:**
```bash
# Remove quarantine attribute
xattr -dr com.apple.quarantine /Applications/Sorcery\ Desktop.app

# Or right-click app in Finder → Open → Open anyway (first time only)
```

### Multiple Protocol Handlers

**Symptom:** Wrong app opens when clicking `srcuri://` links.

**Solution:**
```bash
# List all handlers for srcuri://
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -dump | grep -B 5 -A 5 srcuri

# Remove old versions
rm -rf /Applications/Sorcery\ Desktop.app
rm -rf ~/Applications/Sorcery\ Desktop.app

# Reinstall
make install
```

### Build Errors

**Symptom:** `cargo tauri build` fails.

**Common causes:**
```bash
# Missing Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Outdated dependencies
cd src-tauri
cargo update

# Clean build
cargo clean
cargo tauri build --debug
```

## How Protocol Registration Works

### macOS Architecture

```
Browser/Terminal
    ↓
    srcuri:// link clicked
    ↓
macOS LaunchServices
    ↓
    Checks HKEY_CLASSES_ROOT (Windows) or Info.plist (macOS)
    ↓
    Launches /Applications/Sorcery Desktop.app
    ↓
    Passes URL as argument OR fires deep-link event
    ↓
Sorcery Desktop processes URL
    ↓
    Opens file in configured editor
```

### Registration Files

**macOS:** `/Applications/Sorcery Desktop.app/Contents/Info.plist`
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

**Linux:** `/usr/share/applications/sorcery-desktop.desktop` or `~/.local/share/applications/sorcery-desktop.desktop`
```ini
[Desktop Entry]
MimeType=x-scheme-handler/srcuri;
Exec=/usr/bin/sorcery-desktop %u
```

**Windows:** Registry key `HKEY_CLASSES_ROOT\srcuri`

### Two Launch Modes

**Mode 1: Command-line** (fast, exits immediately)
```bash
open "srcuri:///etc/hosts:1"
# macOS passes URL as argv[1]
# App handles URL, opens editor, exits
# No GUI, no background process
```

**Mode 2: Deep-link event** (browser, stays running)
```
User clicks link in browser
# macOS launches app without args
# Tauri fires "deep-link://new-url" event
# App handles URL, opens editor, stays running
# Background process for subsequent links
```

See `src-tauri/src/main.rs:58-78` for command-line handling and `main.rs:99-189` for event handling.

## Running Tests

```bash
# Unit tests
cd src-tauri
cargo test

# Integration tests (requires Docker)
cd tests/docker
./run-tests.sh

# Protocol handler tests
cd tests/docker
./run-protocol-tests.sh
```

## Production Builds

### macOS

```bash
# Build universal binary (Intel + Apple Silicon)
cd src-tauri
cargo tauri build --target universal-apple-darwin

# Output: target/release/bundle/dmg/Sorcery Desktop_0.1.0_universal.dmg
```

### Linux

```bash
# Build DEB package
cargo tauri build

# Output: target/release/bundle/deb/sorcery-desktop_0.1.0_amd64.deb
```

### Windows

```powershell
# Build MSI installer
cargo tauri build

# Output: target/release/bundle/msi/Sorcery Desktop_0.1.0_x64_en-US.msi
```

## Development Tips

1. **Use `make dev` for rapid iteration** - builds, installs, and tests in one command

2. **Check logs** - the app logs to stdout/stderr when run from terminal:
   ```bash
   /Applications/Sorcery\ Desktop.app/Contents/MacOS/sorcery-desktop
   ```

3. **Test both launch modes**:
   - Command-line: `open "srcuri://..."`
   - Browser: paste URL in address bar

4. **Kill background instances** before testing:
   ```bash
   pkill -9 sorcery-desktop
   ```

5. **Verify protocol registration**:
   ```bash
   defaults read /Applications/Sorcery\ Desktop.app/Contents/Info CFBundleURLTypes
   ```

## See Also

- [ai/installers.md](dev/installers.md) - Detailed installation guide for all platforms
- [ai/protocol-handler-fix.md](dev/protocol-handler-fix.md) - Background on protocol handler architecture
- [README.md](README.md) - Project overview and user documentation
