use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Context;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralConfig {
    pub path: Option<String>,
    pub output: Option<String>,
    pub format: Option<String>,
    pub threads: Option<usize>,
    pub strip_prefix: Option<String>,
    pub depth: Option<usize>,
    pub exclude: Option<Vec<String>>,
    pub follow_symlinks: Option<bool>,
    pub progress: Option<bool>,
    pub dry_run: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlgorithmConfig {
    pub name: Option<String>,
    pub xof_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryConfig {
    pub mode: Option<String>,
    pub max_ram: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    pub general: Option<GeneralConfig>,
    pub algorithm: Option<AlgorithmConfig>,
    pub memory: Option<MemoryConfig>,
}

impl RuntimeConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let p = path.as_ref();
        let mut s = String::new();
        let mut f = File::open(p)?;
        f.read_to_string(&mut s)?;
        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "toml" => Ok(toml::from_str(&s)?),
                "yaml" | "yml" => Ok(serde_yaml::from_str(&s)?),
                "json" => Ok(serde_json::from_str(&s)?),
                _ => Err(anyhow::anyhow!("Unsupported config extension: {}", ext)),
            }
        } else {
            Err(anyhow::anyhow!("Config file has no extension"))
        }
    }

    pub fn merge(&mut self, other: RuntimeConfig) {
        if let Some(g) = other.general {
            if self.general.is_none() {
                self.general = Some(g);
            } else {
                let target = self.general.as_mut().unwrap();
                if g.path.is_some() {
                    target.path = g.path;
                }
                if g.output.is_some() {
                    target.output = g.output;
                }
                if g.format.is_some() {
                    target.format = g.format;
                }
                if g.threads.is_some() {
                    target.threads = g.threads;
                }
                if g.strip_prefix.is_some() {
                    target.strip_prefix = g.strip_prefix;
                }
                if g.depth.is_some() {
                    target.depth = g.depth;
                }
                if g.exclude.is_some() {
                    target.exclude = g.exclude;
                }
                if g.follow_symlinks.is_some() {
                    target.follow_symlinks = g.follow_symlinks;
                }
                if g.progress.is_some() {
                    target.progress = g.progress;
                }
                if g.dry_run.is_some() {
                    target.dry_run = g.dry_run;
                }
            }
        }

        if let Some(a) = other.algorithm {
            if self.algorithm.is_none() {
                self.algorithm = Some(a);
            } else {
                let target = self.algorithm.as_mut().unwrap();
                if a.name.is_some() {
                    target.name = a.name;
                }
                if a.xof_length.is_some() {
                    target.xof_length = a.xof_length;
                }
            }
        }

        if let Some(m) = other.memory {
            if self.memory.is_none() {
                self.memory = Some(m);
            } else {
                let target = self.memory.as_mut().unwrap();
                if m.mode.is_some() {
                    target.mode = m.mode;
                }
                if m.max_ram.is_some() {
                    target.max_ram = m.max_ram;
                }
            }
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(g) = &self.general {
            if let Some(format) = g.format.as_deref() {
                let fmt = format.to_lowercase();
                if fmt != "json" && fmt != "csv" {
                    anyhow::bail!("invalid general.format '{}': use json or csv", format);
                }
            }
            if let Some(threads) = g.threads {
                if threads == 0 {
                    anyhow::bail!("general.threads must be greater than 0");
                }
            }
            if let Some(depth) = g.depth {
                if depth == 0 {
                    anyhow::bail!("general.depth must be greater than 0 when provided");
                }
            }
        }

        if let Some(a) = &self.algorithm {
            if let Some(name) = a.name.as_deref() {
                if name.trim().is_empty() {
                    anyhow::bail!("algorithm.name cannot be empty");
                }
            }
            if let Some(len) = a.xof_length {
                if len == 0 {
                    anyhow::bail!("algorithm.xof_length must be greater than 0");
                }
            }
        }

        if let Some(m) = &self.memory {
            if let Some(mode) = m.mode.as_deref() {
                match mode.to_lowercase().as_str() {
                    "stream" | "balanced" | "booster" => {}
                    other => {
                        anyhow::bail!(
                            "memory.mode '{}' is invalid (expected stream|balanced|booster)",
                            other
                        )
                    }
                }
            }
            if let Some(max_ram) = m.max_ram {
                if max_ram == 0 {
                    anyhow::bail!("memory.max_ram must be greater than 0");
                }
            }
        }

        Ok(())
    }
}

const CONFIG_FILENAMES: &[&str] = &["config.toml", "config.yaml", "config.yml", "config.json"];

fn candidates_in_dir(base: &Path) -> Vec<PathBuf> {
    CONFIG_FILENAMES
        .iter()
        .map(|name| base.join(name))
        .collect()
}

fn merge_if_exists(target: &mut RuntimeConfig, path: &Path) -> anyhow::Result<()> {
    if path.exists() {
        let cfg = RuntimeConfig::load_from_file(path)
            .with_context(|| format!("loading config {:?}", path))?;
        target.merge(cfg);
    }
    Ok(())
}

