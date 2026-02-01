#!/usr/bin/env pwsh
# Run clippy linter with warnings as errors

Write-Host "Running clippy linter..." -ForegroundColor Cyan
cargo clippy --all-targets -- -D warnings

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Lint check passed" -ForegroundColor Green
} else {
    Write-Host "✗ Lint check failed" -ForegroundColor Red
    exit 1
}
