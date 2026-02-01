#!/usr/bin/env pwsh
# Build the project in release mode

param(
    [string]$Target = ""
)

Write-Host "Building project..." -ForegroundColor Cyan

if ($Target) {
    Write-Host "Building for target: $Target" -ForegroundColor Yellow
    cargo build --release --target $Target
} else {
    cargo build --release
}

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Build completed successfully" -ForegroundColor Green
} else {
    Write-Host "✗ Build failed" -ForegroundColor Red
    exit 1
}
