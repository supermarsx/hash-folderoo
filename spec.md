**Project Overview**

- Purpose: a desktop toolkit for analyzing and manipulating directories and files using content hashes. Tools include: creating a file hashmap, comparing folders, copying differences, removing empty folders, and renaming helpers.
- Origin: Electron-based JavaScript app. Goal: produce a Rust clone with equivalent CLI and optional native GUI.

**High Level Goals**

- Provide a fast, cross-platform (Windows/Linux/macOS) command-line toolset that reproduces the Electron app's features.
- Use idiomatic Rust: strong type safety, efficient file IO, streaming hashing, parallelism, clear error handling, and test coverage.
- Offer an optional GUI (desktop) front-end later using Tauri or a native Rust GUI (egui/iced) that mirrors the original UX.

**User-Facing Features (Commands)**

- `hashmap` - walk a directory, compute per-file hashes (configurable algorithm), write output as JSON or CSV.
  - Flags: `--path <DIR>`, `--output <FILE>`, `--format {json,csv}`, `--strip-prefix`, `--threads <N>`, `--exclude <GLOB>`, `--follow-symlinks`, `--progress`, `--dry-run`, `--alg <ALGO>`, `--xof-length <BYTES>`
  - Behavior: recursively walk, hash files with selected algorithm, produce list [{ "path": "rel/path", "hash": "hex" }]. Support writing absolute or relative paths.

- `compare` - compare two hashmaps or two directories and produce a report (unique, duplicate, changed)
  - Flags: `--left <FILE|DIR>`, `--right <FILE|DIR>`, `--output <FILE>`, `--format {json,csv}`, `--ignore-time`, `--progress`, `--alg <ALGO>`
  - Behavior: load or compute maps using same algorithm; join by hash; classify entries: identical (same hash, same path), moved (same hash different path), duplicate (same hash multiple paths), missing, changed.

- `copydiff` - copy files present in source but missing or different in destination based on hashes
  - Flags: `--src <DIR|MAP>`, `--dst <DIR>`, `--map <FILE>`, `--dry-run`, `--preserve-times`, `--threads`, `--conflict {overwrite,skip,rename}`, `--alg <ALGO>`
  - Behavior: compute differences, copy files efficiently with buffered streams, preserve metadata if requested. Provide progress and dry-run.

- `removempty` - remove empty directories
  - Flags: `--path <DIR>`, `--dry-run`, `--min-empty-depth`, `--exclude <GLOB>`
  - Behavior: bottom-up traversal, remove directories that contain no files (after excluding patterns), print or log removed directories.

- `renamer` - batch rename files based on rules or a mapping file
  - Flags: `--path <DIR>`, `--map <CSV|JSON>`, `--pattern <regex> --replace <string>`, `--dry-run`, `--preview`
  - Behavior: generate rename plan, optionally show preview, execute with safe moves and rollbacks on failure.

- `benchmark` - benchmark hashing algorithms and configurations (detailed in Benchmark Test Mode).

- `report` - generate operational reports from runs or existing maps (detailed below).

**Supported Hash Algorithms (configurable)**

- BLAKE3 XOF 1024 (extendable output) — default parity option matching original BLAKE3 usage with XOF to produce 1024-bit digests when requested.
- SHA3 family with SHAKE256 XOF — support variable-length output (e.g., 1024-bit/128 bytes by `--xof-length 128`).
- BLAKE2b-1024 (if available via crate extensions) — truncated/extended to 1024-bit output.
- TurboSHAKE256 (if available) — XOF variant of SHAKE supporting extended outputs.
- ParallelHash (ParallelHash256 XOF 1024) — if crate available, include for benchmarks/comparisons.
- BLAKE2bp — parallelized BLAKE2b variant for multi-threaded hashing.
- KangarooTwelve (K12) 1024 — extendable output variant based on Keccak.
- WyHash / MeowHash 1024 — non-cryptographic fast hashes with 1024-bit output (via repeated chaining or XOF-like expansion) for performance benchmarking (note: non-cryptographic, not suitable for security integrity guarantees).
- XXHash3 1024 — high-speed non-cryptographic hash extended to 1024 bits for comparison and performance testing.

