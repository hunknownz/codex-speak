use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Local};
use serde::Serialize;

use crate::bundled;
use crate::config::{self, Config};
use crate::model_catalog;
use crate::pet_state::PetState;
use crate::pronunciation;
use crate::settings;

#[derive(Debug, Serialize)]
pub struct Status {
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub enabled: bool,
    pub final_guide_enabled: bool,
    pub progress_prompts_enabled: bool,
    pub pet_enabled: bool,
    pub language: String,
    pub child_mode: bool,
    pub provider: String,
    pub voice_profile: String,
    pub speed: f32,
    pub num_threads: usize,
    pub max_read_chars: usize,
    pub pronunciation_terms: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation_error: Option<String>,
    pub paths: StatusPaths,
    pub checks: StatusChecks,
    pub providers: Vec<ProviderStatus>,
    pub pet_state: PetState,
    pub last_spoken: Option<String>,
    pub last_spoken_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StatusPaths {
    pub config: String,
    pub pronunciation_dictionary: String,
    pub release_manifest: String,
    pub cli: String,
    pub control_app: String,
    pub pet_helper: String,
    pub model: String,
    pub spool: String,
    pub plugin: String,
    pub marketplace: String,
}

#[derive(Debug, Serialize)]
pub struct StatusChecks {
    pub config_exists: bool,
    pub pronunciation_dictionary_valid: bool,
    pub release_manifest_exists: bool,
    pub cli_exists: bool,
    pub control_app_exists: bool,
    pub pet_helper_supported: bool,
    pub pet_helper_exists: bool,
    pub model_exists: bool,
    pub sherpa_exists: bool,
    pub notify_configured: bool,
    pub notify_hook_current: bool,
    pub codex_skill_installed: bool,
    pub codex_skill_current: bool,
    pub plugin_installed: bool,
    pub plugin_current: bool,
    pub plugin_skill_installed: bool,
    pub plugin_skill_current: bool,
    pub plugin_mcp_config_installed: bool,
    pub plugin_mcp_config_current: bool,
    pub plugin_mcp_script_installed: bool,
    pub plugin_mcp_script_current: bool,
    pub marketplace_configured: bool,
    pub player_available: bool,
}

#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    pub id: String,
    pub label: String,
    pub languages: String,
    pub footprint: String,
    pub role: String,
    pub recommended: bool,
    pub requires_model: bool,
    pub installed: bool,
    pub reason: Option<String>,
}

