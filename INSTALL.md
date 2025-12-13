# Installation Scripts

This project includes two convenience scripts for building and installing Sorcery Desktop on macOS:

## Scripts

### `./install-local.sh` - Full Installation

Creates a complete app bundle and installs it to `/Applications`.

**Use this when:**
- First time setup
- You've changed UI files, icons, or other resources
- You need a complete rebuild

**What it does:**
1. Builds the Rust binary (`cargo build`)
2. Creates the macOS app bundle (`cargo tauri build --debug`)
3. Stops any running instances
4. Copies the bundle to `/Applications/Sorcery Desktop.app`
5. Registers the `srcuri://` protocol handler

**Time:** ~10-20 seconds (depending on changes)

### `./install-dev.sh` - Fast Binary Update

Updates only the binary inside the existing app bundle.

**Use this when:**
- You've only changed Rust code
- You want fast iteration during development
- The app bundle already exists in `/Applications`

**What it does:**
1. Builds the Rust binary (`cargo build`)
2. Stops any running instances
3. Replaces the binary in `/Applications/Sorcery Desktop.app/Contents/MacOS/`
4. Re-registers the protocol handler

**Time:** ~2-5 seconds

**Note:** This requires that you've run `./install-local.sh` at least once to create the app bundle.

## Usage Examples

```bash
# First time setup or after resource changes
./install-local.sh

# Quick iteration during development
./install-dev.sh

# Typical workflow
./install-dev.sh  # After code changes
./install-dev.sh  # After more code changes
./install-local.sh  # After changing icons/UI
./install-dev.sh  # Back to quick iterations
```

## Protocol Handler Registration

Both scripts automatically register the `srcuri://` protocol handler with macOS. The registration runs in the background and may take a few seconds to complete.

To verify registration:
```bash
# Check if protocol is registered
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -dump | grep -i srcuri

# Test a protocol link
open "srcuri://myproject/README.md:10"
```

## Single Instance Behavior

The app uses single-instance mode - only one copy can run at a time:
- Launching a second instance will activate the first one
- Deep links passed to the second instance are forwarded to the first
- The second instance then exits automatically

## Troubleshooting

**App won't open after install:**
```bash
# Check if it's running
ps aux | grep sorcery-desktop

# Kill any stuck instances
pkill -f sorcery-desktop

# Try installing again
./install-dev.sh
```

**Protocol handler not working:**
```bash
# Force re-registration (foreground, to see errors)
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "/Applications/Sorcery Desktop.app"

# Restart Finder/Dock (sometimes needed)
killall Finder
killall Dock
```

**Build errors:**
```bash
# Clean build
cd src-tauri
cargo clean
cargo build

# Then install
cd ..
./install-local.sh
```

**Warning about bundle identifier:**
You may see this warning during builds:
```
Warn The bundle identifier "com.srcuri.app" set in `tauri.conf.json identifier` ends with `.app`
```
This is safe to ignore - it's just a warning about naming conventions.
