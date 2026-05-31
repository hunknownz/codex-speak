use std::fs;

use anyhow::Result;
use serde::Serialize;

use crate::config::{self, Config};

#[derive(Debug, Serialize)]
pub struct Status {
    pub enabled: bool,
    pub language: String,
    pub child_mode: bool,
    pub provider: String,
    pub voice_profile: String,
    pub speed: f32,
    pub num_threads: usize,
    pub max_read_chars: usize,
    pub paths: StatusPaths,
    pub checks: StatusChecks,
    pub last_spoken: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StatusPaths {
    pub config: String,
    pub cli: String,
    pub model: String,
    pub spool: String,
}

#[derive(Debug, Serialize)]
pub struct StatusChecks {
    pub config_exists: bool,
    pub cli_exists: bool,
    pub model_exists: bool,
    pub sherpa_exists: bool,
    pub notify_configured: bool,
}

pub fn collect(cfg: &Config) -> Result<Status> {
    let config_path = config::config_path()?;
    let cli_path = config::bin_dir()?.join(binary_name());
    let model_path = config::model_dir()?.join("model.onnx");
    let sherpa_path = config::sherpa_bin()?;
    let codex_config = config::codex_home()?.join("config.toml");
    let codex_config_raw = fs::read_to_string(codex_config).unwrap_or_default();
    let last_spoken = fs::read_to_string(config::logs_dir()?.join("last-spoken.txt"))
        .ok()
        .map(|text| preview(&text));

    Ok(Status {
        enabled: cfg.enabled,
        language: cfg.language.clone(),
        child_mode: cfg.child_mode,
        provider: cfg.provider.clone(),
        voice_profile: cfg.voice_profile.clone(),
        speed: cfg.speed,
        num_threads: cfg.num_threads,
        max_read_chars: cfg.max_read_chars,
        paths: StatusPaths {
            config: config_path.display().to_string(),
            cli: cli_path.display().to_string(),
            model: model_path.display().to_string(),
            spool: config::spool_dir()?.display().to_string(),
        },
        checks: StatusChecks {
            config_exists: config_path.is_file(),
            cli_exists: cli_path.is_file(),
            model_exists: model_path.is_file(),
            sherpa_exists: sherpa_path.is_file(),
            notify_configured: codex_config_raw.contains("codex-speak-notify"),
        },
        last_spoken,
    })
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "codex-speak.exe"
    } else {
        "codex-speak"
    }
}

fn preview(text: &str) -> String {
    let mut result: String = text.chars().take(180).collect();
    if text.chars().count() > 180 {
        result.push('…');
    }
    result
}
