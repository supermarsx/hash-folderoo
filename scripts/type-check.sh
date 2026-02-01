#!/usr/bin/env bash
# Run Rust type checker (cargo check)

set -e

echo "Running type check..."
cargo check --all-targets

echo "✓ Type check passed"
