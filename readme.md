# hash-folderoo

Rust toolkit for hashing folders, comparing snapshots, and automating clean-up tasks.  
This crate rebuilds the feature set as an idiomatic, multi-command CLI with strong typing, streaming IO, and cross-platform support. The long-term design and extra context live in `spec.md`.

## Highlights

- Fast hashing pipeline backed by Rayon workers, reusable buffer pools, and selectable memory modes (`stream`, `balanced`, `booster`).
- Multiple commands in one binary: build hash maps, diff two states, copy only what changed, delete empty folders, batch-rename files, generate reports, and benchmark algorithms.
- Built-in map formats (JSON + CSV) with deterministic ordering and metadata headers (root, timestamp, algorithm, etc.).
- Config layering: system -> user -> project -> env vars -> CLI flags, with support for TOML/YAML/JSON configuration files and `HASH_FOLDEROO_*` overrides.
- Batteries-included developer experience: `cargo test` runs unit + integration smoke tests (see `tests/cli_smoke.rs`).

## Commands at a glance

| Command | Purpose | Handy flags |
| --- | --- | --- |
| `hashmap` | Walk a directory, hash every file, and write a map in JSON or CSV. | `--path`, `--output`, `--format`, `--algorithm`, `--xof-length`, `--strip-prefix`, `--exclude`, `--threads`, `--mem-mode`, `--max-ram`, `--progress`, `--dry-run` |
| `compare` | Compare two maps or directories and classify files as identical, changed, moved, missing, or new. | `--source`, `--target`, `--format {json,csv}`, `--algorithm` |
| `copydiff` | Generate and optionally execute copy ops derived from a comparison. | `--source`, `--target`, `--plan <json>`, `--execute`, `--dry-run`, `--conflict {overwrite,skip,rename}`, `--preserve-times`, `--algorithm` |
| `removempty` | Delete now-empty directories (post-order) with glob exclusions. | `--path`, `--dry-run`, `--min-empty-depth`, `--exclude` |
| `renamer` | Apply a simple `old->new` replacement across filenames. | `--path`, `--pattern`, `--dry-run` |
| `report` | Summarize a hash map (stats, duplicates, largest files, etc.). | `--input`, `--format {json,text}`, `--include`, `--top-n` |
| `benchmark` | Benchmark supported algorithms over an in-memory buffer. | `--algorithm {blake3,shake256,all}`, `--size <bytes>` |

Run `cargo run -- --help` for global options and `cargo run -- <command> --help` for per-command flags. Use `--alg-list` to print the currently compiled hashing algorithms (BLAKE3, BLAKE2b, BLAKE2bp, SHAKE256, TurboSHAKE256, ParallelHash256, XXH3-1024, WyHash-1024, KangarooTwelve).

`xxh3-1024` and `wyhash-1024` are non-cryptographic options that expand fast hashes into 1024-bit digests via deterministic counters, suitable for high-speed comparisons/benchmarks instead of integrity/security guarantees.

## Installation

### Prerequisites

- Rust 1.75+ (edition 2021) and Cargo.
- Windows, macOS, or Linux. The code avoids platform-specific assumptions.

### Build or install locally

```bash
# Build release binaries in target/release/hash-folderoo
cargo build --release

# Install into ~/.cargo/bin for easy access
cargo install --path .
```

## Quick start workflows

### 1. Hash a directory

```bash
cargo run -- hashmap \
  --path ./sample-data \
  --output snapshots/sample.json \
  --format json \
  --algorithm blake3 \
  --xof-length 64 \
  --exclude "**/.git/**" \
  --progress \
  --mem-mode balanced
```

- Paths recorded in the map are relative unless `--strip-prefix` is used.
- Streams files through a bounded buffer pool so even booster mode stays bounded by `--max-ram`.

### 2. Compare two snapshots (or live folders)

```bash
cargo run -- compare \
  --source snapshots/sample.json \
  --target ./backup-drive \
  --format json
```

