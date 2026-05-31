use std::fs;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config;
use crate::extract;

const FRESH_SECONDS: u64 = 180;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakItem {
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakSpool {
    pub version: u8,
    pub audience: Option<String>,
    pub style: Option<String>,
    pub lang: Option<String>,
    pub source: Option<String>,
    pub items: Vec<SpeakItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrittenSpool {
    pub path: String,
    pub text: String,
}

pub fn write_latest(spool: &SpeakSpool, max_chars: usize) -> Result<WrittenSpool> {
    let text = spool_text(spool, max_chars);
    if text.is_empty() {
        anyhow::bail!("side-channel text is empty");
    }

    let dir = config::spool_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join("latest.json");
    let raw = serde_json::to_string_pretty(spool)?;
    fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))?;

    Ok(WrittenSpool {
        path: path.display().to_string(),
        text,
    })
}

pub fn read_fresh_latest(max_chars: usize, consume: bool) -> Result<Option<String>> {
    let path = config::spool_dir()?.join("latest.json");
    if !path.exists() {
        return Ok(None);
    }

    let modified = fs::metadata(&path)?.modified()?;
    if !is_fresh(modified) {
        return Ok(None);
    }

    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let spool: SpeakSpool = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let text = spool_text(&spool, max_chars);
    if text.is_empty() {
        Ok(None)
    } else {
        if consume {
            consume_latest(&path)?;
        }
        Ok(Some(text))
    }
}

pub fn spool_text(spool: &SpeakSpool, max_chars: usize) -> String {
    let joined = spool
        .items
        .iter()
        .filter(|item| is_allowed_role(&item.role))
        .map(|item| item.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("");
    extract::truncate_chars(&joined, max_chars)
}

fn is_fresh(modified: SystemTime) -> bool {
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return true;
    };
    age <= Duration::from_secs(FRESH_SECONDS)
}

fn is_allowed_role(role: &str) -> bool {
    matches!(
        role,
        "did" | "why" | "code-summary" | "command-summary" | "result" | "next" | "warning"
    )
}

fn consume_latest(path: &std::path::Path) -> Result<()> {
    let consumed_path = config::spool_dir()?.join("last-consumed.json");
    if consumed_path.exists() {
        fs::remove_file(&consumed_path)?;
    }
    fs::rename(path, &consumed_path)
        .or_else(|_| {
            fs::copy(path, &consumed_path)?;
            fs::remove_file(path)
        })
        .with_context(|| format!("failed to consume {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_allowed_side_channel_items() {
        let spool = SpeakSpool {
            version: 1,
            audience: Some("beginner".to_string()),
            style: Some("clear-bright".to_string()),
            lang: Some("zh-CN".to_string()),
            source: Some("test".to_string()),
            items: vec![
                SpeakItem {
                    role: "did".to_string(),
                    text: "我做了插件。".to_string(),
                },
                SpeakItem {
                    role: "unknown".to_string(),
                    text: "不要读。".to_string(),
                },
                SpeakItem {
                    role: "next".to_string(),
                    text: "下一步可以测试。".to_string(),
                },
            ],
        };

        assert_eq!(spool_text(&spool, 200), "我做了插件。下一步可以测试。");
    }
}
