# Protocol Registration Improvements

## Overview

This document describes the improvements made to Sorcery Desktop's protocol registration system to make development easier and end-user installation automatic on all platforms.

## Problems Solved

### 1. Development Workflow on macOS

**Problem:** During development, the protocol handler wouldn't work from browsers because:
- Each build creates a bundle at `src-tauri/target/debug/bundle/macos/sorcery.app`
- macOS only recognizes protocol handlers from `/Applications/` or `~/Applications/`
- Developers had to manually copy the app and re-register with LaunchServices after each build

**Solution:** Created automated build and installation scripts:
- `Makefile` with simple commands (`make dev`, `make build`, `make install`)
- `scripts/quick-install-macos.sh` for fast installation without rebuilding
- `install-dev.sh` for full build + install workflow

**Usage:**
```bash
# One command to build, install, and test
make dev

# Or step by step
make build          # Build app bundle only (skips DMG)
make install        # Install to /Applications and register
make test-protocol  # Test the protocol handler
```

### 2. DMG Bundling Error

**Problem:** `cargo tauri build --debug` tried to create a DMG and failed, blocking the build.

**Solution:** Modified Makefile to build only the `.app` bundle for development:
```bash
cargo tauri build --debug --bundles app
```

This is faster and skips the DMG creation which is only needed for distribution.

### 3. Linux Protocol Registration

**Problem:** On Linux, users had to manually run `xdg-mime default srcuri.desktop x-scheme-handler/srcuri` after installation.

**Solution:** Added automatic protocol registration on first run:
- Created `src-tauri/src/protocol_registration/mod.rs`
- Checks if protocol is registered when app launches
- Automatically creates `.desktop` file and registers with `xdg-mime`
- Falls back to manual registration if automatic fails (logs warning)

**Implementation:**
```rust
// In main.rs
#[cfg(target_os = "linux")]
{
    if !protocol_registration::ProtocolRegistration::is_registered() {
        tracing::info!("Protocol handler not registered, attempting registration...");
        if let Err(e) = protocol_registration::ProtocolRegistration::register() {
            tracing::warn!("Failed to auto-register protocol handler: {}. You may need to run: xdg-mime default srcuri.desktop x-scheme-handler/srcuri", e);
        }
    }
}
```

## New Files Created

### 1. `Makefile`
Simple commands for development workflow:
- `make help` - Show available commands
- `make build` - Build debug version (app bundle only)
- `make build-release` - Build release version
- `make install` - Build and install to /Applications
- `make install-quick` - Install existing build (no rebuild)
- `make test-protocol` - Test protocol handler
- `make clean` - Clean build artifacts
- `make dev` - Build, install, and test in one command

### 2. `scripts/quick-install-macos.sh`
Fast installation script for macOS development:
- Kills running instances
- Removes old version from /Applications
- Installs new version
- Registers with LaunchServices
- No rebuild required

### 3. `install-dev.sh`
Full development installation script supporting macOS and Linux:
- Builds the app
- Installs to system location
- Registers protocol handler
- Verifies installation

### 4. `DEVELOPMENT.md`
Comprehensive development guide including:
- Quick start commands
- Protocol registration architecture
- Troubleshooting guide
- Testing procedures
- Build tips

### 5. `src-tauri/src/protocol_registration/mod.rs`
Cross-platform protocol registration module:
- Platform-specific implementations
- Automatic registration on Linux
- Desktop file creation
- Registration verification

## Platform Support Status

| Platform | End User Install | Development | Auto-registration |
|----------|-----------------|-------------|-------------------|
| **macOS** | ✅ DMG installer | ✅ `make dev` | ✅ Via bundle Info.plist |
| **Linux** | ✅ DEB package | ✅ `./install-dev.sh` | ✅ On first run |
| **Windows** | ✅ MSI installer | ⚠️ Manual (needs script) | ✅ Via MSI installer |

## Testing

### macOS Development Testing

```bash
# Full workflow test
make dev

# Should output:
# ==> Building debug version (app bundle only)...
# ==> Installing to /Applications...
# ==> Registering protocol handler...
# ==> Testing protocol handler...
# Opening /etc/hosts at line 1...

# Verify protocol registration
defaults read /Applications/sorcery.app/Contents/Info CFBundleURLTypes

# Test from browser - paste in address bar:
srcuri:///etc/hosts:1
```

### Linux Testing (when running on Linux)

```bash
# Install
./install-dev.sh

# Verify registration
xdg-mime query default x-scheme-handler/srcuri
# Should output: srcuri.desktop

# Test from command line
xdg-open "srcuri:///etc/hosts:1"

# Test from browser
# Paste in address bar: srcuri:///etc/hosts:1
```

## Architecture Notes

### macOS Protocol Registration Flow

```
1. Bundle created with Info.plist containing CFBundleURLTypes
2. App copied to /Applications/
3. lsregister command registers protocol with LaunchServices
4. macOS creates association: srcuri:// → /Applications/sorcery.app
5. Browser/Terminal use LaunchServices to open links
```

### Linux Protocol Registration Flow

```
1. App launches
2. Checks: xdg-mime query default x-scheme-handler/srcuri
3. If not registered:
   a. Create ~/.local/share/applications/srcuri.desktop
   b. Run: xdg-mime default srcuri.desktop x-scheme-handler/srcuri
   c. Run: update-desktop-database ~/.local/share/applications
4. Browser/Terminal use xdg-open to handle links
```

### Windows Protocol Registration Flow

```
1. MSI installer creates registry keys:
   HKEY_CLASSES_ROOT\srcuri
   └── shell\open\command → "C:\Program Files\Sorcery\srcuri.exe" "%1"
2. Windows associates srcuri:// → srcuri.exe
3. Browser/Terminal use registry to open links
```

## Future Improvements

### Short-term
- [x] macOS development workflow automation
- [x] Linux automatic registration
- [ ] Windows development installation script
- [ ] CI/CD integration for automated builds

### Long-term
- [ ] System tray notification on first registration
- [ ] User prompt to choose default editor on first run
- [ ] Migration tool for settings from old installations
- [ ] Uninstallation script that removes protocol registration

## Documentation Updates

Updated the following files:
- `README.md` - Added quick start section with `make dev` command
- `ai/installers.md` - Updated Linux auto-registration status
- `DEVELOPMENT.md` - New comprehensive development guide

## Breaking Changes

None. All changes are additive and backward-compatible.

## Summary

Sorcery Desktop's protocol registration system is now:
- **Automatic** - Works on first run for all platforms
- **Simple** - One command (`make dev`) for development
- **Robust** - Handles errors gracefully with fallback instructions
- **Cross-platform** - Consistent behavior on macOS, Linux, and Windows
- **Well-documented** - Clear guides for developers and users

The development workflow is now as simple as:
```bash
make dev
```

And end users get automatic protocol registration on all platforms with zero manual steps required.
