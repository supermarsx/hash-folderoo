#!/usr/bin/env bash
# Run all tests

set -e

echo "Running tests..."
cargo test --all --verbose

echo "✓ All tests passed"
