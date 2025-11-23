use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};

use chrono::Utc;
use clap::Parser;
use globset::{Glob, GlobSetBuilder};
use log::{info, warn};
use serde::Serialize;

use hash_folderoo::algorithms::Algorithm;
use hash_folderoo::cli::Cli;
use hash_folderoo::compare as compare_mod;
use hash_folderoo::config;
use hash_folderoo::copy;
use hash_folderoo::hash::hash_path_with_pool;
use hash_folderoo::io;
use hash_folderoo::memory::MemoryMode;
use hash_folderoo::pipeline::Pipeline;
use hash_folderoo::utils::setup_logging;

fn format_entry_path(path: &Path, strip_prefix: Option<&Path>, root: &Path) -> String {
    let logical = strip_prefix
        .and_then(|prefix| path.strip_prefix(prefix).ok())
        .or_else(|| path.strip_prefix(root).ok())
        .unwrap_or(path);
    logical
        .components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/")
}
fn print_algorithm_list() {
    println!("Available algorithms:\n");
    for alg in Algorithm::all() {
        let info = alg.create().info();
        println!(
            "- {name:<10} default_len: {len:>3} bytes  cryptographic: {crypto}  xof: {xof}",
            name = info.name,
            len = info.output_len_default,
            crypto = if info.is_cryptographic { "yes" } else { "no" },
            xof = if info.supports_xof { "yes" } else { "no" }
        );
    }
}

#[derive(Serialize)]
struct MapHeader {
    version: u8,
    generated_by: &'static str,
    timestamp: String,
    root: String,
    algorithm: AlgorithmMeta,
}

#[derive(Serialize)]
struct AlgorithmMeta {
    name: String,
    params: Option<serde_json::Value>,
}

#[derive(Clone)]
struct FileTiming {
    path: String,
    duration: Duration,
}

fn build_exclude_set(patterns: &[String]) -> anyhow::Result<Option<globset::GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        let g = Glob::new(p)?;
        builder.add(g);
    }
    Ok(Some(builder.build()?))
}

