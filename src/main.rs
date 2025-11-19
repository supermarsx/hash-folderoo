use clap::{Arg, Command};
use env_logger;
use log::{info, warn};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use hash_folderoo::algorithms::Algorithm;
use hash_folderoo::HasherImpl;
use hash_folderoo::RuntimeConfig;
use serde::Serialize;
use walkdir::WalkDir;
use globset::{Glob, GlobSetBuilder};
use chrono::Utc;

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

#[derive(Serialize)]
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
    env_logger::init();

    let matches = Command::new("hash-folderoo")
        .version("0.1.0")
        .about("Hash-based folder toolkit (prototype)")
        .subcommand_required(false)
        .subcommand(
            Command::new("hashmap")
                .about("Create a hashmap of files in a directory")
                .arg(Arg::new("path").short('p').long("path").required(true))
                .arg(Arg::new("output").short('o').long("output"))
                .arg(Arg::new("alg").long("alg").default_value("blake3"))
                .arg(Arg::new("xof-length").long("xof-length"))
                .arg(Arg::new("strip-prefix").long("strip-prefix").num_args(1))
                .arg(Arg::new("exclude").long("exclude").num_args(1..))
                .arg(Arg::new("config").long("config").num_args(1)),
        )
        .get_matches();

        if let Some(sub) = matches.subcommand_matches("hashmap") {
        // Start with empty runtime config; if --config provided we will load and merge
        let mut runtime_cfg = RuntimeConfig::default();
        if let Some(cfg_path) = sub.get_one::<String>("config") {
            match RuntimeConfig::load_from_file(cfg_path) {
                Ok(c) => runtime_cfg.merge(c),
                Err(e) => warn!("Failed loading config {}: {}", cfg_path, e),
            }
        }

        // CLI args override config
        let path = match sub.get_one::<String>("path") {
            Some(p) => p.clone(),
            None => runtime_cfg
                .general
                .as_ref()
                .and_then(|g| g.path.clone())
                .expect("path is required either via CLI or config"),
        };

        let output = sub
            .get_one::<String>("output")
            .map(|s| s.as_str())
            .or_else(|| runtime_cfg.general.as_ref().and_then(|g| g.output.as_deref()));

        let alg = sub
            .get_one::<String>("alg")
            .map(|s| s.as_str())
            .or_else(|| runtime_cfg.algorithm.as_ref().and_then(|a| a.name.as_deref()))
            .unwrap_or("blake3");

        let xof_len = sub
            .get_one::<String>("xof-length")
            .and_then(|s| s.parse::<usize>().ok())
            .or_else(|| runtime_cfg.algorithm.as_ref().and_then(|a| a.xof_length));

        let strip_prefix = sub
            .get_one::<String>("strip-prefix")
            .map(|s| s.clone())
            .or_else(|| runtime_cfg.general.as_ref().and_then(|g| g.path.clone()));

        let excludes: Vec<String> = if sub.contains_id("exclude") {
            sub.get_many::<String>("exclude").unwrap().map(|s| s.to_string()).collect()
        } else {
            runtime_cfg
                .general
                .as_ref()
                .and_then(|g| None)
                .unwrap_or_default()
        };

        info!("Computing hashmap for {} using alg {}", path, alg);

        let alg_enum = match Algorithm::from_str(alg) {
            Some(a) => a,
            None => {
                warn!("Unknown algorithm {}, falling back to blake3", alg);
                Algorithm::Blake3
            }
        };

        let mut probe = alg_enum.create();
        let default_out = probe.info().output_len_default;
        let out_len = xof_len.unwrap_or(default_out);

        let exclude_set = build_exclude_set(&excludes)?;

        let header = MapHeader {
            version: 1,
            generated_by: "hash-folderoo",
            timestamp: Utc::now().to_rfc3339(),
            root: path.to_string(),
            algorithm: AlgorithmMeta {
                name: probe.info().name,
                params: None,
            },
        };


        // Prepare output writer
        let mut writer: Box<dyn Write> = match output {
            Some(p) => Box::new(File::create(p)?),
            None => Box::new(std::io::stdout()),
        };

        // Write header and open entries array
        write!(writer, "{{\n")?;
        write!(writer, "  \"version\": {},\n", header.version)?;
        write!(writer, "  \"generated_by\": \"{}\",\n", header.generated_by)?;
        write!(writer, "  \"timestamp\": \"{}\",\n", header.timestamp)?;
        write!(writer, "  \"root\": \"{}\",\n", header.root)?;
        write!(writer, "  \"algorithm\": {{ \"name\": \"{}\" }},\n", header.algorithm.name)?;
        write!(writer, "  \"entries\": [\n")?;

        let mut first = true;

        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let p = entry.path().to_path_buf();
                let rel = if let Some(strip) = &strip_prefix {
                    match p.strip_prefix(strip) {
                        Ok(s) => s.to_string_lossy().into_owned(),
                        Err(_) => p.to_string_lossy().into_owned(),
                    }
                } else {
                    p.to_string_lossy().into_owned()
                };

                // Apply excludes
                if let Some(gs) = &exclude_set {
                    if gs.is_match(&p) {
                        info!("Excluding {}", p.display());
                        continue;
                    }
                }

                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let mut hasher = alg_enum.create();
                match hash_file(hasher.as_mut(), &p, out_len) {
                    Ok(h) => {
                        if !first {
                            write!(writer, ",\n")?;
                        }
                        first = false;
                        let me = MapEntry {
                            path: rel,
                            hash: h,
                            size,
                        };
                        let s = serde_json::to_string(&me)?;
                        write!(writer, "    {}", s)?;
                    }
                    Err(e) => warn!("Failed hashing {}: {}", p.display(), e),
                }
            }
        }

        // Close entries and object
        write!(writer, "\n  ]\n}}\n")?;
    } else {
        println!("Run with --help for usage");
    }

    Ok(())
}
