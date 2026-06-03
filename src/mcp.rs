use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::config::Config;
use crate::{
    extract, install, pet_state, process, pronunciation, settings, side_channel, status, tts,
};

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
            "description": "Write mode-aware spoken guide items to the Codex Speak side-channel for the next hook playback.",
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
                                    "enum": ["did", "why", "code-summary", "command-summary", "visual-summary", "result", "next", "warning"]
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
            "description": "Speak or dry-run a short text using the local Codex Speak TTS settings. Set background=true for short progress prompts so the tool returns immediately.",
            "inputSchema": {
                "type": "object",
                "required": ["text"],
                "properties": {
                    "text": { "type": "string" },
                    "no_play": { "type": "boolean", "default": false },
                    "background": { "type": "boolean", "default": false }
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
        },
        {
            "name": "codex_speak_update_config",
            "description": "Update Codex Speak settings such as playback switches, TTS provider, child mode, speed, max read length, or voice profile.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "enabled": { "type": "boolean" },
                    "final_guide_enabled": { "type": "boolean" },
                    "progress_prompts_enabled": { "type": "boolean" },
                    "pet_enabled": { "type": "boolean" },
                    "child_mode": { "type": "boolean" },
                    "provider": {
                        "type": "string",
                        "enum": ["sherpa_melo", "sherpa_kokoro", "sherpa_zipvoice", "piper", "system"]
                    },
                    "speed": { "type": "number", "minimum": 0.6, "maximum": 1.3 },
                    "max_read_chars": { "type": "integer", "minimum": 80, "maximum": 2000 },
                    "voice_profile": {
                        "type": "string",
                        "enum": ["clear_bright", "slow_clear", "quick_preview"]
                    }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_set_provider",
            "description": "Switch the TTS provider used by Codex Speak.",
            "inputSchema": {
                "type": "object",
                "required": ["provider"],
                "properties": {
                    "provider": {
                        "type": "string",
                        "enum": ["sherpa_melo", "sherpa_kokoro", "sherpa_zipvoice", "piper", "system"]
                    }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_install_model",
            "description": "Download or repair the local model for the current or selected TTS provider.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "provider": {
                        "type": "string",
                        "enum": ["sherpa_melo", "sherpa_kokoro", "sherpa_zipvoice", "piper", "system"]
                    }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_set_child_mode",
            "description": "Enable or disable child-friendly speech mode.",
            "inputSchema": {
                "type": "object",
                "required": ["child_mode"],
                "properties": {
                    "child_mode": { "type": "boolean" }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_set_speed",
            "description": "Set Codex Speak speed. Recommended range is 0.82 to 1.0.",
            "inputSchema": {
                "type": "object",
                "required": ["speed"],
                "properties": {
                    "speed": { "type": "number", "minimum": 0.6, "maximum": 1.3 }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_set_voice_profile",
            "description": "Set a local voice tuning profile.",
            "inputSchema": {
                "type": "object",
                "required": ["voice_profile"],
                "properties": {
                    "voice_profile": {
                        "type": "string",
                        "enum": ["clear_bright", "slow_clear", "quick_preview"]
                    }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_list_pronunciation",
            "description": "List local pronunciation replacements used before speech playback.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_set_pronunciation",
            "description": "Add or update a local pronunciation replacement, for example reading a project name or acronym in natural spoken Chinese.",
            "inputSchema": {
                "type": "object",
                "required": ["term", "spoken"],
                "properties": {
                    "term": { "type": "string" },
                    "spoken": { "type": "string" }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "codex_speak_remove_pronunciation",
            "description": "Remove a local pronunciation replacement.",
            "inputSchema": {
                "type": "object",
                "required": ["term"],
                "properties": {
                    "term": { "type": "string" }
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

    let text = call_tool_by_name(name, args, cfg)?;

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ]
    }))
}

pub(crate) fn call_tool_by_name(name: &str, args: Value, cfg: Config) -> Result<String> {
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
            let _ = pet_state::write_state("ready", Some(&written.text), "mcp");
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
            let background = args
                .get("background")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let cleaned = extract::clean_for_speech(input, cfg.max_read_chars);
            if background && !no_play {
                if !cfg.progress_prompts_enabled {
                    return Ok(format!("skipped: progress prompts disabled: {}", cleaned));
                }
                speak_text_in_background(&cleaned)?;
                format!("queued: {}", cleaned)
            } else {
                tts::speak(&cfg, &cleaned, no_play)?;
                format!("ok: {}", cleaned)
            }
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
            update_config(
                cfg,
                settings::ConfigPatch {
                    enabled: Some(enabled),
                    ..Default::default()
                },
            )?
        }
        "codex_speak_update_config" => {
            let patch = settings::ConfigPatch {
                enabled: args.get("enabled").and_then(Value::as_bool),
                final_guide_enabled: args.get("final_guide_enabled").and_then(Value::as_bool),
                progress_prompts_enabled: args
                    .get("progress_prompts_enabled")
                    .and_then(Value::as_bool),
                pet_enabled: args.get("pet_enabled").and_then(Value::as_bool),
                child_mode: args.get("child_mode").and_then(Value::as_bool),
                provider: optional_string(&args, "provider"),
                speed: args.get("speed").and_then(Value::as_f64).map(|v| v as f32),
                max_read_chars: args
                    .get("max_read_chars")
                    .and_then(Value::as_u64)
                    .map(|v| v as usize),
                voice_profile: optional_string(&args, "voice_profile"),
            };
            update_config(cfg, patch)?
        }
        "codex_speak_set_provider" => {
            let provider = optional_string(&args, "provider").context("missing provider")?;
            update_config(
                cfg,
                settings::ConfigPatch {
                    provider: Some(provider),
                    ..Default::default()
                },
            )?
        }
        "codex_speak_install_model" => {
            let provider =
                optional_string(&args, "provider").unwrap_or_else(|| cfg.provider.clone());
            install::install_model(&provider)?;
            let cfg = Config::load_or_default()?;
            serde_json::to_string_pretty(&status::collect(&cfg)?)?
        }
        "codex_speak_set_child_mode" => {
            let child_mode = args
                .get("child_mode")
                .and_then(Value::as_bool)
                .context("missing child_mode")?;
            update_config(
                cfg,
                settings::ConfigPatch {
                    child_mode: Some(child_mode),
                    ..Default::default()
                },
            )?
        }
        "codex_speak_set_speed" => {
            let speed = args
                .get("speed")
                .and_then(Value::as_f64)
                .context("missing speed")? as f32;
            update_config(
                cfg,
                settings::ConfigPatch {
                    speed: Some(speed),
                    ..Default::default()
                },
            )?
        }
        "codex_speak_set_voice_profile" => {
            let voice_profile =
                optional_string(&args, "voice_profile").context("missing voice_profile")?;
            update_config(
                cfg,
                settings::ConfigPatch {
                    voice_profile: Some(voice_profile),
                    ..Default::default()
                },
            )?
        }
        "codex_speak_list_pronunciation" => {
            serde_json::to_string_pretty(&pronunciation::load_user_dictionary()?)?
        }
        "codex_speak_set_pronunciation" => {
            let term = args
                .get("term")
                .and_then(Value::as_str)
                .context("missing term")?;
            let spoken = args
                .get("spoken")
                .and_then(Value::as_str)
                .context("missing spoken")?;
            let update = pronunciation::set_user_term(term, spoken)?;
            serde_json::to_string_pretty(&update)?
        }
        "codex_speak_remove_pronunciation" => {
            let term = args
                .get("term")
                .and_then(Value::as_str)
                .context("missing term")?;
            let update = pronunciation::remove_user_term(term)?;
            serde_json::to_string_pretty(&update)?
        }
        other => anyhow::bail!("unknown tool: {other}"),
    };
    Ok(text)
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

fn update_config(cfg: Config, patch: settings::ConfigPatch) -> Result<String> {
    let update = settings::apply_patch(cfg, patch)?;
    update.config.save()?;
    Ok(serde_json::to_string_pretty(&update)?)
}

fn speak_text_in_background(text: &str) -> Result<()> {
    let exe = std::env::current_exe().context("failed to locate current codex-speak binary")?;
    Command::new(exe)
        .arg("speak")
        .arg("--text")
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to start background speech process")?;
    Ok(())
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
