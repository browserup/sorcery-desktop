#!/usr/bin/env bash
set -e

cd "$(dirname "$0")/.."

echo "==> Sorcery Desktop macOS Release Build"
echo ""

# Check for required environment variables for notarization
if [ -z "$APPLE_ID" ] || [ -z "$APPLE_PASSWORD" ] || [ -z "$APPLE_TEAM_ID" ]; then
    echo "WARNING: Notarization environment variables not set."
    echo "  For notarized builds, set:"
    echo "    APPLE_ID          - Your Apple ID email"
    echo "    APPLE_PASSWORD    - App-specific password (from appleid.apple.com)"
    echo "    APPLE_TEAM_ID     - Your 10-character Team ID"
    echo ""
    echo "Building without notarization..."
    echo ""
fi

# Check for signing identity
if [ -z "$APPLE_SIGNING_IDENTITY" ]; then
    echo "Checking for available signing identities..."
    IDENTITIES=$(security find-identity -v -p codesigning 2>/dev/null | grep "Developer ID Application" || true)
    if [ -n "$IDENTITIES" ]; then
        echo "Found signing identities:"
        echo "$IDENTITIES"
        echo ""
        echo "Set APPLE_SIGNING_IDENTITY to use one of these."
    else
        echo "No Developer ID Application certificates found."
        echo "Building unsigned (won't pass Gatekeeper)..."
    fi
    echo ""
fi

echo "==> Building universal binary (Intel + Apple Silicon)..."
cd src-tauri
cargo tauri build --target universal-apple-darwin

echo ""
echo "==> Build complete!"
echo ""

# Show output location
DMG_DIR="../target/universal-apple-darwin/release/bundle/dmg"
APP_DIR="../target/universal-apple-darwin/release/bundle/macos"

if [ -d "$DMG_DIR" ]; then
    echo "DMG location:"
    ls -la "$DMG_DIR"/*.dmg 2>/dev/null || echo "  (no DMG found)"
fi

echo ""
echo "App bundle location:"
ls -la "$APP_DIR"/*.app 2>/dev/null || echo "  (no app bundle found)"

# Verify signing if built
if [ -d "$APP_DIR/Sorcery Desktop.app" ]; then
    echo ""
    echo "==> Verifying code signature..."
    codesign -dv --verbose=2 "$APP_DIR/Sorcery Desktop.app" 2>&1 | head -10 || echo "  (not signed)"
fi
