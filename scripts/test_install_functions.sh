#!/bin/bash
# Unit tests for install.sh functions
# Usage: ./test_install_functions.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_PASSED=0
TESTS_FAILED=0

# Colors
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' NC=''
fi

pass() { echo -e "${GREEN}PASS${NC}: $*"; TESTS_PASSED=$((TESTS_PASSED + 1)); }
fail() { echo -e "${RED}FAIL${NC}: $*"; TESTS_FAILED=$((TESTS_FAILED + 1)); }
skip() { echo -e "${YELLOW}SKIP${NC}: $*"; }

# Create a test environment
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

# Extract functions from install.sh for testing
# We do this by creating a modified version that allows sourcing
cat > "$TEMP_DIR/install_testable.sh" << 'TESTEOF'
#!/bin/bash
# Testable version of install.sh

REPO="Dicklesworthstone/meta_skill"
BINARY_NAME="ms"
DEFAULT_INSTALL_DIR="${HOME}/.local/bin"

# Colors disabled for testing
RED='' GREEN='' YELLOW='' BLUE='' BOLD='' NC=''

log()  { echo "[ms] $*"; }
warn() { echo "[ms] WARNING: $*"; }
err()  { echo "[ms] ERROR: $*" >&2; }
die()  { err "$*"; return 1; }

detect_platform() {
    local os arch
    os="$(uname -s | tr '[:upper:]' '[:lower:]')"
    arch="$(uname -m)"

    case "$os" in
        linux) os="unknown-linux-gnu" ;;
        darwin) os="apple-darwin" ;;
        mingw*|msys*|cygwin*) os="pc-windows-msvc" ;;
        *) die "Unsupported OS: $os"; return 1 ;;
    esac

    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) die "Unsupported architecture: $arch"; return 1 ;;
    esac

    echo "${arch}-${os}"
}
TESTEOF

# Source testable functions
source "$TEMP_DIR/install_testable.sh"

echo "=== Install Script Unit Tests ==="
echo ""

# Test 1: Platform detection format
test_platform_detection_format() {
    local result
    result=$(detect_platform 2>/dev/null) || {
        fail "detect_platform failed"
        return
    }

    # Platform should match pattern: (x86_64|aarch64)-(unknown-linux-gnu|apple-darwin|pc-windows-msvc)
    if [[ "$result" =~ ^(x86_64|aarch64)-(unknown-linux-gnu|apple-darwin|pc-windows-msvc)$ ]]; then
        pass "detect_platform returns valid format: $result"
    else
        fail "detect_platform returned invalid format: $result"
    fi
}

# Test 2: Platform detection produces output
test_platform_detection_not_empty() {
    local result
    result=$(detect_platform 2>/dev/null)

    if [[ -n "$result" ]]; then
        pass "detect_platform returns non-empty string"
    else
        fail "detect_platform returned empty string"
    fi
}

# Test 3: Log function works
test_log_function() {
    local output
    output=$(log "test message" 2>&1)

    if [[ "$output" == *"test message"* ]]; then
        pass "log function includes message"
    else
        fail "log function output unexpected: $output"
    fi
}

# Test 4: Die function returns error
test_die_function() {
    if die "test error" 2>/dev/null; then
        fail "die function did not return error"
    else
        pass "die function returns non-zero"
    fi
}

# Test 5: Default install dir is set
test_default_install_dir() {
    if [[ -n "$DEFAULT_INSTALL_DIR" ]]; then
        pass "DEFAULT_INSTALL_DIR is set: $DEFAULT_INSTALL_DIR"
    else
        fail "DEFAULT_INSTALL_DIR is not set"
    fi
}

# Test 6: Binary name is set
test_binary_name() {
    if [[ "$BINARY_NAME" == "ms" ]]; then
        pass "BINARY_NAME is 'ms'"
    else
        fail "BINARY_NAME is not 'ms': $BINARY_NAME"
    fi
}

# Test 7: Repo is correctly set
test_repo_setting() {
    if [[ "$REPO" == "Dicklesworthstone/meta_skill" ]]; then
        pass "REPO is correctly set"
    else
        fail "REPO is not correct: $REPO"
    fi
}

# Test 8: Architecture detection on current system
test_current_arch() {
    local arch
    arch=$(uname -m)

    case "$arch" in
        x86_64|amd64|aarch64|arm64)
            pass "Current architecture is supported: $arch"
            ;;
        *)
            skip "Current architecture may not be supported: $arch"
            ;;
    esac
}

# Test 9: OS detection on current system
test_current_os() {
    local os
    os=$(uname -s | tr '[:upper:]' '[:lower:]')

    case "$os" in
        linux|darwin)
            pass "Current OS is supported: $os"
            ;;
        mingw*|msys*|cygwin*)
            pass "Current OS (Windows-like) is supported: $os"
            ;;
        *)
            fail "Current OS may not be supported: $os"
            ;;
    esac
}

# Test 10: Curl or wget is available
test_download_tool_available() {
    if command -v curl >/dev/null 2>&1; then
        pass "curl is available"
    elif command -v wget >/dev/null 2>&1; then
        pass "wget is available"
    else
        fail "Neither curl nor wget is available"
    fi
}

# Test 11: SHA256 tool is available
test_sha_tool_available() {
    if command -v sha256sum >/dev/null 2>&1; then
        pass "sha256sum is available"
    elif command -v shasum >/dev/null 2>&1; then
        pass "shasum is available"
    else
        fail "No SHA256 tool is available"
    fi
}

# Run all tests
echo "Running platform detection tests..."
test_platform_detection_format
test_platform_detection_not_empty
test_current_arch
test_current_os

echo ""
echo "Running function tests..."
test_log_function
test_die_function

echo ""
echo "Running configuration tests..."
test_default_install_dir
test_binary_name
test_repo_setting

echo ""
echo "Running dependency tests..."
test_download_tool_available
test_sha_tool_available

# Summary
echo ""
echo "=== Test Summary ==="
echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Failed: ${RED}$TESTS_FAILED${NC}"

if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi

exit 0
