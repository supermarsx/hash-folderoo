#!/usr/bin/env pwsh
# Run Phase 1 CI checks (format, lint, type-check, test) in sequence
# These would run in parallel in CI but run sequentially locally for simplicity

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Running Phase 1: CI Checks" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$failed = $false

# Format check
Write-Host "[1/4] Format Check" -ForegroundColor Cyan
& "$PSScriptRoot\format.ps1"
if ($LASTEXITCODE -ne 0) { $failed = $true }
Write-Host ""

# Lint check
Write-Host "[2/4] Lint Check" -ForegroundColor Cyan
& "$PSScriptRoot\lint.ps1"
if ($LASTEXITCODE -ne 0) { $failed = $true }
Write-Host ""

# Type check
Write-Host "[3/4] Type Check" -ForegroundColor Cyan
& "$PSScriptRoot\type-check.ps1"
if ($LASTEXITCODE -ne 0) { $failed = $true }
Write-Host ""

# Tests
Write-Host "[4/4] Tests" -ForegroundColor Cyan
& "$PSScriptRoot\test.ps1"
if ($LASTEXITCODE -ne 0) { $failed = $true }
Write-Host ""

if ($failed) {
    Write-Host "========================================" -ForegroundColor Red
    Write-Host "✗ CI Checks Failed" -ForegroundColor Red
    Write-Host "========================================" -ForegroundColor Red
    exit 1
} else {
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "✓ All CI Checks Passed" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
}
