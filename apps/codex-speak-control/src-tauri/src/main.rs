use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPatch {
    enabled: Option<bool>,
    child_mode: Option<bool>,
    speed: Option<f32>,
    max_read_chars: Option<usize>,
    voice_profile: Option<String>,
}

#[tauri::command]
fn load_status() -> Result<Value, String> {
    cli_json(["status"])
}

#[tauri::command]
fn update_settings(patch: SettingsPatch) -> Result<Value, String> {
    let mut args = vec!["config".to_string(), "set".to_string()];
    if let Some(enabled) = patch.enabled {
        args.push("--enabled".to_string());
        args.push(enabled.to_string());
    }
    if let Some(child_mode) = patch.child_mode {
        args.push("--child-mode".to_string());
        args.push(child_mode.to_string());
    }
    if let Some(speed) = patch.speed {
        args.push("--speed".to_string());
        args.push(format!("{speed:.2}"));
    }
    if let Some(max_read_chars) = patch.max_read_chars {
        args.push("--max-read-chars".to_string());
        args.push(max_read_chars.to_string());
    }
    if let Some(voice_profile) = patch.voice_profile {
        args.push("--voice-profile".to_string());
        args.push(voice_profile);
    }

    if args.len() == 2 {
        return load_status();
    }

    run_cli(args.iter().map(String::as_str))?;
    load_status()
}

#[tauri::command]
fn speak_sample() -> Result<(), String> {
    let sample = "你好，我是 Codex Speak。现在的声音会更清楚，也会按你的设置来朗读。";
    Command::new(cli_path()?)
        .args(["speak", "--text", sample])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to start sample speech")
        .map_err(to_string)?;
    Ok(())
}

#[tauri::command]
fn stop_speech() -> Result<(), String> {
    run_cli(["stop"]).map(|_| ())
}

#[tauri::command]
fn run_doctor() -> Result<String, String> {
    run_cli(["doctor"])
}

fn cli_json<I, S>(args: I) -> Result<Value, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let output = run_cli(args)?;
    serde_json::from_str(&output).map_err(to_string)
}

fn run_cli<I, S>(args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let output = Command::new(cli_path()?)
        .args(args.into_iter().map(|arg| arg.as_ref().to_string()))
        .output()
        .context("failed to run codex-speak")
        .map_err(to_string)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("codex-speak failed: {stderr}{stdout}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn cli_path() -> Result<PathBuf, String> {
    let installed = dirs::home_dir()
        .context("could not determine home directory")
        .map_err(to_string)?
        .join(".codex")
        .join("codex-speak")
        .join("bin")
        .join(if cfg!(windows) {
            "codex-speak.exe"
        } else {
            "codex-speak"
        });
    if installed.is_file() {
        return Ok(installed);
    }
    Ok(PathBuf::from("codex-speak"))
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_status,
            update_settings,
            speak_sample,
            stop_speech,
            run_doctor
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Speak control app");
}
