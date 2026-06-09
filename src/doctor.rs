use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use chrono::Local;
use serde::Serialize;

use crate::{bundled, config, pronunciation};

const MODEL_CHECK_IDS: &[&str] = &["sherpa_tts", "melo_model", "melo_lexicon", "melo_tokens"];

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

pub fn verify_install(allow_missing_models: bool) -> Result<()> {
    let report = collect()?;
    print_install_verification(&report, allow_missing_models);

    let failures = blocking_failures(&report, allow_missing_models);
    if failures.is_empty() {
        println!("Install verification passed.");
        return Ok(());
    }

    println!();
    println!("Install verification failed. Blocking checks:");
    for check in failures {
        println!("- {} ({})", check.label, check.id);
    }
    println!(
        "Run `codex-speak support-bundle` and share the output directory for troubleshooting."
    );

    anyhow::bail!("install verification failed");
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
    check_pronunciation_dictionary(&mut checks)?;
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
    check_stop_hook(&mut checks)?;
    check_file_matches(
        "codex_stop_hook_script",
        "Codex Stop hook script",
        &hook_path()?,
        &bundled::hook_content(&config::bin_dir()?.join(binary_name())),
        &mut checks,
    );
    check_agents_hint(&mut checks)?;
    check_queue(&mut checks)?;
    check_file_matches(
        "codex_skill",
        "Codex Speak skill",
        &config::codex_home()?.join("skills/codex-speak/SKILL.md"),
        bundled::CODEX_SKILL,
        &mut checks,
    );
    check_legacy_mcp_config(&mut checks)?;
    check_legacy_plugin_install(&mut checks)?;
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

fn print_install_verification(report: &DoctorReport, allow_missing_models: bool) {
    println!(
        "Codex Speak install verification {} on {} {}",
        report.version, report.os, report.arch
    );

    for check in &report.checks {
        let allowed_model_failure =
            allow_missing_models && check.status == CheckStatus::Fail && is_model_check(check.id);
        let prefix = match (check.status, allowed_model_failure) {
            (_, true) => "WARN",
            (CheckStatus::Ok, false) => "OK  ",
            (CheckStatus::Warn, false) => "WARN",
            (CheckStatus::Fail, false) => "FAIL",
            (CheckStatus::Skip, false) => "SKIP",
        };
        println!("{prefix} {}: {}", check.label, check.detail);

        if allowed_model_failure {
            println!(
                "     hint: model check allowed because --allow-missing-models was set; run `codex-speak models install --provider sherpa_melo` later."
            );
        } else if let Some(hint) = check.hint {
            println!("     hint: {hint}");
        }
    }
}

fn blocking_failures<'a>(
    report: &'a DoctorReport,
    allow_missing_models: bool,
) -> Vec<&'a DoctorCheck> {
    report
        .checks
        .iter()
        .filter(|check| check.required && check.status == CheckStatus::Fail)
        .filter(|check| !(allow_missing_models && is_model_check(check.id)))
        .collect()
}

fn is_model_check(id: &str) -> bool {
    MODEL_CHECK_IDS.contains(&id)
}

fn hint_for(id: &str) -> Option<&'static str> {
    match id {
        "codex_home" | "config" | "cli" => {
            Some("Run the Codex Speak installer again from the release package.")
        }
        "pronunciation_dictionary" => Some(
            "Fix ~/.codex/codex-speak/pronunciation.toml, or recreate entries with `codex-speak pronunciation set --term ... --spoken ...`.",
        ),
        "control_app" => Some(
            "Rerun the installer without --skip-control-app, or install from a release package that includes the desktop app.",
        ),
        "pet_helper" => Some(
            "On macOS, rerun install-macos.sh without --skip-control-app so the native desktop pet helper is copied or built.",
        ),
        "sherpa_tts" | "melo_model" | "melo_lexicon" | "melo_tokens" => Some(
            "Run `codex-speak models install --provider sherpa_melo`, or switch the provider to `system` for a no-download fallback.",
        ),
        "codex_stop_hook" => Some(
            "Run `codex-speak install` so Codex runs the Codex Speak Stop hook.",
        ),
        "codex_stop_hook_script" => Some(
            "Run `codex-speak install` so the Stop hook wrapper matches the current CLI.",
        ),
        "codex_agents_hint" => Some(
            "Run `codex-speak install` so ~/.codex/AGENTS.md contains the growth-mode hint.",
        ),
        "speech_queue" => Some(
            "Run `codex-speak install` so the local speech queue directories are created.",
        ),
        "codex_skill" => Some(
            "Run `codex-speak install` to refresh the Codex Speak skill.",
        ),
        "legacy_codex_global_mcp" => Some(
            "Run `codex-speak uninstall` or remove the legacy mcp_servers.codex_speak block if you want Hook-first only.",
        ),
        "legacy_codex_plugin_install" => Some(
            "The plugin path is legacy for Hook-first mode. Remove codex-speak@personal if it still auto-loads MCP.",
        ),
        "player" => Some(
            "macOS needs /usr/bin/afplay. Windows needs powershell.exe available for SoundPlayer playback.",
        ),
        _ => None,
    }
}

