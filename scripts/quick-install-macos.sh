#!/usr/bin/env bash
set -e

# Quick install script for macOS
# Run this after building to install and register the protocol handler

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Find the app bundle
APP_PATH="target/debug/bundle/macos/Sorcery Desktop.app"

if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: No app bundle found at $APP_PATH"
    echo "Run 'make build' or 'cargo tauri build --debug --bundles app' first"
    exit 1
fi

# Kill any running instances
echo "==> Stopping any running instances..."
pkill -9 sorcery-desktop 2>/dev/null || true

# Remove old version
if [ -d "/Applications/Sorcery Desktop.app" ]; then
    echo "==> Removing old version..."
    rm -rf "/Applications/Sorcery Desktop.app"
fi

# Install new version
echo "==> Installing to /Applications..."
cp -r "$APP_PATH" /Applications/

# Register with LaunchServices
echo "==> Registering protocol handler..."
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "/Applications/Sorcery Desktop.app"

echo ""
echo "✓ Installation complete!"
echo ""
echo "Test with: open \"srcuri:///etc/hosts:1\""
