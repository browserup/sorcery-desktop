# Protocol Handler End-to-End Testing

## Overview

This document describes the strategy for end-to-end testing of the `srcuri://` protocol handler with actual editor managers.

## Current Test Approach

The current Docker tests (`run-editor-tests.sh`) verify that editors can be launched with line/column arguments via command-line interfaces. These are **CLI-level tests** that ensure:
- Editors are installed correctly
- Editors accept `--line` and `--column` arguments (or equivalent)
- Editors launch successfully in the Docker environment

**What these tests DON'T cover:**
- Protocol handler parsing (`srcuri://` URLs)
- Dispatcher logic (choosing the right editor)
- Editor manager implementations (JetBrains, VSCode, etc.)
- Workspace resolution and path matching
- Settings integration

## End-to-End Protocol Testing Strategy

### Why Not in Docker?

Building the full Tauri application in Docker is complex because:
1. Long build times (compiling Tauri + WebKit bindings)
2. Protocol registration requires system-level changes
3. The app needs to run as a background service
4. Workspace configuration requires real file paths

### Recommended Approach: Local E2E Tests

Create a separate test suite that runs on the host machine where srcuri is installed:

```bash
# tests/e2e/protocol-handler-tests.sh
#!/bin/bash

# Prerequisites:
# 1. srcuri is installed and running
# 2. Protocol handler is registered
# 3. Test workspace is configured

# Test srcuri:// protocol with various editors
test_protocol() {
    local url="$1"
    local expected_editor="$2"

    echo "Testing: $url"

    # Open the URL (this goes through the protocol handler)
    xdg-open "$url"  # Linux
    # open "$url"    # macOS

    sleep 2

    # Verify the correct editor launched
    if pgrep -f "$expected_editor" > /dev/null; then
        echo "✓ PASSED: $expected_editor launched"
        pkill -f "$expected_editor"
        return 0
    else
        echo "✗ FAILED: $expected_editor did not launch"
        return 1
    fi
}

# Test cases
test_protocol "srcuri://srcuri/README.md:10:5" "code"
test_protocol "srcuri://srcuri/src/main.rs:42" "idea"
test_protocol "srcuri:///absolute/path/file.txt:100" "vim"
```

### Integration Test Structure

```
tests/
├── docker/
│   ├── run-editor-tests.sh      # CLI-level tests (current)
│   └── Dockerfile
├── e2e/
│   ├── protocol-handler-tests.sh # End-to-end protocol tests
│   ├── setup-test-workspace.sh   # Configure test workspace
│   └── README.md
└── README.md
```

## Future Work

### Option 1: Lightweight Protocol Test in Docker

Instead of building the full Tauri app, create a minimal Rust binary that:
1. Parses `srcuri://` URLs
2. Calls the dispatcher logic
3. Launches editors via editor managers

This would test most of the stack without requiring the full Tauri GUI application.

### Option 2: Mock Editor Managers

Create a test mode where editor managers log their actions instead of actually launching editors:

```rust
#[cfg(test)]
impl EditorManager for TestEditorManager {
    async fn open(&self, path: &Path, options: &OpenOptions) -> EditorResult<()> {
        // Log the call instead of launching
        println!("Would launch {} with file={} line={:?} column={:?}",
            self.id(), path.display(), options.line, options.column);
        Ok(())
    }
}
```

### Option 3: CI Integration with Real Install

Run end-to-end tests in CI by:
1. Installing srcuri from built artifacts
2. Registering the protocol handler
3. Configuring a test workspace
4. Running protocol tests

## Current Status

- ✅ CLI-level tests working (11 editors tested)
- ⏳ End-to-end protocol tests - planned but not yet implemented
- ⏳ Docker-based E2E tests - deferred due to complexity

## Next Steps

1. Implement local E2E test script that can run on developer machines
2. Document how to set up test environment (workspace config, protocol registration)
3. Consider lightweight protocol test approach for Docker
4. Integrate E2E tests into CI pipeline
