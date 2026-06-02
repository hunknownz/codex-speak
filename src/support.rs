use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};

use crate::{config, doctor, pet_state, pronunciation, status};

const MAX_TEXT_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct SupportBundleOptions {
    pub include_private: bool,
}

impl Default for SupportBundleOptions {
    fn default() -> Self {
        Self {
            include_private: false,
        }
    }
}

pub fn write_bundle(output: Option<&Path>, options: SupportBundleOptions) -> Result<PathBuf> {
    let dir = match output {
        Some(path) => path.to_path_buf(),
        None => default_bundle_dir()?,
    };
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let redactor = Redactor::new(options.include_private)?;
    write_text(
        &dir.join("README.txt"),
        &bundle_readme(options.include_private),
    )?;
    write_json(
        &dir.join("support-bundle-metadata.json"),
        &bundle_metadata(options.include_private),
    )?;
    write_json_redacted(&dir.join("doctor.json"), &doctor::collect()?, &redactor)?;
    write_environment(&dir, &redactor)?;
    write_release_manifest(&dir)?;
    write_config_and_status(&dir, &redactor)?;
    write_pronunciation_dictionary(&dir, &redactor, options.include_private)?;
    write_recent_logs(&dir, &redactor, options.include_private)?;

    Ok(dir)
}

fn default_bundle_dir() -> Result<PathBuf> {
    let ts = Local::now().format("%Y%m%d-%H%M%S");
    Ok(config::app_home()?
        .join("support")
        .join(format!("codex-speak-support-{ts}")))
}

fn write_release_manifest(dir: &Path) -> Result<()> {
    let source = config::release_manifest_path()?;
    if source.is_file() {
        let source_display = source.display().to_string();
        fs::copy(&source, dir.join("release-manifest.json"))
            .with_context(|| format!("failed to copy release manifest {source_display}"))?;
    } else {
        write_text(
            &dir.join("release-manifest-missing.txt"),
            "release-manifest.json was not found in the installed Codex Speak directory.\n\
             This usually means Codex Speak was installed from source or from an older package.\n",
        )?;
    }
    Ok(())
}

fn write_pronunciation_dictionary(
    dir: &Path,
    redactor: &Redactor,
    include_private: bool,
) -> Result<()> {
    let path = pronunciation::dictionary_path()?;
    if !path.exists() {
        write_json_redacted(
            &dir.join("pronunciation-dictionary.json"),
            &json!({
                "configured": false,
                "redacted": !include_private,
                "path": path.display().to_string(),
                "terms": 0
            }),
            redactor,
        )?;
        return Ok(());
    }

    match pronunciation::load_user_dictionary() {
        Ok(dictionary) if include_private => write_json_redacted(
            &dir.join("pronunciation-dictionary.json"),
            &dictionary,
            redactor,
        )?,
        Ok(dictionary) => write_json_redacted(
            &dir.join("pronunciation-dictionary.json"),
            &json!({
                "configured": true,
                "redacted": true,
                "path": path.display().to_string(),
                "terms": dictionary.terms.len()
            }),
            redactor,
        )?,
        Err(err) => write_text(
            &dir.join("pronunciation-dictionary-error.txt"),
            &format!("{err:#}\n"),
        )?,
    }
    Ok(())
}

fn write_config_and_status(dir: &Path, redactor: &Redactor) -> Result<()> {
    match config::Config::load_or_default() {
        Ok(cfg) => {
            let mut safe_cfg = cfg.clone();
            safe_cfg.previous_notify = None;
            write_json_redacted(&dir.join("config.json"), &safe_cfg, redactor)?;
            write_json_redacted(&dir.join("status.json"), &status::collect(&cfg)?, redactor)?;
        }
        Err(err) => write_text(&dir.join("config-error.txt"), &err.to_string())?,
    }
    write_json_redacted(
        &dir.join("models.json"),
        &status::provider_statuses()?,
        redactor,
    )?;
    write_json_redacted(
        &dir.join("pet-state.json"),
        &pet_state::read_state().unwrap_or_default(),
        redactor,
    )?;
    Ok(())
}

fn write_environment(dir: &Path, redactor: &Redactor) -> Result<()> {
    write_json_redacted(
        &dir.join("environment.json"),
        &json!({
            "version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "app_home": config::app_home()?.display().to_string(),
            "codex_home": config::codex_home()?.display().to_string(),
            "agents_home": config::agents_home()?.display().to_string()
        }),
        redactor,
    )
}

fn write_recent_logs(dir: &Path, redactor: &Redactor, include_private: bool) -> Result<()> {
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
            if name == "last-spoken.txt" && !include_private {
                write_text(
                    &target.join(name),
                    "[redacted by codex-speak support-bundle]\n\
                     Recent spoken text can include private user content.\n\
                     Re-run with `codex-speak support-bundle --include-private` only if you explicitly want to share it.\n",
                )?;
                continue;
            }
            let raw = fs::read_to_string(&source)
                .unwrap_or_else(|_| "<binary or unreadable log>".to_string());
            let redacted = redactor.redact_text(&raw);
            write_text(
                &target.join(name),
                &truncate_text(&redacted, MAX_TEXT_BYTES),
            )?;
        }
    }
    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let raw = serde_json::to_string_pretty(value)?;
    write_text(path, &raw)
}

