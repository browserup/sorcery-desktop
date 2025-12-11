# Sorcery Integration Tests

This directory contains integration tests for verifying editor launches across different platforms.

## Test Structure

```
tests/
├── docker/
│   ├── Dockerfile           # Ubuntu-based test environment with all Linux editors
│   └── run-tests.sh         # Script to run tests in Docker
└── integration/
    └── editor_launch_tests.rs  # Integration tests for editor launching
```

## Running Tests

### Linux (via Docker)

Run all editor integration tests in a containerized environment:

```bash
# Build container
docker compose build

# Run shell-based tests (recommended - no compilation needed)
docker compose exec test-env bash -c "cd /workspace/sorcery && tests/docker/run-editor-tests.sh"

# Or run interactively
docker compose run --rm test-env bash
# Inside container:
cd /workspace/sorcery && tests/docker/run-editor-tests.sh
```

To rebuild and test from scratch:

```bash
# Rebuild container (if dependencies changed)
docker compose down
docker compose build

# Start container
docker compose up -d

# Run tests
docker compose exec test-env bash -c "cd /workspace/sorcery && tests/docker/run-editor-tests.sh"

# Stop container
docker compose down
```

### Interactive Docker Shell

For debugging or manual testing:

```bash
docker-compose up -d
docker-compose exec test-env bash

# Inside container:
cd /workspace/sorcery/src-tauri
cargo test --test integration --features docker-tests -- --nocapture
```

### macOS (Local)

Run tests directly on your Mac (tests will use locally installed editors):

```bash
cd src-tauri
cargo test --test integration
```

## Test Environment

The Docker container includes:

**GUI Editors:**
- Visual Studio Code (with `--no-sandbox` flag for Docker)
- VSCodium (with `--no-sandbox` flag for Docker)
- Sublime Text
- Gedit
- Kate
- IntelliJ IDEA Community Edition
- PyCharm Community Edition

**Terminal Editors:**
- Vim
- Neovim
- Emacs
- Nano
- Micro

**Terminal Emulators:**
- Kitty
- GNOME Terminal
- Xterm

**Test Utilities:**
- Xvfb (virtual display)
- wmctrl (window management)
- xdotool (X11 automation)
- lsof (file access verification)
- pgrep/pkill (process management)

## What the Tests Verify

1. **Process Spawning**: Editor processes start successfully
2. **Line Number Support**: Editors navigate to the specified line numbers
3. **Column Number Support**: Editors that support column positioning (VSCode, VSCodium, Sublime Text, Gedit, Kate, Micro) can navigate to specific columns
4. **Graceful Degradation**: Editors that don't support columns still open to the correct line

## Test Features

- `--test-threads=1`: Tests run sequentially to avoid window/process conflicts
- `--nocapture`: Show test output for debugging
- `--features docker-tests`: Enable Docker-specific test configurations

## Adding New Editor Tests

1. Ensure the editor is installed in the Dockerfile
2. Add a test function in `editor_launch_tests.rs`:

```rust
#[test]
fn test_my_editor_launches() {
    let (_temp_dir, test_file) = setup();

    let result = Command::new("my-editor")
        .arg(&test_file)
        .spawn();

    assert!(result.is_ok());
    assert!(wait_for_process("my-editor", 10));

    cleanup("my-editor");
}
```

## Troubleshooting

**Docker build fails:**
- Check your internet connection (downloads editors)
- Try `docker system prune` to free up space
- Ensure you're using Ubuntu 24.04 (required for webkit2gtk-6.0)

**Tests timeout:**
- Increase timeout values in test code
- Check Docker container resources (CPU/memory)

**Process not detected:**
- Verify process name with `pgrep -a <name>` in container
- Some editors fork/daemonize, use parent process name

**VSCode/VSCodium won't start:**
- Ensure `--no-sandbox` and `--user-data-dir` flags are included (required for root in Docker)
- Error: "trying to start as super user" means these flags are missing

**Xvfb issues:**
- Ensure Xvfb is running (`ps aux | grep Xvfb`)
- Check `DISPLAY` environment variable is set to `:99`

## CI Integration

Tests can run in GitHub Actions or other CI systems with Docker support:

```yaml
- name: Run editor integration tests
  run: ./tests/docker/run-tests.sh
```
