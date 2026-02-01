#!/usr/bin/env pwsh
# Run smoke tests

Write-Host "Running smoke tests..." -ForegroundColor Cyan

Write-Host "Running git-diff smoke tests..." -ForegroundColor Yellow
cargo test --test cli_dry_git_diff -- --nocapture

Write-Host "Running resume plan smoke tests..." -ForegroundColor Yellow
cargo test --test cli_resume_plan -- --nocapture

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ All smoke tests passed" -ForegroundColor Green
} else {
    Write-Host "✗ Smoke tests failed" -ForegroundColor Red
    exit 1
}