fn check_stop_hook(checks: &mut Vec<DoctorCheck>) -> Result<()> {
    let path = config::codex_home()?.join("hooks.json");
    let raw = fs::read_to_string(&path).unwrap_or_default();
    if raw.contains("codex-speak-stop-hook") && raw.contains("\"Stop\"") {
        checks.push(DoctorCheck::ok(
            "codex_stop_hook",
            "Codex Stop hook",
            "Codex Speak Stop hook is configured",
        ));
    } else {
        checks.push(DoctorCheck::fail(
            "codex_stop_hook",
            "Codex Stop hook",
            format!("Codex Speak Stop hook not found in {}", path.display()),
        ));
    }
    Ok(())
}

fn check_agents_hint(checks: &mut Vec<DoctorCheck>) -> Result<()> {
    let path = config::codex_home()?.join("AGENTS.md");
    let raw = fs::read_to_string(&path).unwrap_or_default();
    if crate::install::agents_block_present(&raw) {
        checks.push(DoctorCheck::ok(
            "codex_agents_hint",
            "Codex AGENTS hint",
            "growth-mode hint is installed",
        ));
    } else {
        checks.push(DoctorCheck::fail(
            "codex_agents_hint",
            "Codex AGENTS hint",
            format!("growth-mode hint not found in {}", path.display()),
        ));
    }
    Ok(())
}

fn check_queue(checks: &mut Vec<DoctorCheck>) -> Result<()> {
    for dir in [
        config::queue_pending_dir()?,
        config::queue_running_dir()?,
        config::queue_done_dir()?,
        config::queue_failed_dir()?,
    ] {
        fs::create_dir_all(&dir)?;
    }
    checks.push(DoctorCheck::ok(
        "speech_queue",
        "Speech queue",
        config::queue_dir()?.display().to_string(),
    ));
    Ok(())
}

fn check_pronunciation_dictionary(checks: &mut Vec<DoctorCheck>) -> Result<()> {
    let path = config::pronunciation_dictionary_path()?;
    if !path.exists() {
        checks.push(DoctorCheck::skip(
            "pronunciation_dictionary",
            "Pronunciation dictionary",
            format!("not configured at {}", path.display()),
        ));
        return Ok(());
    }

    match pronunciation::load_user_dictionary() {
        Ok(dictionary) => checks.push(DoctorCheck::ok(
            "pronunciation_dictionary",
            "Pronunciation dictionary",
            format!("{} term(s) in {}", dictionary.terms.len(), path.display()),
        )),
        Err(err) => checks.push(DoctorCheck::warn(
            "pronunciation_dictionary",
            "Pronunciation dictionary",
            format!("{err:#}"),
        )),
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

fn hook_path() -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "codex-speak-stop-hook.ps1"
    } else {
        "codex-speak-stop-hook"
    };
    Ok(config::codex_home()?.join("hooks").join(name))
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

fn check_file_matches(
    id: &'static str,
    label: &'static str,
    path: &Path,
    expected: &str,
    checks: &mut Vec<DoctorCheck>,
) {
    match fs::read_to_string(path) {
        Ok(actual) if actual == expected => {
            checks.push(DoctorCheck::ok(id, label, path.display().to_string()));
        }
        Ok(_) => {
            checks.push(DoctorCheck::fail(
                id,
                label,
                format!("{} differs from current CLI bundle", path.display()),
            ));
        }
        Err(_) => {
            checks.push(DoctorCheck::fail(
                id,
                label,
                format!("missing {}", path.display()),
            ));
        }
    }
}

fn check_legacy_mcp_config(checks: &mut Vec<DoctorCheck>) -> Result<()> {
    let path = config::codex_home()?.join("config.toml");
    let raw = fs::read_to_string(&path).unwrap_or_default();
    if raw.contains("[mcp_servers.codex_speak]") {
        checks.push(DoctorCheck::warn(
            "legacy_codex_global_mcp",
            "Legacy Codex global MCP",
            "mcp_servers.codex_speak is still configured",
        ));
    } else {
        checks.push(DoctorCheck::ok(
            "legacy_codex_global_mcp",
            "Legacy Codex global MCP",
            "not configured",
        ));
    }
    Ok(())
}

fn check_legacy_plugin_install(checks: &mut Vec<DoctorCheck>) -> Result<()> {
    let config_path = config::codex_home()?.join("config.toml");
    let config_raw = fs::read_to_string(&config_path).unwrap_or_default();
    if plugin_enabled_in_codex_config(&config_raw) {
        checks.push(DoctorCheck::warn(
            "legacy_codex_plugin_install",
            "Legacy Codex plugin install",
            "codex-speak@personal is still enabled",
        ));
    } else {
        checks.push(DoctorCheck::ok(
            "legacy_codex_plugin_install",
            "Legacy Codex plugin install",
            "not enabled",
        ));
    }
    Ok(())
}

fn plugin_enabled_in_codex_config(config_raw: &str) -> bool {
    let mut in_plugin_table = false;
    for line in config_raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_plugin_table = trimmed == "[plugins.\"codex-speak@personal\"]";
            continue;
        }
        if in_plugin_table && trimmed == "enabled = true" {
            return true;
        }
    }
    false
}

