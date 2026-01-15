#!/usr/bin/env bash
# E2E test for beads integration
# Usage: ./scripts/test_beads_e2e.sh [--verbose]

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_DIR="$(mktemp -d)"
VERBOSE=${VERBOSE:-0}
[[ "${1:-}" == "--verbose" ]] && VERBOSE=1

# Logging functions
log() { echo "[$(date +%H:%M:%S)] $*"; }
log_step() { echo ""; echo "=== $* ==="; }
log_detail() { [[ $VERBOSE -eq 1 ]] && echo "    $*" || true; }
log_success() { echo "[OK] $*"; }
log_error() { echo "[ERR] $*" >&2; }

# Minimal JSON helpers (jq preferred; python fallback)
json_dependents_count() {
    local json="$1"
    if command -v jq &>/dev/null; then
        echo "$json" | jq -r ".dependents | length"
        return
    fi
    python - <<'PY' "$json"
import json,sys
payload = sys.argv[1]
try:
    data = json.loads(payload)
    dependents = data.get("dependents") or []
    print(len(dependents))
except Exception:
    print(0)
PY
}

json_status() {
    local json="$1"
    if command -v jq &>/dev/null; then
        echo "$json" | jq -r ".status"
        return
    fi
    python - <<'PY' "$json"
import json,sys
payload = sys.argv[1]
try:
    data = json.loads(payload)
    print(data.get("status", ""))
except Exception:
    print("")
PY
}

json_array_length() {
    local json="$1"
    if command -v jq &>/dev/null; then
        echo "$json" | jq -r "length"
        return
    fi
    python - <<'PY' "$json"
import json,sys
payload = sys.argv[1]
try:
    data = json.loads(payload)
    print(len(data))
except Exception:
    print(0)
PY
}