pub fn collect(cfg: &Config) -> Result<Status> {
    let config_path = config::config_path()?;
    let pronunciation_dictionary_path = config::pronunciation_dictionary_path()?;
    let release_manifest_path = config::release_manifest_path()?;
    let cli_path = config::bin_dir()?.join(binary_name());
    let control_app_path = config::control_app_path()?;
    let pet_helper_path = config::pet_helper_path()?;
    let model_path = config::model_dir()?.join("model.onnx");
    let sherpa_path = config::sherpa_bin()?;
    let plugin_path = config::installed_plugin_dir()?;
    let marketplace_path = config::personal_marketplace_path()?;
    let codex_config = config::codex_home()?.join("config.toml");
    let codex_config_raw = fs::read_to_string(codex_config).unwrap_or_default();
    let notify_hook_path = hook_path()?;
    let codex_skill_path = config::codex_home()?.join("skills/codex-speak/SKILL.md");
    let plugin_manifest_path = plugin_path.join(".codex-plugin/plugin.json");
    let plugin_skill_path = plugin_path.join("skills/codex-speak/SKILL.md");
    let plugin_mcp_config_path = plugin_path.join(".mcp.json");
    let plugin_mcp_script_path = plugin_path.join(mcp_script_path());
    let last_spoken_path = config::logs_dir()?.join("last-spoken.txt");
    let last_spoken = fs::read_to_string(&last_spoken_path)
        .ok()
        .map(|text| preview(&text));
    let last_spoken_at = fs::metadata(&last_spoken_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(|modified| {
            let datetime: DateTime<Local> = modified.into();
            datetime.to_rfc3339()
        });
    let (pronunciation_terms, pronunciation_error) = match pronunciation::load_user_dictionary() {
        Ok(dictionary) => (dictionary.terms.len(), None),
        Err(err) => (0, Some(err.to_string())),
    };

    Ok(Status {
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        enabled: cfg.enabled,
        final_guide_enabled: cfg.final_guide_enabled,
        progress_prompts_enabled: cfg.progress_prompts_enabled,
        pet_enabled: cfg.pet_enabled,
        language: cfg.language.clone(),
        child_mode: cfg.child_mode,
        provider: cfg.provider.clone(),
        voice_profile: cfg.voice_profile.clone(),
        speed: cfg.speed,
        num_threads: cfg.num_threads,
        max_read_chars: cfg.max_read_chars,
        pronunciation_terms,
        pronunciation_error: pronunciation_error.clone(),
        paths: StatusPaths {
            config: config_path.display().to_string(),
            pronunciation_dictionary: pronunciation_dictionary_path.display().to_string(),
            release_manifest: release_manifest_path.display().to_string(),
            cli: cli_path.display().to_string(),
            control_app: control_app_path.display().to_string(),
            pet_helper: pet_helper_path.display().to_string(),
            model: model_path.display().to_string(),
            spool: config::spool_dir()?.display().to_string(),
            plugin: plugin_path.display().to_string(),
            marketplace: marketplace_path.display().to_string(),
        },
        checks: StatusChecks {
            config_exists: config_path.is_file(),
            pronunciation_dictionary_valid: pronunciation_error.is_none(),
            release_manifest_exists: release_manifest_path.is_file(),
            cli_exists: cli_path.is_file(),
            control_app_exists: control_app_path.exists(),
            pet_helper_supported: config::pet_helper_supported(),
            pet_helper_exists: pet_helper_path.is_file(),
            model_exists: model_path.is_file(),
            sherpa_exists: sherpa_path.is_file(),
            notify_configured: codex_config_raw.contains("codex-speak-notify"),
            notify_hook_current: file_matches(&notify_hook_path, &bundled::hook_content(&cli_path)),
            codex_skill_installed: codex_skill_path.is_file(),
            codex_skill_current: file_matches(&codex_skill_path, bundled::CODEX_SKILL),
            plugin_installed: plugin_manifest_path.is_file(),
            plugin_current: file_matches(&plugin_manifest_path, bundled::PLUGIN_MANIFEST),
            plugin_skill_installed: plugin_skill_path.is_file(),
            plugin_skill_current: file_matches(&plugin_skill_path, bundled::PLUGIN_SKILL),
            plugin_mcp_config_installed: plugin_mcp_config_path.is_file(),
            plugin_mcp_config_current: file_matches(
                &plugin_mcp_config_path,
                &bundled::plugin_mcp_config(&cli_path),
            ),
            plugin_mcp_script_installed: plugin_mcp_script_path.is_file(),
            plugin_mcp_script_current: file_matches(&plugin_mcp_script_path, expected_mcp_script()),
            marketplace_configured: marketplace_has_plugin(&marketplace_path),
            player_available: crate::doctor::player_available(),
        },
        providers: provider_statuses()?,
        pet_state: crate::pet_state::read_state().unwrap_or_default(),
        last_spoken,
        last_spoken_at,
    })
}

fn marketplace_has_plugin(path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    value
        .get("plugins")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|plugins| {
            plugins.iter().any(|plugin| {
                plugin.get("name").and_then(serde_json::Value::as_str) == Some("codex-speak")
            })
        })
}

pub fn provider_statuses() -> Result<Vec<ProviderStatus>> {
    let mut providers = Vec::new();
    for id in settings::supported_providers() {
        providers.push(provider_status(id)?);
    }
    Ok(providers)
}

