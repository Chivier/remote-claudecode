#!/bin/bash
# Run all tests for CloudCLI
#
# Usage:
#   ./tests/run_all_tests.sh              # Unit tests + API tests (requires services running)
#   ./tests/run_all_tests.sh --unit       # Unit tests only
#   ./tests/run_all_tests.sh --api        # Backend API tests only
#   ./tests/run_all_tests.sh --broker     # Broker WS tests only
#   ./tests/run_all_tests.sh --remote     # Remote fuji1 tests only
#   ./tests/run_all_tests.sh --all        # Everything including remote tests

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
CARGO="${CARGO:-cargo}"

# Try to find cargo
if ! command -v "$CARGO" >/dev/null 2>&1; then
  if [ -x "$HOME/.cargo/bin/cargo" ]; then
    CARGO="$HOME/.cargo/bin/cargo"
  else
    echo "ERROR: cargo not found"
    exit 1
  fi
fi

bold()  { printf "\033[1m%s\033[0m\n" "$1"; }
green() { printf "\033[32m%s\033[0m\n" "$1"; }
red()   { printf "\033[31m%s\033[0m\n" "$1"; }

MODE="${1:---unit}"
TOTAL_PASS=0
TOTAL_FAIL=0

run_section() {
  local name="$1"
  shift
  bold ""
  bold "============================================"
  bold "  $name"
  bold "============================================"
  if "$@"; then
    green "  Section passed"
  else
    red "  Section had failures"
    TOTAL_FAIL=$((TOTAL_FAIL + 1))
  fi
  TOTAL_PASS=$((TOTAL_PASS + 1))
}

# ---- Unit Tests ----
if [ "$MODE" = "--unit" ] || [ "$MODE" = "--all" ] || [ "$MODE" = "" ]; then
  run_section "Rust Unit Tests (cargo test --workspace)" \
    "$CARGO" test --workspace --manifest-path "$PROJECT_DIR/Cargo.toml"
fi

# ---- Backend API Tests ----
if [ "$MODE" = "--api" ] || [ "$MODE" = "--all" ]; then
  run_section "Backend REST API Tests" \
    bash "$SCRIPT_DIR/test_backend_api.sh"
fi

# ---- Broker WS Tests ----
if [ "$MODE" = "--broker" ] || [ "$MODE" = "--all" ]; then
  run_section "Broker WebSocket Tests" \
    node "$SCRIPT_DIR/test_broker_ws.js"
fi

# ---- Remote Tests ----
if [ "$MODE" = "--remote" ] || [ "$MODE" = "--all" ]; then
  run_section "Remote Server (fuji1) Tests" \
    bash "$SCRIPT_DIR/test_remote_fuji1.sh"
fi

# ---- Summary ----
bold ""
bold "============================================"
bold "  All test sections: $TOTAL_PASS ran, $TOTAL_FAIL failures"
bold "============================================"

[ "$TOTAL_FAIL" -eq 0 ] && exit 0 || exit 1
