use std::io::{self, BufRead, BufReader, Read, Write};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::config::Config;
use crate::{extract, process, side_channel, status, tts};

pub fn run() -> Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    while let Some(request) = read_message(&mut reader)? {
        let Some(method) = request.get("method").and_then(Value::as_str) else {
            continue;
        };
        if method.starts_with("notifications/") {
            continue;
        }

        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let response = match handle_request(&request) {
            Ok(result) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            }),
            Err(err) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32000,
                    "message": err.to_string()
                }
            }),
        };
        write_message(&mut writer, &response)?;
    }

    Ok(())
}

fn handle_request(request: &Value) -> Result<Value> {
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .context("missing method")?;
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": request
                .pointer("/params/protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2024-11-05"),
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "codex-speak",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => call_tool(request),
        "ping" => Ok(json!({})),
        "shutdown" => Ok(Value::Null),
        other => anyhow::bail!("unsupported MCP method: {other}"),
    }
}

fn tools() -> Value {
    json!([
        {
            "name": "codex_speak_status",
            "description": "Read Codex Speak config, installation checks, and last spoken preview.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_prepare",
            "description": "Write child-friendly spoken guide items to the Codex Speak side-channel for the next hook playback.",
            "inputSchema": {
                "type": "object",
                "required": ["items"],
                "properties": {
                    "version": { "type": "integer", "default": 1 },
                    "audience": { "type": "string", "default": "beginner" },
                    "style": { "type": "string", "default": "clear-bright" },
                    "lang": { "type": "string", "default": "zh-CN" },
                    "items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["role", "text"],
                            "properties": {
                                "role": {
                                    "type": "string",
                                    "enum": ["did", "why", "code-summary", "command-summary", "result", "next", "warning"]
                                },
                                "text": { "type": "string" }
                            },
                            "additionalProperties": false
                        }
                    }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_extract",
            "description": "Preview what Codex Speak would read from a reply or protocol block.",
            "inputSchema": {
                "type": "object",
                "required": ["text"],
                "properties": {
                    "text": { "type": "string" }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_speak_text",
            "description": "Speak or dry-run a short text using the local Codex Speak TTS settings.",
            "inputSchema": {
                "type": "object",
                "required": ["text"],
                "properties": {
                    "text": { "type": "string" },
                    "no_play": { "type": "boolean", "default": false }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_stop",
            "description": "Stop current Codex Speak playback.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_set_enabled",
            "description": "Enable or disable Codex Speak playback.",
            "inputSchema": {
                "type": "object",
                "required": ["enabled"],
                "properties": {
                    "enabled": { "type": "boolean" }
                },
                "additionalProperties": false
            }
        }
    ])
}

fn call_tool(request: &Value) -> Result<Value> {
    let name = request
        .pointer("/params/name")
        .and_then(Value::as_str)
        .context("missing tool name")?;
    let args = request
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let cfg = Config::load_or_default()?;

    let text = match name {
        "codex_speak_status" => serde_json::to_string_pretty(&status::collect(&cfg)?)?,
        "codex_speak_prepare" => {
            let spool = side_channel::SpeakSpool {
                version: args.get("version").and_then(Value::as_u64).unwrap_or(1) as u8,
                audience: optional_string(&args, "audience")
                    .or_else(|| Some("beginner".to_string())),
                style: optional_string(&args, "style").or_else(|| Some("clear-bright".to_string())),
                lang: optional_string(&args, "lang").or_else(|| Some("zh-CN".to_string())),
                source: Some("codex-speak-plugin".to_string()),
                items: parse_items(args.get("items").context("missing items")?)?,
            };
            let written = side_channel::write_latest(&spool, cfg.max_read_chars)?;
            serde_json::to_string_pretty(&written)?
        }
        "codex_speak_extract" => {
            let input = args
                .get("text")
                .and_then(Value::as_str)
                .context("missing text")?;
            extract::clean_for_speech(input, cfg.max_read_chars)
        }
        "codex_speak_speak_text" => {
            let input = args
                .get("text")
                .and_then(Value::as_str)
                .context("missing text")?;
            let no_play = args
                .get("no_play")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let cleaned = extract::clean_for_speech(input, cfg.max_read_chars);
            tts::speak(&cfg, &cleaned, no_play)?;
            format!("ok: {}", cleaned)
        }
        "codex_speak_stop" => {
            process::stop_speech()?;
            "ok: stopped".to_string()
        }
        "codex_speak_set_enabled" => {
            let enabled = args
                .get("enabled")
                .and_then(Value::as_bool)
                .context("missing enabled")?;
            let mut next = cfg;
            next.enabled = enabled;
            next.save()?;
            format!("ok: enabled={enabled}")
        }
        other => anyhow::bail!("unknown tool: {other}"),
    };

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ]
    }))
}

fn parse_items(value: &Value) -> Result<Vec<side_channel::SpeakItem>> {
    let array = value.as_array().context("items must be an array")?;
    let mut items = Vec::new();
    for item in array {
        items.push(side_channel::SpeakItem {
            role: item
                .get("role")
                .and_then(Value::as_str)
                .context("item.role is required")?
                .to_string(),
            text: item
                .get("text")
                .and_then(Value::as_str)
                .context("item.text is required")?
                .to_string(),
        });
    }
    Ok(items)
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn read_message(reader: &mut BufReader<impl Read>) -> Result<Option<Value>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>()?);
        }
    }

    let Some(length) = content_length else {
        anyhow::bail!("missing Content-Length header");
    };
    let mut buffer = vec![0; length];
    reader.read_exact(&mut buffer)?;
    let message = serde_json::from_slice(&buffer)?;
    Ok(Some(message))
}

fn write_message(writer: &mut impl Write, value: &Value) -> Result<()> {
    let body = serde_json::to_vec(value)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()?;
    Ok(())
}
