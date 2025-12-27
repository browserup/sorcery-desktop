#!/bin/bash
# Standalone editor launch tests - no Rust compilation needed

set -e

DISPLAY=:99

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

PASSED=0
FAILED=0
TESTS=()

# Create a test file
TEST_DIR=$(mktemp -d)
TEST_FILE="$TEST_DIR/test.rs"
echo 'fn main() { println!("Hello, world!"); }' > "$TEST_FILE"

cleanup() {
    pkill -f "$1" 2>/dev/null || true
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

run_test() {
    local test_name="$1"
    local editor_command="$2"
    local process_name="$3"
    local line_arg="$4"
    local column_arg="$5"

    echo -n "Testing $test_name... "

    # Build command
    if [ -n "$column_arg" ]; then
        eval "$editor_command \"$line_arg\" \"$column_arg\" \"$TEST_FILE\" &" 2>/dev/null
    elif [ -n "$line_arg" ]; then
        eval "$editor_command \"$line_arg\" \"$TEST_FILE\" &" 2>/dev/null
    else
        eval "$editor_command \"$TEST_FILE\" &" 2>/dev/null
    fi

    if wait_for_process "$process_name"; then
        echo -e "${GREEN}PASSED${NC}"
        PASSED=$((PASSED + 1))
        TESTS+=("✓ $test_name")
    else
        echo -e "${RED}FAILED${NC} - process did not start"
        FAILED=$((FAILED + 1))
        TESTS+=("✗ $test_name")
    fi

    cleanup "$process_name"
}

echo "======================================"
echo "Editor Launch Integration Tests"
echo "======================================"
echo ""

# Test VSCode with line and column (Docker requires --no-sandbox and --user-data-dir as root)
run_test "VSCode (line:column)" "code --no-sandbox --user-data-dir=/tmp/vscode-data --goto" "code" "$TEST_FILE:5:10" ""

# Test VSCodium with line and column (Docker requires --no-sandbox and --user-data-dir as root)
run_test "VSCodium (line:column)" "codium --no-sandbox --user-data-dir=/tmp/codium-data --goto" "codium" "$TEST_FILE:5:10" ""

# Test Sublime Text with line and column
run_test "Sublime Text (line:column)" "subl" "sublime_text" "$TEST_FILE:5:10" ""

# Test Vim with line (column not supported)
run_test "Vim (line only)" "xterm -e vim" "vim" "+5" ""

# Test Neovim with line (column not supported easily)
run_test "Neovim (line only)" "xterm -e nvim" "nvim" "+5" ""

# Test Emacs (no window mode)
run_test "Emacs (line only)" "xterm -e emacs -nw" "emacs" "+5" ""

# Test Gedit with line and column
run_test "Gedit (line:column)" "gedit" "gedit" "+5:10" ""

# Test Kate with line and column
run_test "Kate (line:column)" "kate --line 5 --column 10" "kate" "" ""

# Test Micro with line and column
run_test "Micro (line:column)" "xterm -e micro" "micro" "+5:10" ""

# Test IntelliJ IDEA with line and column
run_test "IntelliJ IDEA (line:column)" "idea --line 5 --column 10" "idea" "" ""

# Test PyCharm with line and column
run_test "PyCharm (line:column)" "pycharm --line 5 --column 10" "pycharm" "" ""

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
rm -rf "$TEST_DIR"

if [ $FAILED -eq 0 ]; then
    echo "✓ All tests passed!"
    exit 0
else
    echo "✗ Some tests failed"
    exit 1
fi
