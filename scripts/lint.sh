#!/usr/bin/env bash
# Run clippy linter with warnings as errors

set -e

echo "Running clippy linter..."
cargo clippy --all-targets -- -D warnings

echo "✓ Lint check passed"