Notes on algorithms and support:
- Not all algorithms have first-class Rust crates supporting XOF 1024 outputs; where native XOF or 1024-bit output isn't available, implement deterministic expansion (e.g., using sponge/KDF or repeated keyed hashing) with clear documentation and an opt-in flag `--force-expand` so users know they are using non-standard expansions for compatibility or benchmarking.
- Default algorithm: `blake3` (with option to request XOF output length explicitly with `--xof-length`). Maintain default behavior for parity with the original app.
- Provide an `--alg-list` runtime flag to print available algorithms and their properties (cryptographic vs non-cryptographic, stateful, XOF-supported, performance notes).

**Data Formats**

- JSON Hashmap schema (recommended):
  - Top-level object: `{ "version": "1", "generated_by": "hash-folder-rs", "timestamp": "ISO8601", "root": "relative-or-abs-root", "algorithm": { "name": "blake3", "params": { "xof_length": 128 } }, "entries": [ { "path": "relative/path", "hash": "<hex>", "size": 12345, "mtime": 1600000000 } ] }`
  - `algorithm` field encodes which algorithm and parameters (e.g., `xof_length`) were used to produce the digest.

- CSV schema (simple): header `path,hash,size,mtime,alg_name,alg_params`

- Hash format: hex lowercase; length depends on algorithm and `--xof-length`. For 1024-bit outputs, hex length is 256 chars.

**Rust Architecture & Module Layout**

- Workspace or single crate layout suggestion (binary + library):
  - `src/main.rs` - CLI entry using `clap`, dispatches subcommands
  - `src/cli.rs` - CLI parsing and help text (enum of algorithms and normalized args)
  - `src/lib.rs` - exposes library API for programmatic usage
  - `src/hash.rs` - hashing abstraction layer, trait definitions for algorithms, streaming XOF support, algorithm registry
  - `src/algorithms/` - per-algorithm implementations/adapters (e.g., `blake3.rs`, `shake256.rs`, `blake2b.rs`, `xxh3.rs`, `blake2bp.rs`, `k12.rs`, `wyhash.rs`, `meowhash.rs`, `parallelhash.rs`). Each adapter implements a common `Hasher` trait.
  - `src/walk.rs` - directory walk utilities (using `walkdir`), filtering, glob/exclude
  - `src/io.rs` - read/write JSON/CSV maps, atomic file write helpers
  - `src/compare.rs` - comparison algorithms, dedupe, diff generation
  - `src/copy.rs` - efficient file copy with concurrency and metadata preservation
  - `src/renamer.rs` - rename plan and execution
  - `src/removempty.rs` - empty-dir removal
  - `src/bench.rs` - benchmark mode and harness utilities
  - `src/report.rs` - report generation and formats
  - `src/utils.rs` - logging, progress wrappers, error types
  - `tests/` - unit and integration tests

**Hashing Abstraction (trait)**

