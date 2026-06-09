use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;
use walkdir::WalkDir;

use crate::config::{self, Config};
use crate::{extract, side_channel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LatestAssistantActivity {
    Final,
    NonFinal,
    User,
    Unknown,
}

pub fn missing_side_channel_notice() -> String {
    missing_side_channel_notice_for_policy("diagnostic_notice")
}

pub fn missing_side_channel_notice_for_policy(policy: &str) -> String {
    match policy {
        "silent" => String::new(),
        "brief_notice" => {
            "这次回答完成了，但当前会话没有送来朗读导览。".to_string()
        }
        _ => "我没有收到本地插件准备好的朗读导览。这次先不乱读屏幕内容。请新开一个 Codex 会话，或者运行自检看看插件有没有加载。".to_string(),
    }
}

pub fn resolve_text(text: Option<String>, fixture: Option<&Path>, cfg: &Config) -> Result<String> {
    resolve_text_with_options(text, fixture, cfg, false)
}

pub fn resolve_text_for_speech(
    text: Option<String>,
    fixture: Option<&Path>,
    cfg: &Config,
) -> Result<String> {
    resolve_text_with_options(text, fixture, cfg, true)
}

fn resolve_text_with_options(
    text: Option<String>,
    fixture: Option<&Path>,
    cfg: &Config,
    consume_side_channel: bool,
) -> Result<String> {
    if let Some(text) = text {
        return Ok(extract::clean_for_speech(&text, cfg.max_read_chars));
    }

    if fixture.is_none() {
        if consume_side_channel
            && latest_assistant_activity().unwrap_or(LatestAssistantActivity::Final)
                == LatestAssistantActivity::NonFinal
        {
            return Ok(String::new());
        }
        if let Some(text) =
            side_channel::read_fresh_latest(cfg.max_read_chars, consume_side_channel)?
        {
            return Ok(text);
        }
        if consume_side_channel {
            return Ok(missing_side_channel_notice_for_policy(
                &cfg.missing_guide_policy,
            ));
        }
    }

    let raw = if let Some(path) = fixture {
        last_message_from_file(path)?
    } else {
        let path = newest_session_file()?;
        last_message_from_file(&path)?
    };

    let cleaned = if consume_side_channel {
        if extract::has_explicit_spoken_source(&raw) {
            extract::fallback_reply_guide(&raw, cfg.max_read_chars)
        } else {
            extract::conservative_reply_notice(&raw, cfg.max_read_chars)
        }
    } else {
        extract::clean_for_speech(&raw, cfg.max_read_chars)
    };
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
        if is_user_event(&item) {
            anyhow::bail!(
                "latest session activity is a user message, not an assistant final answer"
            );
        }
        if let Some(message) = message_from_event(&item) {
            if !message.trim().is_empty() {
                return Ok(message);
            }
        }
    }
    anyhow::bail!("no assistant message found in {}", path.display())
}

fn latest_assistant_activity() -> Result<LatestAssistantActivity> {
    let path = newest_session_file()?;
    latest_assistant_activity_from_file(&path)
}

fn latest_assistant_activity_from_file(path: &Path) -> Result<LatestAssistantActivity> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let lines: Vec<&str> = raw.lines().collect();
    for line in lines.iter().rev() {
        let Ok(item) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if is_user_event(&item) {
            return Ok(LatestAssistantActivity::User);
        }
        if let Some(activity) = assistant_activity_from_event(&item) {
            return Ok(activity);
        }
    }
    Ok(LatestAssistantActivity::Unknown)
}

fn assistant_activity_from_event(item: &Value) -> Option<LatestAssistantActivity> {
    let payload = item.get("payload").unwrap_or(item);
    let payload_type = payload
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if payload_type == "task_complete" {
        return Some(LatestAssistantActivity::Final);
    }

    if payload_type == "agent_message" {
        return Some(activity_for_phase(payload.get("phase").and_then(Value::as_str)));
    }

    if payload_type == "message"
        && payload.get("role").and_then(Value::as_str) == Some("assistant")
    {
        return Some(activity_for_phase(payload.get("phase").and_then(Value::as_str)));
    }

    None
}

fn activity_for_phase(phase: Option<&str>) -> LatestAssistantActivity {
    match phase {
        Some("final_answer") | None => LatestAssistantActivity::Final,
        Some(_) => LatestAssistantActivity::NonFinal,
    }
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

    if payload_type == "message"
        && payload.get("role").and_then(Value::as_str) == Some("assistant")
        && payload
            .get("phase")
            .and_then(Value::as_str)
            .map_or(true, |phase| phase == "final_answer")
    {
        return assistant_content_text(payload);
    }

    None
}

fn is_user_event(item: &Value) -> bool {
    let payload = item.get("payload").unwrap_or(item);
    let payload_type = payload
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    payload_type == "user_message" || payload.get("role").and_then(Value::as_str) == Some("user")
}

