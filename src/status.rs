use std::fs;

use anyhow::Result;
use serde::Serialize;

use crate::config::{self, Config};
use crate::settings;

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
    pub providers: Vec<ProviderStatus>,
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

#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    pub id: String,
    pub label: String,
    pub installed: bool,
    pub reason: Option<String>,
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
        providers: provider_statuses()?,
        last_spoken,
    })
}

fn provider_statuses() -> Result<Vec<ProviderStatus>> {
    let mut providers = Vec::new();
    for id in settings::supported_providers() {
        providers.push(provider_status(id)?);
    }
    Ok(providers)
}

fn provider_status(id: &str) -> Result<ProviderStatus> {
    let (label, installed, reason) = match id {
        "sherpa_melo" => {
            let model = config::model_dir()?;
            let ok = config::sherpa_bin()?.is_file()
                && model.join("model.onnx").is_file()
                && model.join("tokens.txt").is_file()
                && model.join("lexicon.txt").is_file();
            (
                "MeloTTS 中文女声".to_string(),
                ok,
                missing_reason(ok, "MeloTTS 模型或 Sherpa-ONNX 未安装"),
            )
        }
        "sherpa_kokoro" => {
            let model = config::kokoro_model_dir()?;
            let ok = config::sherpa_bin()?.is_file()
                && model.join("model.onnx").is_file()
                && model.join("voices.bin").is_file()
                && model.join("tokens.txt").is_file()
                && (model.join("lexicon.txt").is_file() || model.join("espeak-ng-data").is_dir());
            (
                "Kokoro".to_string(),
                ok,
                missing_reason(ok, "Kokoro 模型未安装"),
            )
        }
        "sherpa_zipvoice" => {
            let model = config::zipvoice_model_dir()?;
            let ok = config::sherpa_bin()?.is_file()
                && model.join("encoder.onnx").is_file()
                && model.join("decoder.onnx").is_file()
                && model.join("tokens.txt").is_file()
                && model.join("vocoder.onnx").is_file()
                && model.join("reference.wav").is_file()
                && model.join("reference.txt").is_file();
            (
                "ZipVoice 中文/英文".to_string(),
                ok,
                missing_reason(ok, "ZipVoice 模型或参考音频未安装"),
            )
        }
        "piper" => {
            let model = config::piper_model_dir()?;
            let ok = config::piper_bin()?.is_file()
                && model.join("model.onnx").is_file()
                && model.join("model.onnx.json").is_file();
            (
                "Piper 轻量语音".to_string(),
                ok,
                missing_reason(ok, "Piper 引擎或模型未安装"),
            )
        }
        "system" => (
            "系统语音".to_string(),
            system_voice_available(),
            missing_reason(system_voice_available(), "系统语音不可用"),
        ),
        other => (other.to_string(), false, Some("未知引擎".to_string())),
    };

    Ok(ProviderStatus {
        id: id.to_string(),
        label,
        installed,
        reason,
    })
}

fn missing_reason(ok: bool, reason: &str) -> Option<String> {
    if ok {
        None
    } else {
        Some(reason.to_string())
    }
}

fn system_voice_available() -> bool {
    if cfg!(target_os = "macos") {
        std::path::Path::new("/usr/bin/say").is_file()
    } else {
        cfg!(windows)
    }
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
