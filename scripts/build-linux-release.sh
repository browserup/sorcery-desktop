#!/usr/bin/env bash
set -e

cd "$(dirname "$0")/.."

echo "==> Sorcery Desktop Linux Release Build"
echo ""

# Check for required dependencies
echo "Checking dependencies..."
MISSING_DEPS=""

if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    MISSING_DEPS="$MISSING_DEPS libwebkit2gtk-4.1-dev"
fi

if ! pkg-config --exists ayatana-appindicator3-0.1 2>/dev/null; then
    MISSING_DEPS="$MISSING_DEPS libayatana-appindicator3-dev"
fi

if ! pkg-config --exists librsvg-2.0 2>/dev/null; then
    MISSING_DEPS="$MISSING_DEPS librsvg2-dev"
fi

if [ -n "$MISSING_DEPS" ]; then
    echo "Missing dependencies:$MISSING_DEPS"
    echo ""
    echo "Install with:"
    echo "  sudo apt-get install$MISSING_DEPS"
    exit 1
fi

echo "All dependencies found."
echo ""

echo "==> Building release..."
cd src-tauri
cargo tauri build

echo ""
echo "==> Build complete!"
echo ""

# Show output locations
BUNDLE_DIR="../target/release/bundle"

echo "Build artifacts:"
ls -la "$BUNDLE_DIR/deb/"*.deb 2>/dev/null && echo "" || true
ls -la "$BUNDLE_DIR/rpm/"*.rpm 2>/dev/null && echo "" || true
ls -la "$BUNDLE_DIR/appimage/"*.AppImage 2>/dev/null && echo "" || true

echo ""
echo "To install the .deb package:"
echo "  sudo dpkg -i $BUNDLE_DIR/deb/*.deb"
