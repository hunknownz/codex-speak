use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use tauri::Manager;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPatch {
    enabled: Option<bool>,
    final_guide_enabled: Option<bool>,
    progress_prompts_enabled: Option<bool>,
    pet_enabled: Option<bool>,
    child_mode: Option<bool>,
    provider: Option<String>,
    speed: Option<f32>,
    vits_noise_scale: Option<f32>,
    vits_noise_scale_w: Option<f32>,
    tts_silence_scale: Option<f32>,
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
    if let Some(final_guide_enabled) = patch.final_guide_enabled {
        args.push("--final-guide-enabled".to_string());
        args.push(final_guide_enabled.to_string());
    }
    if let Some(progress_prompts_enabled) = patch.progress_prompts_enabled {
        args.push("--progress-prompts-enabled".to_string());
        args.push(progress_prompts_enabled.to_string());
    }
    if let Some(pet_enabled) = patch.pet_enabled {
        args.push("--pet-enabled".to_string());
        args.push(pet_enabled.to_string());
    }
    if let Some(child_mode) = patch.child_mode {
        args.push("--child-mode".to_string());
        args.push(child_mode.to_string());
    }
    if let Some(provider) = patch.provider {
        args.push("--provider".to_string());
        args.push(provider);
    }
    if let Some(speed) = patch.speed {
        args.push("--speed".to_string());
        args.push(format!("{speed:.2}"));
    }
    if let Some(vits_noise_scale) = patch.vits_noise_scale {
        args.push("--vits-noise-scale".to_string());
        args.push(format!("{vits_noise_scale:.2}"));
    }
    if let Some(vits_noise_scale_w) = patch.vits_noise_scale_w {
        args.push("--vits-noise-scale-w".to_string());
        args.push(format!("{vits_noise_scale_w:.2}"));
    }
    if let Some(tts_silence_scale) = patch.tts_silence_scale {
        args.push("--tts-silence-scale".to_string());
        args.push(format!("{tts_silence_scale:.2}"));
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
    sync_pet_visibility()?;
    load_status()
}

#[tauri::command]
fn speak_sample() -> Result<(), String> {
    let sample = "你好，我是 Codex Speak。现在的声音会更清楚，也会按你的设置来朗读。";
    run_cli(["speak", "--text", sample]).map(|_| ())
}

#[tauri::command]
fn install_current_model() -> Result<Value, String> {
    run_cli(["models", "install"])?;
    load_status()
}

#[tauri::command]
fn load_pronunciation() -> Result<Value, String> {
    cli_json(["pronunciation", "list"])
}

#[tauri::command]
fn set_pronunciation(term: String, spoken: String) -> Result<Value, String> {
    cli_json(["pronunciation", "set", "--term", &term, "--spoken", &spoken])
}

#[tauri::command]
fn remove_pronunciation(term: String) -> Result<Value, String> {
    cli_json(["pronunciation", "remove", "--term", &term])
}

#[tauri::command]
fn preview_pronunciation(text: String) -> Result<String, String> {
    run_cli(["pronunciation", "preview", "--text", &text]).map(|output| output.trim().to_string())
}

#[tauri::command]
fn stop_speech() -> Result<(), String> {
    run_cli(["stop"]).map(|_| ())
}

#[tauri::command]
fn open_control_window(app: tauri::AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("main window is not available".to_string());
    };
    window.show().map_err(to_string)?;
    window.set_focus().map_err(to_string)?;
    Ok(())
}

#[tauri::command]
fn run_doctor() -> Result<String, String> {
    run_cli(["doctor"])
}

#[tauri::command]
fn write_support_bundle() -> Result<String, String> {
    run_cli(["support-bundle"]).map(|output| output.trim().to_string())
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
    for candidate in local_binary_candidates() {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Ok(PathBuf::from("codex-speak"))
}

fn local_binary_candidates() -> Vec<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(repo_root) = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
    else {
        return Vec::new();
    };
    let name = if cfg!(windows) {
        "codex-speak.exe"
    } else {
        "codex-speak"
    };
    vec![
        repo_root.join("target/release").join(name),
        repo_root.join("target/debug").join(name),
    ]
}

fn app_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("codex-speak")
}

fn pet_helper_path() -> PathBuf {
    app_home().join("bin").join("codex-speak-pet-macos")
}

fn pet_helper_pid_path() -> PathBuf {
    app_home().join("state").join("pet-helper.pid")
}

fn launch_native_pet() {
    if !cfg!(target_os = "macos") {
        return;
    }
    if !pet_enabled() {
        stop_native_pet();
        return;
    }
    let helper = pet_helper_path();
    if !helper.is_file() {
        return;
    }
    let cli = cli_path().unwrap_or_else(|_| PathBuf::from("codex-speak"));
    let _ = Command::new(helper)
        .arg("--state")
        .arg(app_home().join("state").join("pet-state.json"))
        .arg("--cli")
        .arg(cli)
        .arg("--asset-dir")
        .arg(app_home().join("assets").join("pet"))
        .arg("--parent")
        .arg(std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn stop_native_pet() {
    let pid_path = pet_helper_pid_path();
    if let Ok(raw) = std::fs::read_to_string(&pid_path) {
        if let Ok(pid) = raw.trim().parse::<i32>() {
            if is_pet_helper_process(pid) {
                let _ = Command::new("/bin/kill").arg(pid.to_string()).status();
            }
        }
    }
    let _ = std::fs::remove_file(pid_path);
}

fn is_pet_helper_process(pid: i32) -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }
    let Ok(output) = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "comm=", "-o", "args="])
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    String::from_utf8_lossy(&output.stdout).contains("codex-speak-pet-macos")
}

fn pet_enabled() -> bool {
    cli_json(["config", "get"])
        .ok()
        .and_then(|value| value.get("pet_enabled").and_then(Value::as_bool))
        .unwrap_or(true)
}

fn sync_pet_visibility() -> Result<(), String> {
    if pet_enabled() {
        launch_native_pet();
    } else {
        stop_native_pet();
    }
    Ok(())
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            launch_native_pet();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_status,
            update_settings,
            speak_sample,
            install_current_model,
            load_pronunciation,
            set_pronunciation,
            remove_pronunciation,
            preview_pronunciation,
            stop_speech,
            open_control_window,
            run_doctor,
            write_support_bundle
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Speak control app");
}
