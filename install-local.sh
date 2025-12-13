#!/bin/bash
set -e

echo "Building Sorcery Desktop..."
cd "$(dirname "$0")/src-tauri"

# Build the app bundle
cargo build

# Bundle the macOS app (skip DMG creation)
cargo tauri build --debug --bundles app

# Find the built app (Tauri outputs to workspace root's target dir)
BUNDLE_PATH="../target/debug/bundle/macos/Sorcery Desktop.app"

if [ ! -d "$BUNDLE_PATH" ]; then
    echo "Error: Bundle not found at $BUNDLE_PATH"
    exit 1
fi

echo "Stopping any running instances..."
pkill -f "Sorcery Desktop.app" || true
pkill -f "sorcery-desktop" || true
pkill -f "target/debug/sorcery-desktop" || true
sleep 1

echo "Installing to /Applications..."
rm -rf "/Applications/Sorcery Desktop.app"
cp -R "$BUNDLE_PATH" /Applications/

echo "Registering protocol handler (running in background)..."
# Force macOS to re-register the app's protocol handler
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "/Applications/Sorcery Desktop.app" > /dev/null 2>&1 &

echo ""
echo "✅ Sorcery Desktop installed successfully!"
echo "   Location: /Applications/Sorcery Desktop.app"
echo "   Protocol handler: srcuri:// registering in background..."
echo ""
echo "You can now open links like: srcuri://myproject/README.md:10"
echo ""
echo "Note: Protocol registration may take a few seconds to complete."
