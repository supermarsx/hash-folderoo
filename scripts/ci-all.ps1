#!/usr/bin/env pwsh
# Run all CI stages: checks, smoke tests, and build

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Magenta
Write-Host "Running Full CI Pipeline" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta
Write-Host ""

# Phase 1: CI Checks
& "$PSScriptRoot\ci-checks.ps1"
if ($LASTEXITCODE -ne 0) { exit 1 }
Write-Host ""

# Phase 2: Smoke Tests
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Running Phase 2: Smoke Tests" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
& "$PSScriptRoot\smoke.ps1"
if ($LASTEXITCODE -ne 0) { exit 1 }
Write-Host ""

# Phase 3: Build
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Running Phase 3: Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
& "$PSScriptRoot\build.ps1"
if ($LASTEXITCODE -ne 0) { exit 1 }
Write-Host ""

Write-Host "========================================" -ForegroundColor Green
Write-Host "✓ Full CI Pipeline Completed Successfully" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