fn write_json_redacted(path: &Path, value: &impl Serialize, redactor: &Redactor) -> Result<()> {
    let mut value = serde_json::to_value(value)?;
    redact_json_value(&mut value, redactor);
    write_json(path, &value)
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))
}

fn bundle_readme(include_private: bool) -> String {
    let privacy = if include_private {
        "This bundle was created with --include-private, so it may contain local paths and recent spoken text.\n"
    } else {
        "This bundle is redacted by default: local home paths and recent spoken text are replaced before sharing.\n\
         For deep debugging only, re-run `codex-speak support-bundle --include-private` to include raw local paths and logs.\n"
    };
    format!(
        "Codex Speak support bundle\n\
         {privacy}\
         Useful files: doctor.json, status.json, models.json, environment.json, pronunciation-dictionary.json, release-manifest.json, support-bundle-metadata.json, and logs/*.log.\n"
    )
}

fn bundle_metadata(include_private: bool) -> Value {
    json!({
        "schemaVersion": 1,
        "product": "codex-speak",
        "version": env!("CARGO_PKG_VERSION"),
        "generatedAt": Local::now().to_rfc3339(),
        "redacted": !include_private,
        "includePrivate": include_private,
        "redaction": {
            "localPaths": !include_private,
            "recentSpokenText": !include_private
        }
    })
}

struct Redactor {
    include_private: bool,
    replacements: Vec<(String, &'static str)>,
}

impl Redactor {
    fn new(include_private: bool) -> Result<Self> {
        let mut replacements = Vec::new();
        for (path, placeholder) in [
            (config::app_home()?, "<codex-speak-home>"),
            (config::codex_home()?, "<codex-home>"),
            (config::agents_home()?, "<agents-home>"),
            (config::home_dir()?, "<home>"),
        ] {
            push_path_replacement(&mut replacements, path, placeholder);
        }
        replacements.sort_by(|left, right| right.0.len().cmp(&left.0.len()));
        Ok(Self {
            include_private,
            replacements,
        })
    }

    fn redact_text(&self, text: &str) -> String {
        if self.include_private {
            return text.to_string();
        }
        let mut result = text.to_string();
        for (needle, replacement) in &self.replacements {
            result = result.replace(needle, replacement);
        }
        redact_generic_paths(&result)
    }
}

fn redact_generic_paths(text: &str) -> String {
    let unix_home = Regex::new(r"/(?:Users|home)/[^/\s:]+(?:/[^\s:]+)*")
        .expect("generic Unix home path regex should compile");
    let windows_home = Regex::new(r"(?i)[A-Z]:\\Users\\[^\\\s:]+(?:\\[^\s:]+)*")
        .expect("generic Windows home path regex should compile");
    let text = unix_home.replace_all(text, "<local-path>");
    windows_home.replace_all(&text, "<local-path>").into_owned()
}

fn push_path_replacement(
    replacements: &mut Vec<(String, &'static str)>,
    path: PathBuf,
    placeholder: &'static str,
) {
    let text = path.display().to_string();
    if !text.is_empty() {
        replacements.push((text.clone(), placeholder));
        if text.contains('\\') {
            replacements.push((text.replace('\\', "/"), placeholder));
        }
    }
}

fn redact_json_value(value: &mut Value, redactor: &Redactor) {
    match value {
        Value::String(text) => {
            *text = redactor.redact_text(text);
        }
        Value::Array(items) => {
            for item in items {
                redact_json_value(item, redactor);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                if key == "last_spoken" && !redactor.include_private {
                    *item = Value::String("<redacted recent spoken text>".to_string());
                } else {
                    redact_json_value(item, redactor);
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
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
    use super::{redact_generic_paths, redact_json_value, truncate_text, Redactor};
    use serde_json::json;

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

    #[test]
    fn redacts_configured_paths_in_text() {
        let redactor = Redactor {
            include_private: false,
            replacements: vec![("/Users/example/.codex".to_string(), "<codex-home>")],
        };
        assert_eq!(
            redactor.redact_text("/Users/example/.codex/codex-speak"),
            "<codex-home>/codex-speak"
        );
    }

    #[test]
    fn redacts_generic_home_paths_from_tool_logs() {
        assert_eq!(
            redact_generic_paths("/Users/runner/work/file.cc:120"),
            "<local-path>:120"
        );
        assert_eq!(
            redact_generic_paths(r"C:\Users\runner\work\file.cc:120"),
            "<local-path>:120"
        );
    }

    #[test]
    fn include_private_keeps_text() {
        let redactor = Redactor {
            include_private: true,
            replacements: vec![("/Users/example".to_string(), "<home>")],
        };
        assert_eq!(
            redactor.redact_text("/Users/example/file"),
            "/Users/example/file"
        );
    }

    #[test]
    fn redacts_last_spoken_json_field() {
        let redactor = Redactor {
            include_private: false,
            replacements: Vec::new(),
        };
        let mut value = json!({"last_spoken": "secret spoken text"});
        redact_json_value(&mut value, &redactor);
        assert_eq!(value["last_spoken"], json!("<redacted recent spoken text>"));
    }
}