#[allow(dead_code)]
fn check_global_mcp_config(checks: &mut Vec<DoctorCheck>) -> Result<()> {
    let path = config::codex_home()?.join("config.toml");
    let raw = fs::read_to_string(&path).unwrap_or_default();
    let expected_cli = config::bin_dir()?.join(binary_name()).display().to_string();

    if global_mcp_config_matches(&raw, &expected_cli) {
        checks.push(DoctorCheck::ok(
            "codex_global_mcp",
            "Codex global MCP",
            "codex_speak is configured",
        ));
    } else {
        checks.push(DoctorCheck::fail(
            "codex_global_mcp",
            "Codex global MCP",
            format!(
                "mcp_servers.codex_speak missing or stale in {}",
                path.display()
            ),
        ));
    }
    Ok(())
}

fn global_mcp_config_matches(config_raw: &str, expected_cli: &str) -> bool {
    let Ok(value) = toml::from_str::<toml::Value>(config_raw) else {
        return false;
    };
    let Some(server) = value
        .get("mcp_servers")
        .and_then(|servers| servers.get("codex_speak"))
    else {
        return false;
    };
    let command_ok = server
        .get("command")
        .and_then(toml::Value::as_str)
        .is_some_and(|command| command == expected_cli);
    let args_ok = server
        .get("args")
        .and_then(toml::Value::as_array)
        .is_some_and(|args| args.len() == 1 && args[0].as_str() == Some("mcp"));
    command_ok && args_ok
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
    use super::{
        blocking_failures, global_mcp_config_matches, plugin_enabled_in_codex_config, CheckStatus,
        DoctorCheck, DoctorReport,
    };

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
    fn verify_can_allow_missing_models() {
        let report = DoctorReport::new(vec![
            DoctorCheck::ok("cli", "Codex Speak CLI", "present"),
            DoctorCheck::fail("melo_model", "Melo model", "missing"),
            DoctorCheck::fail("melo_tokens", "Melo tokens", "missing"),
        ]);

        assert!(!report.ok);
        assert!(blocking_failures(&report, true).is_empty());
        assert_eq!(blocking_failures(&report, false).len(), 2);
    }

    #[test]
    fn verify_does_not_allow_core_failures() {
        let report = DoctorReport::new(vec![
            DoctorCheck::fail("cli", "Codex Speak CLI", "missing"),
            DoctorCheck::fail("melo_model", "Melo model", "missing"),
        ]);

        let failures = blocking_failures(&report, true);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].id, "cli");
    }

    #[test]
    fn failed_checks_include_actionable_hint() {
        let check = DoctorCheck::fail("codex_stop_hook", "Codex Stop hook", "missing");
        assert!(check.hint.is_some());
    }

    #[test]
    fn stale_integration_checks_include_actionable_hint() {
        let check = DoctorCheck::fail("codex_skill", "Codex Speak skill", "stale");
        assert!(check.hint.is_some());
        let check = DoctorCheck::fail("codex_stop_hook_script", "Codex Stop hook script", "stale");
        assert!(check.hint.is_some());
    }

    #[test]
    fn ok_checks_do_not_include_hint() {
        let check = DoctorCheck::ok("player", "Player", "present");
        assert!(check.hint.is_none());
    }

    #[test]
    fn detects_enabled_codex_plugin_config_block() {
        let raw = r#"
[plugins."documents@openai-primary-runtime"]
enabled = true

[plugins."codex-speak@personal"]
enabled = true
"#;
        assert!(plugin_enabled_in_codex_config(raw));
        assert!(!plugin_enabled_in_codex_config(
            "[plugins.\"codex-speak@personal\"]\nenabled = false\n"
        ));
    }

    #[test]
    fn detects_global_mcp_server_config() {
        let raw = r#"
[mcp_servers.codex_speak]
args = ["mcp"]
command = "/Users/me/.codex/codex-speak/bin/codex-speak"
startup_timeout_sec = 120
"#;
        assert!(global_mcp_config_matches(
            raw,
            "/Users/me/.codex/codex-speak/bin/codex-speak"
        ));
        assert!(!global_mcp_config_matches(raw, "/other/codex-speak"));
    }
}
