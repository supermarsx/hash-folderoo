#!/usr/bin/env bash
# Run smoke tests

set -e

echo "Running smoke tests..."

echo "Running git-diff smoke tests..."
cargo test --test cli_dry_git_diff -- --nocapture

echo "Running resume plan smoke tests..."
cargo test --test cli_resume_plan -- --nocapture

echo "✓ All smoke tests passed"