/// Load runtime configuration honoring precedence:
/// system (/etc) < user (~/.config/hash-folderoo) < project (cwd) < env (HASH_FOLDEROO_CONFIG) < CLI --config
pub fn load_runtime_config(cli_path: Option<&Path>) -> anyhow::Result<RuntimeConfig> {
    let mut cfg = RuntimeConfig::default();

    // System-wide configs
    let system_base = Path::new("/etc/hash-folderoo");
    for candidate in candidates_in_dir(system_base) {
        merge_if_exists(&mut cfg, &candidate)?;
    }

    // User config directory (e.g., ~/.config/hash-folderoo)
    if let Some(config_dir) = dirs::config_dir() {
        let user_base = config_dir.join("hash-folderoo");
        for candidate in candidates_in_dir(&user_base) {
            merge_if_exists(&mut cfg, &candidate)?;
        }
    }

    // Project-level configs in current working directory
    if let Ok(cwd) = std::env::current_dir() {
        for candidate in candidates_in_dir(&cwd) {
            merge_if_exists(&mut cfg, &candidate)?;
        }
    }

    // Environment override
    if let Some(env_path) = env::var_os("HASH_FOLDEROO_CONFIG") {
        let env_path = PathBuf::from(env_path);
        let cfg_env = RuntimeConfig::load_from_file(&env_path).with_context(|| {
            format!("loading config from HASH_FOLDEROO_CONFIG ({:?})", env_path)
        })?;
        cfg.merge(cfg_env);
    }

    // CLI --config overrides highest
    if let Some(p) = cli_path {
        let cli_cfg = RuntimeConfig::load_from_file(p)
            .with_context(|| format!("loading config from --config {:?}", p))?;
        cfg.merge(cli_cfg);
    }

    cfg.validate()?;

    Ok(cfg)
}

fn parse_usize(val: &str) -> Option<usize> {
    val.trim().parse::<usize>().ok()
}

fn parse_u64(val: &str) -> Option<u64> {
    val.trim().parse::<u64>().ok()
}

fn parse_list(val: &str) -> Vec<String> {
    val.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn parse_bool(val: &str) -> Option<bool> {
    match val.trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Apply environment variable overrides (precedence just below CLI args).
pub fn apply_env_overrides(cfg: &mut RuntimeConfig) {
    if let Ok(path) = env::var("HASH_FOLDEROO_PATH") {
        cfg.general.get_or_insert_with(Default::default).path = Some(path);
    }
    if let Ok(output) = env::var("HASH_FOLDEROO_OUTPUT") {
        cfg.general.get_or_insert_with(Default::default).output = Some(output);
    }
    if let Ok(format) = env::var("HASH_FOLDEROO_FORMAT") {
        cfg.general.get_or_insert_with(Default::default).format = Some(format);
    }
    if let Ok(threads_str) = env::var("HASH_FOLDEROO_THREADS") {
        if let Some(threads) = parse_usize(&threads_str) {
            cfg.general.get_or_insert_with(Default::default).threads = Some(threads);
        }
    }

    if let Ok(depth_str) = env::var("HASH_FOLDEROO_DEPTH") {
        if let Some(depth) = parse_usize(&depth_str) {
            cfg.general.get_or_insert_with(Default::default).depth = Some(depth);
        }
    }

    if let Ok(strip) = env::var("HASH_FOLDEROO_STRIP_PREFIX") {
        cfg.general
            .get_or_insert_with(Default::default)
            .strip_prefix = Some(strip);
    }

    if let Ok(exclude_str) = env::var("HASH_FOLDEROO_EXCLUDE") {
        let patterns = parse_list(&exclude_str);
        if !patterns.is_empty() {
            cfg.general.get_or_insert_with(Default::default).exclude = Some(patterns);
        }
    }

    if let Ok(follow) = env::var("HASH_FOLDEROO_FOLLOW_SYMLINKS") {
        if let Some(val) = parse_bool(&follow) {
            cfg.general
                .get_or_insert_with(Default::default)
                .follow_symlinks = Some(val);
        }
    }

    if let Ok(progress) = env::var("HASH_FOLDEROO_PROGRESS") {
        if let Some(val) = parse_bool(&progress) {
            cfg.general.get_or_insert_with(Default::default).progress = Some(val);
        }
    }

    if let Ok(dry_run) = env::var("HASH_FOLDEROO_DRY_RUN") {
        if let Some(val) = parse_bool(&dry_run) {
            cfg.general.get_or_insert_with(Default::default).dry_run = Some(val);
        }
    }

    if let Ok(alg) = env::var("HASH_FOLDEROO_ALG") {
        cfg.algorithm.get_or_insert_with(Default::default).name = Some(alg);
    }
    if let Ok(xof) = env::var("HASH_FOLDEROO_XOF_LENGTH") {
        if let Some(len) = parse_usize(&xof) {
            cfg.algorithm
                .get_or_insert_with(Default::default)
                .xof_length = Some(len);
        }
    }

    if let Ok(mode) = env::var("HASH_FOLDEROO_MEMORY_MODE") {
        cfg.memory.get_or_insert_with(Default::default).mode = Some(mode);
    }
    if let Ok(max_ram) = env::var("HASH_FOLDEROO_MAX_RAM") {
        if let Some(bytes) = parse_u64(&max_ram) {
            cfg.memory.get_or_insert_with(Default::default).max_ram = Some(bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_format() {
        let cfg = RuntimeConfig {
            general: Some(GeneralConfig {
                format: Some("xml".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn accepts_valid_config() {
        let cfg = RuntimeConfig {
            general: Some(GeneralConfig {
                format: Some("json".to_string()),
                threads: Some(4),
                ..Default::default()
            }),
            memory: Some(MemoryConfig {
                mode: Some("balanced".to_string()),
                max_ram: Some(1024),
            }),
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
    }
}
