use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config;
use crate::extract;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetState {
    pub state: String,
    pub message: Option<String>,
    pub source: String,
    pub updated_at_ms: u64,
}

impl Default for PetState {
    fn default() -> Self {
        Self {
            state: "idle".to_string(),
            message: Some("我在这里，等 Codex 回复。".to_string()),
            source: "default".to_string(),
            updated_at_ms: now_ms(),
        }
    }
}

pub fn read_state() -> Result<PetState> {
    let path = config::pet_state_path()?;
    if !path.exists() {
        return Ok(PetState::default());
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn write_state(state: &str, message: Option<&str>, source: &str) -> Result<PetState> {
    validate_state(state)?;
    let pet_state = PetState {
        state: state.to_string(),
        message: message.map(|text| extract::truncate_chars(text.trim(), 140)),
        source: source.to_string(),
        updated_at_ms: now_ms(),
    };

    let path = config::pet_state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(&pet_state)?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(pet_state)
}

fn validate_state(state: &str) -> Result<()> {
    match state {
        "idle" | "ready" | "speaking" | "done" | "error" => Ok(()),
        other => anyhow::bail!("unsupported pet state: {other}"),
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_state() {
        assert!(validate_state("thinking").is_err());
    }

    #[test]
    fn default_state_is_idle() {
        assert_eq!(PetState::default().state, "idle");
    }
}
