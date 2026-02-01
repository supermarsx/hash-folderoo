#!/usr/bin/env bash
# Build the project in release mode

set -e

echo "Building project..."

if [ -n "$1" ]; then
    echo "Building for target: $1"
    cargo build --release --target "$1"
else
    cargo build --release
fi

echo "✓ Build completed successfully"
