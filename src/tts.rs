use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::config::{self, Config};
use crate::process;

pub fn speak(cfg: &Config, text: &str, no_play: bool) -> Result<()> {
    if !cfg.enabled {
        return Ok(());
    }

    fs::create_dir_all(config::logs_dir()?)?;
    fs::create_dir_all(config::cache_dir()?)?;
    fs::write(config::logs_dir()?.join("last-spoken.txt"), text)?;

    process::stop_speech()?;

    let result = speak_with_sherpa(cfg, text, no_play).or_else(|err| {
        fs::write(
            config::logs_dir()?.join("last-error.log"),
            format!("{err:#}\nfalling back to system voice\n"),
        )?;
        speak_with_system(text, no_play)
    });

    result?;
    Ok(())
}

fn speak_with_sherpa(_cfg: &Config, text: &str, no_play: bool) -> Result<()> {
    let sherpa = config::sherpa_bin()?;
    let model = config::model_dir()?;
    require_file(&sherpa)?;
    require_file(&model.join("model.onnx"))?;
    require_file(&model.join("lexicon.txt"))?;
    require_file(&model.join("tokens.txt"))?;

    let wav = config::cache_dir()?.join("last.wav");
    let output = Command::new(&sherpa)
        .arg(format!(
            "--vits-model={}",
            model.join("model.onnx").display()
        ))
        .arg(format!(
            "--vits-lexicon={}",
            model.join("lexicon.txt").display()
        ))
        .arg(format!(
            "--vits-tokens={}",
            model.join("tokens.txt").display()
        ))
        .arg(format!(
            "--tts-rule-fsts={},{}",
            model.join("date.fst").display(),
            model.join("number.fst").display()
        ))
        .arg(format!("--output-filename={}", wav.display()))
        .arg(text)
        .output()
        .with_context(|| format!("failed to run {}", sherpa.display()))?;
    fs::write(
        config::logs_dir()?.join("last-tts.stdout.log"),
        &output.stdout,
    )?;
    fs::write(
        config::logs_dir()?.join("last-tts.stderr.log"),
        &output.stderr,
    )?;
    if !output.status.success() {
        anyhow::bail!("sherpa-onnx failed with status {}", output.status);
    }

    if !no_play {
        play_wav(&wav)?;
    }
    Ok(())
}

fn speak_with_system(text: &str, no_play: bool) -> Result<()> {
    if no_play {
        return Ok(());
    }
    if cfg!(target_os = "macos") {
        Command::new("/usr/bin/say")
            .args(["-v", "Tingting", "-r", "175", text])
            .status()
            .context("failed to run macOS say")?;
    } else if cfg!(windows) {
        let script = format!(
            "Add-Type -AssemblyName System.Speech; \
             $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
             $s.Speak({:?});",
            text
        );
        Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status()
            .context("failed to run Windows system speech")?;
    } else {
        anyhow::bail!("no system speech fallback for this platform");
    }
    Ok(())
}

fn play_wav(path: &Path) -> Result<()> {
    if cfg!(target_os = "macos") {
        let status = Command::new("/usr/bin/afplay")
            .arg(path)
            .status()
            .context("failed to run afplay")?;
        if !status.success() {
            anyhow::bail!("afplay failed with status {status}");
        }
    } else if cfg!(windows) {
        let script = format!(
            "(New-Object Media.SoundPlayer {:?}).PlaySync();",
            path.display().to_string()
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status()
            .context("failed to play wav with PowerShell")?;
        if !status.success() {
            anyhow::bail!("PowerShell audio playback failed with status {status}");
        }
    } else {
        anyhow::bail!("no wav player configured for this platform");
    }
    Ok(())
}

fn require_file(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("required file missing: {}", path.display());
    }
    Ok(())
}