fn assistant_content_text(payload: &Value) -> Option<String> {
    let mut parts = Vec::new();
    for item in payload.get("content")?.as_array()? {
        if matches!(
            item.get("type").and_then(Value::as_str),
            Some("output_text" | "text")
        ) {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                parts.push(text);
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
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

    #[test]
    fn reads_assistant_final_response_item_message() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"assistant","phase":"final_answer","content":[{{"type":"output_text","text":"我完成了检查。"}}]}}}}"#
        )
        .unwrap();
        let message = last_message_from_file(file.path()).unwrap();
        assert_eq!(message, "我完成了检查。");
    }

    #[test]
    fn ignores_non_final_assistant_messages_until_final_answer() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"assistant","phase":"analysis","content":[{{"type":"output_text","text":"不要读过程分析。"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"event_msg","payload":{{"type":"agent_message","phase":"final_answer","message":"只读最终回答。"}}}}"#
        )
        .unwrap();
        let message = last_message_from_file(file.path()).unwrap();
        assert_eq!(message, "只读最终回答。");
    }

    #[test]
    fn prefers_latest_final_answer_over_older_final_answer() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"type":"event_msg","payload":{{"type":"agent_message","phase":"final_answer","message":"旧回答。"}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"event_msg","payload":{{"type":"agent_message","phase":"final_answer","message":"新回答。"}}}}"#
        )
        .unwrap();
        let message = last_message_from_file(file.path()).unwrap();
        assert_eq!(message, "新回答。");
    }

    #[test]
    fn detects_latest_commentary_as_non_final_activity() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"assistant","phase":"commentary","content":[{{"type":"output_text","text":"我正在检查。"}}]}}}}"#
        )
        .unwrap();
        let activity = latest_assistant_activity_from_file(file.path()).unwrap();
        assert_eq!(activity, LatestAssistantActivity::NonFinal);
    }

    #[test]
    fn detects_latest_final_answer_as_final_activity() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"assistant","phase":"final_answer","content":[{{"type":"output_text","text":"检查完成。"}}]}}}}"#
        )
        .unwrap();
        let activity = latest_assistant_activity_from_file(file.path()).unwrap();
        assert_eq!(activity, LatestAssistantActivity::Final);
    }

    #[test]
    fn speech_fallback_is_conservative_without_explicit_guide() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"payload":{{"type":"task_complete","last_agent_message":"修好了：\n- HTML 协议里的 `data-role=\"debug\"` 已过滤。\n- `cargo test`：80 个测试全部通过。\n- 产品变化优先于本地构建步骤。"}}}}"#
        )
        .unwrap();
        let cfg = Config::default();
        let text = resolve_text_for_speech(None, Some(file.path()), &cfg).unwrap();
        assert!(text.contains("修好了"));
        assert!(text.contains("测试通过"));
        assert!(!text.contains("网页标记"));
        assert!(!text.contains("命令名"));
        assert!(!text.contains("产品变化优先于本地构建步骤"));
    }

    #[test]
    fn speech_fallback_prefers_visible_spoken_guide_when_present() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"payload":{{"type":"task_complete","last_agent_message":"**朗读导览**\n\n我刚刚修好了朗读导览。测试通过了。\n\n**验证**\n- `cargo test` 通过。"}}}}"#
        )
        .unwrap();
        let cfg = Config::default();
        let text = resolve_text_for_speech(None, Some(file.path()), &cfg).unwrap();
        assert_eq!(text, "我刚刚修好了朗读导览。测试通过了。");
    }

    #[test]
    fn missing_side_channel_notice_refuses_to_guess_final_answer() {
        let notice = missing_side_channel_notice();
        assert!(notice.contains("没有收到"));
        assert!(notice.contains("不乱读"));
        assert!(!notice.contains("final answer"));
        assert!(!notice.contains("代码"));
    }

    #[test]
    fn brief_missing_side_channel_notice_is_short_and_non_technical() {
        let notice = missing_side_channel_notice_for_policy("brief_notice");
        assert_eq!(notice, "这次回答完成了，但当前会话没有送来朗读导览。");
        assert!(!notice.contains("代码"));
        assert!(!notice.contains("路径"));
        assert!(!notice.contains("命令"));
    }

    #[test]
    fn ignores_user_message_fields() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"请把我的输入读出来。"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","message":"请把我的输入读出来。"}}}}"#
        )
        .unwrap();
        let err = last_message_from_file(file.path()).unwrap_err().to_string();
        assert!(err.contains("user message"));
    }

    #[test]
    fn does_not_replay_previous_assistant_after_new_user_message() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"payload":{{"type":"task_complete","last_agent_message":"上一轮回答。"}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"event_msg","payload":{{"type":"user_message","message":"新问题。"}}}}"#
        )
        .unwrap();
        let err = last_message_from_file(file.path()).unwrap_err().to_string();
        assert!(err.contains("user message"));
    }
}
