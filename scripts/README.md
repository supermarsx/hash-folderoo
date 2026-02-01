# Development Scripts

This directory contains scripts for running CI checks locally before pushing to GitHub.

## Available Scripts

All scripts are available in both PowerShell (`.ps1`) and Bash (`.sh`) versions.

### Individual Check Scripts

- **`format`** - Check code formatting with `rustfmt`
- **`lint`** - Run clippy linter with warnings as errors
- **`type-check`** - Run Rust type checker (`cargo check`)
- **`test`** - Run all unit and integration tests
- **`smoke`** - Run smoke tests (git-diff and resume plan tests)
- **`build`** - Build the project in release mode

### Combined Scripts

- **`ci-checks`** - Run Phase 1 checks (format, lint, type-check, test)
- **`ci-all`** - Run the complete CI pipeline (checks → smoke → build)

## Usage

### Windows (PowerShell)

```powershell
# Run individual checks
.\scripts\format.ps1
.\scripts\lint.ps1
.\scripts\type-check.ps1
.\scripts\test.ps1
.\scripts\smoke.ps1
.\scripts\build.ps1

# Run Phase 1 checks
.\scripts\ci-checks.ps1

# Run full CI pipeline
.\scripts\ci-all.ps1

# Build for specific target
.\scripts\build.ps1 -Target x86_64-pc-windows-msvc
```

### Linux/macOS (Bash)

```bash
# Make scripts executable (first time only)
chmod +x scripts/*.sh

# Run individual checks
./scripts/format.sh
./scripts/lint.sh
./scripts/type-check.sh
./scripts/test.sh
./scripts/smoke.sh
./scripts/build.sh

# Run Phase 1 checks
./scripts/ci-checks.sh

# Run full CI pipeline
./scripts/ci-all.sh

# Build for specific target
./scripts/build.sh x86_64-unknown-linux-gnu
```

## CI Pipeline Stages

The GitHub Actions CI runs these stages:

1. **Phase 1: Parallel Checks**
   - Format check
   - Lint (clippy)
   - Type check
   - Tests

2. **Phase 2: Smoke Tests** (runs after Phase 1 passes)
   - Cross-platform smoke tests on Ubuntu, Windows, and macOS

3. **Phase 3: Build** (runs after Phase 2 passes)
   - Cross-platform release builds for multiple targets

## Quick Start

To verify your changes before pushing:

```powershell
# Windows
.\scripts\ci-all.ps1

# Linux/macOS
./scripts/ci-all.sh
```

This runs the same checks that CI will run, catching issues early.