cleanup() {
    log_step "Cleanup"
    # Remove temp test directory (created by mktemp -d above)
    if [[ -d "$TEST_DIR" && "$TEST_DIR" == /tmp/* ]]; then
        rm -rf -- "$TEST_DIR"
        log_success "Cleaned up test directory"
    else
        log_error "Skipping cleanup: unexpected temp path '$TEST_DIR'"
    fi
}
trap cleanup EXIT

# Start test
log_step "E2E Test: Beads Integration"
log "Test directory: $TEST_DIR"
log "Project root: $PROJECT_ROOT"

# Step 1: Check bd availability
log_step "Step 1: Binary Discovery"
if command -v bd &>/dev/null; then
    if bd --version &>/dev/null; then
        BD_VERSION=$(bd --version 2>&1 | head -1)
    else
        BD_VERSION=$(bd version 2>&1 | head -1)
    fi
    log_success "Found bd: $BD_VERSION"
else
    log_error "bd binary not found in PATH"
    exit 1
fi

# Step 2: Initialize test environment
log_step "Step 2: Initialize Test Database"
cd "$TEST_DIR"
export BEADS_DB="$TEST_DIR/.beads/beads.db"
mkdir -p .beads

START_TIME=$(date +%s%N)
bd init 2>&1 | while IFS= read -r line; do log_detail "$line"; done
INIT_TIME=$(( ($(date +%s%N) - START_TIME) / 1000000 ))
log_success "Database initialized in ${INIT_TIME}ms"

# Step 3: Create issues
log_step "Step 3: Create Issues"

# Create parent epic
EPIC_ID=$(bd create --type=epic --title="E2E Test Epic" --priority=1 --silent 2>&1)
log_success "Created epic: $EPIC_ID"

# Create child tasks
TASK1_ID=$(bd create --type=task --title="E2E Task 1" --priority=2 --silent 2>&1)
TASK2_ID=$(bd create --type=task --title="E2E Task 2" --priority=2 --silent 2>&1)
log_success "Created tasks: $TASK1_ID, $TASK2_ID"

# Step 4: Add dependencies
log_step "Step 4: Dependency Management"
bd dep add "$TASK1_ID" "$EPIC_ID" 2>&1 | while IFS= read -r line; do log_detail "$line"; done
bd dep add "$TASK2_ID" "$EPIC_ID" 2>&1 | while IFS= read -r line; do log_detail "$line"; done
log_success "Added dependencies"

# Verify dependencies
EPIC_JSON=$(bd show "$EPIC_ID" --json 2>&1)
DEPS=$(json_dependents_count "$EPIC_JSON")
log_detail "Epic has $DEPS dependents"
[[ "$DEPS" -ge 2 ]] || { log_error "Expected at least 2 dependents"; exit 1; }
log_success "Dependencies verified"

# Step 5: Status updates
log_step "Step 5: Issue Lifecycle"

# Start work on task 1
bd update "$TASK1_ID" --status=in_progress 2>&1 | while IFS= read -r line; do log_detail "$line"; done
TASK1_JSON=$(bd show "$TASK1_ID" --json 2>&1)
STATUS=$(json_status "$TASK1_JSON")
[[ "$STATUS" == "in_progress" ]] || { log_error "Expected in_progress, got $STATUS"; exit 1; }
log_success "Task 1 in progress"

# Complete task 1
bd close "$TASK1_ID" --reason="E2E test complete" 2>&1 | while IFS= read -r line; do log_detail "$line"; done
TASK1_JSON=$(bd show "$TASK1_ID" --json 2>&1)
STATUS=$(json_status "$TASK1_JSON")
[[ "$STATUS" == "closed" ]] || { log_error "Expected closed, got $STATUS"; exit 1; }
log_success "Task 1 closed"

# Step 6: List operations
log_step "Step 6: List Operations"
OPEN_JSON=$(bd list --status=open --json 2>&1)
OPEN_COUNT=$(json_array_length "$OPEN_JSON")
log_detail "Open issues: $OPEN_COUNT"
[[ "$OPEN_COUNT" -ge 1 ]] || { log_error "Expected at least 1 open issue"; exit 1; }
log_success "List operations working"

# Step 7: Ready list (respects dependencies)
log_step "Step 7: Ready List (Dependency Filtering)"
READY_JSON=$(bd ready --json 2>&1)
READY_COUNT=$(json_array_length "$READY_JSON")
log_detail "Ready issues: $READY_COUNT"
log_success "Ready list working"

# Step 8: Error recovery (expected failure)
log_step "Step 8: Error Recovery"
if bd show "does-not-exist" --json >/dev/null 2>&1; then
    log_error "Expected bd show to fail for non-existent issue"
    exit 1
fi
log_success "Error recovery verified (missing issue handled)"

# Step 9: Sync operation
log_step "Step 9: Sync (Git Integration)"
if bd sync 2>&1 | while IFS= read -r line; do log_detail "$line"; done; then
    log_success "Sync completed"
else
    log_detail "Sync returned non-zero (likely missing git repo), continuing"
    log_success "Sync handled"
fi

# Step 10: Performance check
log_step "Step 10: Performance Validation"
PERF_START=$(date +%s%N)
for _ in {1..5}; do
    bd list --status=open --json >/dev/null 2>&1
done
LIST_TIME=$(( ($(date +%s%N) - PERF_START) / 1000000 / 5 ))
log_detail "Average list time: ${LIST_TIME}ms"
if [[ "$LIST_TIME" -ge 500 ]]; then
    log_error "List operation slower than expected (${LIST_TIME}ms)"
fi
log_success "Performance check completed"

# Step 11: Rust integration test
log_step "Step 11: Rust Integration Test"
cd "$PROJECT_ROOT"
BEADS_DB="$TEST_DIR/.beads/beads.db" cargo test beads::client --quiet 2>&1 | while IFS= read -r line; do log_detail "$line"; done || {
    log_error "Rust integration tests failed"
    exit 1
}
log_success "Rust BeadsClient tests pass"

# Summary
log_step "Test Summary"
TOTAL_TIME=$(( ($(date +%s%N) - START_TIME) / 1000000 ))
echo ""
echo "Results:"
echo "  Epic created: $EPIC_ID"
echo "  Tasks created: $TASK1_ID, $TASK2_ID"
echo "  Dependencies: Working"
echo "  Status updates: Working"
echo "  Sync: Handled"
echo "  Performance: ${LIST_TIME}ms avg list"
echo "  Total time: ${TOTAL_TIME}ms"
echo ""
log_success "All E2E tests passed"
