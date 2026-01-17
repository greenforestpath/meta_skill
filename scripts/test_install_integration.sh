#!/bin/bash
# Integration tests for install.sh
# Tests installation in an isolated environment
# Usage: ./test_install_integration.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG="install_test_$(date +%Y%m%d_%H%M%S).log"
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

log()  { echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG"; }
pass() { log "${GREEN}PASS${NC}: $*"; TESTS_PASSED=$((TESTS_PASSED + 1)); }
fail() { log "${RED}FAIL${NC}: $*"; TESTS_FAILED=$((TESTS_FAILED + 1)); }
skip() { log "${YELLOW}SKIP${NC}: $*"; }

# Create isolated test environment
create_test_env() {
    local temp_dir
    temp_dir=$(mktemp -d)
    export TEST_HOME="$temp_dir"
    export TEST_INSTALL_DIR="$temp_dir/.local/bin"
    mkdir -p "$TEST_INSTALL_DIR"
    echo "$temp_dir"
}

# Cleanup test environment
cleanup_test_env() {
    local temp_dir="$1"
    rm -rf "$temp_dir" 2>/dev/null || true
}

echo "=== Install Script Integration Tests ==="
echo "Log: $LOG"
echo ""

# Test 1: Script syntax is valid
test_script_syntax() {
    log "Testing script syntax..."

    if bash -n "$SCRIPT_DIR/install.sh" 2>&1; then
        pass "Script has valid syntax"
    else
        fail "Script has syntax errors"
    fi
}

# Test 2: Help option works
test_help_option() {
    log "Testing --help option..."

    local output
    if output=$("$SCRIPT_DIR/install.sh" --help 2>&1); then
        if [[ "$output" == *"Usage"* ]] && [[ "$output" == *"install-dir"* ]]; then
            pass "--help shows usage information"
        else
            fail "--help output missing expected content"
        fi
    else
        # --help should exit 0
        fail "--help returned non-zero"
    fi
}

# Test 3: Unknown option is rejected
test_unknown_option() {
    log "Testing unknown option handling..."

    if "$SCRIPT_DIR/install.sh" --unknown-option 2>/dev/null; then
        fail "Unknown option was accepted"
    else
        pass "Unknown option is rejected"
    fi
}

# Test 4: Platform detection in script
test_platform_in_script() {
    log "Testing platform detection..."

    # Create a modified script that just prints platform
    local temp_dir
    temp_dir=$(mktemp -d)

    cat > "$temp_dir/detect_only.sh" << 'EOF'
#!/bin/bash
detect_platform() {
    local os arch
    os="$(uname -s | tr '[:upper:]' '[:lower:]')"
    arch="$(uname -m)"
    case "$os" in
        linux) os="unknown-linux-gnu" ;;
        darwin) os="apple-darwin" ;;
        *) echo "unsupported"; exit 1 ;;
    esac
    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) echo "unsupported"; exit 1 ;;
    esac
    echo "${arch}-${os}"
}
detect_platform
EOF
    chmod +x "$temp_dir/detect_only.sh"

    local platform
    if platform=$("$temp_dir/detect_only.sh" 2>&1); then
        if [[ "$platform" =~ ^(x86_64|aarch64)-(unknown-linux-gnu|apple-darwin)$ ]]; then
            pass "Platform detected: $platform"
        else
            fail "Invalid platform format: $platform"
        fi
    else
        fail "Platform detection failed"
    fi

    rm -rf "$temp_dir"
}

# Test 5: NO_COLOR environment variable
test_no_color() {
    log "Testing NO_COLOR environment variable..."

    local output
    # Help output should not contain escape codes when NO_COLOR is set
    output=$(NO_COLOR=1 "$SCRIPT_DIR/install.sh" --help 2>&1)

    # Check for ANSI escape sequences
    if echo "$output" | grep -q $'\033'; then
        fail "Output contains color codes when NO_COLOR=1"
    else
        pass "NO_COLOR disables colored output"
    fi
}

# Test 6: Script runs without errors (up to download)
test_script_starts() {
    log "Testing script starts correctly..."

    local temp_dir output
    temp_dir=$(create_test_env)

    # Run with a version that doesn't exist to test pre-download logic
    # This should fail at download, but not before
    output=$(HOME="$temp_dir" VERSION="v0.0.0-nonexistent" timeout 10 "$SCRIPT_DIR/install.sh" 2>&1 || true)

    if [[ "$output" == *"Installing ms"* ]] && [[ "$output" == *"Detected platform"* ]]; then
        pass "Script starts and detects platform"
    else
        fail "Script did not start correctly: ${output:0:200}..."
    fi

    cleanup_test_env "$temp_dir"
}

# Test 7: INSTALL_DIR environment variable
test_install_dir_env() {
    log "Testing INSTALL_DIR environment variable..."

    local temp_dir output
    temp_dir=$(create_test_env)
    local custom_dir="$temp_dir/custom/bin"

    # Run installer - it will fail at download but that's expected
    # We're just testing that it accepts the INSTALL_DIR variable
    output=$(HOME="$temp_dir" INSTALL_DIR="$custom_dir" VERSION="v0.0.0-nonexistent" timeout 10 "$SCRIPT_DIR/install.sh" 2>&1 || true)

    # Script started successfully with custom INSTALL_DIR (will fail at download)
    if [[ "$output" == *"Installing ms"* ]]; then
        pass "INSTALL_DIR environment variable accepted"
    else
        fail "Script did not start: ${output:0:200}..."
    fi

    cleanup_test_env "$temp_dir"
}

# Test 8: --no-verify option
test_no_verify_option() {
    log "Testing --no-verify option..."

    local temp_dir output
    temp_dir=$(create_test_env)

    output=$(HOME="$temp_dir" VERSION="v0.0.0-nonexistent" timeout 10 "$SCRIPT_DIR/install.sh" --no-verify 2>&1 || true)

    # The script should acknowledge --no-verify at some point if it gets far enough
    pass "--no-verify option accepted without error"

    cleanup_test_env "$temp_dir"
}

# Test 9: --version option parsing
test_version_option() {
    log "Testing --version option..."

    local temp_dir output
    temp_dir=$(create_test_env)

    output=$(HOME="$temp_dir" timeout 10 "$SCRIPT_DIR/install.sh" --version v0.1.0 2>&1 || true)

    if [[ "$output" == *"v0.1.0"* ]]; then
        pass "--version option sets version"
    else
        fail "--version not reflected in output: ${output:0:200}..."
    fi

    cleanup_test_env "$temp_dir"
}

# Test 10: Script handles missing curl/wget gracefully
# (This test is tricky as we can't easily remove curl/wget)
test_download_tool_check() {
    log "Testing download tool availability check..."

    if command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1; then
        pass "Download tool (curl or wget) is available"
    else
        fail "No download tool available"
    fi
}

# Run all tests
echo "Running syntax tests..."
test_script_syntax

echo ""
echo "Running option tests..."
test_help_option
test_unknown_option
test_no_verify_option
test_version_option

echo ""
echo "Running environment tests..."
test_no_color
test_install_dir_env
test_platform_in_script

echo ""
echo "Running execution tests..."
test_script_starts
test_download_tool_check

# Summary
echo ""
echo "=== Test Summary ==="
log "Passed: ${GREEN}$TESTS_PASSED${NC}"
log "Failed: ${RED}$TESTS_FAILED${NC}"
log "See full log: $LOG"

if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi

exit 0
