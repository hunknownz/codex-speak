use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{self, Config};
use crate::process::PlaybackPolicy;
use crate::tts;

const MAX_PENDING_JOBS: usize = 5;
const DAEMON_IDLE_SLEEP: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechJob {
    pub id: String,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub transcript_path: Option<String>,
    pub created_at_ms: u128,
    pub source: String,
    pub priority: String,
    pub text: String,
    #[serde(default)]
    pub attempts: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkerOptions {
    pub daemon: bool,
    pub no_play: bool,
}

pub fn enqueue(job: SpeechJob) -> Result<bool> {
    enqueue_in_root(&config::queue_dir()?, job)
}

pub fn spawn_worker(no_play: bool) -> Result<()> {
    let exe = std::env::current_exe().context("failed to locate current codex-speak binary")?;
    let mut command = std::process::Command::new(exe);
    command.arg("queue-worker").arg("--once");
    if no_play {
        command.arg("--no-play");
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to start queue worker")?;
    Ok(())
}

pub fn run_worker(cfg: &Config, options: WorkerOptions) -> Result<()> {
    let root = config::queue_dir()?;
    ensure_queue_dirs(&root)?;
    let Some(_guard) = WorkerLock::acquire(&config::queue_worker_lock_path()?)? else {
        return Ok(());
    };

    loop {
        let did_work = process_one_in_root(&root, cfg, options.no_play)?;
        if !options.daemon {
            if !did_work {
                return Ok(());
            }
            if next_pending_job(&root)?.is_none() {
                return Ok(());
            }
            continue;
        }
        if !did_work {
            thread::sleep(DAEMON_IDLE_SLEEP);
        }
    }
}

pub(crate) fn enqueue_in_root(root: &Path, job: SpeechJob) -> Result<bool> {
    ensure_queue_dirs(root)?;
    if job.text.trim().is_empty() {
        return Ok(false);
    }
    if job_exists(root, &job.id)? {
        return Ok(false);
    }

    trim_pending(root)?;

    let pending = root.join("pending");
    let filename = format!("{}-{}.json", job.created_at_ms, stable_hash(&job.id));
    let path = pending.join(filename);
    let temp = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&temp, serde_json::to_string_pretty(&job)?)
        .with_context(|| format!("failed to write {}", temp.display()))?;
    fs::rename(&temp, &path).with_context(|| {
        format!(
            "failed to move queue job {} to {}",
            temp.display(),
            path.display()
        )
    })?;
    trim_pending(root)?;
    Ok(true)
}

pub(crate) fn process_one_in_root(root: &Path, cfg: &Config, no_play: bool) -> Result<bool> {
    ensure_queue_dirs(root)?;
    let Some(path) = next_pending_job(root)? else {
        return Ok(false);
    };

    let running = root.join("running").join(
        path.file_name()
            .context("pending queue path has no filename")?,
    );
    fs::rename(&path, &running).or_else(|_| {
        fs::copy(&path, &running)?;
        fs::remove_file(&path)
    })?;

    let mut job = read_job(&running)?;
    let result = if no_play {
        Ok(())
    } else {
        tts::speak_with_policy(cfg, &job.text, false, PlaybackPolicy::Queue).map(|_| ())
    };

    match result {
        Ok(()) => {
            move_job(&running, &root.join("done"))?;
        }
        Err(err) if job.attempts == 0 => {
            job.attempts += 1;
            fs::write(&running, serde_json::to_string_pretty(&job)?)?;
            move_job(&running, &root.join("pending"))?;
            fs::create_dir_all(config::logs_dir()?)?;
            fs::write(
                config::logs_dir()?.join("last-queue-error.log"),
                format!("{err:#}\n"),
            )?;
        }
        Err(err) => {
            fs::create_dir_all(config::logs_dir()?)?;
            fs::write(
                config::logs_dir()?.join("last-queue-error.log"),
                format!("{err:#}\n"),
            )?;
            move_job(&running, &root.join("failed"))?;
        }
    }

    Ok(true)
}

pub(crate) fn next_pending_job(root: &Path) -> Result<Option<PathBuf>> {
    let pending = root.join("pending");
    if !pending.is_dir() {
        return Ok(None);
    }
    let mut entries = fs::read_dir(&pending)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    entries.sort();
    Ok(entries.into_iter().next())
}

fn ensure_queue_dirs(root: &Path) -> Result<()> {
    for name in ["pending", "running", "done", "failed"] {
        fs::create_dir_all(root.join(name))?;
    }
    Ok(())
}

fn job_exists(root: &Path, id: &str) -> Result<bool> {
    for name in ["pending", "running", "done"] {
        let dir = root.join(name);
        if !dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            if read_job(&path).is_ok_and(|job| job.id == id) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn trim_pending(root: &Path) -> Result<()> {
    let pending = root.join("pending");
    if !pending.is_dir() {
        return Ok(());
    }
    let mut entries = fs::read_dir(&pending)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    entries.sort();
    while entries.len() > MAX_PENDING_JOBS {
        if let Some(path) = entries.first().cloned() {
            let _ = fs::remove_file(path);
        }
        entries.remove(0);
    }
    Ok(())
}

fn read_job(path: &Path) -> Result<SpeechJob> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn move_job(path: &Path, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let target = dir.join(path.file_name().context("queue path has no filename")?);
    if target.exists() {
        fs::remove_file(&target)?;
    }
    fs::rename(path, &target).or_else(|_| {
        fs::copy(path, &target)?;
        fs::remove_file(path)
    })?;
    Ok(())
}

fn stable_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    hex[..16].to_string()
}

pub fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

struct WorkerLock {
    path: PathBuf,
}

impl WorkerLock {
    fn acquire(path: &Path) -> Result<Option<Self>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(mut file) => {
                writeln!(file, "{}", std::process::id())?;
                Ok(Some(Self {
                    path: path.to_path_buf(),
                }))
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(None),
            Err(err) => Err(err).with_context(|| format!("failed to lock {}", path.display())),
        }
    }
}

impl Drop for WorkerLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn job(id: &str, created_at_ms: u128) -> SpeechJob {
        SpeechJob {
            id: id.to_string(),
            session_id: Some("s".to_string()),
            turn_id: Some(id.to_string()),
            transcript_path: None,
            created_at_ms,
            source: "test".to_string(),
            priority: "normal".to_string(),
            text: format!("job {id}"),
            attempts: 0,
        }
    }

    #[test]
    fn enqueue_skips_duplicate_ids() {
        let dir = tempdir().unwrap();
        assert!(enqueue_in_root(dir.path(), job("a", 1)).unwrap());
        assert!(!enqueue_in_root(dir.path(), job("a", 2)).unwrap());
        assert_eq!(fs::read_dir(dir.path().join("pending")).unwrap().count(), 1);
    }

    #[test]
    fn next_pending_uses_fifo_filename_order() {
        let dir = tempdir().unwrap();
        enqueue_in_root(dir.path(), job("b", 2)).unwrap();
        enqueue_in_root(dir.path(), job("a", 1)).unwrap();
        let next = next_pending_job(dir.path()).unwrap().unwrap();
        assert!(next.file_name().unwrap().to_string_lossy().starts_with("1-"));
    }

    #[test]
    fn trims_pending_to_max_jobs() {
        let dir = tempdir().unwrap();
        for i in 0..7 {
            enqueue_in_root(dir.path(), job(&format!("j{i}"), i)).unwrap();
        }
        assert_eq!(fs::read_dir(dir.path().join("pending")).unwrap().count(), 5);
    }

    #[test]
    fn dry_run_worker_moves_job_to_done() {
        let dir = tempdir().unwrap();
        enqueue_in_root(dir.path(), job("a", 1)).unwrap();
        let cfg = Config::default();
        assert!(process_one_in_root(dir.path(), &cfg, true).unwrap());
        assert!(next_pending_job(dir.path()).unwrap().is_none());
        assert_eq!(fs::read_dir(dir.path().join("done")).unwrap().count(), 1);
    }
}