- Both `--source` and `--target` accept JSON/CSV maps **or** directories. When directories are given, the tool hashes them on the fly with the chosen algorithm.
- Output as JSON (structured `ComparisonReport`) or CSV (flattened rows with status columns).

### 3. Copy only what changed

```bash
# Preview (default)
cargo run -- copydiff --source ./src --target ./dst --algorithm blake3

# Execute plan with conflict handling
cargo run -- copydiff \
  --source ./src \
  --target ./dst \
  --execute \
  --conflict rename \
  --preserve-times
```

- Without `--execute` the plan is printed (dry-run). Add `--execute` to copy files.
- `--plan <file>` lets you feed an existing JSON plan (matching the `CopyPlan` schema) instead of computing a diff.

### 4. Clean up empty directories

```bash
cargo run -- removempty --path ./tmp --min-empty-depth 2 --dry-run
```

Glob exclusions are relative to the provided root (e.g., `--exclude "**/node_modules/**"`).

### 5. Batch rename files

```bash
cargo run -- renamer --path ./photos --pattern "-draft->" --dry-run
```

Patterns follow `old->new`; omitting `->` means "replace with nothing".

### 6. Generate a report

```bash
cargo run -- report \
  --input snapshots/sample.json \
  --format json \
  --include stats,duplicates,largest \
  --top-n 10
```

Reports compute totals, duplicate groups, wasted bytes, top extensions, and largest files. Text format prints a human summary; JSON is structured for automation.

### 7. Benchmark hashing throughput

```bash
cargo run -- benchmark --algorithm all --size 134217728   # 128 MiB buffer
```

Use this to gauge algorithm speed on your hardware.

### 8. Discover algorithms at runtime

```bash
cargo run -- --alg-list
```

Outputs default digest lengths, whether the algorithm is cryptographic, and XOF capabilities.

## Configuration & environment

`hash-folderoo` merges configuration from several locations (lowest to highest precedence):

1. `/etc/hash-folderoo/{config.toml,config.yaml,config.json}`
2. `${XDG_CONFIG_HOME:-~/.config}/hash-folderoo/*`
3. `./config.{toml,yaml,json}` in the current working directory
4. `HASH_FOLDEROO_CONFIG=/path/to/config.{toml,yaml,json}`
5. Environment variables (`HASH_FOLDEROO_*`)
6. CLI flags

Each file can be TOML, YAML, or JSON. Unsupported keys are ignored.

### Example `config.toml`

```toml
[general]
path = "D:/datasets"
output = "snapshots/datasets.json"
format = "json"
threads = 8
exclude = ["**/.git/**", "target/**"]
follow_symlinks = false
progress = true

[algorithm]
name = "shake256"
xof_length = 64

[memory]
mode = "stream"      # stream | balanced | booster
max_ram = 2147483648 # 2 GiB
```

### Configuration keys

| Section | Keys | Notes |
| --- | --- | --- |
| `[general]` | `path` (string), `output` (string), `format` (`json` or `csv`), `threads` (u32 > 0), `strip_prefix` (string), `depth` (u32 > 0), `exclude` (array of globs), `follow_symlinks` (bool), `progress` (bool), `dry_run` (bool) | Matches CLI flags for `hashmap`; invalid formats or zero-valued counts are rejected during config validation. |
| `[algorithm]` | `name` (string), `xof_length` (bytes > 0) | `name` must map to a supported algorithm (`blake3`, `blake2b`, `blake2bp`, `shake256`, `turboshake256`, `k12`, …). |
| `[memory]` | `mode` (`stream`, `balanced`, or `booster`), `max_ram` (bytes > 0) | Controls the buffer-plan recommender; invalid modes result in a startup error. |

Configs loaded from `/etc`, `$XDG_CONFIG_HOME`, the project directory, env overrides, and `--config` all go through the same validator so mistakes are caught early.

### Supported environment variables