fn provider_status(id: &str) -> Result<ProviderStatus> {
    let meta = model_catalog::provider_meta(id);
    let (installed, reason) = match id {
        "sherpa_melo" => {
            let model = config::model_dir()?;
            let ok = config::sherpa_bin()?.is_file()
                && model.join("model.onnx").is_file()
                && model.join("tokens.txt").is_file()
                && model.join("lexicon.txt").is_file();
            (ok, missing_reason(ok, "MeloTTS 模型或 Sherpa-ONNX 未安装"))
        }
        "sherpa_kokoro" => {
            let model = config::kokoro_model_dir()?;
            let ok = config::sherpa_bin()?.is_file()
                && model.join("model.onnx").is_file()
                && model.join("voices.bin").is_file()
                && model.join("tokens.txt").is_file()
                && (!kokoro_lexicons(&model).is_empty() || model.join("espeak-ng-data").is_dir());
            (ok, missing_reason(ok, "Kokoro 模型未安装"))
        }
        "sherpa_zipvoice" => {
            let model = config::zipvoice_model_dir()?;
            let ok = config::sherpa_bin()?.is_file()
                && any_file(&[model.join("encoder.onnx"), model.join("encoder.int8.onnx")])
                && any_file(&[model.join("decoder.onnx"), model.join("decoder.int8.onnx")])
                && model.join("tokens.txt").is_file()
                && any_file(&[model.join("vocoder.onnx"), model.join("vocos_24khz.onnx")])
                && any_file(&[
                    model.join("reference.wav"),
                    model.join("test_wavs/leijun-1.wav"),
                    model.join("test_wavs/en-1.wav"),
                ]);
            (ok, missing_reason(ok, "ZipVoice 模型或参考音频未安装"))
        }
        "piper" => {
            let model = config::piper_model_dir()?;
            let ok = config::sherpa_bin()?.is_file()
                && model.join("model.onnx").is_file()
                && model.join("tokens.txt").is_file()
                && model.join("lexicon.txt").is_file();
            (ok, missing_reason(ok, "Piper 中文模型未安装"))
        }
        "system" => {
            let ok = system_voice_available();
            (ok, missing_reason(ok, "系统语音不可用"))
        }
        _ => (false, Some("未知引擎".to_string())),
    };

    Ok(ProviderStatus {
        id: id.to_string(),
        label: meta.map(|item| item.label).unwrap_or(id).to_string(),
        languages: meta.map(|item| item.languages).unwrap_or("-").to_string(),
        footprint: meta.map(|item| item.footprint).unwrap_or("-").to_string(),
        role: meta.map(|item| item.role).unwrap_or("-").to_string(),
        recommended: meta.map(|item| item.recommended).unwrap_or(false),
        requires_model: meta.map(|item| item.requires_model).unwrap_or(true),
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

fn kokoro_lexicons(model: &Path) -> Vec<PathBuf> {
    [
        model.join("lexicon.txt"),
        model.join("lexicon-us-en.txt"),
        model.join("lexicon-zh.txt"),
    ]
    .into_iter()
    .filter(|path| path.is_file())
    .collect()
}

fn any_file(paths: &[PathBuf]) -> bool {
    paths.iter().any(|path| path.is_file())
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "codex-speak.exe"
    } else {
        "codex-speak"
    }
}

fn hook_path() -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "codex-speak-notify.ps1"
    } else {
        "codex-speak-notify"
    };
    Ok(config::codex_home()?.join("hooks").join(name))
}

fn mcp_script_path() -> &'static str {
    if cfg!(windows) {
        "scripts/codex-speak-mcp.ps1"
    } else {
        "scripts/codex-speak-mcp"
    }
}

fn expected_mcp_script() -> &'static str {
    if cfg!(windows) {
        bundled::PLUGIN_MCP_SCRIPT_WINDOWS
    } else {
        bundled::PLUGIN_MCP_SCRIPT_UNIX
    }
}

fn file_matches(path: &Path, expected: &str) -> bool {
    fs::read_to_string(path).is_ok_and(|actual| actual == expected)
}

fn preview(text: &str) -> String {
    let mut result: String = text.chars().take(180).collect();
    if text.chars().count() > 180 {
        result.push('…');
    }
    result
}
