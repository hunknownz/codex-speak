use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;
use walkdir::WalkDir;

use crate::config::{self, Config};
use crate::{extract, side_channel};

pub fn resolve_text(text: Option<String>, fixture: Option<&Path>, cfg: &Config) -> Result<String> {
    if let Some(text) = text {
        return Ok(extract::clean_for_speech(&text, cfg.max_read_chars));
    }

    if fixture.is_none() {
        if let Some(text) = side_channel::read_fresh_latest(cfg.max_read_chars)? {
            return Ok(text);
        }
    }

    let raw = if let Some(path) = fixture {
        last_message_from_file(path)?
    } else {
        let path = newest_session_file()?;
        last_message_from_file(&path)?
    };

    let cleaned = extract::clean_for_speech(&raw, cfg.max_read_chars);
    if cleaned.is_empty() {
        Ok("Codex 说：做好啦。".to_string())
    } else {
        Ok(cleaned)
    }
}

pub fn newest_session_file() -> Result<PathBuf> {
    let root = config::codex_home()?.join("sessions");
    let mut files = Vec::new();
    if !root.exists() {
        anyhow::bail!("Codex sessions directory not found: {}", root.display());
    }

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            let modified = entry.metadata()?.modified()?;
            files.push((modified, path.to_path_buf()));
        }
    }

    files.sort_by_key(|(modified, _)| *modified);
    files
        .pop()
        .map(|(_, path)| path)
        .context("no Codex session jsonl files found")
}

pub fn last_message_from_file(path: &Path) -> Result<String> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let lines: Vec<&str> = raw.lines().collect();
    for line in lines.iter().rev() {
        let Ok(item) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(message) = message_from_event(&item) {
            if !message.trim().is_empty() {
                return Ok(message);
            }
        }
    }
    anyhow::bail!("no assistant message found in {}", path.display())
}

fn message_from_event(item: &Value) -> Option<String> {
    let payload = item.get("payload").unwrap_or(item);
    let payload_type = payload
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if payload_type == "task_complete" {
        if let Some(message) = payload.get("last_agent_message").and_then(Value::as_str) {
            return Some(message.to_string());
        }
    }

    if payload_type == "agent_message" {
        let phase = payload
            .get("phase")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if phase == "final_answer" {
            if let Some(message) = payload.get("message").and_then(Value::as_str) {
                return Some(message.to_string());
            }
        }
    }

    find_string_key(payload, "last_agent_message").or_else(|| find_string_key(payload, "message"))
}

fn find_string_key(value: &Value, key: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(found) = map.get(key).and_then(Value::as_str) {
                return Some(found.to_string());
            }
            map.values().find_map(|v| find_string_key(v, key))
        }
        Value::Array(values) => values.iter().find_map(|v| find_string_key(v, key)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn reads_task_complete_message() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"payload":{{"type":"task_complete","last_agent_message":"<!-- codex-speak\n你好\n-->"}}}}"#
        )
        .unwrap();
        let message = last_message_from_file(file.path()).unwrap();
        assert!(message.contains("codex-speak"));
    }
}