- Define a trait to unify algorithms:

  - `pub trait HasherImpl: Send + Sync + 'static {
      fn name(&self) -> &str;
      fn is_cryptographic(&self) -> bool;
      fn supports_xof(&self) -> bool;
      fn output_len_default(&self) -> usize; // bytes
      fn new() -> Self where Self: Sized;
      fn update_reader<R: Read>(&mut self, r: &mut R) -> anyhow::Result<()>; // read all and update state
      fn finalize_hex(&self, out_len: usize) -> String; // out_len in bytes
    }

- Provide adapter types that implement this trait, with streaming reads for large files. For algorithms without native XOF support, document expansion strategy and make it opt-in.

**Algorithm Implementation Notes**

- BLAKE3 XOF: use the `blake3` crate's XOF API `Hasher::finalize_xof()` and `reader.xof_read()` to produce arbitrary-length output.
- SHAKE256 (SHA-3 XOF): use `sha3` crate with `Shake256` and `XofReader` to produce requested length.
- BLAKE2b-1024: `blake2` crate supports variable output lengths for BLAKE2b? If not, use `blake2b_simd` or a wrapper. If 1024 not natively supported, consider chained keyed hashes or domain separation.
- TurboSHAKE256: if a crate exists (e.g., `turboshake`), use it. If not, provide `--opt-not-available` warning.
- ParallelHash256 XOF: requires specialized implementation; include if a crate is available.
- BLAKE2bp: use `blake2` crate parallel variant or `blake2b_simd::blake2bp` where available.
- KangarooTwelve: use `kangarootwelve` crate providing K12 XOF.
- WyHash / MeowHash / XXHash3 1024: these are non-cryptographic and may not provide XOF natively. Implement deterministic expansion via keyed streaming (e.g., use keyed XXH3 64-bit outputs repeated with counter and HMAC-like construction) — document that these are for benchmarking only and not suitable for integrity security.

**Deterministic Expansion Behavior (Implementation Details)**

For algorithms that do not natively support XOF (extendable output functions) or arbitrary-length outputs, hash-folderoo implements deterministic expansion to enable uniform output lengths across all algorithms for comparison and benchmarking purposes.

**Expansion Strategy:**

1. **Native XOF Algorithms (BLAKE3, SHAKE256, K12, TurboSHAKE, ParallelHash):**
   - Use the algorithm's native XOF capability to produce arbitrary-length outputs.
   - No expansion needed; outputs are cryptographically secure and standardized.

2. **Fixed-Output Algorithms (BLAKE2b, BLAKE2bp, XXH3, WyHash):**
   - When output length requested exceeds native digest size, apply deterministic chaining.
   - **BLAKE2b/BLAKE2bp Expansion Algorithm:**
     - Compute native digest `D0` of input
     - For each required chunk `i` (where `i = 0, 1, 2, ...`):
       - Compute `Di = Hash(D0 || counter_i)` where `counter_i` is `i` as 4-byte little-endian
       - Concatenate chunks until desired output length reached
       - Truncate final output to exact requested length
     - This construction is deterministic and collision-resistant (dependent on underlying hash properties)
   - **XXH3 Expansion Algorithm:**
     - Compute native 64-bit digest `seed` of input
     - For each required 8-byte chunk at index `i`:
       - Compute `chunk_i = XXH3_64(seed || counter_i, tweak_i)` 
       - Where `counter_i` is `i` as 8-byte LE, `tweak_i = seed + i * 0x9E37_79B1_85EB_CA87`
       - Concatenate chunks, truncate to requested length
   - **WyHash Expansion Algorithm:**
     - Compute native 64-bit digest `seed` of input
     - For each required 8-byte chunk at index `i`:
       - Compute `chunk_i = WyHash(counter_i, seed_i)` where `seed_i = seed + i * 0xA076_1D64_78BD_642F`
       - Concatenate chunks, truncate to requested length

**Guarantees:**
- **Deterministic:** Same input and output length always produce identical digest
- **Reproducible:** Implementation-defined expansion is documented and tested with reference vectors
- **Non-standard:** Expanded outputs for fixed-length algorithms are NOT standardized and should NOT be used for cryptographic verification or interoperability with other tools
- **Performance vs Security Trade-off:** Non-cryptographic expansions (XXH3, WyHash) are suitable ONLY for performance benchmarking, NOT for integrity checking

**Testing & Validation:**
- Unit tests in `src/algorithms/tests.rs` include reference vectors for:
  - BLAKE2b expansion (80, 128, 160 bytes)
  - SHAKE256 outputs (32, 64, 128 bytes)
  - All other algorithms with smoke tests
- Integration tests in `tests/expand_vectors.rs` verify deterministic behavior across multiple runs
- Reference vectors are computed from the actual implementation and verified for consistency

**Usage Notes:**
- XOF-capable algorithms should be preferred for variable-length outputs
- Fixed-output algorithm expansions are provided for benchmarking comparisons only
- Algorithm metadata (`supports_xof`) indicates native XOF support vs. deterministic expansion
- Users can query algorithm properties via `--alg-list` CLI flag

**RAM Guardrails & Memory Modes**

- Provide runtime memory guardrails and explicit memory modes to control memory footprint and performance trade-offs.

- Global flags and config keys:
  - `--max-ram <BYTES>` or config `max_ram` — upper bound on total memory (approx) the process should use; tool tries to enforce by limiting buffer pool sizes, threads, and preallocations.
  - `--memory-mode {stream,balanced,booster}` or config `memory.mode` — pre-defined operating modes:
    - `stream` (low-memory): minimize memory use. Use small per-thread buffers (e.g., 64KB–1MB), single-file streaming per thread, minimal preallocation, favor IO-bound throughput over CPU parallelism. Suitable for systems with low RAM.
    - `balanced` (default): balance memory and throughput. Moderate buffers (e.g., 4–16 MiB) and thread count set to `min(num_cpus, max_threads_by_ram)`.
    - `booster` (in-memory booster): preallocate large buffer pools and possibly read multiple files fully into memory before hashing to maximize CPU utilization and hashing throughput; useful on high-memory systems for best performance.

- Memory accounting strategy:
  - Reserve an estimated budget for each thread: `per_thread_budget = clamp(max_ram / threads, min_buf, max_buf)`.
  - Buffer pool: allocate a shared, bounded buffer pool providing `N` buffers of `buffer_size` each; control total memory via `N * buffer_size`.
  - If preallocation fails due to OS limits, gracefully fall back to `balanced` or `stream` mode and log a warning.

- Thread/worker adjustments:
  - Dynamically cap worker threads based on `max_ram` and `per_thread_budget` to avoid oversubscription.
  - If `--memory-mode booster` is active, and `max_ram` is not explicitly set, adopt conservative auto-detection: `auto_max_ram = total_system_ram * 0.7`.

**Memory Mode Implementation Guarantees (Status: IMPLEMENTED)**

The following guarantees are provided by the current implementation in `src/memory.rs` and `src/pipeline.rs`:

1. **Buffer Pool Accounting:**
   - `BufferPool` tracks allocated buffers via `AtomicUsize` counter
   - Pool enforces soft maximum on buffer count; allocation attempts wait briefly before creating new buffers
   - Buffers are returned to pool on drop via RAII `PooledBuffer` wrapper
   - Prevents unbounded memory growth during high-concurrency operations

2. **Memory Budget Enforcement:**
   - `recommend_config()` function computes thread count, buffer size, and buffer count based on mode and `max_ram`
   - If computed total exceeds `max_ram`, scales down buffer count proportionally
   - Thread count reduced if fewer buffers available than threads
   - Logs warnings when scaling occurs

3. **Mode-Specific Behavior:**
   - **Stream Mode:**
     - Buffer size: 64 KB
     - Threads: `cpus / 2` (reduced parallelism for lower memory pressure)
     - Buffers per thread: 2
     - Directory listing: streaming (no prefetch) to minimize peak memory
   - **Balanced Mode (default):**
     - Buffer size: 256 KB
     - Threads: `cpus` (full CPU utilization)
     - Buffers per thread: 4
     - Directory listing: prefetched for better scheduling
   - **Booster Mode:**
     - Buffer size: 1 MB
     - Threads: `cpus * 2` (over-subscription for I/O hiding)
     - Buffers per thread: 6
     - Directory listing: prefetched
     - Auto RAM detection: uses 70% of system RAM if `max_ram` not specified

4. **Soft Backpressure:**
   - Pipeline workers check `pool.allocated_buffers() > pool.max_buffers()` before processing files
   - When exceeded, worker sleeps 5ms to allow buffer returns before proceeding
   - Prevents runaway allocation under pathological workloads

5. **Graceful Degradation:**
   - If BufferPool exhausted, logs warning and allocates beyond budget rather than blocking
   - Prevents deadlocks while maintaining observability
   - Allocated counter tracks all allocations for monitoring

6. **Testing:**
   - Unit tests in `src/memory.rs::tests` verify:
     - `recommend_config()` respects `max_ram` parameter
     - BufferPool basic get/put cycle functionality
     - Configuration computation for all three modes
   - Integration test in `tests/memory_integration.rs` verifies low-memory scaling

**Stream Modes (low-memory)**

- Streaming pipeline:
  - Use small, fixed-size buffers with a streaming hasher (XOF reader where supported) per file.
  - Each worker reads `chunk_size` bytes from disk, updates its hasher, writes nothing to memory beyond the buffer.
  - No file contents are retained between reads; write results immediately to disk or output channel.
  - Use OS-level buffered IO to avoid extra copies; prefer `read_to_end` avoidance.

- Trade-offs: streaming reduces RAM but may limit hashing throughput due to more frequent syscalls and smaller IO bursts. Use asynchronous IO or `sendfile` where available to improve throughput.

**In-Memory Booster Mode**

- Purpose: maximize throughput when RAM and CPU are abundant.

- Behavior:
  - Pre-allocate a pool of buffers sized for target chunk size or whole-file reads up to a configured per-file limit.
  - Optionally stage a batch of files into memory concurrently (bounded by `max_ram`) and feed them to CPU-bound hashing threads to reduce IO latency.
  - Use NUMA-aware allocations where possible on multi-socket systems (optional advanced optimization).

- Safety and fallback:
  - Always validate that the preallocation succeeded; if not, drop to `balanced` or `stream` mode.
  - Provide `--booster-max-file-size` config to avoid loading very large files into memory.
  - Provide `--booster-batch-size` to control how many files are staged at once.

- Risk: higher memory pressure increases swap activity if `max_ram` incorrectly set; document this clearly and default to conservative values.

**Config Files: TOML, YAML, JSON**

- Support configuration via files in three formats: `config.toml`, `config.yaml`/`config.yml`, and `config.json`.
- Config precedence (highest to lowest): CLI flags > environment variables > project-level config (cwd) > user config (home) > system config (/etc).
- Sample config keys (TOML example):

```
[general]
path = "/data"
output = "map.json"
format = "json"
threads = 8

