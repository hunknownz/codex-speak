use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::config;

pub fn run() -> Result<()> {
    let mut failed = false;

    check_dir("Codex home", &config::codex_home()?, &mut failed);
    check_file("Config", &config::config_path()?, &mut failed);
    check_file(
        "Codex Speak CLI",
        &config::bin_dir()?.join(binary_name()),
        &mut failed,
    );
    check_file("Sherpa TTS", &config::sherpa_bin()?, &mut failed);
    check_file(
        "Melo model",
        &config::model_dir()?.join("model.onnx"),
        &mut failed,
    );
    check_file(
        "Melo lexicon",
        &config::model_dir()?.join("lexicon.txt"),
        &mut failed,
    );
    check_file(
        "Melo tokens",
        &config::model_dir()?.join("tokens.txt"),
        &mut failed,
    );
    check_notify(&mut failed)?;
    check_player(&mut failed);

    if failed {
        anyhow::bail!("doctor found problems");
    }
    Ok(())
}

fn check_notify(failed: &mut bool) -> Result<()> {
    let path = config::codex_home()?.join("config.toml");
    let raw = fs::read_to_string(&path).unwrap_or_default();
    if raw.contains("codex-speak-notify") {
        println!("OK   Codex notify: codex-speak-notify is configured");
    } else {
        println!(
            "FAIL Codex notify: codex-speak-notify not found in {}",
            path.display()
        );
        *failed = true;
    }
    Ok(())
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "codex-speak.exe"
    } else {
        "codex-speak"
    }
}

fn check_dir(label: &str, path: &Path, failed: &mut bool) {
    if path.is_dir() {
        println!("OK   {label}: {}", path.display());
    } else {
        println!("FAIL {label}: missing {}", path.display());
        *failed = true;
    }
}

fn check_file(label: &str, path: &Path, failed: &mut bool) {
    if path.is_file() {
        println!("OK   {label}: {}", path.display());
    } else {
        println!("FAIL {label}: missing {}", path.display());
        *failed = true;
    }
}

fn check_player(failed: &mut bool) {
    if cfg!(target_os = "macos") {
        let path = Path::new("/usr/bin/afplay");
        check_file("Player", path, failed);
    } else if cfg!(windows) {
        println!("OK   Player: Windows PowerShell SoundPlayer");
    } else {
        println!("FAIL Player: unsupported platform");
        *failed = true;
    }
}
