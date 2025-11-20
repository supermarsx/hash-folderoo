use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use clap::Parser;
use log::{info, warn};
use serde::Serialize;
use walkdir::WalkDir;
use globset::{Glob, GlobSetBuilder};
use chrono::Utc;

use hash_folderoo::algorithms::Algorithm;
use hash_folderoo::HasherImpl;
use hash_folderoo::RuntimeConfig;
use hash_folderoo::cli::Cli;
use hash_folderoo::utils::setup_logging;
use hash_folderoo::pipeline::Pipeline;
use hash_folderoo::memory::{MemoryMode, recommend_config};
use hash_folderoo::io;
use hash_folderoo::compare as compare_mod;
use hash_folderoo::copy;

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

#[derive(Serialize, Clone)]
struct MapEntry {
    path: String,
    hash: String,
    size: u64,
}

fn hash_file(hasher: &mut dyn HasherImpl, path: &Path, out_len: usize) -> anyhow::Result<String> {
    let f = File::open(path)?;
    let mut reader = BufReader::new(f);
    hasher.update_reader(&mut reader)?;
    Ok(hasher.finalize_hex(out_len))
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

    match &cli.command {
        Some(hash_folderoo::cli::Commands::Hashmap(args)) => {
            // Start with empty runtime config; if --config provided we will load and merge
            let mut runtime_cfg = RuntimeConfig::default();
            // Note: Phase 1 CLI doesn't include all previous flags (e.g. strip-prefix, xof-length).
            // Where applicable the runtime config can still provide defaults.

            // CLI args override config
            let path = match &args.path {
                Some(p) => p.clone().to_string_lossy().into_owned(),
                None => runtime_cfg
                    .general
                    .as_ref()
                    .and_then(|g| g.path.clone())
                    .expect("path is required either via CLI or config"),
            };

            let output = args
                .output
                .as_ref()
                .map(|p| p.as_path().to_string_lossy().into_owned())
                .or_else(|| runtime_cfg.general.as_ref().and_then(|g| g.output.clone()));

            let alg = args
                .algorithm
                .as_deref()
                .or_else(|| runtime_cfg.algorithm.as_ref().and_then(|a| a.name.as_deref()))
                .unwrap_or("blake3");

            let xof_len = runtime_cfg.algorithm.as_ref().and_then(|a| a.xof_length);

            // strip-prefix not exposed in Phase 1 CLI; leave as None
            let strip_prefix: Option<String> = None;

            let excludes: Vec<String> = if !args.exclude.is_empty() {
                args.exclude.clone()
            } else {
                runtime_cfg
                    .general
                    .as_ref()
                    .and_then(|g| None)
                    .unwrap_or_default()
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
            let mut probe = alg_enum.create();
            let default_out = probe.info().output_len_default;
            let out_len = xof_len.unwrap_or(default_out);

            let exclude_set = build_exclude_set(&excludes)?;

            // Determine memory mode from CLI (defaults to Balanced)
            let mem_mode_str = args.mem_mode.as_deref().unwrap_or("balanced");
            let mode = MemoryMode::from_str(mem_mode_str);

            // Create pipeline with chosen memory mode
            let pipeline = Pipeline::new(mode);

            // Shared vector to collect results from workers
            let entries: Arc<Mutex<Vec<MapEntry>>> = Arc::new(Mutex::new(Vec::new()));

            // Worker closure: hash a single file and push MapEntry into shared vector
            let alg_for_worker = alg_enum;
            let entries_clone = entries.clone();
            let strip_prefix_clone = strip_prefix.clone();
            let exclude_set_clone = exclude_set.clone();
            let out_len_inner = out_len;

            let worker = move |path_buf: PathBuf, _pool: Arc<hash_folderoo::memory::BufferPool>| -> anyhow::Result<()> {
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

                let rel = if let Some(strip) = &strip_prefix_clone {
                    match path_buf.strip_prefix(strip) {
                        Ok(s) => s.to_string_lossy().into_owned(),
                        Err(_) => path_buf.to_string_lossy().into_owned(),
                    }
                } else {
                    path_buf.to_string_lossy().into_owned()
                };

                let size = path_buf.metadata().map(|m| m.len()).unwrap_or(0);
                let mut hasher = alg_for_worker.create();
                match hash_file(hasher.as_mut(), &path_buf, out_len_inner) {
                    Ok(h) => {
                        let me = MapEntry {
                            path: rel,
                            hash: h,
                            size,
                        };
                        let mut guard = entries_clone.lock().unwrap();
                        guard.push(me);
                    }
                    Err(e) => {
                        warn!("Failed hashing {}: {}", path_buf.display(), e);
                    }
                }
                Ok(())
            };

            // Run the pipeline
            let processed = pipeline
                .run(&path, &excludes, worker)
                .map_err(|e| anyhow::anyhow!("pipeline error: {}", e))?;

            if !args.silent {
                info!("Processed {} files", processed);
            }

            // Build header + entries for output
            let header = MapHeader {
                version: 1,
                generated_by: "hash-folderoo",
                timestamp: Utc::now().to_rfc3339(),
                root: path.clone(),
                algorithm: AlgorithmMeta {
                    name: probe.info().name,
                    params: None,
                },
            };

            let mut entries_vec = entries.lock().unwrap().clone();

            // Sort entries by path for deterministic output
            entries_vec.sort_by(|a, b| a.path.cmp(&b.path));

            // Handle output format: json (default) or csv
            let format = args.format.as_deref().unwrap_or("json").to_lowercase();

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
                        entries: &'a [MapEntry],
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
                        entries: &'a [MapEntry],
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
                    let mut s = serde_json::to_vec_pretty(&serde_json::json!({
                        "version": header.version,
                        "generated_by": header.generated_by,
                        "timestamp": header.timestamp,
                        "root": header.root,
                        "algorithm": { "name": header.algorithm.name },
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
                    let mut s = serde_json::to_vec_pretty(&serde_json::json!({
                        "version": header.version,
                        "generated_by": header.generated_by,
                        "timestamp": header.timestamp,
                        "root": header.root,
                        "algorithm": { "name": header.algorithm.name },
                        "entries": entries_vec,
                    }))?;
                    stdout.write_all(&s)?;
                }
            }
        }
        Some(hash_folderoo::cli::Commands::Compare(args)) => {
            let source = args.source.as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| anyhow::anyhow!("--source is required"))?;
            let target = args.target.as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| anyhow::anyhow!("--target is required"))?;

            if !args.format.as_deref().unwrap_or("json").is_empty() {
                // noop; format will be used below
            }

            if !args.output.is_none() {
                // noop; output will be used below
            }

            let src_map = compare_mod::get_map_from_input(&source).map_err(|e| anyhow::anyhow!(e))?;
            let tgt_map = compare_mod::get_map_from_input(&target).map_err(|e| anyhow::anyhow!(e))?;

            let report = compare_mod::compare_maps(src_map, tgt_map);

            let format = args.format.as_deref().unwrap_or("json");
            let out_path = args.output.as_ref().map(|p| p.as_path());

            compare_mod::write_report(&report, out_path, format).map_err(|e| anyhow::anyhow!(e))?;
        }
        Some(hash_folderoo::cli::Commands::Copydiff(args)) => {
            // Load plan from file if provided, otherwise generate by running a comparison
            let plan = if let Some(p) = &args.plan {
                // load JSON plan
                let f = File::open(p).map_err(|e| anyhow::anyhow!("failed opening plan {:?}: {}", p, e))?;
                let reader = BufReader::new(f);
                let plan: copy::CopyPlan = serde_json::from_reader(reader).map_err(|e| anyhow::anyhow!("failed parsing plan {:?}: {}", p, e))?;
                plan
            } else {
                // require source and target to generate comparison-based plan
                let source = args.source.as_ref().map(|p| p.to_string_lossy().into_owned())
                    .ok_or_else(|| anyhow::anyhow!("--source is required when --plan is not provided"))?;
                let target = args.target.as_ref().map(|p| p.to_string_lossy().into_owned())
                    .ok_or_else(|| anyhow::anyhow!("--target is required when --plan is not provided"))?;

                let src_map = compare_mod::get_map_from_input(&source).map_err(|e| anyhow::anyhow!(e))?;
                let tgt_map = compare_mod::get_map_from_input(&target).map_err(|e| anyhow::anyhow!(e))?;
                let report = compare_mod::compare_maps(src_map, tgt_map);

                // If the provided source/target are directories, pass them as roots to help construct dst paths
                let source_root = args.source.as_ref().and_then(|p| {
                    if p.exists() && p.is_dir() { Some(p.as_path()) } else { None }
                });
                let target_root = args.target.as_ref().and_then(|p| {
                    if p.exists() && p.is_dir() { Some(p.as_path()) } else { None }
                });

                copy::generate_copy_plan(&report, source_root, target_root)
            };

            if args.execute {
                copy::execute_copy_plan(&plan).map_err(|e| anyhow::anyhow!(e))?;
            } else {
                // default to dry-run output
                copy::dry_run_copy_plan(&plan);
            }
        }
        Some(hash_folderoo::cli::Commands::Removempty(args)) => {
            let path = args.path.as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| anyhow::anyhow!("--path is required"))?;
            hash_folderoo::remove_empty_directories(std::path::Path::new(&path), args.dry_run)
                .map_err(|e| anyhow::anyhow!("removempty error: {}", e))?;
        }
        Some(hash_folderoo::cli::Commands::Renamer(args)) => {
            let path = args.path.as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| anyhow::anyhow!("--path is required"))?;
            let pattern = args.pattern.as_ref()
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
            let input = args.input.as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .ok_or_else(|| anyhow::anyhow!("--input is required"))?;
            let format = args.format.as_deref().unwrap_or("text");
            hash_folderoo::generate_report(&input, format).map_err(|e| anyhow::anyhow!(e))?;
        }
        Some(_) => {
            println!("Subcommand not implemented in phase 1. Run with --help for usage");
        }
        None => {
            println!("Run with --help for usage");
        }
    }

    Ok(())
}
