#!/bin/bash
# End-to-end protocol handler tests
# Tests the full srcuri:// protocol flow through dispatcher and editor managers

set -e

DISPLAY=:99

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

PASSED=0
FAILED=0
TESTS=()

# Test workspace directory
TEST_WORKSPACE="/tmp/sorcery-test-workspace"
TEST_FILE="$TEST_WORKSPACE/test.rs"

setup_workspace() {
    echo "Setting up test workspace..."
    mkdir -p "$TEST_WORKSPACE"

    # Create a test file
    cat > "$TEST_FILE" << 'EOF'
// Test file for srcuri protocol tests
fn main() {
    println!("Line 1");
    println!("Line 2");
    println!("Line 3");
    println!("Line 4");
    println!("Line 5");
    println!("Line 6");
    println!("Line 7");
    println!("Line 8");
    println!("Line 9");
    println!("Line 10");
}
EOF

    echo "Test workspace created at: $TEST_WORKSPACE"
}

start_sorcery() {
    echo "Starting sorcery server..."

    # Build sorcery if not already built
    if [ ! -f "/workspace/sorcery/src-tauri/target/debug/sorcery-desktop" ]; then
        echo "Building sorcery..."
        cd /workspace/sorcery
        cargo build --manifest-path=src-tauri/Cargo.toml
    fi

    # Start sorcery in background
    cd /workspace/sorcery
    ./src-tauri/target/debug/sorcery-desktop &
    SORCERY_PID=$!

    # Wait for server to start
    echo "Waiting for sorcery to start (PID: $SORCERY_PID)..."
    sleep 3

    if ! kill -0 $SORCERY_PID 2>/dev/null; then
        echo "ERROR: sorcery failed to start"
        return 1
    fi

    echo "sorcery server started (PID: $SORCERY_PID)"
}

cleanup_process() {
    local process_name="$1"
    pkill -f "$process_name" 2>/dev/null || true
    sleep 0.5
}

wait_for_process() {
    local process_name="$1"
    local timeout=10
    local elapsed=0

    while [ $elapsed -lt $timeout ]; do
        if pgrep -f "$process_name" > /dev/null; then
            return 0
        fi
        sleep 0.5
        elapsed=$((elapsed + 1))
    done
    return 1
}

test_protocol_url() {
    local test_name="$1"
    local url="$2"
    local expected_process="$3"

    echo -n "Testing $test_name... "

    # Send protocol URL to sorcery via command-line
    # (In real usage, this would come from the system protocol handler)
    /workspace/sorcery/src-tauri/target/debug/sorcery-desktop "$url" &>/dev/null &

    if wait_for_process "$expected_process"; then
        echo -e "${GREEN}PASSED${NC}"
        PASSED=$((PASSED + 1))
        TESTS+=("✓ $test_name")
    else
        echo -e "${RED}FAILED${NC} - $expected_process did not start"
        FAILED=$((FAILED + 1))
        TESTS+=("✗ $test_name")
    fi

    cleanup_process "$expected_process"
}

echo "======================================"
echo "Protocol Handler Integration Tests"
echo "======================================"
echo ""

# Setup
setup_workspace

# Start sorcery server
start_sorcery

# Give the server time to initialize
sleep 2

echo ""
echo "Running protocol tests..."
echo ""

# Test absolute path with line and column
test_protocol_url "Absolute path (VSCode)" \
    "srcuri://$TEST_FILE:5:10" \
    "code"

# Test with VSCodium
test_protocol_url "Absolute path (VSCodium)" \
    "srcuri://$TEST_FILE:3:5" \
    "codium"

# Test with Sublime Text
test_protocol_url "Absolute path (Sublime)" \
    "srcuri://$TEST_FILE:7:1" \
    "sublime_text"

# Test with Vim (line only)
test_protocol_url "Absolute path (Vim)" \
    "srcuri://$TEST_FILE:4" \
    "vim"

# Test with IntelliJ IDEA
test_protocol_url "Absolute path (IntelliJ)" \
    "srcuri://$TEST_FILE:8:15" \
    "idea"

echo ""
echo "======================================"
echo "Test Summary"
echo "======================================"
for test in "${TESTS[@]}"; do
    echo "$test"
done
echo ""
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo "Total: $((PASSED + FAILED))"
echo ""

# Cleanup
echo "Cleaning up..."
if [ -n "$SORCERY_PID" ]; then
    kill $SORCERY_PID 2>/dev/null || true
fi
rm -rf "$TEST_WORKSPACE"

if [ $FAILED -eq 0 ]; then
    echo "✓ All protocol tests passed!"
    exit 0
else
    echo "✗ Some protocol tests failed"
    exit 1
fi
