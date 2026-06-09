use anyhow::{bail, Result};
use serde::Serialize;

use crate::config::Config;
use crate::model_catalog;

#[derive(Debug, Default, Clone)]
pub struct ConfigPatch {
    pub enabled: Option<bool>,
    pub final_guide_enabled: Option<bool>,
    pub progress_prompts_enabled: Option<bool>,
    pub pet_enabled: Option<bool>,
    pub missing_guide_policy: Option<String>,
    pub child_mode: Option<bool>,
    pub provider: Option<String>,
    pub speed: Option<f32>,
    pub vits_noise_scale: Option<f32>,
    pub vits_noise_scale_w: Option<f32>,
    pub tts_silence_scale: Option<f32>,
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
    if let Some(final_guide_enabled) = patch.final_guide_enabled {
        cfg.final_guide_enabled = final_guide_enabled;
        changed.push("final_guide_enabled".to_string());
    }
    if let Some(progress_prompts_enabled) = patch.progress_prompts_enabled {
        cfg.progress_prompts_enabled = progress_prompts_enabled;
        changed.push("progress_prompts_enabled".to_string());
    }
    if let Some(pet_enabled) = patch.pet_enabled {
        cfg.pet_enabled = pet_enabled;
        changed.push("pet_enabled".to_string());
    }
    if let Some(missing_guide_policy) = patch.missing_guide_policy {
        validate_missing_guide_policy(&missing_guide_policy)?;
        cfg.missing_guide_policy = missing_guide_policy;
        changed.push("missing_guide_policy".to_string());
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
    if let Some(vits_noise_scale) = patch.vits_noise_scale {
        validate_vits_noise_scale(vits_noise_scale)?;
        cfg.vits_noise_scale = vits_noise_scale;
        changed.push("vits_noise_scale".to_string());
    }
    if let Some(vits_noise_scale_w) = patch.vits_noise_scale_w {
        validate_vits_noise_scale_w(vits_noise_scale_w)?;
        cfg.vits_noise_scale_w = vits_noise_scale_w;
        changed.push("vits_noise_scale_w".to_string());
    }
    if let Some(tts_silence_scale) = patch.tts_silence_scale {
        validate_tts_silence_scale(tts_silence_scale)?;
        cfg.tts_silence_scale = tts_silence_scale;
        changed.push("tts_silence_scale".to_string());
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
    model_catalog::supported_provider_ids()
}

fn validate_provider(provider: &str) -> Result<()> {
    if !supported_providers().contains(&provider) {
        bail!("unsupported provider: {provider}");
    }
    Ok(())
}

pub fn supported_missing_guide_policies() -> &'static [&'static str] {
    &["silent", "brief_notice", "diagnostic_notice"]
}

fn validate_missing_guide_policy(policy: &str) -> Result<()> {
    if !supported_missing_guide_policies().contains(&policy) {
        bail!("unsupported missing guide policy: {policy}");
    }
    Ok(())
}

fn apply_voice_profile(cfg: &mut Config, profile: &str) -> Result<()> {
    match profile {
        "clear_bright" => {
            cfg.voice_profile = profile.to_string();
            cfg.speed = 0.82;
            cfg.vits_noise_scale = 0.26;
            cfg.vits_noise_scale_w = 0.36;
            cfg.tts_silence_scale = 0.58;
        }
        "slow_clear" => {
            cfg.voice_profile = profile.to_string();
            cfg.speed = 0.74;
            cfg.vits_noise_scale = 0.22;
            cfg.vits_noise_scale_w = 0.3;
            cfg.tts_silence_scale = 0.72;
        }
        "quick_preview" => {
            cfg.voice_profile = profile.to_string();
            cfg.speed = 0.92;
            cfg.vits_noise_scale = 0.34;
            cfg.vits_noise_scale_w = 0.44;
            cfg.tts_silence_scale = 0.44;
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

fn validate_vits_noise_scale(value: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&value) {
        bail!("vits_noise_scale must be between 0.0 and 1.0");
    }
    Ok(())
}

fn validate_vits_noise_scale_w(value: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&value) {
        bail!("vits_noise_scale_w must be between 0.0 and 1.0");
    }
    Ok(())
}

fn validate_tts_silence_scale(value: f32) -> Result<()> {
    if !(0.1..=1.5).contains(&value) {
        bail!("tts_silence_scale must be between 0.1 and 1.5");
    }
    Ok(())
}

fn validate_max_read_chars(max_read_chars: usize) -> Result<()> {
    if !(80..=2000).contains(&max_read_chars) {
        bail!("max_read_chars must be between 80 and 2000");
    }
    Ok(())
}
