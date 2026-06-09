use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const APP_DIR_NAME: &str = "codex-speak";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub enabled: bool,
    pub final_guide_enabled: bool,
    pub progress_prompts_enabled: bool,
    pub pet_enabled: bool,
    pub missing_guide_policy: String,
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
            final_guide_enabled: true,
            progress_prompts_enabled: true,
            pet_enabled: true,
            missing_guide_policy: "silent".to_string(),
            language: "zh".to_string(),
            child_mode: true,
            max_read_chars: 800,
            provider: "sherpa_melo".to_string(),
            voice_profile: "clear_bright".to_string(),
            speed: 0.82,
            num_threads: 4,
            vits_noise_scale: 0.26,
            vits_noise_scale_w: 0.36,
            tts_silence_scale: 0.58,
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

pub fn agents_home() -> Result<PathBuf> {
    Ok(home_dir()?.join(".agents"))
}

pub fn personal_plugins_root() -> Result<PathBuf> {
    Ok(agents_home()?.join("plugins"))
}

pub fn personal_plugin_sources_root() -> Result<PathBuf> {
    Ok(home_dir()?.join("plugins"))
}

pub fn personal_marketplace_path() -> Result<PathBuf> {
    Ok(personal_plugins_root()?.join("marketplace.json"))
}

pub fn installed_plugin_dir() -> Result<PathBuf> {
    Ok(personal_plugin_sources_root()?.join(APP_DIR_NAME))
}

#[allow(dead_code)]
pub fn installed_plugin_cache_root() -> Result<PathBuf> {
    Ok(codex_home()?
        .join("plugins")
        .join("cache")
        .join("personal")
        .join(APP_DIR_NAME))
}

pub fn app_home() -> Result<PathBuf> {
    Ok(codex_home()?.join(APP_DIR_NAME))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(app_home()?.join("config.toml"))
}

pub fn pronunciation_dictionary_path() -> Result<PathBuf> {
    Ok(app_home()?.join("pronunciation.toml"))
}

pub fn release_manifest_path() -> Result<PathBuf> {
    Ok(app_home()?.join("release-manifest.json"))
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

pub fn pet_state_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("pet-state.json"))
}

pub fn playback_pid_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("playback.pid"))
}

pub fn playback_lock_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("playback.lock"))
}

pub fn playback_stop_signal_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("playback.stop"))
}

pub fn spool_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("spool"))
}

pub fn queue_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("queue"))
}

pub fn queue_pending_dir() -> Result<PathBuf> {
    Ok(queue_dir()?.join("pending"))
}

pub fn queue_running_dir() -> Result<PathBuf> {
    Ok(queue_dir()?.join("running"))
}

pub fn queue_done_dir() -> Result<PathBuf> {
    Ok(queue_dir()?.join("done"))
}

pub fn queue_failed_dir() -> Result<PathBuf> {
    Ok(queue_dir()?.join("failed"))
}

pub fn queue_worker_lock_path() -> Result<PathBuf> {
    Ok(queue_dir()?.join("worker.lock"))
}

pub fn bin_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("bin"))
}

pub fn apps_dir() -> Result<PathBuf> {
    Ok(app_home()?.join("apps"))
}

pub fn control_app_path() -> Result<PathBuf> {
    let name = if cfg!(target_os = "macos") {
        "Codex Speak.app"
    } else if cfg!(windows) {
        "codex-speak-control.exe"
    } else {
        "codex-speak-control"
    };
    Ok(apps_dir()?.join(name))
}

pub fn pet_helper_path() -> Result<PathBuf> {
    Ok(bin_dir()?.join("codex-speak-pet-macos"))
}

pub fn pet_helper_supported() -> bool {
    cfg!(target_os = "macos")
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

pub fn kokoro_model_dir() -> Result<PathBuf> {
    Ok(models_dir()?.join("kokoro-zh-en"))
}

pub fn zipvoice_model_dir() -> Result<PathBuf> {
    Ok(models_dir()?.join("zipvoice-zh-en"))
}

pub fn piper_model_dir() -> Result<PathBuf> {
    Ok(models_dir()?.join("piper-zh-cn"))
}

pub fn sherpa_bin() -> Result<PathBuf> {
    let exe = if cfg!(windows) {
        "sherpa-onnx-offline-tts.exe"
    } else {
        "sherpa-onnx-offline-tts"
    };
    Ok(tools_dir()?.join("sherpa-onnx").join("bin").join(exe))
}
