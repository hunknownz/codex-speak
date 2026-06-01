use std::fs;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::config;
use crate::pet_state;

pub fn stop_speech() -> Result<()> {
    stop_tracked_playback()?;
    if cfg!(target_os = "macos") {
        let _ = Command::new("/usr/bin/pkill")
            .args(["-x", "afplay"])
            .status();
        let _ = Command::new("/usr/bin/pkill").args(["-x", "say"]).status();
        let _ = Command::new("/usr/bin/pkill")
            .args(["-f", "sherpa-onnx-offline-tts"])
            .status();
    } else if cfg!(windows) {
        let _ = Command::new("taskkill")
            .args(["/IM", "sherpa-onnx-offline-tts.exe", "/F"])
            .status();
    }
    let _ = pet_state::write_state("idle", Some("朗读已停止。"), "stop");
    Ok(())
}

pub fn run_tracked(mut command: Command, label: &str) -> Result<()> {
    let mut child = command
        .spawn()
        .with_context(|| format!("failed to start {label}"))?;
    let pid = child.id();
    write_playback_pid(pid)?;
    let status = child
        .wait()
        .with_context(|| format!("failed to wait for {label}"))?;
    let was_stopped = !playback_pid_matches(pid)?;
    clear_playback_pid(pid)?;
    if !status.success() {
        if was_stopped {
            return Ok(());
        }
        anyhow::bail!("{label} failed with status {status}");
    }
    Ok(())
}

fn write_playback_pid(pid: u32) -> Result<()> {
    fs::create_dir_all(config::state_dir()?)?;
    fs::write(config::playback_pid_path()?, pid.to_string())?;
    Ok(())
}

fn clear_playback_pid(pid: u32) -> Result<()> {
    let path = config::playback_pid_path()?;
    let raw = fs::read_to_string(&path).unwrap_or_default();
    if raw.trim() == pid.to_string() {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

fn playback_pid_matches(pid: u32) -> Result<bool> {
    let raw = fs::read_to_string(config::playback_pid_path()?).unwrap_or_default();
    Ok(raw.trim() == pid.to_string())
}

fn stop_tracked_playback() -> Result<()> {
    let path = config::playback_pid_path()?;
    let raw = fs::read_to_string(&path).unwrap_or_default();
    let Some(pid) = raw.trim().parse::<u32>().ok() else {
        return Ok(());
    };

    if cfg!(windows) {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    } else {
        let _ = Command::new("/bin/kill")
            .args(["-TERM", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = fs::remove_file(path);
    Ok(())
}
