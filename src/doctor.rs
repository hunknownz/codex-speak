use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use chrono::Local;
use serde::Serialize;

use crate::config;

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub generated_at: String,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub id: &'static str,
    pub label: &'static str,
    pub status: CheckStatus,
    pub required: bool,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Ok,
    Warn,
    Fail,
    Skip,
}

pub fn run(json: bool) -> Result<()> {
    let report = collect()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text(&report);
    }

    if !report.ok {
        anyhow::bail!("doctor found problems");
    }
    Ok(())
}

pub fn collect() -> Result<DoctorReport> {
    let mut checks = Vec::new();

    check_dir(
        "codex_home",
        "Codex home",
        &config::codex_home()?,
        &mut checks,
    );
    check_file("config", "Config", &config::config_path()?, &mut checks);
    check_file(
        "cli",
        "Codex Speak CLI",
        &config::bin_dir()?.join(binary_name()),
        &mut checks,
    );
    check_optional_path(
        "control_app",
        "Control app",
        &config::control_app_path()?,
        &mut checks,
    );
    if config::pet_helper_supported() {
        check_optional_path(
            "pet_helper",
            "Native pet helper",
            &config::pet_helper_path()?,
            &mut checks,
        );
    } else {
        checks.push(DoctorCheck::skip(
            "pet_helper",
            "Native pet helper",
            "macOS only",
        ));
    }
    check_file(
        "sherpa_tts",
        "Sherpa TTS",
        &config::sherpa_bin()?,
        &mut checks,
    );
    check_file(
        "melo_model",
        "Melo model",
        &config::model_dir()?.join("model.onnx"),
        &mut checks,
    );
    check_file(
        "melo_lexicon",
        "Melo lexicon",
        &config::model_dir()?.join("lexicon.txt"),
        &mut checks,
    );
    check_file(
        "melo_tokens",
        "Melo tokens",
        &config::model_dir()?.join("tokens.txt"),
        &mut checks,
    );
    check_notify(&mut checks)?;
    check_file(
        "plugin",
        "Codex Speak plugin",
        &config::installed_plugin_dir()?.join(".codex-plugin/plugin.json"),
        &mut checks,
    );
    check_file(
        "plugin_skill",
        "Codex Speak plugin skill",
        &config::installed_plugin_dir()?.join("skills/codex-speak/SKILL.md"),
        &mut checks,
    );
    check_file(
        "plugin_mcp_config",
        "Codex Speak MCP config",
        &config::installed_plugin_dir()?.join(".mcp.json"),
        &mut checks,
    );
    check_file(
        "plugin_mcp_script",
        "Codex Speak MCP script",
        &config::installed_plugin_dir()?.join(mcp_script_path()),
        &mut checks,
    );
    check_marketplace(&mut checks)?;
    check_player(&mut checks);

    Ok(DoctorReport::new(checks))
}

pub fn player_available() -> bool {
    if cfg!(target_os = "macos") {
        Path::new("/usr/bin/afplay").is_file()
    } else if cfg!(windows) {
        Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", "exit 0"])
            .status()
            .is_ok_and(|status| status.success())
    } else {
        false
    }
}

impl DoctorReport {
    fn new(checks: Vec<DoctorCheck>) -> Self {
        let ok = checks
            .iter()
            .all(|check| !check.required || check.status != CheckStatus::Fail);
        Self {
            ok,
            version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            generated_at: Local::now().to_rfc3339(),
            checks,
        }
    }
}

impl DoctorCheck {
    fn ok(id: &'static str, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            id,
            label,
            status: CheckStatus::Ok,
            required: true,
            detail: detail.into(),
            hint: None,
        }
    }

    fn warn(id: &'static str, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            id,
            label,
            status: CheckStatus::Warn,
            required: false,
            detail: detail.into(),
            hint: hint_for(id),
        }
    }

    fn fail(id: &'static str, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            id,
            label,
            status: CheckStatus::Fail,
            required: true,
            detail: detail.into(),
            hint: hint_for(id),
        }
    }

    fn skip(id: &'static str, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            id,
            label,
            status: CheckStatus::Skip,
            required: false,
            detail: detail.into(),
            hint: None,
        }
    }
}

fn print_text(report: &DoctorReport) {
    for check in &report.checks {
        let prefix = match check.status {
            CheckStatus::Ok => "OK  ",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
            CheckStatus::Skip => "SKIP",
        };
        println!("{prefix} {}: {}", check.label, check.detail);
        if let Some(hint) = check.hint {
            println!("     hint: {hint}");
        }
    }
}

fn hint_for(id: &str) -> Option<&'static str> {
    match id {
        "codex_home" | "config" | "cli" => {
            Some("Run the Codex Speak installer again from the release package.")
        }
        "control_app" => Some(
            "Rerun the installer without --skip-control-app, or install from a release package that includes the desktop app.",
        ),
        "pet_helper" => Some(
            "On macOS, rerun install-macos.sh without --skip-control-app so the native desktop pet helper is copied or built.",
        ),
        "sherpa_tts" | "melo_model" | "melo_lexicon" | "melo_tokens" => Some(
            "Run `codex-speak models install --provider sherpa_melo`, or switch the provider to `system` for a no-download fallback.",
        ),
        "codex_notify" => Some(
            "Run `codex-speak install` so the Codex notify hook points at codex-speak-notify.",
        ),
        "plugin" | "plugin_skill" | "plugin_mcp_config" | "plugin_mcp_script" => Some(
            "Run `codex-speak install` to refresh the local Codex Speak plugin files.",
        ),
        "plugin_marketplace" => Some(
            "Run `codex-speak install` to add Codex Speak to the personal plugin marketplace.",
        ),
        "player" => Some(
            "macOS needs /usr/bin/afplay. Windows needs powershell.exe available for SoundPlayer playback.",
        ),
        _ => None,
    }
}