| Variable | Meaning |
| --- | --- |
| `HASH_FOLDEROO_CONFIG` | Absolute path to a config file to merge last. |
| `HASH_FOLDEROO_PATH`, `HASH_FOLDEROO_OUTPUT`, `HASH_FOLDEROO_FORMAT` | Override the corresponding general settings. |
| `HASH_FOLDEROO_THREADS`, `HASH_FOLDEROO_DEPTH`, `HASH_FOLDEROO_STRIP_PREFIX` | Numeric overrides for CLI-style options. |
| `HASH_FOLDEROO_EXCLUDE` | Comma-separated glob list (e.g., `target/**,**/.git/**`). |
| `HASH_FOLDEROO_FOLLOW_SYMLINKS`, `HASH_FOLDEROO_PROGRESS`, `HASH_FOLDEROO_DRY_RUN` | Boolean toggles (`true/false`, `1/0`, `on/off`). |
| `HASH_FOLDEROO_ALG`, `HASH_FOLDEROO_XOF_LENGTH` | Select hashing backend and output length (bytes). |
| `HASH_FOLDEROO_MEMORY_MODE`, `HASH_FOLDEROO_MAX_RAM` | Tune memory mode and total buffer budget (bytes). |

## Map and report formats

Hash maps written by the `hashmap` command follow this JSON shape:

```jsonc
{
  "version": 1,
  "generated_by": "hash-folderoo",
  "timestamp": "2025-11-23T19:19:42Z",
  "root": "/absolute/or/relative/root",
  "algorithm": {
    "name": "blake3",
    "params": { "xof_length": 64 }
  },
  "entries": [
    { "path": "foo/bar.txt", "hash": "<hex>", "size": 12345, "mtime": 1700000000 },
    { "path": "baz.bin", "hash": "<hex>", "size": 42 }
  ]
}
```

CSV output contains the same fields (`path,hash,size,mtime`) and is always sorted by path for deterministic diffs.

`compare` JSON output matches `compare::ComparisonReport` with arrays `identical`, `changed`, `moved`, `missing`, and `new`. CSV output flattens each row with a `status` column so it can be consumed by spreadsheets.

`copydiff` plans are serialized as:

```json
{ "ops": [ { "src": "/src/file.txt", "dst": "/dst/file.txt", "op": "copy" } ] }
```

`report` JSON output bundles the requested sections (`stats`, `duplicates`, `largest`) and is safe to post-process.

## Performance & memory modes

`memory.rs` encapsulates the heuristics used by the hashing pipeline:

- **stream** - smallest buffers (~64 KiB), half of logical CPUs, minimal RAM footprint for slow disks or low-memory machines.
- **balanced** (default) - moderates between throughput and memory: full logical CPUs, ~256 KiB buffers, glob prefetch disabled when RAM is tight.
- **booster** - aggressive parallelism (up to 2x logical CPUs) with 1 MiB buffers and directory prefetching; ideal for SSDs and generous RAM. Specify `--max-ram` to keep it in check.

Use `--threads` and `--max-ram` to override the auto plan. The buffer pool enforces the byte budget so multiple commands can run concurrently without starving the system.

## Development

- Format/lint: `cargo fmt` and `cargo clippy --all-targets`.
- Tests: `cargo test` covers units plus `tests/cli_smoke.rs`, which exercises the CLI end-to-end (hashing, comparing, copydiff, removempty, renamer, report, benchmark).
- Logging is powered by `env_logger`; set `RUST_LOG=debug` for verbose traces while hacking.
- See `spec.md` for the long-term blueprint (extra algorithms, GUI front-ends, richer copy planners, etc.).

## Roadmap

The current binary ships with BLAKE3, BLAKE2b, BLAKE2bp, SHAKE256, TurboSHAKE256, ParallelHash256, XXH3-1024, WyHash-1024, and KangarooTwelve hashing backends plus the core CLI workflow. The design document (`spec.md`) covers upcoming work such as additional algorithms (MeowHash, etc.), richer booster-mode controls, persisted copy plans, and a GUI front-end. Contributions aligning with that plan are welcome - open an issue to discuss larger changes.

## License

No license has been declared yet. Please add an explicit license (e.g., MIT/Apache-2.0) before distributing binaries outside this repository.
