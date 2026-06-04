use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::config;
use crate::pet_state;

const QUEUE_POLL_INTERVAL: Duration = Duration::from_millis(120);
const QUEUE_WAIT_TIMEOUT: Duration = Duration::from_secs(300);
const STALE_LOCK_SECONDS: u64 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackPolicy {
    Interrupt,
    Queue,
    SkipIfBusy,
}

#[derive(Debug)]
pub struct PlaybackGuard {
    path: PathBuf,
    token: String,
}

impl Drop for PlaybackGuard {
    fn drop(&mut self) {
        let raw = fs::read_to_string(&self.path).unwrap_or_default();
        if raw.trim() == self.token.trim() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

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
    let _ = fs::remove_file(config::playback_lock_path()?);
    write_stop_signal()?;
    let _ = pet_state::write_state("idle", Some("朗读已停止。"), "stop");
    Ok(())
}

pub fn acquire_playback(policy: PlaybackPolicy) -> Result<Option<PlaybackGuard>> {
    fs::create_dir_all(config::state_dir()?)?;
    if policy == PlaybackPolicy::Interrupt {
        stop_speech()?;
    }

    let lock_path = config::playback_lock_path()?;
    let stop_marker = read_stop_signal();
    let token = format!("{}:{}", std::process::id(), now_millis());
    let deadline = Instant::now() + QUEUE_WAIT_TIMEOUT;

    loop {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                writeln!(file, "{token}")?;
                return Ok(Some(PlaybackGuard {
                    path: lock_path,
                    token,
                }));
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                remove_stale_lock(&lock_path)?;
                if policy == PlaybackPolicy::SkipIfBusy {
                    return Ok(None);
                }
                if read_stop_signal() != stop_marker {
                    return Ok(None);
                }
                if Instant::now() >= deadline {
                    anyhow::bail!("timed out waiting for speech playback queue");
                }
                thread::sleep(QUEUE_POLL_INTERVAL);
            }
            Err(err) => {
                return Err(err).with_context(|| {
                    format!("failed to acquire playback lock {}", lock_path.display())
                })
            }
        }
    }
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

fn remove_stale_lock(path: &PathBuf) -> Result<()> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };
    let Ok(modified) = metadata.modified() else {
        return Ok(());
    };
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return Ok(());
    };
    if age.as_secs() > STALE_LOCK_SECONDS {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

fn write_stop_signal() -> Result<()> {
    fs::create_dir_all(config::state_dir()?)?;
    fs::write(
        config::playback_stop_signal_path()?,
        now_millis().to_string(),
    )?;
    Ok(())
}

fn read_stop_signal() -> Option<String> {
    fs::read_to_string(config::playback_stop_signal_path().ok()?)
        .ok()
        .map(|value| value.trim().to_string())
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
