use std::fs;
use std::path::{Path, PathBuf};
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

    process::stop_speech()?;

    if let Err(err) = speak_with_provider(cfg, text, no_play) {
        fs::write(
            config::logs_dir()?.join("last-error.log"),
            format!("{err:#}\n"),
        )?;
        if cfg.provider == "sherpa_melo" && cfg.fallback_provider == "system" {
            speak_with_system(text, no_play)?;
        } else {
            return Err(err);
        }
    }

    fs::write(config::logs_dir()?.join("last-spoken.txt"), text)?;
    Ok(())
}

fn speak_with_provider(cfg: &Config, text: &str, no_play: bool) -> Result<()> {
    match cfg.provider.as_str() {
        "sherpa_melo" => speak_with_sherpa_melo(cfg, text, no_play),
        "sherpa_kokoro" => speak_with_sherpa_kokoro(cfg, text, no_play),
        "sherpa_zipvoice" => speak_with_sherpa_zipvoice(cfg, text, no_play),
        "piper" => speak_with_piper(cfg, text, no_play),
        "system" => speak_with_system(text, no_play),
        other => anyhow::bail!("unsupported TTS provider: {other}"),
    }
}

fn speak_with_sherpa_melo(cfg: &Config, text: &str, no_play: bool) -> Result<()> {
    let sherpa = config::sherpa_bin()?;
    let model = config::model_dir()?;
    require_file(&sherpa)?;
    require_file(&model.join("model.onnx"))?;
    require_file(&model.join("lexicon.txt"))?;
    require_file(&model.join("tokens.txt"))?;

    let wav = config::cache_dir()?.join("last.wav");
    let output = Command::new(&sherpa)
        .arg(format!("--num-threads={}", cfg.num_threads))
        .arg(format!("--speed={}", cfg.speed))
        .arg(format!("--vits-noise-scale={}", cfg.vits_noise_scale))
        .arg(format!("--vits-noise-scale-w={}", cfg.vits_noise_scale_w))
        .arg(format!("--tts-silence-scale={}", cfg.tts_silence_scale))
        .arg("--print-args=false")
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
    write_tts_logs(&output.stdout, &output.stderr)?;
    if !output.status.success() {
        anyhow::bail!("sherpa-onnx melo failed with status {}", output.status);
    }

    if !no_play {
        play_wav(&wav)?;
    }
    Ok(())
}

fn speak_with_sherpa_kokoro(cfg: &Config, text: &str, no_play: bool) -> Result<()> {
    let sherpa = config::sherpa_bin()?;
    let model = config::kokoro_model_dir()?;
    let lexicons = kokoro_lexicons(&model);
    require_file(&sherpa)?;
    require_file(&model.join("model.onnx"))?;
    require_file(&model.join("voices.bin"))?;
    require_file(&model.join("tokens.txt"))?;
    if lexicons.is_empty() && !model.join("espeak-ng-data").is_dir() {
        anyhow::bail!(
            "Kokoro requires lexicon files or espeak-ng-data in {}",
            model.display()
        );
    }

    let wav = config::cache_dir()?.join("last.wav");
    let mut command = Command::new(&sherpa);
    command
        .arg(format!("--num-threads={}", cfg.num_threads))
        .arg(format!("--speed={}", cfg.speed))
        .arg(format!("--tts-silence-scale={}", cfg.tts_silence_scale))
        .arg("--print-args=false")
        .arg(format!(
            "--kokoro-model={}",
            model.join("model.onnx").display()
        ))
        .arg(format!(
            "--kokoro-voices={}",
            model.join("voices.bin").display()
        ))
        .arg(format!(
            "--kokoro-tokens={}",
            model.join("tokens.txt").display()
        ))
        .arg(format!("--sid={}", kokoro_sid(cfg)));
    if !lexicons.is_empty() {
        command.arg(format!("--kokoro-lexicon={}", join_paths(&lexicons)));
    }
    if model.join("espeak-ng-data").is_dir() {
        command.arg(format!(
            "--kokoro-data-dir={}",
            model.join("espeak-ng-data").display()
        ));
    }
    command
        .arg(format!("--output-filename={}", wav.display()))
        .arg(text);
    run_tts_command(command, "sherpa-onnx kokoro")?;

    if !no_play {
        play_wav(&wav)?;
    }
    Ok(())
}

