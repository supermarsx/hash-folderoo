#!/usr/bin/env pwsh
# Run Rust type checker (cargo check)

Write-Host "Running type check..." -ForegroundColor Cyan
cargo check --all-targets

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Type check passed" -ForegroundColor Green
} else {
    Write-Host "✗ Type check failed" -ForegroundColor Red
    exit 1
}
