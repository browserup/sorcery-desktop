#!/bin/bash

# Smart rebuild hook for sorcery
# Only rebuilds when there are actual code changes (not just plan files or markdown)

set -e

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"
DIGEST_FILE="$PROJECT_DIR/.last-build-digest"

log() {
  echo "🔨 Rebuild Hook: $1"
}

# Get git diff of actual source files (excluding plans, markdown, etc.)
get_source_digest() {
  cd "$PROJECT_DIR"

  # Get diff of source files only (Rust, HTML, JS, JSON configs)
  git diff HEAD -- \
    'src-tauri/**/*.rs' \
    'src-tauri/**/Cargo.toml' \
    'src-tauri/**/Cargo.lock' \
    'public/**/*.html' \
    'public/**/*.js' \
    'public/**/*.css' \
    'src-tauri/tauri.conf.json' \
    2>/dev/null | shasum -a 256 | cut -d' ' -f1
}

main() {
  log "Checking for code changes..."

  # Get current source digest
  CURRENT_DIGEST=$(get_source_digest)

  # Read last digest if it exists
  LAST_DIGEST=""
  if [ -f "$DIGEST_FILE" ]; then
    LAST_DIGEST=$(cat "$DIGEST_FILE")
  fi

  # Compare digests
  if [ "$CURRENT_DIGEST" = "$LAST_DIGEST" ] && [ -n "$CURRENT_DIGEST" ]; then
    log "No code changes detected. Skipping rebuild."
    exit 0
  fi

  log "Code changes detected! Rebuilding sorcery..."

  # Kill existing sorcery process
  pkill -9 sorcery-desktop 2>/dev/null || true
  log "Stopped existing sorcery process"

  # Build and run
  cd "$PROJECT_DIR"
  log "Building with cargo tauri dev..."

  # Run in background and detach
  nohup cargo tauri dev > /dev/null 2>&1 &

  # Save the new digest
  echo "$CURRENT_DIGEST" > "$DIGEST_FILE"

  log "✅ Rebuild started in background"
}

main
exit 0
