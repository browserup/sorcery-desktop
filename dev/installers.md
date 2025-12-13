# Installer Notes and Build Dependencies

## Overview
This document captures build dependencies, installation quirks, and platform-specific requirements for creating installers for Sorcery Desktop.

## Build Dependencies

### Linux (Ubuntu/Debian)

**Minimum Ubuntu Version:** 24.04 (Noble)
- Earlier versions (22.04) only have webkit2gtk-4.0, but Tauri 2.x requires webkit2gtk-4.1 or webkitgtk-6.0

**Required Packages:**
```bash
apt-get install -y \
    build-essential \
    pkg-config \
    libglib2.0-dev \
    libgtk-3-dev \
    libwebkitgtk-6.0-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    curl \
    wget
```

**Note:** Both `libwebkitgtk-6.0-dev` and `libwebkit2gtk-4.1-dev` are required:
- `libwebkitgtk-6.0-dev` - Newer WebKitGTK 6.0 (recommended for runtime)
- `libwebkit2gtk-4.1-dev` - Required by some Tauri Rust bindings during build

**Key Dependencies:**
- `build-essential` - GCC, g++, make, libc-dev
- `pkg-config` - Required for finding library paths
- `libglib2.0-dev` - GLib development files
- `libgtk-3-dev` - GTK3 development files
- `libwebkitgtk-6.0-dev` - WebKitGTK for Tauri (critical - version 6.0 required)
- `libayatana-appindicator3-dev` - System tray support
- `librsvg2-dev` - SVG rendering support

