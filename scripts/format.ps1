#!/usr/bin/env pwsh
# Check code formatting with rustfmt

Write-Host "Checking code formatting..." -ForegroundColor Cyan
cargo fmt --all -- --check

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Format check passed" -ForegroundColor Green
} else {
    Write-Host "✗ Format check failed" -ForegroundColor Red
    Write-Host "Run 'cargo fmt --all' to fix formatting issues" -ForegroundColor Yellow
    exit 1
}
