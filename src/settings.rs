use anyhow::{bail, Result};
use serde::Serialize;

use crate::config::Config;

#[derive(Debug, Default, Clone)]
pub struct ConfigPatch {
    pub enabled: Option<bool>,
    pub child_mode: Option<bool>,
    pub provider: Option<String>,
    pub speed: Option<f32>,
    pub max_read_chars: Option<usize>,
    pub voice_profile: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConfigUpdate {
    pub config: Config,
    pub changed: Vec<String>,
}

pub fn apply_patch(mut cfg: Config, patch: ConfigPatch) -> Result<ConfigUpdate> {
    let mut changed = Vec::new();

    if let Some(enabled) = patch.enabled {
        cfg.enabled = enabled;
        changed.push("enabled".to_string());
    }
    if let Some(child_mode) = patch.child_mode {
        cfg.child_mode = child_mode;
        changed.push("child_mode".to_string());
    }
    if let Some(provider) = patch.provider {
        validate_provider(&provider)?;
        cfg.provider = provider;
        changed.push("provider".to_string());
    }
    if let Some(speed) = patch.speed {
        validate_speed(speed)?;
        cfg.speed = speed;
        changed.push("speed".to_string());
    }
    if let Some(max_read_chars) = patch.max_read_chars {
        validate_max_read_chars(max_read_chars)?;
        cfg.max_read_chars = max_read_chars;
        changed.push("max_read_chars".to_string());
    }
    if let Some(profile) = patch.voice_profile {
        apply_voice_profile(&mut cfg, &profile)?;
        changed.push("voice_profile".to_string());
    }

    Ok(ConfigUpdate {
        config: cfg,
        changed,
    })
}

pub fn supported_providers() -> &'static [&'static str] {
    &[
        "sherpa_melo",
        "sherpa_kokoro",
        "sherpa_zipvoice",
        "piper",
        "system",
    ]
}

fn validate_provider(provider: &str) -> Result<()> {
    if !supported_providers().contains(&provider) {
        bail!("unsupported provider: {provider}");
    }
    Ok(())
}

fn apply_voice_profile(cfg: &mut Config, profile: &str) -> Result<()> {
    match profile {
        "clear_bright" => {
            cfg.voice_profile = profile.to_string();
            cfg.speed = 0.9;
            cfg.vits_noise_scale = 0.45;
            cfg.vits_noise_scale_w = 0.6;
            cfg.tts_silence_scale = 0.25;
        }
        "slow_clear" => {
            cfg.voice_profile = profile.to_string();
            cfg.speed = 0.82;
            cfg.vits_noise_scale = 0.42;
            cfg.vits_noise_scale_w = 0.55;
            cfg.tts_silence_scale = 0.3;
        }
        "quick_preview" => {
            cfg.voice_profile = profile.to_string();
            cfg.speed = 1.0;
            cfg.vits_noise_scale = 0.5;
            cfg.vits_noise_scale_w = 0.65;
            cfg.tts_silence_scale = 0.18;
        }
        other => bail!("unsupported voice profile: {other}"),
    }
    Ok(())
}

fn validate_speed(speed: f32) -> Result<()> {
    if !(0.6..=1.3).contains(&speed) {
        bail!("speed must be between 0.6 and 1.3");
    }
    Ok(())
}

fn validate_max_read_chars(max_read_chars: usize) -> Result<()> {
    if !(80..=2000).contains(&max_read_chars) {
        bail!("max_read_chars must be between 80 and 2000");
    }
    Ok(())
}
