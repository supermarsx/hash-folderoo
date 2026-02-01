#!/usr/bin/env bash
# Check code formatting with rustfmt

set -e

echo "Checking code formatting..."
cargo fmt --all -- --check

echo "✓ Format check passed"
