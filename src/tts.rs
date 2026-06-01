use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::config::{self, Config};
use crate::pet_state;
use crate::process;
use crate::pronunciation;

pub fn speak(cfg: &Config, text: &str, no_play: bool) -> Result<()> {
    if !cfg.enabled {
        return Ok(());
    }

    fs::create_dir_all(config::logs_dir()?)?;
    fs::create_dir_all(config::cache_dir()?)?;

    let spoken_text = pronunciation::normalize_for_tts(text);
    let spoken_text = if spoken_text.trim().is_empty() {
        text.to_string()
    } else {
        spoken_text
    };

    process::stop_speech()?;
    let _ = pet_state::write_state("speaking", Some(&spoken_text), "tts");

    let result = if let Err(err) = speak_with_provider(cfg, &spoken_text, no_play) {
        fs::write(
            config::logs_dir()?.join("last-error.log"),
            format!("{err:#}\n"),
        )?;
        if cfg.provider == "sherpa_melo" && cfg.fallback_provider == "system" {
            speak_with_system(&spoken_text, no_play)
        } else {
            Err(err)
        }
    } else {
        Ok(())
    };

    if let Err(err) = result {
        let _ = pet_state::write_state("error", Some(&err.to_string()), "tts");
        return Err(err);
    }

    fs::write(config::logs_dir()?.join("last-spoken.txt"), spoken_text)?;
    let _ = pet_state::write_state("done", Some("朗读完成。"), "tts");
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
    let segments = split_system_speech_segments(text);
    if segments.is_empty() {
        return Ok(());
    }
    if cfg!(target_os = "macos") {
        let script = segments
            .iter()
            .map(|segment| {
                format!(
                    "/usr/bin/say -v {} -r 175 {}",
                    macos_voice(segment.lang),
                    sh_literal(&segment.text)
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let mut command = Command::new("/bin/sh");
        command.args(["-c", &script]);
        process::run_tracked(command, "macOS say")?;
    } else if cfg!(windows) {
        let mut script = String::from(
            "Add-Type -AssemblyName System.Speech; \
             Add-Type -AssemblyName System.Globalization; \
             $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
             function Use-Culture($name) { \
               try { \
                 $s.SelectVoiceByHints(\
                   [System.Speech.Synthesis.VoiceGender]::NotSet, \
                   [System.Speech.Synthesis.VoiceAge]::NotSet, \
                   0, \
                   [System.Globalization.CultureInfo]::GetCultureInfo($name)\
                 ); \
               } catch {} \
             };",
        );
        for segment in &segments {
            script.push_str(&format!(
                " Use-Culture '{}'; $s.Speak({});",
                windows_culture(segment.lang),
                ps_literal(&segment.text)
            ));
        }
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-Command", &script]);
        process::run_tracked(command, "Windows system speech")?;
    } else {
        anyhow::bail!("no system speech fallback for this platform");
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SystemSpeechLang {
    Chinese,
    English,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SystemSpeechSegment {
    lang: SystemSpeechLang,
    text: String,
}

fn split_system_speech_segments(text: &str) -> Vec<SystemSpeechSegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_lang: Option<SystemSpeechLang> = None;

    for ch in text.chars() {
        if let Some(next_lang) = classify_speech_char(ch) {
            if let Some(lang) = current_lang {
                if lang != next_lang && !current.trim().is_empty() {
                    segments.push(SystemSpeechSegment {
                        lang,
                        text: current,
                    });
                    current = String::new();
                }
            }
            current_lang = Some(next_lang);
        }
        current.push(ch);
    }

    if !current.trim().is_empty() {
        segments.push(SystemSpeechSegment {
            lang: current_lang.unwrap_or(SystemSpeechLang::Chinese),
            text: current,
        });
    }

    segments
}

fn classify_speech_char(ch: char) -> Option<SystemSpeechLang> {
    if ch.is_ascii_alphabetic() {
        Some(SystemSpeechLang::English)
    } else if is_cjk(ch) {
        Some(SystemSpeechLang::Chinese)
    } else {
        None
    }
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2B73F}'
            | '\u{2B740}'..='\u{2B81F}'
            | '\u{2B820}'..='\u{2CEAF}'
    )
}

fn macos_voice(lang: SystemSpeechLang) -> &'static str {
    match lang {
        SystemSpeechLang::Chinese => "Tingting",
        SystemSpeechLang::English => "Samantha",
    }
}

fn windows_culture(lang: SystemSpeechLang) -> &'static str {
    match lang {
        SystemSpeechLang::Chinese => "zh-CN",
        SystemSpeechLang::English => "en-US",
    }
}

fn sh_literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn play_wav(path: &Path) -> Result<()> {
    if cfg!(target_os = "macos") {
        let mut command = Command::new("/usr/bin/afplay");
        command.arg(path);
        process::run_tracked(command, "afplay")?;
    } else if cfg!(windows) {
        let script = format!(
            "(New-Object Media.SoundPlayer {}).PlaySync();",
            ps_literal(&path.display().to_string())
        );
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-Command", &script]);
        process::run_tracked(command, "PowerShell audio playback")?;
    } else {
        anyhow::bail!("no wav player configured for this platform");
    }
    Ok(())
}

fn ps_literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powershell_literal_escapes_single_quotes() {
        assert_eq!(ps_literal("C:\\Kids\\莎莎.wav"), "'C:\\Kids\\莎莎.wav'");
        assert_eq!(ps_literal("it's ok"), "'it''s ok'");
    }

    #[test]
    fn shell_literal_escapes_single_quotes() {
        assert_eq!(sh_literal("it's ok"), "'it'\\''s ok'");
    }

    #[test]
    fn splits_mixed_chinese_and_english_for_system_speech() {
        let segments = split_system_speech_segments("我运行 hello world，然后继续。");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].lang, SystemSpeechLang::Chinese);
        assert_eq!(segments[1].lang, SystemSpeechLang::English);
        assert_eq!(segments[1].text, "hello world，");
        assert_eq!(segments[2].lang, SystemSpeechLang::Chinese);
    }
}