[algorithm]
name = "blake3"
xof_length = 128

[memory]
mode = "balanced"
max_ram = 8589934592 # bytes (8 GiB)
booster_max_file_size = 1073741824 # 1 GiB

[bench]
files = 1000
repeat = 3

[report]
format = "json"
include = ["duplicates","largest_files","slowest_files"]
```

- Implement a small config loader that: reads any of the supported file types, validates keys and types, merges with CLI flags, and returns a final runtime config struct.
- Use `serde` + `serde_yaml` + `toml` crates for parsing and `config` crates where helpful. Provide helpful validation errors when keys are missing or have wrong types.

**Report Mode**

- Purpose: produce a human- and machine-readable summary of runs or maps to help auditing, forensics and performance analysis.

- `report` command/flag modes:
  - `hash-folder report --from-map map.json --output report.json --format json --include duplicates,stats,slowest,largest --top-n 50`
  - `report` can also read multiple maps to summarize differences across snapshots.

- Report content options:
  - `stats`: total files, total bytes, unique hashes count, duplication ratio, average file size, median file size.
  - `duplicates`: groups of files sharing the same hash (show counts and top groups by total bytes wasted by duplicates).
  - `largest_files`: top N largest files with path, size, mtime.
  - `slowest_files`: top N files that took the longest to hash during the run (requires logging timings during map creation).
  - `histograms`: file size distribution and per-file hashing time distribution.
  - `resource_usage`: peak memory usage, CPU seconds, IO bytes read (requires runtime measurement instrumentation).
  - `algorithm`: algorithm used, parameters (e.g., xof_length), and deterministic expansion notes if applicable.

- Output formats: `json`, `yaml`, `csv`, `html` (simple report with embedded charts via static assets) — HTML optional behind feature flag.

- Example report JSON schema (excerpt):

```
{
  "version": "1",
  "timestamp": "...",
  "map_source": "map.json",
  "stats": { "total_files": 1234, "total_bytes": 1234567890, "unique_hashes": 1200 },
  "duplicates": [ { "hash": "...", "count": 3, "paths": ["a","b","c"], "wasted_bytes": 12345 } ],
  "largest_files": [ { "path": "...", "size": 1073741824 } ]
}
```

**Extended Mechanism Description (Hashing Pipeline)**

- Overview pipeline:
  1. Configuration & args parsing (merge CLI + env + config files).
  2. Directory walk: produce candidate file list, apply `--exclude` glob filters; optionally sample for benchmark mode.
  3. Scheduling: split candidate list into work queues and batches based on `memory.mode`, `threads`, `buffer_pool` and `booster` settings.
  4. Buffer management: borrow buffers from the buffer pool (or allocate on demand in stream mode).
  5. Worker threads: each worker reads from disk into its buffer(s), feeds the data to a streaming hasher adapter (XOF readers where available), and finalizes digest when file done.
  6. Output: write file entry to output channel (atomic writer), progress tracker, and optional per-file timing logs.
  7. Post-processing: apply path-stripping, write final map file, compute report if requested.

- Memory accounting and enforcement:
  - Track: allocated buffer count * buffer size + per-file staging memory + internal per-algorithm allocations.
  - Before spawning workers, compute expected worst-case memory. If > `max_ram`, reduce threads or buffer sizes to meet the target.
  - Implement a soft-guard that logs and throttles when memory usage approaches `max_ram`, and a hard-guard that refuses to start if a safe minimum cannot be guaranteed.

- I/O and hashing concurrency patterns:
  - Prefer a producer-consumer model: one or more IO reader threads produce file data into buffers; a pool of worker threads consume buffers to hash.
  - Optionally colocate IO+hashing in the same thread for simpler designs (works well with `rayon::par_iter`), but producer-consumer improves throughput on systems where IO and CPU are balancing bottlenecks.

**Benchmark Test Mode (updated)**

- The `benchmark` subcommand also respects `memory.mode`, `max_ram`, `booster` and `stream` modes to allow reproducible trade-off tests.
- Bench harness must record: elapsed time per file, aggregated throughput, peak memory, CPU seconds, and optionally IO stats.
- Provide a `--bench-report` to save a structured results file and `--bench-compare` to compare with baselines.

**Testing & Benchmarks**

- Unit tests for each adapter using known test vectors where available. For XOFs, include reference outputs for short lengths and verify extension correctness.
- Integration tests that run the `hashmap` command with a selection of algorithms to verify output sizes and deterministic operation.
- Benchmark harness using `criterion` or custom timing harness under `benches/` plus `bench.rs` subcommand for reproducible runs. Save results in `benches/results/`.

**CLI UX updates**

- `--alg <ALGO>` accepts algorithm names (case-insensitive). Examples: `blake3`, `blake3:xof`, `shake256`, `shake256:xof`, `blake2b-1024`, `blake2bp`, `k12-1024`, `xxh3-1024`, `wyhash-1024`, `meowhash-1024`, `parallelhash-1024`, `turboshake256`.
- `--xof-length` allows specifying output bytes for XOF-capable algorithms. Validate against algorithm capability.
- `--alg-list` prints supported algorithms, default output lengths, and notes (cryptographic vs non-cryptographic).
- Configuration file support: `--config <FILE>` to load and merge a config file (TOML/YAML/JSON) using documented precedence.

**Logging, Instrumentation & Resource Measurement**

- Provide runtime instrumentation points to measure per-file hashing time, bytes read, and per-algorithm memory allocations when possible.
- Expose optional OS-level measurements (peak RSS, CPU seconds) using platform-specific crates behind features.
- Structured logs (JSON) for CI and forensic use via `--log-format {text,json}`.

**Security & Legal Notes**

- For cryptographic hashes, document that only cryptographic algorithms should be used for integrity use-cases (e.g., BLAKE3, SHAKE256, K12), and non-cryptographic hashes are for performance measurement only.
- Verify licenses of any third-party crates (e.g., meowhash implementations might have non-permissive licenses); gate inclusion behind optional features and document licensing in `README`.

**Backward Compatibility & Interop**

- Store algorithm metadata in output files so different tools can detect mismatched algorithms or digest lengths early.
- Offer `--compat` mode to accept non-standard expansions that mimic other implementations.

**CI / Release**

- Expand CI to run benchmark smoke tests on selected runners to detect regressions in speed or memory (optional, run on schedule not every PR).
- When building release binaries, ensure all optional algorithm crates are gated behind features so packaging can exclude heavy or platform-specific dependencies.

**Next Steps / Implementation Plan (updated)**

1. Add `src/hash.rs` abstraction and `src/algorithms/blake3.rs` with XOF support.
2. Implement CLI arg parsing for `--alg`, `--xof-length`, `--memory-mode`, `--max-ram`, and `--config` and include algorithm registry.
3. Implement buffer-pool, memory accounting, and modes (`stream`, `balanced`, `booster`) with graceful fallbacks.
4. Implement `benchmark` and `report` subcommands and the `bench` harness storing results.
5. Add additional algorithm adapters behind optional features progressively (`blake2b`, `blake2bp`, `k12`, `turboshake`, `parallelhash`, `wyhash`, `meowhash`, `xxh3`) and tests for each.
6. Document config schema for TOML/YAML/JSON and produce example config files in `examples/`.
7. Add CI jobs for unit tests, integration tests, and scheduled benchmark regression jobs.

**Appendix — Example Config Files**

- `config.toml` example:

```
[general]
path = "/data"
output = "map.json"
format = "json"
threads = 8

[algorithm]
name = "blake3"
xof_length = 128

[memory]
mode = "booster"
max_ram = 17179869184 # 16 GiB
booster_max_file_size = 536870912 # 512 MiB
booster_batch_size = 8

[report]
format = "json"
include = ["duplicates","stats","slowest_files"]
```

- `config.yaml` and `config.json` follow the same key structure.

**Deliverables for the enhanced Rust MVP**

- Config-loading support (TOML/YAML/JSON) with documented precedence.
- Memory guardrails and three memory modes with enforced budgets and graceful fallbacks.
- Streaming low-memory mode and in-memory booster mode with safe defaults.
- `benchmark` and `report` subcommands storing and comparing results.
- Adapter-based hashing layer with BLAKE3 and SHAKE256 (others behind features).
- Tests and examples demonstrating memory-mode effects and reporting.


