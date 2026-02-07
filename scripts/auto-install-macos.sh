#!/usr/bin/env bash
set -euo pipefail

if ! command -v fswatch >/dev/null 2>&1; then
    echo "ERROR: fswatch is required. Install with: brew install fswatch"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

WATCH_PATHS=(
    "public"
    "src-tauri/src"
    "src-tauri/Cargo.toml"
    "src-tauri/tauri.conf.json"
)

echo "==> Auto-install watcher (macOS)"
echo "Watching:"
for path in "${WATCH_PATHS[@]}"; do
    echo "  - $path"
done
echo ""
echo "Running initial install..."
echo ""
make install
echo ""
echo "Watching for changes. Press Ctrl+C to stop."

run_install() {
    local timestamp
    timestamp="$(date '+%H:%M:%S')"
    echo ""
    echo "[$timestamp] Change detected. Running make install..."
    if make install; then
        echo "[$(date '+%H:%M:%S')] Install complete."
    else
        echo "[$(date '+%H:%M:%S')] Install failed. Watching for next change."
    fi
}

fswatch -o --latency=0.6 "${WATCH_PATHS[@]}" | while read -r _; do
    run_install
done
