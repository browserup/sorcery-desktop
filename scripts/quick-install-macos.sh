#!/usr/bin/env bash
set -e

# Quick install script for macOS
# Run this after building to install and register the protocol handler

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Find the app bundle (accept "release" or "debug" as argument, default to debug)
BUILD_TYPE="${1:-debug}"
APP_PATH="target/${BUILD_TYPE}/bundle/macos/Sorcery Desktop.app"

if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: No app bundle found at $APP_PATH"
    echo "Run 'make build' (debug) or 'make release' first"
    exit 1
fi

# Kill any running instances (try multiple patterns to be thorough)
echo "==> Stopping any running instances..."
pkill -9 -f "sorcery" 2>/dev/null || true
pkill -9 -f "Sorcery" 2>/dev/null || true
sleep 1

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

# Start the app
echo "==> Starting Sorcery Desktop..."
open "/Applications/Sorcery Desktop.app"

echo "Test with: open \"srcuri:///etc/hosts@L1\""
