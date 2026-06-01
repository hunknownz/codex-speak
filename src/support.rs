use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use serde::Serialize;
use serde_json::json;

use crate::{config, doctor, pet_state, status};

const MAX_TEXT_BYTES: usize = 32 * 1024;

pub fn write_bundle(output: Option<&Path>) -> Result<PathBuf> {
    let dir = match output {
        Some(path) => path.to_path_buf(),
        None => default_bundle_dir()?,
    };
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    write_text(
        &dir.join("README.txt"),
        "Codex Speak support bundle\n\
         Review these files before sharing them. They may include local paths and recent speech logs.\n\
         Useful files: doctor.json, status.json, models.json, environment.json, and logs/*.log.\n",
    )?;
    write_json(&dir.join("doctor.json"), &doctor::collect()?)?;
    write_environment(&dir)?;
    write_config_and_status(&dir)?;
    write_recent_logs(&dir)?;

    Ok(dir)
}

fn default_bundle_dir() -> Result<PathBuf> {
    let ts = Local::now().format("%Y%m%d-%H%M%S");
    Ok(config::app_home()?
        .join("support")
        .join(format!("codex-speak-support-{ts}")))
}

fn write_config_and_status(dir: &Path) -> Result<()> {
    match config::Config::load_or_default() {
        Ok(cfg) => {
            let mut safe_cfg = cfg.clone();
            safe_cfg.previous_notify = None;
            write_json(&dir.join("config.json"), &safe_cfg)?;
            write_json(&dir.join("status.json"), &status::collect(&cfg)?)?;
        }
        Err(err) => write_text(&dir.join("config-error.txt"), &err.to_string())?,
    }
    write_json(&dir.join("models.json"), &status::provider_statuses()?)?;
    write_json(
        &dir.join("pet-state.json"),
        &pet_state::read_state().unwrap_or_default(),
    )?;
    Ok(())
}

fn write_environment(dir: &Path) -> Result<()> {
    write_json(
        &dir.join("environment.json"),
        &json!({
            "version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "app_home": config::app_home()?.display().to_string(),
            "codex_home": config::codex_home()?.display().to_string(),
            "agents_home": config::agents_home()?.display().to_string()
        }),
    )
}

fn write_recent_logs(dir: &Path) -> Result<()> {
    let logs_dir = config::logs_dir()?;
    let target = dir.join("logs");
    fs::create_dir_all(&target)?;
    for name in [
        "last-spoken.txt",
        "last-error.log",
        "last-tts.stdout.log",
        "last-tts.stderr.log",
    ] {
        let source = logs_dir.join(name);
        if source.is_file() {
            let raw = fs::read_to_string(&source)
                .unwrap_or_else(|_| "<binary or unreadable log>".to_string());
            write_text(&target.join(name), &truncate_text(&raw, MAX_TEXT_BYTES))?;
        }
    }
    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let raw = serde_json::to_string_pretty(value)?;
    write_text(path, &raw)
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))
}

fn truncate_text(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n\n[truncated by codex-speak support-bundle]\n",
        &text[..end]
    )
}

#[cfg(test)]
mod tests {
    use super::truncate_text;

    #[test]
    fn truncates_without_splitting_utf8() {
        let text = "你好，Codex Speak";
        let truncated = truncate_text(text, 8);
        assert!(truncated.starts_with("你好"));
        assert!(truncated.contains("truncated"));
    }

    #[test]
    fn keeps_short_text_unchanged() {
        assert_eq!(truncate_text("ok", 32), "ok");
    }
}
