use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const APP_DIR_NAME: &str = "codex-speak";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub enabled: bool,
    pub language: String,
    pub child_mode: bool,
    pub max_read_chars: usize,
    pub provider: String,
    pub voice_profile: String,
    pub speed: f32,
    pub num_threads: usize,
    pub vits_noise_scale: f32,
    pub vits_noise_scale_w: f32,
    pub tts_silence_scale: f32,
    pub fallback_provider: String,
    pub previous_notify: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            language: "zh".to_string(),
            child_mode: true,
            max_read_chars: 800,
            provider: "sherpa_melo".to_string(),
            voice_profile: "clear_bright".to_string(),
            speed: 0.9,
            num_threads: 4,
            vits_noise_scale: 0.45,
            vits_noise_scale_w: 0.6,
            tts_silence_scale: 0.25,
            fallback_provider: "system".to_string(),
            previous_notify: None,
        }
    }
}

impl Config {
    pub fn load_or_default() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        Ok(toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self)?;
        fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))
    }
}

pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("could not determine home directory")
}

pub fn codex_home() -> Result<PathBuf> {
    Ok(home_dir()?.join(".codex"))
}

pub fn app_home() -> Result<PathBuf> {
    Ok(codex_home()?.join(APP_DIR_NAME))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(app_home()?.join("config.toml"))
}

pub fn logs_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("logs"))
}

pub fn cache_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("cache"))
}

pub fn state_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("state"))
}

pub fn spool_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("spool"))
}

pub fn bin_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("bin"))
}

pub fn tools_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("tools"))
}

pub fn models_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("models"))
}

pub fn model_dir() -> Result<PathBuf> {
    Ok(models_dir()?.join("vits-melo-tts-zh_en"))
}

pub fn sherpa_bin() -> Result<PathBuf> {
    let exe = if cfg!(windows) {
        "sherpa-onnx-offline-tts.exe"
    } else {
        "sherpa-onnx-offline-tts"
    };
    Ok(tools_dir()?.join("sherpa-onnx").join("bin").join(exe))
}