**Alternative WebKit Packages:**
- Ubuntu 24.04+: `libwebkitgtk-6.0-dev` (recommended)
- If 6.0 unavailable: `libwebkit2gtk-4.1-dev` (Tauri minimum)
- Ubuntu 22.04 only has: `libwebkit2gtk-4.0-dev` (too old, won't work)

### macOS

**Minimum macOS Version:** TBD

**Required Tools:**
- Xcode Command Line Tools
- Rust toolchain

**Dependencies:**
Most dependencies are provided by macOS or installed via Homebrew:
```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Additional tools (if needed)
brew install pkg-config
```

### Windows

**Minimum Windows Version:** TBD

**Required Tools:**
- Visual Studio Build Tools or Visual Studio Community
- Rust toolchain

**Dependencies:**
- WebView2 runtime (bundled with Windows 11, needs separate install on Windows 10)

## Rust Toolchain

**Installation:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

**Minimum Version:** TBD (currently using stable)

## Platform-Specific Notes

### Linux

**Distribution Compatibility:**
- ✅ Ubuntu 24.04+ (Noble and newer)
- ✅ Debian 13+ (Trixie)
- ⚠️ Ubuntu 22.04 (Jammy) - webkit2gtk too old, requires PPA or manual WebKit build
- ⚠️ Older LTS releases - will need webkit2gtk-4.1 backport

**Desktop Environment:**
- Works on: GNOME, KDE, XFCE, etc.
- System tray requires: libayatana-appindicator3 or libappindicator3

**Wayland vs X11:**
- Both supported via GTK3
- Tested primarily on Xvfb (X11 virtual framebuffer) in Docker

### macOS

**Architecture:**
- Apple Silicon (arm64) - native support
- Intel (x86_64) - native support
- Universal binary - TBD

**Notarization:**
- Required for distribution outside Mac App Store
- Needs Apple Developer account

### Windows

**Architecture:**
- x64 - primary target
- arm64 - TBD

**Installer Types:**
- MSI - recommended for enterprise
- NSIS - alternative
- Portable - no installation required

## Testing Environment

### Docker Test Environment

**Base Image:** `ubuntu:24.04`

**Installed Editors (for integration testing):**
- VSCode, VSCodium, Sublime Text
- Vim, Neovim, Emacs, Nano, Micro
- Gedit, Kate
- IntelliJ IDEA Community, PyCharm Community

**Test Tools:**
- Xvfb - Virtual X server for headless GUI testing
- xdotool, wmctrl - X11 window management utilities
- pgrep, pkill, lsof - Process management for test verification

## Known Issues and Workarounds

### WebKit Version Conflicts

**Problem:** Tauri 2.x requires webkit2gtk-4.1 or newer, but Ubuntu 22.04 LTS only provides 4.0

**Workarounds:**
1. Use Ubuntu 24.04 or newer (recommended)
2. Add webkit2gtk PPA for 22.04 (maintenance burden)
3. Build WebKit from source (complex, not recommended)

**Impact on Installers:**
- Target Ubuntu 24.04+ for official packages
- Document Ubuntu 22.04 workaround for users
- Consider AppImage for maximum compatibility (bundles WebKit)

### VSCode/VSCodium Root User Launch

**Problem:** VSCode and VSCodium refuse to run as root (common in Docker containers) without additional flags

**Error Message:**
```
You are trying to start Visual Studio Code as a super user which isn't recommended.
If this was intended, please add the argument `--no-sandbox` and specify an alternate
user data directory using the `--user-data-dir` argument.
```

**Solution:**
Add these flags when launching as root:
- `--no-sandbox` - Disables Chromium sandbox (required for root)
- `--user-data-dir=/path/to/temp/dir` - Uses temporary user data directory

**Example:**
```bash
code --no-sandbox --user-data-dir=/tmp/vscode-data --goto file.txt:10:5
```

**Impact on Testing:**
- Docker tests must include these flags
- Not an issue for normal user installations

### Editor Detection

**Cursor, Zed, Helix:**
- Not in standard Ubuntu repos
- Cursor: AppImage download has issues
- Zed: Install script may fail on some architectures
- Helix: Not in Ubuntu 22.04/24.04 default repos

**Decision:** Skip these in Docker tests, focus on commonly available editors

## Future Installer Work

### Package Formats

**Linux:**
- [ ] .deb (Debian/Ubuntu)
- [ ] .rpm (Fedora/RHEL)
- [ ] AppImage (universal, bundles dependencies)
- [ ] Snap (Ubuntu Software Center)
- [ ] Flatpak (Flathub)

**macOS:**
- [ ] .dmg (standard distribution)
- [ ] .pkg (for enterprise deployment)
- [ ] Mac App Store submission

**Windows:**
- [ ] .msi (Windows Installer)
- [ ] .exe (NSIS installer)
- [ ] Portable .zip (no installation)
- [ ] Windows Store submission

### CI/CD Considerations

**Build Matrix:**
- Linux: Ubuntu 24.04 (x86_64, arm64)
- macOS: Latest (Intel, Apple Silicon)
- Windows: Latest (x64)

**Test Strategy:**
- Unit tests: Run on all platforms
- Integration tests: Docker (Linux), local (macOS/Windows)
- Editor launch tests: Requires GUI environment (Xvfb on Linux)

## References

- [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)
- [WebKitGTK Releases](https://webkitgtk.org/)
- [Ubuntu Packages](https://packages.ubuntu.com/)

---

# Protocol Handler Installation Guide

This section describes how to register the `srcuri://` custom URL protocol on macOS, Linux, and Windows. Once registered, clicking `srcuri://` links (Sorcery protocol) in browsers, terminals, or other applications will launch Sorcery Desktop and open the specified file.

## Overview

### What Protocol Registration Does

- **Browser Support**: Clicking `srcuri://` links in Chrome, Firefox, Safari opens files
- **Terminal Support**: Shell scripts can use `open srcuri://...` (macOS) or `xdg-open srcuri://...` (Linux)
- **Application Support**: Any app can generate srcuri links for inter-app navigation
- **System-wide**: Works across all applications once registered

### Current Status

Sorcery Desktop already includes protocol registration configuration in `tauri.conf.json`:

```json
{
  "plugins": {
    "deep-link": {
      "desktop": {
        "schemes": ["srcuri"]
      }
    }
  }
}
```

This generates the necessary platform-specific registration files during build.

---

## macOS Installation

### How It Works

macOS uses the `Info.plist` file inside `.app` bundles to register URL schemes.

**Location**: `sorcery.app/Contents/Info.plist`

**Key entry**:
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

### Installation Steps

#### Method 1: DMG Installer (Recommended for Users)

1. **Build DMG**:
   ```bash
   cd src-tauri
   cargo tauri build --target universal-apple-darwin
   ```

2. **Output**: `src-tauri/target/release/bundle/dmg/srcuri_0.1.0_universal.dmg`

3. **User Installation**:
   - Double-click DMG
   - Drag `sorcery.app` to `/Applications`
   - First launch: Right-click → Open (bypass Gatekeeper)
   - Protocol registered automatically on first launch

4. **Verification**:
   ```bash
   # Check registration
   /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -dump | grep -i srcuri

   # Test in browser or terminal
   open "srcuri:///etc/hosts:1"
   ```

#### Method 2: Development Build

1. **Build app**:
   ```bash
   cd src-tauri
   cargo tauri build --debug
   ```

2. **Output**: `src-tauri/target/debug/bundle/macos/sorcery.app`

3. **Install to Applications**:
   ```bash
   cp -r src-tauri/target/debug/bundle/macos/sorcery.app /Applications/
   ```

4. **Register with LaunchServices**:
   ```bash
   /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/sorcery.app
   ```

5. **Verify**:
   ```bash
   # Test protocol opens srcuri
   open "srcuri:///Users/$USER/test.txt:1"
   ```

### Troubleshooting macOS

#### Protocol Not Working

```bash
# Force re-register
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -kill -r -domain local -domain system -domain user

# Rebuild LaunchServices database
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/sorcery.app

# Verify registration
defaults read /Applications/sorcery.app/Contents/Info CFBundleURLTypes
```

#### Multiple Handlers Installed

```bash
# List all handlers for srcuri://
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -dump | grep -B 5 -A 5 srcuri
```

### Browser Testing on macOS

#### Safari
- Paste `srcuri:///etc/hosts:1` in address bar
- Press Enter
- Dialog: "Open Sorcery Desktop?" → Click "Allow"

#### Chrome
- Paste link in address bar
- Dialog: "Open Sorcery Desktop?" → Click "Open"
- Checkbox: "Always open these types of links in the associated app"

#### Firefox
- Paste link in address bar
- Dialog: "Launch Application" → Select "Sorcery Desktop" → Click "OK"
- Checkbox: "Remember my choice for srcuri links"

---

## Linux Installation

### How It Works

Linux uses `.desktop` files to register protocol handlers via the XDG specification.

**Location**: `~/.local/share/applications/srcuri.desktop`

### Installation Steps

#### Method 1: DEB Package (Debian/Ubuntu)

1. **Build DEB**:
   ```bash
   cd src-tauri
   cargo tauri build
   ```

2. **Output**: `src-tauri/target/release/bundle/deb/srcuri_0.1.0_amd64.deb`

3. **Install**:
   ```bash
   sudo dpkg -i srcuri_*.deb
   # Or use apt to handle dependencies
   sudo apt install ./srcuri_*.deb
   ```

4. **Desktop file** (auto-installed to `/usr/share/applications/srcuri.desktop`):
   ```ini
   [Desktop Entry]
   Version=1.0
   Type=Application
   Name=Sorcery Desktop
   Comment=Editor-agnostic deep link handler
   Exec=/usr/bin/srcuri %u
   Icon=srcuri
   Terminal=false
   Categories=Development;Utility;
   MimeType=x-scheme-handler/srcuri;
   ```

5. **Register protocol** (automatic on first run):
   ```bash
   # Registration happens automatically when you first launch the app
   # If it fails, you can manually register with:
   xdg-mime default srcuri.desktop x-scheme-handler/srcuri
   ```

6. **Verify**:
   ```bash
   # Check handler
   xdg-mime query default x-scheme-handler/srcuri
   # Should output: srcuri.desktop

   # Test protocol
   xdg-open "srcuri:///etc/hosts:1"
   ```

#### Method 2: AppImage (Universal)

1. **Build AppImage**:
   ```bash
   cd src-tauri
   cargo tauri build
   ```

2. **Output**: `src-tauri/target/release/bundle/appimage/srcuri_0.1.0_amd64.AppImage`

3. **Install**:
   ```bash
   # Move to standard location
   mkdir -p ~/.local/bin
   mv srcuri_*.AppImage ~/.local/bin/sorcery.appimage
   chmod +x ~/.local/bin/sorcery.appimage

   # Create launcher script
   cat > ~/.local/bin/srcuri << 'EOF'
#!/bin/bash
~/.local/bin/sorcery.appimage "$@"
EOF
   chmod +x ~/.local/bin/srcuri

   # Create desktop entry
   cat > ~/.local/share/applications/srcuri.desktop << 'EOF'
[Desktop Entry]
Version=1.0
Type=Application
Name=Sorcery Desktop
Comment=Editor-agnostic deep link handler
Exec=$HOME/.local/bin/srcuri %u
Icon=srcuri
Terminal=false
Categories=Development;Utility;
MimeType=x-scheme-handler/srcuri;
StartupWMClass=srcuri
EOF

   # Update desktop database
   update-desktop-database ~/.local/share/applications

   # Register protocol handler
   xdg-mime default srcuri.desktop x-scheme-handler/srcuri
   ```

### Troubleshooting Linux

#### Protocol Not Working

```bash
# Check current handler
xdg-mime query default x-scheme-handler/srcuri

# If empty or wrong, re-register
xdg-mime default srcuri.desktop x-scheme-handler/srcuri

# Verify desktop file exists
ls -l ~/.local/share/applications/srcuri.desktop
# or
ls -l /usr/share/applications/srcuri.desktop

# Update database
update-desktop-database ~/.local/share/applications
```

#### Desktop File Not Found

```bash
# List all desktop files
find ~/.local/share/applications /usr/share/applications -name "*.desktop" | grep -i srcuri

# Validate desktop file syntax
desktop-file-validate ~/.local/share/applications/srcuri.desktop
```

### Browser Testing on Linux

#### Chrome/Chromium
- Paste `srcuri:///etc/hosts:1` in address bar
- Dialog: "Open xdg-open?" → Click "Open xdg-open"

#### Firefox
- Paste link in address bar
- Dialog: "Choose Application" → Select "Sorcery Desktop" → Click "Open link"
- Checkbox: "Remember my choice for srcuri links"

---

## Windows Installation

### How It Works

Windows uses the Registry to map URL schemes to executable applications.

**Registry Location**: `HKEY_CLASSES_ROOT\srcuri`

**Registry Structure**:
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

### Installation Steps

#### Method 1: MSI Installer (Recommended)

1. **Build MSI**:
   ```powershell
   cd src-tauri
   cargo tauri build
   ```

2. **Output**: `src-tauri\target\release\bundle\msi\srcuri_0.1.0_x64_en-US.msi`

3. **User Installation**:
   - Double-click MSI
   - Follow installer wizard
   - Protocol registered automatically during installation

4. **Verification**:
   ```powershell
   # Check registry
   reg query HKEY_CLASSES_ROOT\srcuri

   # Test protocol
   start srcuri:///C:/Users/%USERNAME%/test.txt:1
   ```

#### Method 2: Manual Registry (Development)

1. **Build executable**:
   ```powershell
   cd src-tauri
   cargo build --release
   ```

2. **Create registry file** (`register-protocol.reg`):
   ```reg
   Windows Registry Editor Version 5.00

   [HKEY_CLASSES_ROOT\srcuri]
   @="URL:Sorcery Protocol"
   "URL Protocol"=""

   [HKEY_CLASSES_ROOT\srcuri\DefaultIcon]
   @="C:\\Program Files\\Sorcery\\srcuri.exe,0"

   [HKEY_CLASSES_ROOT\srcuri\shell]

   [HKEY_CLASSES_ROOT\srcuri\shell\open]

   [HKEY_CLASSES_ROOT\srcuri\shell\open\command]
   @="\"C:\\Program Files\\Sorcery\\srcuri.exe\" \"%1\""
   ```

3. **Install**:
   ```powershell
   # Copy executable (requires admin)
   mkdir "C:\Program Files\Sorcery"
   copy target\release\srcuri.exe "C:\Program Files\Sorcery\"

   # Import registry file (requires admin)
   reg import register-protocol.reg
   ```

### Troubleshooting Windows

#### Protocol Not Working

```powershell
# Check registry
reg query HKEY_CLASSES_ROOT\srcuri\shell\open\command

# Expected output:
# (Default)    REG_SZ    "C:\Program Files\Sorcery\srcuri.exe" "%1"
```

#### Re-register Protocol

```powershell
# Re-import registry file
reg import register-protocol.reg

# Or reinstall MSI
```

### Browser Testing on Windows

#### Chrome
- Paste `srcuri:///C:/Users/User/test.txt:1` in address bar
- Dialog: "Open srcuri?" → Click "Open srcuri"

#### Edge
- Paste link in address bar
- Dialog: "This site is trying to open Sorcery Desktop" → Click "Open"

#### Firefox
- Paste link in address bar
- Dialog: "Launch Application" → Select "Sorcery Desktop" → Click "OK"

---

## Cross-Platform Testing

### Test HTML Page

Create `test-protocol.html`:

```html
<!DOCTYPE html>
<html>
<head>
    <title>Sorcery Protocol Test</title>
</head>
<body>
    <h1>Sorcery Protocol Handler Test</h1>

    <h2>Test Links</h2>
    <ul>
        <li><a href="srcuri:///etc/hosts:1">Open /etc/hosts at line 1</a> (macOS/Linux)</li>
        <li><a href="srcuri:///C:/Windows/System32/drivers/etc/hosts:1">Open hosts at line 1</a> (Windows)</li>
        <li><a href="srcuri:///tmp/test.txt:42:10">Open test.txt at line 42, column 10</a></li>
    </ul>

    <h2>JavaScript Test</h2>
    <button onclick="testProtocol()">Test Protocol Handler</button>

    <script>
        function testProtocol() {
            const url = "srcuri:///tmp/test.txt:1";
            window.location.href = url;
        }
    </script>
</body>
</html>
```

**Test**:
```bash
# macOS/Linux
open test-protocol.html

# Windows
start test-protocol.html
```

### Command-Line Testing

#### macOS
```bash
open "srcuri:///etc/hosts:22"
```

#### Linux
```bash
xdg-open "srcuri:///etc/hosts:22"
```

#### Windows
```powershell
start srcuri:///C:/Windows/System32/drivers/etc/hosts:22
```

---

## Summary

| Platform | Registration Method | Auto-registered? | Browser Support |
|----------|-------------------|------------------|-----------------|
| **macOS** | Info.plist in .app | ✅ Yes (via Tauri) | ✅ All browsers |
| **Linux** | .desktop file + xdg-mime | ✅ Yes (on first run) | ✅ All browsers |
| **Windows** | Registry | ✅ Yes (via MSI installer) | ✅ All browsers |

**Answer to your question**: Yes, once registered, `srcuri://` links work from the Chrome address bar (and all other browsers). The browser will prompt the first time, then can remember your choice.

### Quick Start (Development)

#### macOS
```bash
cd src-tauri
cargo tauri build --debug
cp -r target/debug/bundle/macos/sorcery.app /Applications/
open "srcuri:///etc/hosts:1"
```

#### Linux
```bash
cd src-tauri
cargo tauri build
sudo dpkg -i target/release/bundle/deb/srcuri_*.deb
xdg-mime default srcuri.desktop x-scheme-handler/srcuri
xdg-open "srcuri:///etc/hosts:1"
```

#### Windows
```powershell
cd src-tauri
cargo tauri build
# Run the MSI installer
start srcuri:///C:/Windows/System32/drivers/etc/hosts:1
```
