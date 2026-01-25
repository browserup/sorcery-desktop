#!/usr/bin/env bash
set -e

cd "$(dirname "$0")/.."

APP_PATH="/Applications/Sorcery Desktop.app"

if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: App not found at $APP_PATH"
    echo "Run 'make release' or install the app first"
    exit 1
fi

if [ -z "$APPLE_ID" ] || [ -z "$APPLE_PASSWORD" ] || [ -z "$APPLE_TEAM_ID" ]; then
    echo "ERROR: Notarization requires these environment variables:"
    echo "  APPLE_ID       - Your Apple ID email"
    echo "  APPLE_PASSWORD - App-specific password"
    echo "  APPLE_TEAM_ID  - Your 10-character Team ID"
    exit 1
fi

echo "==> Creating zip for notarization..."
ZIP_PATH="/tmp/sorcery-desktop-notarize.zip"
ditto -c -k --keepParent "$APP_PATH" "$ZIP_PATH"

echo "==> Submitting to Apple for notarization..."
xcrun notarytool submit "$ZIP_PATH" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait

echo "==> Stapling notarization ticket..."
xcrun stapler staple "$APP_PATH"

rm -f "$ZIP_PATH"

echo ""
echo "✓ Notarization complete!"