fn check_notify(checks: &mut Vec<DoctorCheck>) -> Result<()> {
    let path = config::codex_home()?.join("config.toml");
    let raw = fs::read_to_string(&path).unwrap_or_default();
    if raw.contains("codex-speak-notify") {
        checks.push(DoctorCheck::ok(
            "codex_notify",
            "Codex notify",
            "codex-speak-notify is configured",
        ));
    } else {
        checks.push(DoctorCheck::fail(
            "codex_notify",
            "Codex notify",
            format!("codex-speak-notify not found in {}", path.display()),
        ));
    }
    Ok(())
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "codex-speak.exe"
    } else {
        "codex-speak"
    }
}

fn mcp_script_path() -> &'static str {
    if cfg!(windows) {
        "scripts/codex-speak-mcp.ps1"
    } else {
        "scripts/codex-speak-mcp"
    }
}

fn check_dir(id: &'static str, label: &'static str, path: &Path, checks: &mut Vec<DoctorCheck>) {
    if path.is_dir() {
        checks.push(DoctorCheck::ok(id, label, path.display().to_string()));
    } else {
        checks.push(DoctorCheck::fail(
            id,
            label,
            format!("missing {}", path.display()),
        ));
    }
}

fn check_optional_path(
    id: &'static str,
    label: &'static str,
    path: &Path,
    checks: &mut Vec<DoctorCheck>,
) {
    if path.exists() {
        checks.push(DoctorCheck::ok(id, label, path.display().to_string()));
    } else {
        checks.push(DoctorCheck::warn(
            id,
            label,
            format!("missing {}", path.display()),
        ));
    }
}

fn check_file(id: &'static str, label: &'static str, path: &Path, checks: &mut Vec<DoctorCheck>) {
    if path.is_file() {
        checks.push(DoctorCheck::ok(id, label, path.display().to_string()));
    } else {
        checks.push(DoctorCheck::fail(
            id,
            label,
            format!("missing {}", path.display()),
        ));
    }
}

fn check_marketplace(checks: &mut Vec<DoctorCheck>) -> Result<()> {
    let path = config::personal_marketplace_path()?;
    let raw = fs::read_to_string(&path).unwrap_or_default();
    if raw.contains("\"codex-speak\"") {
        checks.push(DoctorCheck::ok(
            "plugin_marketplace",
            "Plugin marketplace",
            "codex-speak is configured",
        ));
    } else {
        checks.push(DoctorCheck::fail(
            "plugin_marketplace",
            "Plugin marketplace",
            format!("codex-speak not found in {}", path.display()),
        ));
    }
    Ok(())
}

fn check_player(checks: &mut Vec<DoctorCheck>) {
    if cfg!(target_os = "macos") {
        let path = Path::new("/usr/bin/afplay");
        check_file("player", "Player", path, checks);
    } else if cfg!(windows) {
        if player_available() {
            checks.push(DoctorCheck::ok(
                "player",
                "Player",
                "Windows PowerShell SoundPlayer",
            ));
        } else {
            checks.push(DoctorCheck::fail(
                "player",
                "Player",
                "powershell.exe is not available",
            ));
        }
    } else {
        checks.push(DoctorCheck::fail(
            "player",
            "Player",
            "unsupported platform",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{CheckStatus, DoctorCheck, DoctorReport};

    #[test]
    fn required_fail_makes_report_not_ok() {
        let report = DoctorReport::new(vec![DoctorCheck::fail("config", "Config", "missing")]);
        assert!(!report.ok);
    }

    #[test]
    fn optional_warn_does_not_fail_report() {
        let report = DoctorReport::new(vec![DoctorCheck::warn(
            "control_app",
            "Control app",
            "missing",
        )]);
        assert!(report.ok);
        assert_eq!(report.checks[0].status, CheckStatus::Warn);
    }

    #[test]
    fn report_includes_runtime_metadata() {
        let report = DoctorReport::new(Vec::new());
        assert_eq!(report.version, env!("CARGO_PKG_VERSION"));
        assert!(!report.os.is_empty());
        assert!(!report.arch.is_empty());
        assert!(!report.generated_at.is_empty());
    }

    #[test]
    fn failed_checks_include_actionable_hint() {
        let check = DoctorCheck::fail("plugin_mcp_script", "Codex Speak MCP script", "missing");
        assert!(check.hint.is_some());
    }

    #[test]
    fn ok_checks_do_not_include_hint() {
        let check = DoctorCheck::ok("plugin", "Codex Speak plugin", "present");
        assert!(check.hint.is_none());
    }
}
