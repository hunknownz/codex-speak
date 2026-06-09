use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::Local;
use serde::Serialize;

use crate::config::{self, Config};
use crate::settings::{self, ConfigPatch};
use crate::status;

#[derive(Debug, Clone, Serialize)]
pub struct ControlVerificationReport {
    pub ok: bool,
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub generated_at: String,
    pub checks: Vec<ControlCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlCheck {
    pub id: &'static str,
    pub label: &'static str,
    pub status: ControlCheckStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ControlCheckStatus {
    Pass,
    Fail,
}

pub fn run(json: bool) -> Result<()> {
    let report = collect()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text(&report);
    }

    if !report.ok {
        anyhow::bail!("control verification failed");
    }
    Ok(())
}

fn collect() -> Result<ControlVerificationReport> {
    let config_path = config::config_path()?;
    let original_raw = if config_path.exists() {
        Some(
            fs::read(&config_path)
                .with_context(|| format!("failed to read {}", config_path.display()))?,
        )
    } else {
        None
    };
    let original = Config::load_or_default()?;
    let mut checks = vec![ControlCheck::pass(
        "load_config",
        "Load config",
        format!("loaded {}", config_path.display()),
    )];

    run_control_roundtrip(&original, &mut checks);
    verify_status_view(&mut checks);
    restore_config(&config_path, original_raw.as_deref(), &mut checks);

    let ok = checks
        .iter()
        .all(|check| check.status == ControlCheckStatus::Pass);
    Ok(ControlVerificationReport {
        ok,
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        generated_at: Local::now().to_rfc3339(),
        checks,
    })
}

fn run_control_roundtrip(original: &Config, checks: &mut Vec<ControlCheck>) {
    let mut cfg = original.clone();

    apply_and_expect(
        &mut cfg,
        ConfigPatch {
            enabled: Some(!original.enabled),
            ..ConfigPatch::default()
        },
        "enabled_toggle",
        "Auto speech toggle",
        |loaded| loaded.enabled != original.enabled,
        checks,
    );

    apply_and_expect(
        &mut cfg,
        ConfigPatch {
            final_guide_enabled: Some(!original.final_guide_enabled),
            ..ConfigPatch::default()
        },
        "final_guide_enabled_toggle",
        "Final guide toggle",
        |loaded| loaded.final_guide_enabled != original.final_guide_enabled,
        checks,
    );

    apply_and_expect(
        &mut cfg,
        ConfigPatch {
            progress_prompts_enabled: Some(!original.progress_prompts_enabled),
            ..ConfigPatch::default()
        },
        "progress_prompts_enabled_toggle",
        "Progress prompts toggle",
        |loaded| loaded.progress_prompts_enabled != original.progress_prompts_enabled,
        checks,
    );

    apply_and_expect(
        &mut cfg,
        ConfigPatch {
            pet_enabled: Some(!original.pet_enabled),
            ..ConfigPatch::default()
        },
        "pet_enabled_toggle",
        "Pet visibility toggle",
        |loaded| loaded.pet_enabled != original.pet_enabled,
        checks,
    );

    apply_and_expect(
        &mut cfg,
        ConfigPatch {
            child_mode: Some(!original.child_mode),
            ..ConfigPatch::default()
        },
        "child_mode_toggle",
        "Child mode toggle",
        |loaded| loaded.child_mode != original.child_mode,
        checks,
    );

    let speed = alternate_speed(original.speed);
    apply_and_expect(
        &mut cfg,
        ConfigPatch {
            speed: Some(speed),
            ..ConfigPatch::default()
        },
        "speed_roundtrip",
        "Speed setting",
        |loaded| close_enough(loaded.speed, speed),
        checks,
    );

    let max_read_chars = alternate_max_read_chars(original.max_read_chars);
    apply_and_expect(
        &mut cfg,
        ConfigPatch {
            max_read_chars: Some(max_read_chars),
            ..ConfigPatch::default()
        },
        "max_read_chars_roundtrip",
        "Max read length",
        |loaded| loaded.max_read_chars == max_read_chars,
        checks,
    );

    let voice_profile = alternate_voice_profile(&original.voice_profile);
    apply_and_expect(
        &mut cfg,
        ConfigPatch {
            voice_profile: Some(voice_profile.to_string()),
            ..ConfigPatch::default()
        },
        "voice_profile_roundtrip",
        "Voice profile",
        |loaded| loaded.voice_profile == voice_profile,
        checks,
    );

    let provider = alternate_provider(&original.provider);
    apply_and_expect(
        &mut cfg,
        ConfigPatch {
            provider: Some(provider.to_string()),
            ..ConfigPatch::default()
        },
        "provider_roundtrip",
        "TTS provider",
        |loaded| loaded.provider == provider,
        checks,
    );
}

fn apply_and_expect<F>(
    cfg: &mut Config,
    patch: ConfigPatch,
    id: &'static str,
    label: &'static str,
    expect: F,
    checks: &mut Vec<ControlCheck>,
) where
    F: Fn(&Config) -> bool,
{
    let update = match settings::apply_patch(cfg.clone(), patch) {
        Ok(update) => update,
        Err(error) => {
            checks.push(ControlCheck::fail(id, label, error.to_string()));
            return;
        }
    };

    if let Err(error) = update.config.save() {
        checks.push(ControlCheck::fail(id, label, error.to_string()));
        return;
    }

    match Config::load_or_default() {
        Ok(loaded) if expect(&loaded) => {
            *cfg = loaded;
            checks.push(ControlCheck::pass(id, label, "saved and reloaded"));
        }
        Ok(loaded) => {
            checks.push(ControlCheck::fail(
                id,
                label,
                format!("saved value did not roundtrip: {loaded:?}"),
            ));
        }
        Err(error) => checks.push(ControlCheck::fail(id, label, error.to_string())),
    }
}

fn verify_status_view(checks: &mut Vec<ControlCheck>) {
    let cfg = match Config::load_or_default() {
        Ok(cfg) => cfg,
        Err(error) => {
            checks.push(ControlCheck::fail(
                "status_view",
                "Status reflects controls",
                error.to_string(),
            ));
            return;
        }
    };

    match status::collect(&cfg) {
        Ok(view)
            if view.enabled == cfg.enabled
                && view.final_guide_enabled == cfg.final_guide_enabled
                && view.progress_prompts_enabled == cfg.progress_prompts_enabled
                && view.pet_enabled == cfg.pet_enabled
                && view.missing_guide_policy == cfg.missing_guide_policy
                && view.child_mode == cfg.child_mode
                && view.provider == cfg.provider
                && view.voice_profile == cfg.voice_profile
                && close_enough(view.speed, cfg.speed)
                && view.max_read_chars == cfg.max_read_chars =>
        {
            checks.push(ControlCheck::pass(
                "status_view",
                "Status reflects controls",
                "status JSON matches saved config",
            ));
        }
        Ok(_) => checks.push(ControlCheck::fail(
            "status_view",
            "Status reflects controls",
            "status JSON did not match saved config",
        )),
        Err(error) => checks.push(ControlCheck::fail(
            "status_view",
            "Status reflects controls",
            error.to_string(),
        )),
    }
}

fn restore_config(path: &Path, original_raw: Option<&[u8]>, checks: &mut Vec<ControlCheck>) {
    let result = match original_raw {
        Some(raw) => fs::write(path, raw).map(|_| "original config restored".to_string()),
        None => match fs::remove_file(path) {
            Ok(()) => Ok("temporary config removed".to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok("no original config existed".to_string())
            }
            Err(error) => Err(error),
        },
    };

    match result {
        Ok(detail) => checks.push(ControlCheck::pass(
            "restore_config",
            "Restore config",
            detail,
        )),
        Err(error) => checks.push(ControlCheck::fail(
            "restore_config",
            "Restore config",
            error.to_string(),
        )),
    }
}

fn alternate_provider(current: &str) -> &'static str {
    if current == "system" {
        "sherpa_melo"
    } else {
        "system"
    }
}

