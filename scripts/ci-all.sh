#!/usr/bin/env bash
# Run all CI stages: checks, smoke tests, and build

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "========================================"
echo "Running Full CI Pipeline"
echo "========================================"
echo ""

# Phase 1: CI Checks
"$SCRIPT_DIR/ci-checks.sh"
echo ""

# Phase 2: Smoke Tests
echo "========================================"
echo "Running Phase 2: Smoke Tests"
echo "========================================"
echo ""
"$SCRIPT_DIR/smoke.sh"
echo ""

# Phase 3: Build
echo "========================================"
echo "Running Phase 3: Build"
echo "========================================"
echo ""
"$SCRIPT_DIR/build.sh"
echo ""

echo "========================================"
echo "✓ Full CI Pipeline Completed Successfully"
echo "========================================"