fn main() -> anyhow::Result<()> {
    setup_logging();

    let cli = Cli::parse();
    if cli.alg_list {
        print_algorithm_list();
        return Ok(());
    }

    let mut runtime_cfg = config::load_runtime_config(cli.config.as_deref())?;
    config::apply_env_overrides(&mut runtime_cfg);
    runtime_cfg.validate()?;

    match &cli.command {
        Some(hash_folderoo::cli::Commands::Hashmap(args)) => {
            let runtime_cfg = runtime_cfg.clone();
            // Note: Phase 1 CLI doesn't include all previous flags (e.g. strip-prefix, xof-length).
            // Where applicable the runtime config can still provide defaults.

            // CLI args override config
            let path = args
                .path
                .as_ref()
                .map(|p| p.clone().to_string_lossy().into_owned())
                .or_else(|| runtime_cfg.general.as_ref().and_then(|g| g.path.clone()))
                .ok_or_else(|| {
                    anyhow::anyhow!("--path is required (CLI flag or config general.path)")
                })?;

            let output = args
                .output
                .as_ref()
                .map(|p| p.as_path().to_string_lossy().into_owned())
                .or_else(|| runtime_cfg.general.as_ref().and_then(|g| g.output.clone()));

            let alg = args
                .algorithm
                .as_deref()
                .or_else(|| {
                    runtime_cfg
                        .algorithm
                        .as_ref()
                        .and_then(|a| a.name.as_deref())
                })
                .unwrap_or("blake3");

            let xof_len = args
                .xof_length
                .or_else(|| runtime_cfg.algorithm.as_ref().and_then(|a| a.xof_length));

            let strip_prefix: Option<PathBuf> = args.strip_prefix.clone().or_else(|| {
                runtime_cfg
                    .general
                    .as_ref()
                    .and_then(|g| g.strip_prefix.clone().map(PathBuf::from))
            });

            let mut excludes = runtime_cfg
                .general
                .as_ref()
                .and_then(|g| g.exclude.clone())
                .unwrap_or_default();
            if !args.exclude.is_empty() {
                excludes.extend(args.exclude.clone());
            }

            let depth = args
                .depth
                .or_else(|| runtime_cfg.general.as_ref().and_then(|g| g.depth));

            let follow_symlinks = if args.follow_symlinks {
                true
            } else {
                runtime_cfg
                    .general
                    .as_ref()
                    .and_then(|g| g.follow_symlinks)
                    .unwrap_or(false)
            };

            let show_progress = if args.progress {
                true
            } else {
                runtime_cfg
                    .general
                    .as_ref()
                    .and_then(|g| g.progress)
                    .unwrap_or(false)
            };

            let dry_run = if args.dry_run {
                true
            } else {
                runtime_cfg
                    .general
                    .as_ref()
                    .and_then(|g| g.dry_run)
                    .unwrap_or(false)
            };

            if !args.silent {
                info!("Computing hashmap for {} using alg {}", path, alg);
            }

            let alg_enum = match Algorithm::from_str(alg) {
                Some(a) => a,
                None => {
                    warn!("Unknown algorithm {}, falling back to blake3", alg);
                    Algorithm::Blake3
                }
            };

            // Probe to determine default out length
            let alg_info = alg_enum.create().info();
            if xof_len.is_some() && !alg_info.supports_xof {
                anyhow::bail!("algorithm {} does not support --xof-length", alg_info.name);
            }
            let default_out = alg_info.output_len_default;
            let out_len = xof_len.unwrap_or(default_out);

            let exclude_set = build_exclude_set(&excludes)?;

            // Determine memory mode from CLI/config (defaults to Balanced)
            let mem_mode_str = args
                .mem_mode
                .as_deref()
                .or_else(|| runtime_cfg.memory.as_ref().and_then(|m| m.mode.as_deref()))
                .unwrap_or("balanced");
            let mode = MemoryMode::from_str(mem_mode_str);

            let threads_override = args
                .threads
                .or_else(|| runtime_cfg.general.as_ref().and_then(|g| g.threads));

            let max_ram_override = args
                .max_ram
                .or_else(|| runtime_cfg.memory.as_ref().and_then(|m| m.max_ram));

            // Create pipeline with chosen memory mode
            let pipeline = Pipeline::new(mode)
                .with_threads(threads_override)
                .with_max_ram(max_ram_override);

            // Shared vector to collect results from workers
            let entries: Arc<Mutex<Vec<io::MapEntry>>> = Arc::new(Mutex::new(Vec::new()));
            let timings: Arc<Mutex<Vec<FileTiming>>> = Arc::new(Mutex::new(Vec::new()));

            // Worker closure: hash a single file and push MapEntry into shared vector
            let alg_for_worker = alg_enum;
            let entries_clone = entries.clone();
            let scan_root = PathBuf::from(&path);
            let canonical_root =
                std::fs::canonicalize(&scan_root).unwrap_or_else(|_| scan_root.clone());
            let strip_prefix_abs = strip_prefix.as_ref().map(|p| {
                let candidate = if p.is_absolute() {
                    p.clone()
                } else {
                    canonical_root.join(p)
                };
                std::fs::canonicalize(&candidate).unwrap_or(candidate)
            });

            let exclude_set_clone = exclude_set.clone();
            let out_len_inner = out_len;

            let timings_clone = timings.clone();
            let root_for_worker = canonical_root.clone();
            let strip_for_worker = strip_prefix_abs.clone();

            let worker = move |path_buf: PathBuf,
                               buffer_pool: Arc<hash_folderoo::memory::BufferPool>|
                  -> anyhow::Result<()> {
                // Apply excludes (path-based) if set; note: pipeline already walks with exclusions but double-check
                if let Some(gs) = &exclude_set_clone {
                    if gs.is_match(&path_buf) {
                        return Ok(());
                    }
                }

                // Only process files
                if !path_buf.is_file() {
                    return Ok(());
                }

                let rel =
                    format_entry_path(&path_buf, strip_for_worker.as_deref(), &root_for_worker);

                let metadata = path_buf.metadata().ok();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let mtime = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|dur| dur.as_secs() as i64);
                let mut hasher = alg_for_worker.create();
                let start = Instant::now();
                let hash = match hash_path_with_pool(hasher.as_mut(), &path_buf, &buffer_pool) {
                    Ok(()) => hasher.finalize_hex(out_len_inner),
                    Err(e) => {
                        warn!("Failed hashing {}: {}", path_buf.display(), e);
                        return Ok(());
                    }
                };
                let elapsed = start.elapsed();
                let me = io::MapEntry {
                    path: rel,
                    hash,
                    size,
                    mtime,
                };
                timings_clone.lock().unwrap().push(FileTiming {
                    path: me.path.clone(),
                    duration: elapsed,
                });
                let mut guard = entries_clone.lock().unwrap();
                guard.push(me);
                Ok(())
            };

            // Run the pipeline
            let processed = pipeline
                .run(
                    &scan_root,
                    &excludes,
                    depth,
                    follow_symlinks,
                    show_progress,
                    worker,
                )
                .map_err(|e| anyhow::anyhow!("pipeline error: {}", e))?;

            if !args.silent {
                info!("Processed {} files", processed);
            }

            let mut timings_vec = timings.lock().unwrap().clone();
            if !timings_vec.is_empty() && !args.silent {
                timings_vec.sort_by(|a, b| b.duration.cmp(&a.duration));
                info!("Top slowest files:");
                for timing in timings_vec.iter().take(5) {
                    info!("  {:>8.3?} {}", timing.duration, timing.path);
                }
            }

            // Build header + entries for output
            let algorithm_params = xof_len.map(|len| serde_json::json!({ "xof_length": len }));

            let header = MapHeader {
                version: 1,
                generated_by: "hash-folderoo",
                timestamp: Utc::now().to_rfc3339(),
                root: canonical_root.to_string_lossy().into_owned(),
                algorithm: AlgorithmMeta {
                    name: alg_info.name.clone(),
                    params: algorithm_params,
                },
            };

            let mut entries_vec = entries.lock().unwrap().clone();

            // Sort entries by path for deterministic output
            entries_vec.sort_by(|a, b| a.path.cmp(&b.path));

            // Handle output format: json (default) or csv
            let format = args
                .format
                .as_deref()
                .or_else(|| {
                    runtime_cfg
                        .general
                        .as_ref()
                        .and_then(|g| g.format.as_deref())
                })
                .unwrap_or("json")
                .to_lowercase();

            if dry_run {
                info!(
                    "Dry-run complete: hashed {} files (results not written)",
                    entries_vec.len()
                );
                return Ok(());
            }

            match (output, format.as_str()) {
                (Some(p), "json") => {
                    // create combined object
                    #[derive(Serialize)]
                    struct Out<'a> {
                        version: u8,
                        generated_by: &'static str,
                        timestamp: String,
                        root: String,
                        algorithm: &'a AlgorithmMeta,
                        entries: &'a [io::MapEntry],
                    }

                    let out = Out {
                        version: header.version,
                        generated_by: header.generated_by,
                        timestamp: header.timestamp.clone(),
                        root: header.root.clone(),
                        algorithm: &header.algorithm,
                        entries: &entries_vec,
                    };
                    io::write_json(Path::new(&p), &out).map_err(|e| anyhow::anyhow!(e))?;
                }
                (Some(p), "csv") => {
                    io::write_csv(Path::new(&p), &entries_vec).map_err(|e| anyhow::anyhow!(e))?;
                }
                (Some(p), other) => {
                    warn!("Unknown format {}, falling back to json", other);
                    #[derive(Serialize)]
                    struct Out<'a> {
                        version: u8,
                        generated_by: &'static str,
                        timestamp: String,
                        root: String,
                        algorithm: &'a AlgorithmMeta,
                        entries: &'a [io::MapEntry],
                    }
                    let out = Out {
                        version: header.version,
                        generated_by: header.generated_by,
                        timestamp: header.timestamp.clone(),
                        root: header.root.clone(),
                        algorithm: &header.algorithm,
                        entries: &entries_vec,
                    };
                    io::write_json(Path::new(&p), &out).map_err(|e| anyhow::anyhow!(e))?;
                }
                (None, "json") => {
                    let mut stdout = std::io::stdout();
                    let s = serde_json::to_vec_pretty(&serde_json::json!({
                        "version": header.version,
                        "generated_by": header.generated_by,
                        "timestamp": header.timestamp,
                        "root": header.root,
                        "algorithm": {
                            "name": header.algorithm.name,
                            "params": header.algorithm.params,
                        },
                        "entries": entries_vec,
                    }))?;
                    stdout.write_all(&s)?;
                }
                (None, "csv") => {
                    let mut wtr = csv::Writer::from_writer(std::io::stdout());
                    for rec in &entries_vec {
                        wtr.serialize(rec)?;
                    }
                    wtr.flush()?;
                }
                (None, other) => {
                    warn!("Unknown format {}, falling back to json", other);
                    let mut stdout = std::io::stdout();
                    let s = serde_json::to_vec_pretty(&serde_json::json!({
                        "version": header.version,
                        "generated_by": header.generated_by,
                        "timestamp": header.timestamp,
                        "root": header.root,
                        "algorithm": {
                            "name": header.algorithm.name,
                            "params": header.algorithm.params,
                        },
                        "entries": entries_vec,
                    }))?;
                    stdout.write_all(&s)?;
                }
            }
        }
        Some(hash_folderoo::cli::Commands::Compare(args)) => {
            let source = args
                .source
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| anyhow::anyhow!("--source is required"))?;
            let target = args
                .target
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| anyhow::anyhow!("--target is required"))?;

            let compare_alg = args
                .algorithm
                .as_deref()
                .and_then(|name| Algorithm::from_str(name))
                .unwrap_or_else(|| {
                    if let Some(name) = args.algorithm.as_deref() {
                        warn!(
                            "Unknown algorithm {} for compare; falling back to blake3",
                            name
                        );
                    }
                    Algorithm::Blake3
                });

            if !args.format.as_deref().unwrap_or("json").is_empty() {
                // noop; format will be used below
            }

            if !args.output.is_none() {
                // noop; output will be used below
            }

            let src_map = compare_mod::get_map_from_input(&source, compare_alg)
                .map_err(|e| anyhow::anyhow!(e))?;
            let tgt_map = compare_mod::get_map_from_input(&target, compare_alg)
                .map_err(|e| anyhow::anyhow!(e))?;

            let report = compare_mod::compare_maps(src_map, tgt_map);

            let format = args.format.as_deref().unwrap_or("json");
            let out_path = args.output.as_ref().map(|p| p.as_path());

            compare_mod::write_report(&report, out_path, format).map_err(|e| anyhow::anyhow!(e))?;
        }
        Some(hash_folderoo::cli::Commands::Copydiff(args)) => {
            // Load plan from file if provided, otherwise generate by running a comparison
            let copy_alg = args
                .algorithm
                .as_deref()
                .and_then(|name| Algorithm::from_str(name))
                .unwrap_or_else(|| {
                    if let Some(name) = args.algorithm.as_deref() {
                        warn!(
                            "Unknown algorithm {} for copydiff; falling back to blake3",
                            name
                        );
                    }
                    Algorithm::Blake3
                });
            let mut plan = if let Some(p) = &args.plan {
                // load JSON plan
                let f = File::open(p)
                    .map_err(|e| anyhow::anyhow!("failed opening plan {:?}: {}", p, e))?;
                let reader = BufReader::new(f);
                let plan: copy::CopyPlan = serde_json::from_reader(reader)
                    .map_err(|e| anyhow::anyhow!("failed parsing plan {:?}: {}", p, e))?;
                plan
            } else {
                // require source and target to generate comparison-based plan
                let source = args
                    .source
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .ok_or_else(|| {
                        anyhow::anyhow!("--source is required when --plan is not provided")
                    })?;
                let target = args
                    .target
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .ok_or_else(|| {
                        anyhow::anyhow!("--target is required when --plan is not provided")
                    })?;

                let src_map = compare_mod::get_map_from_input(&source, copy_alg)
                    .map_err(|e| anyhow::anyhow!(e))?;
                let tgt_map = compare_mod::get_map_from_input(&target, copy_alg)
                    .map_err(|e| anyhow::anyhow!(e))?;
                let report = compare_mod::compare_maps(src_map, tgt_map);

                // If the provided source/target are directories, pass them as roots to help construct dst paths
                let source_root = args.source.as_ref().and_then(|p| {
                    if p.exists() && p.is_dir() {
                        Some(p.as_path())
                    } else {
                        None
                    }
                });
                let target_root = args.target.as_ref().and_then(|p| {
                    if p.exists() && p.is_dir() {
                        Some(p.as_path())
                    } else {
                        None
                    }
                });

                copy::generate_copy_plan(&report, source_root, target_root)
            };

            if args.execute {
                let conflict =
                    copy::ConflictStrategy::from_str(&args.conflict).unwrap_or_else(|| {
                        warn!(
                            "Unknown conflict mode {}; defaulting to overwrite",
                            args.conflict
                        );
                        copy::ConflictStrategy::Overwrite
                    });
                let opts = copy::CopyOptions {
                    conflict,
                    preserve_times: args.preserve_times,
                };
                copy::execute_copy_plan(&mut plan, opts, None).map_err(|e| anyhow::anyhow!(e))?;
            } else {
                // default to dry-run output
                copy::dry_run_copy_plan(&plan);
            }
        }
        Some(hash_folderoo::cli::Commands::Removempty(args)) => {
            let path = args
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| anyhow::anyhow!("--path is required"))?;
            hash_folderoo::remove_empty_directories(
                std::path::Path::new(&path),
                args.dry_run,
                args.min_empty_depth,
                &args.exclude,
            )
            .map_err(|e| anyhow::anyhow!("removempty error: {}", e))?;
        }
        Some(hash_folderoo::cli::Commands::Renamer(args)) => {
            let path = args
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| anyhow::anyhow!("--path is required"))?;
            let pattern = args
                .pattern
                .as_ref()
                .map(|s| s.as_str())
                .ok_or_else(|| anyhow::anyhow!("--pattern is required"))?;
            hash_folderoo::rename_files(std::path::Path::new(&path), pattern, args.dry_run)
                .map_err(|e| anyhow::anyhow!("renamer error: {}", e))?;
        }
        Some(hash_folderoo::cli::Commands::Benchmark(args)) => {
            let alg = args.algorithm.as_deref().unwrap_or("blake3");
            // CLI `size` is in bytes; convert to MB for run_benchmark which accepts size_mb.
            let size_bytes = args.size.unwrap_or(0);
            let size_mb = if size_bytes == 0 {
                0
            } else {
                // round up to nearest MB
                (size_bytes + (1024 * 1024) - 1) / (1024 * 1024)
            };
            hash_folderoo::run_benchmark(alg, size_mb).map_err(|e| anyhow::anyhow!(e))?;
        }
        Some(hash_folderoo::cli::Commands::Report(args)) => {
            let input = args
                .input
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| anyhow::anyhow!("--input is required"))?;
            let format = args.format.as_deref().unwrap_or("text");
            let include = if args.include.is_empty() {
                vec![
                    "stats".to_string(),
                    "duplicates".to_string(),
                    "largest".to_string(),
                ]
            } else {
                args.include.clone()
            };
            let top_n = args.top_n.unwrap_or(5);
            hash_folderoo::generate_report(&input, format, &include, top_n)
                .map_err(|e| anyhow::anyhow!(e))?;
        }
        None => {
            println!("Run with --help for usage");
        }
    }

    Ok(())
}
