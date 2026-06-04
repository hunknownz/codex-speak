use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::config::{self, Config};
use crate::pet_state;
use crate::process::{self, PlaybackPolicy};
use crate::pronunciation;

const WAV_TAIL_SILENCE_MS: u32 = 450;

pub fn speak(cfg: &Config, text: &str, no_play: bool) -> Result<()> {
    speak_with_policy(cfg, text, no_play, PlaybackPolicy::Interrupt).map(|_| ())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeakOutcome {
    Played,
    SkippedBusy,
    Disabled,
}

pub fn speak_with_policy(
    cfg: &Config,
    text: &str,
    no_play: bool,
    policy: PlaybackPolicy,
) -> Result<SpeakOutcome> {
    if !cfg.enabled {
        return Ok(SpeakOutcome::Disabled);
    }

    fs::create_dir_all(config::logs_dir()?)?;
    fs::create_dir_all(config::cache_dir()?)?;

    let spoken_text = pronunciation::normalize_for_tts(text);
    let spoken_text = if spoken_text.trim().is_empty() {
        text.to_string()
    } else {
        spoken_text
    };

    let Some(_playback_guard) = process::acquire_playback(policy)? else {
        fs::write(
            config::logs_dir()?.join("last-skipped.txt"),
            "speech queue was busy; skipped a short prompt\n",
        )?;
        let _ = pet_state::write_state("idle", Some("朗读队列正忙，跳过这句短提示。"), "tts");
        return Ok(SpeakOutcome::SkippedBusy);
    };

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
    Ok(SpeakOutcome::Played)
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
    add_wav_tail_silence(&wav, WAV_TAIL_SILENCE_MS)?;

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
    add_wav_tail_silence(&wav, WAV_TAIL_SILENCE_MS)?;

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
    add_wav_tail_silence(&wav, WAV_TAIL_SILENCE_MS)?;

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
    add_wav_tail_silence(&wav, WAV_TAIL_SILENCE_MS)?;

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

fn add_wav_tail_silence(path: &Path, tail_ms: u32) -> Result<()> {
    if tail_ms == 0 {
        return Ok(());
    }
    let mut bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        anyhow::bail!("{} is not a RIFF/WAVE file", path.display());
    }

    let mut cursor = 12usize;
    let mut byte_rate = None;
    let mut block_align = None;
    let mut data_chunk = None;
    while cursor + 8 <= bytes.len() {
        let chunk_id = &bytes[cursor..cursor + 4];
        let chunk_size = read_u32_le(&bytes, cursor + 4)? as usize;
        let chunk_start = cursor + 8;
        let chunk_end = chunk_start
            .checked_add(chunk_size)
            .context("wav chunk size overflow")?;
        if chunk_end > bytes.len() {
            anyhow::bail!("{} has a truncated wav chunk", path.display());
        }
        if chunk_id == b"fmt " && chunk_size >= 16 {
            byte_rate = Some(read_u32_le(&bytes, chunk_start + 8)?);
            block_align = Some(read_u16_le(&bytes, chunk_start + 12)? as usize);
        } else if chunk_id == b"data" {
            data_chunk = Some((cursor, chunk_start, chunk_size));
            break;
        }
        cursor = chunk_end + (chunk_size % 2);
    }

    let byte_rate = byte_rate.context("wav fmt chunk missing byte rate")?;
    let block_align = block_align.filter(|value| *value > 0).unwrap_or(1);
    let (data_header, data_start, data_size) = data_chunk.context("wav data chunk missing")?;
    let data_end = data_start
        .checked_add(data_size)
        .context("wav data chunk overflow")?;
    let raw_tail_bytes = (byte_rate as usize)
        .saturating_mul(tail_ms as usize)
        .checked_div(1000)
        .unwrap_or(0);
    let tail_bytes = align_up(raw_tail_bytes.max(block_align), block_align);
    let new_data_size = data_size
        .checked_add(tail_bytes)
        .context("wav data size overflow")?;
    if new_data_size > u32::MAX as usize {
        anyhow::bail!("wav data chunk too large after tail padding");
    }

    bytes.splice(data_end..data_end, std::iter::repeat(0).take(tail_bytes));
    write_u32_le(&mut bytes, data_header + 4, new_data_size as u32)?;
    let riff_size = bytes
        .len()
        .checked_sub(8)
        .context("wav riff size underflow")?;
    if riff_size > u32::MAX as usize {
        anyhow::bail!("wav file too large after tail padding");
    }
    write_u32_le(&mut bytes, 4, riff_size as u32)?;
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16> {
    let slice = bytes
        .get(offset..offset + 2)
        .context("wav u16 read out of range")?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32> {
    let slice = bytes
        .get(offset..offset + 4)
        .context("wav u32 read out of range")?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn write_u32_le(bytes: &mut [u8], offset: usize, value: u32) -> Result<()> {
    let target = bytes
        .get_mut(offset..offset + 4)
        .context("wav u32 write out of range")?;
    target.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn align_up(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        value.div_ceil(align) * align
    }
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

    #[test]
    fn adds_wav_tail_silence_and_updates_sizes() {
        let file = tempfile::NamedTempFile::new().expect("temp wav");
        let path = file.path();
        fs::write(path, minimal_wav(4)).expect("write wav");

        add_wav_tail_silence(path, 500).expect("pad wav");

        let bytes = fs::read(path).expect("read wav");
        let riff_size = read_u32_le(&bytes, 4).expect("riff size") as usize;
        let data_size = read_u32_le(&bytes, 40).expect("data size") as usize;
        assert_eq!(riff_size, bytes.len() - 8);
        assert_eq!(data_size, 4 + 16000);
        assert_eq!(bytes.len(), 44 + data_size);
        assert!(bytes[44 + 4..].iter().all(|byte| *byte == 0));
    }

    fn minimal_wav(data_size: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&16000u32.to_le_bytes());
        bytes.extend_from_slice(&32000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        bytes.extend(std::iter::repeat(1).take(data_size as usize));
        bytes
    }
}
