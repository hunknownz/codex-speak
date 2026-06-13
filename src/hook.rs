use std::io::{self, Read};
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::{extract, queue, session};

const SOURCE: &str = "codex-stop-hook";

pub fn run_from_stdin(cfg: &Config, no_play: bool) -> Result<()> {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw)?;
    if raw.trim().is_empty() {
        return Ok(());
    }
    let payload: Value = serde_json::from_str(&raw).context("failed to parse hook payload")?;
    let Some(job) = job_from_payload(&payload, cfg)? else {
        return Ok(());
    };
    if queue::enqueue(job)? {
        queue::spawn_worker(no_play)?;
    }
    Ok(())
}

pub(crate) fn job_from_payload(payload: &Value, cfg: &Config) -> Result<Option<queue::SpeechJob>> {
    if !cfg.enabled || !cfg.final_guide_enabled {
        return Ok(None);
    }

    if !is_stop_event(payload) {
        return Ok(None);
    }

    let raw_text = match direct_final_message(payload) {
        Some(text) => text,
        None => {
            let Some(path) = transcript_path(payload) else {
                return Ok(None);
            };
            match session::last_message_from_file(Path::new(&path)) {
                Ok(text) => text,
                Err(_) => return Ok(None),
            }
        }
    };

    let text = extract::growth_mode_spoken_summary(&raw_text, cfg.max_read_chars);
    if text.trim().is_empty() {
        return Ok(None);
    }

    let session_id = first_string(
        payload,
        &[
            "session_id",
            "session.id",
            "conversation_id",
            "payload.session_id",
            "payload.session.id",
            "payload.conversation_id",
        ],
    );
    let turn_id = first_string(
        payload,
        &[
            "turn_id",
            "turn.id",
            "generation_id",
            "payload.turn_id",
            "payload.turn.id",
            "payload.generation_id",
        ],
    );
    let transcript_path = transcript_path(payload);
    let session_key = session_id
        .clone()
        .or_else(|| transcript_path.clone())
        .unwrap_or_else(|| "unknown-session".to_string());
    let turn_key = turn_id.clone().unwrap_or_else(|| stable_hash(&raw_text));
    let id = format!("{}:{}", stable_hash(&session_key), stable_hash(&turn_key));

    Ok(Some(queue::SpeechJob {
        id,
        session_id,
        turn_id,
        transcript_path,
        created_at_ms: queue::now_millis(),
        source: SOURCE.to_string(),
        priority: "normal".to_string(),
        text,
        attempts: 0,
    }))
}

fn is_stop_event(payload: &Value) -> bool {
    matches!(
        payload
            .get("hook_event_name")
            .or_else(|| payload.get("event"))
            .and_then(Value::as_str),
        Some("Stop" | "SubagentStop")
    )
}

fn direct_final_message(payload: &Value) -> Option<String> {
    first_string(
        payload,
        &[
            "last_assistant_message",
            "last_agent_message",
            "assistant_message",
            "message",
            "payload.last_assistant_message",
            "payload.last_agent_message",
            "payload.assistant_message",
            "payload.message",
        ],
    )
    .filter(|text| !text.trim().is_empty())
}

fn transcript_path(payload: &Value) -> Option<String> {
    first_string(
        payload,
        &[
            "transcript_path",
            "transcript.path",
            "payload.transcript_path",
        ],
    )
    .filter(|path| !path.trim().is_empty())
}

fn first_string(value: &Value, paths: &[&str]) -> Option<String> {
    paths.iter().find_map(|path| string_at_path(value, path))
}

fn string_at_path(value: &Value, path: &str) -> Option<String> {
    let mut current = value;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    current.as_str().map(str::to_string)
}

fn stable_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    hex[..16].to_string()
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use serde_json::json;
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn extracts_direct_last_assistant_message() {
        let payload = json!({
            "hook_event_name": "Stop",
            "session_id": "s1",
            "turn_id": "t1",
            "last_assistant_message": "我已经完成啦。\n```rs\nfn main() {}\n```"
        });
        let job = job_from_payload(&payload, &Config::default())
            .unwrap()
            .unwrap();
        assert_eq!(job.session_id.as_deref(), Some("s1"));
        assert_eq!(job.turn_id.as_deref(), Some("t1"));
        assert!(job.text.contains("我已经完成"));
        assert!(!job.text.contains("fn main"));
    }

    #[test]
    fn summarizes_final_answer_for_growth_mode() {
        let payload = json!({
            "hook_event_name": "Stop",
            "session_id": "s-child",
            "turn_id": "t-child",
            "last_assistant_message": r#"
检查完了：现在本机 Mac 的 App 和相关组件都正常。

- 控制面板 App 已安装并正在运行。
- Stop Hook 已配置且是当前版本。
- AGENTS hint、Skill、queue、MCP legacy cleanup 都已经检查。
- `doctor --json` 是 ok。

下一步可以重启 Codex，试试回答结束后会不会自动朗读。
"#
        });
        let job = job_from_payload(&payload, &Config::default())
            .unwrap()
            .unwrap();
        assert!(job.text.contains("检查完了"));
        assert!(job.text.contains("下一步"));
        assert!(job.text.chars().count() <= 240);
        assert!(!job.text.contains("doctor"));
        assert!(!job.text.contains("AGENTS"));
        assert!(!job.text.contains("MCP legacy cleanup"));
    }

    #[test]
    fn extracts_nested_payload_fields() {
        let payload = json!({
            "hook_event_name": "Stop",
            "payload": {
                "session_id": "nested-session",
                "turn_id": "nested-turn",
                "last_assistant_message": "嵌套字段也可以朗读。"
            }
        });
        let job = job_from_payload(&payload, &Config::default())
            .unwrap()
            .unwrap();
        assert_eq!(job.session_id.as_deref(), Some("nested-session"));
        assert_eq!(job.turn_id.as_deref(), Some("nested-turn"));
        assert!(job.text.contains("嵌套字段"));
    }

    #[test]
    fn respects_final_guide_enabled() {
        let payload = json!({
            "hook_event_name": "Stop",
            "last_assistant_message": "不要入队。"
        });
        let cfg = Config {
            final_guide_enabled: false,
            ..Config::default()
        };
        assert!(job_from_payload(&payload, &cfg).unwrap().is_none());
    }

    #[test]
    fn extracts_from_transcript_path_when_message_missing() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{"type":"event_msg","payload":{{"type":"agent_message","phase":"final_answer","message":"这是当前会话的最终回答。"}}}}"#
        )
        .unwrap();
        let payload = json!({
            "hook_event_name": "Stop",
            "session_id": "s2",
            "turn_id": "t2",
            "transcript_path": file.path()
        });
        let job = job_from_payload(&payload, &Config::default())
            .unwrap()
            .unwrap();
        assert_eq!(
            job.transcript_path.as_deref(),
            Some(file.path().to_str().unwrap())
        );
        assert!(job.text.contains("最终回答"));
    }

    #[test]
    fn ignores_non_stop_events() {
        let payload = json!({
            "hook_event_name": "PreToolUse",
            "last_assistant_message": "不要播。"
        });
        assert!(job_from_payload(&payload, &Config::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn falls_back_to_hash_when_turn_id_missing() {
        let payload = json!({
            "hook_event_name": "Stop",
            "session_id": "s1",
            "last_assistant_message": "完成了。"
        });
        let a = job_from_payload(&payload, &Config::default())
            .unwrap()
            .unwrap();
        let b = job_from_payload(&payload, &Config::default())
            .unwrap()
            .unwrap();
        assert_eq!(a.id, b.id);
    }
}
