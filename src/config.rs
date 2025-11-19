use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralConfig {
    pub path: Option<String>,
    pub output: Option<String>,
    pub format: Option<String>,
    pub threads: Option<usize>,
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
}