fn alternate_voice_profile(current: &str) -> &'static str {
    if current == "slow_clear" {
        "clear_bright"
    } else {
        "slow_clear"
    }
}

fn alternate_speed(current: f32) -> f32 {
    if close_enough(current, 0.82) {
        1.0
    } else {
        0.82
    }
}

fn alternate_max_read_chars(current: usize) -> usize {
    if current == 320 {
        640
    } else {
        320
    }
}

fn close_enough(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.001
}

fn print_text(report: &ControlVerificationReport) {
    println!(
        "Codex Speak control verification {} on {} {}",
        report.version, report.os, report.arch
    );
    for check in &report.checks {
        let prefix = match check.status {
            ControlCheckStatus::Pass => "OK  ",
            ControlCheckStatus::Fail => "FAIL",
        };
        println!("{prefix} {}: {}", check.label, check.detail);
    }
}

impl ControlCheck {
    fn pass(id: &'static str, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            id,
            label,
            status: ControlCheckStatus::Pass,
            detail: detail.into(),
        }
    }

    fn fail(id: &'static str, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            id,
            label,
            status: ControlCheckStatus::Fail,
            detail: detail.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{alternate_max_read_chars, alternate_provider, alternate_speed};

    #[test]
    fn alternate_provider_uses_system_when_possible() {
        assert_eq!(alternate_provider("sherpa_melo"), "system");
        assert_eq!(alternate_provider("system"), "sherpa_melo");
    }

    #[test]
    fn alternate_speed_stays_in_valid_range() {
        assert_eq!(alternate_speed(0.9), 0.82);
        assert_eq!(alternate_speed(0.82), 1.0);
    }

    #[test]
    fn alternate_max_read_chars_stays_in_valid_range() {
        assert_eq!(alternate_max_read_chars(800), 320);
        assert_eq!(alternate_max_read_chars(320), 640);
    }
}
