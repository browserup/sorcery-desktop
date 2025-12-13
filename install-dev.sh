#!/bin/bash
set -e

echo "Building Sorcery Desktop (dev mode - faster)..."
cd "$(dirname "$0")/src-tauri"

# Just build the binary
cargo build

echo "Stopping any running instances..."
pkill -f "Sorcery Desktop.app" || true
pkill -f "sorcery-desktop" || true
pkill -f "target/debug/sorcery-desktop" || true
sleep 1

# Check if the app bundle structure exists in /Applications
if [ ! -d "/Applications/Sorcery Desktop.app" ]; then
    echo "Error: /Applications/Sorcery Desktop.app not found."
    echo "Run ./install-local.sh first to create the full bundle."
    exit 1
fi

echo "Updating binary in /Applications/Sorcery Desktop.app..."
# Just replace the binary inside the existing app bundle
cp target/debug/sorcery-desktop "/Applications/Sorcery Desktop.app/Contents/MacOS/sorcery-desktop"

echo "Touching app to update modification time..."
touch "/Applications/Sorcery Desktop.app"

echo "Re-registering protocol handler (running in background)..."
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "/Applications/Sorcery Desktop.app" > /dev/null 2>&1 &

echo ""
echo "✅ Sorcery Desktop binary updated!"
echo "   Binary updated in: /Applications/Sorcery Desktop.app"
echo "   Protocol handler: registering in background..."
echo ""
echo "This was a fast dev update. For a full rebuild, use ./install-local.sh"
