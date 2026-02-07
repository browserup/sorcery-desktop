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

# Check for signing identity - auto-detect if not set
if [ -z "$APPLE_SIGNING_IDENTITY" ]; then
    echo "Checking for available signing identities..."
    IDENTITY=$(security find-identity -v -p codesigning 2>/dev/null | grep "Developer ID Application" | head -1 | sed 's/.*"\(.*\)".*/\1/')
    if [ -n "$IDENTITY" ]; then
        export APPLE_SIGNING_IDENTITY="$IDENTITY"
        echo "Auto-detected: $APPLE_SIGNING_IDENTITY"
    else
        echo "No Developer ID Application certificates found."
        echo "Building unsigned (won't pass Gatekeeper)..."
    fi
    echo ""
else
    echo "Using signing identity: $APPLE_SIGNING_IDENTITY"
    echo ""
fi

echo "==> Building universal binary (Intel + Apple Silicon)..."
cd src-tauri

# Build - capture exit code but don't fail immediately (DMG bundler has bugs)
set +e
cargo tauri build --target universal-apple-darwin
BUILD_EXIT=$?
set -e

echo ""
APP_DIR="../target/universal-apple-darwin/release/bundle/macos"
DMG_DIR="../target/universal-apple-darwin/release/bundle/dmg"
BUNDLE_DIR="../target/universal-apple-darwin/release/bundle"

# Check if app bundle was created (the important part)
if [ ! -d "$APP_DIR/Sorcery Desktop.app" ]; then
    echo "ERROR: App bundle was not created"
    exit 1
fi

echo "==> App bundle created successfully"

# Check if DMG was created, if not create it manually (Tauri bundler bug workaround)
DMG_FILE=$(ls "$DMG_DIR"/*.dmg 2>/dev/null | head -1)
if [ -z "$DMG_FILE" ]; then
    echo "==> DMG bundler failed, creating DMG manually..."
    mkdir -p "$DMG_DIR"
    DMG_NAME="Sorcery_$(grep '"version"' ../src-tauri/tauri.conf.json | head -1 | sed 's/.*: *"\(.*\)".*/\1/')_universal.dmg"
    hdiutil create -volname "Sorcery Desktop" -srcfolder "$APP_DIR/Sorcery Desktop.app" -ov -format UDZO "$BUNDLE_DIR/$DMG_NAME"
    DMG_FILE="$BUNDLE_DIR/$DMG_NAME"
    echo "  Created: $DMG_FILE"
fi

echo ""
echo "==> Build complete!"
echo ""

# Show output location
echo "DMG location:"
ls -la "$DMG_FILE" 2>/dev/null || ls -la "$BUNDLE_DIR"/*.dmg 2>/dev/null || echo "  (no DMG found)"

echo ""
echo "App bundle location:"
ls -la "$APP_DIR"/*.app 2>/dev/null || echo "  (no app bundle found)"

# Verify signing if built
if [ -d "$APP_DIR/Sorcery Desktop.app" ]; then
    echo ""
    echo "==> Verifying code signature..."
    codesign -dv --verbose=2 "$APP_DIR/Sorcery Desktop.app" 2>&1 | head -10 || echo "  (not signed)"
fi
