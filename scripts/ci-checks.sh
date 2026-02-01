#!/usr/bin/env bash
# Run Phase 1 CI checks (format, lint, type-check, test) in sequence
# These would run in parallel in CI but run sequentially locally for simplicity

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "========================================"
echo "Running Phase 1: CI Checks"
echo "========================================"
echo ""

# Format check
echo "[1/4] Format Check"
"$SCRIPT_DIR/format.sh"
echo ""

# Lint check
echo "[2/4] Lint Check"
"$SCRIPT_DIR/lint.sh"
echo ""

# Type check
echo "[3/4] Type Check"
"$SCRIPT_DIR/type-check.sh"
echo ""

# Tests
echo "[4/4] Tests"
"$SCRIPT_DIR/test.sh"
echo ""

echo "========================================"
echo "✓ All CI Checks Passed"
echo "========================================"