fn speak_with_sherpa_zipvoice(cfg: &Config, text: &str, no_play: bool) -> Result<()> {
    let sherpa = config::sherpa_bin()?;
    let model = config::zipvoice_model_dir()?;
    let encoder = first_existing(&[model.join("encoder.onnx"), model.join("encoder.int8.onnx")])?;
    let decoder = first_existing(&[model.join("decoder.onnx"), model.join("decoder.int8.onnx")])?;
    let vocoder = first_existing(&[model.join("vocoder.onnx"), model.join("vocos_24khz.onnx")])?;
    let reference_audio = first_existing(&[
        model.join("reference.wav"),
        model.join("test_wavs/leijun-1.wav"),
        model.join("test_wavs/en-1.wav"),
    ])?;
    require_file(&sherpa)?;
    require_file(&model.join("tokens.txt"))?;

    let reference_text = if model.join("reference.txt").is_file() {
        fs::read_to_string(model.join("reference.txt"))?
    } else {
        "小米汽车正式发布会现在开始，今天我们要给大家介绍一个全新的产品。".to_string()
    };
    let wav = config::cache_dir()?.join("last.wav");
    let mut command = Command::new(&sherpa);
    command
        .arg(format!("--num-threads={}", cfg.num_threads))
        .arg("--print-args=false")
        .arg(format!("--zipvoice-encoder={}", encoder.display()))
        .arg(format!("--zipvoice-decoder={}", decoder.display()))
        .arg(format!(
            "--zipvoice-tokens={}",
            model.join("tokens.txt").display()
        ))
        .arg(format!("--zipvoice-vocoder={}", vocoder.display()))
        .arg(format!("--reference-audio={}", reference_audio.display()))
        .arg(format!("--reference-text={}", reference_text.trim()))
        .arg("--num-steps=4");
    if model.join("lexicon.txt").is_file() {
        command.arg(format!(
            "--zipvoice-lexicon={}",
            model.join("lexicon.txt").display()
        ));
    }
    if model.join("espeak-ng-data").is_dir() {
        command.arg(format!(
            "--zipvoice-data-dir={}",
            model.join("espeak-ng-data").display()
        ));
    }
    command
        .arg(format!("--output-filename={}", wav.display()))
        .arg(text);
    run_tts_command(command, "sherpa-onnx zipvoice")?;

    if !no_play {
        play_wav(&wav)?;
    }
    Ok(())
}

fn speak_with_piper(cfg: &Config, text: &str, no_play: bool) -> Result<()> {
    let sherpa = config::sherpa_bin()?;
    let model = config::piper_model_dir()?;
    require_file(&sherpa)?;
    require_file(&model.join("model.onnx"))?;
    require_file(&model.join("lexicon.txt"))?;
    require_file(&model.join("tokens.txt"))?;

    let wav = config::cache_dir()?.join("last.wav");
    let mut command = Command::new(&sherpa);
    command
        .arg(format!("--num-threads={}", cfg.num_threads))
        .arg(format!("--speed={}", cfg.speed))
        .arg(format!("--vits-noise-scale={}", cfg.vits_noise_scale))
        .arg(format!("--vits-noise-scale-w={}", cfg.vits_noise_scale_w))
        .arg(format!("--tts-silence-scale={}", cfg.tts_silence_scale))
        .arg("--print-args=false")
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
        .arg(format!("--output-filename={}", wav.display()))
        .arg(text);
    run_tts_command(command, "sherpa-onnx piper-vits")?;

    if !no_play {
        play_wav(&wav)?;
    }
    Ok(())
}

fn run_tts_command(mut command: Command, label: &str) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("failed to run {label}"))?;
    write_tts_logs(&output.stdout, &output.stderr)?;
    if !output.status.success() {
        anyhow::bail!("{label} failed with status {}", output.status);
    }
    Ok(())
}

fn write_tts_logs(stdout: &[u8], stderr: &[u8]) -> Result<()> {
    fs::write(config::logs_dir()?.join("last-tts.stdout.log"), stdout)?;
    fs::write(config::logs_dir()?.join("last-tts.stderr.log"), stderr)?;
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

fn kokoro_lexicons(model: &Path) -> Vec<PathBuf> {
    [
        model.join("lexicon.txt"),
        model.join("lexicon-us-en.txt"),
        model.join("lexicon-zh.txt"),
    ]
    .into_iter()
    .filter(|path| path.is_file())
    .collect()
}

fn kokoro_sid(cfg: &Config) -> u32 {
    match cfg.voice_profile.as_str() {
        "slow_clear" => 47,
        "quick_preview" => 45,
        _ => 48,
    }
}

fn join_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn first_existing(candidates: &[PathBuf]) -> Result<PathBuf> {
    candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .with_context(|| {
            format!(
                "required file missing; tried {}",
                candidates
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}
