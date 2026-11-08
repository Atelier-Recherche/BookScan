use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::pipeline::{EnhanceMode, PipelineConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub library_root: PathBuf,
    #[serde(default)]
    pub pipeline: PipelineConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            library_root: default_library_root(),
            pipeline: PipelineConfig {
                detect: true,
                dewarp: false,
                finger: false,
                handwriting: false,
                enhance: EnhanceMode::Color,
            },
        }
    }
}

pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("BOOKSCAN_CONFIG") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join("bookscan-companion.json");
        }
    }
    default_library_root().join("bookscan-companion.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
            return cfg;
        }
    }
    if let Ok(p) = std::env::var("BOOKSCAN_LIBRARY") {
        let mut cfg = AppConfig::default();
        cfg.library_root = PathBuf::from(p);
        return cfg;
    }
    AppConfig::default()
}

pub fn save_config(cfg: &AppConfig) -> anyhow::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(cfg)?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn default_library_root() -> PathBuf {
    let base = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Documents").join("bookscan")
}
