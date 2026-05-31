use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use chrono::Local;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::config::{self, Config};

const MODEL_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-melo-tts-zh_en.tar.bz2";
const KOKORO_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/kokoro-multi-lang-v1_0.tar.bz2";
const ZIPVOICE_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/sherpa-onnx-zipvoice-distill-int8-zh-en-emilia.tar.bz2";
const ZIPVOICE_VOCODER_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/vocoder-models/vocos_24khz.onnx";
const PIPER_ONNX_URL: &str =
    "https://huggingface.co/csukuangfj/vits-piper-zh_CN-huayan-x_low/resolve/main/zh_CN-huayan-x_low.onnx";
const PIPER_JSON_URL: &str =
    "https://huggingface.co/csukuangfj/vits-piper-zh_CN-huayan-x_low/resolve/main/zh_CN-huayan-x_low.onnx.json";
const PIPER_TOKENS_URL: &str =
    "https://huggingface.co/csukuangfj/vits-piper-zh_CN-huayan-x_low/resolve/main/tokens.txt";
const PIPER_LEXICON_URL: &str =
    "https://huggingface.co/csukuangfj/vits-piper-zh_CN-huayan-x_low/resolve/main/lexicon.txt";
const SHERPA_MACOS_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.2/sherpa-onnx-v1.13.2-osx-universal2-shared.tar.bz2";

pub fn install(skip_tts_download: bool) -> Result<()> {
    create_dirs()?;
    install_self_binary()?;
    install_skill()?;
    install_hook()?;

    let previous = install_notify()?;
    let cfg = Config {
        previous_notify: previous,
        ..Config::default()
    };
    cfg.save()?;

    if !skip_tts_download {
        install_tts_assets()?;
    }

    println!("Codex Speak installed at {}", config::app_home()?.display());
    Ok(())
}

pub fn uninstall(remove_models: bool) -> Result<()> {
    restore_notify()?;
    let _ = fs::remove_file(config::codex_home()?.join("hooks/codex-speak-notify"));
    let _ = fs::remove_dir_all(config::codex_home()?.join("skills/codex-speak"));
    if remove_models {
        let _ = fs::remove_dir_all(config::models_dir()?);
    }
    println!("Codex Speak uninstalled");
    Ok(())
}

fn create_dirs() -> Result<()> {
    for dir in [
        config::app_home()?,
        config::bin_dir()?,
        config::tools_dir()?,
        config::models_dir()?,
        config::cache_dir()?,
        config::logs_dir()?,
        config::state_dir()?,
        config::app_home()?.join("backups"),
        config::codex_home()?.join("hooks"),
        config::codex_home()?.join("skills"),
    ] {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

fn install_self_binary() -> Result<()> {
    let current = std::env::current_exe()?;
    let target = config::bin_dir()?.join(binary_name());
    fs::copy(&current, &target).with_context(|| {
        format!(
            "failed to copy {} to {}",
            current.display(),
            target.display()
        )
    })?;
    make_executable(&target)?;
    Ok(())
}

fn install_skill() -> Result<()> {
    let dir = config::codex_home()?.join("skills/codex-speak");
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("SKILL.md"),
        include_str!("../skills/codex-speak/SKILL.md"),
    )?;
    Ok(())
}

fn install_hook() -> Result<()> {
    let hook_path = config::codex_home()?.join("hooks/codex-speak-notify");
    let cli_path = config::bin_dir()?.join(binary_name());
    let content = format!(
        r#"#!/usr/bin/env bash
set -u
"{}" speak >/dev/null 2>&1 || true
exit 0
"#,
        cli_path.display()
    );
    fs::write(&hook_path, content)?;
    make_executable(&hook_path)?;
    Ok(())
}

fn install_notify() -> Result<Option<Vec<String>>> {
    let codex_config = config::codex_home()?.join("config.toml");
    let existing = fs::read_to_string(&codex_config).unwrap_or_default();
    backup_codex_config(&codex_config, &existing)?;

    let previous = parse_previous_notify(&existing);
    fs::write(
        config::state_dir()?.join("previous-notify.json"),
        serde_json::to_string_pretty(&previous)?,
    )?;

    let hook = config::codex_home()?.join("hooks/codex-speak-notify");
    let notify_line = format!(
        "notify = [\"{}\"]",
        toml_escape(&hook.display().to_string())
    );
    let updated = replace_notify_line(&existing, &notify_line);
    fs::write(&codex_config, updated)?;
    Ok(previous)
}

fn restore_notify() -> Result<()> {
    let codex_config = config::codex_home()?.join("config.toml");
    let existing = fs::read_to_string(&codex_config).unwrap_or_default();
    let previous_path = config::state_dir()?.join("previous-notify.json");
    let previous: Option<Vec<String>> = if previous_path.exists() {
        serde_json::from_str(&fs::read_to_string(previous_path)?)?
    } else {
        None
    };

    let updated = if let Some(previous) = previous {
        let notify = format!(
            "notify = [{}]",
            previous
                .iter()
                .map(|item| format!("\"{}\"", toml_escape(item)))
                .collect::<Vec<_>>()
                .join(", ")
        );
        replace_notify_line(&existing, &notify)
    } else {
        existing
            .lines()
            .filter(|line| !line.trim_start().starts_with("notify ="))
            .collect::<Vec<_>>()
            .join("\n")
    };

    fs::write(codex_config, updated)?;
    Ok(())
}

fn backup_codex_config(path: &Path, content: &str) -> Result<()> {
    let ts = Local::now().format("%Y%m%d-%H%M%S");
    let backup = config::app_home()?
        .join("backups")
        .join(format!("config-{ts}.toml"));
    if path.exists() {
        fs::write(backup, content)?;
    }
    Ok(())
}

fn parse_previous_notify(config_text: &str) -> Option<Vec<String>> {
    let value = toml::from_str::<toml::Value>(config_text).ok()?;
    let notify = value.get("notify")?.as_array()?;
    let mut items = Vec::new();
    for item in notify {
        let Some(s) = item.as_str() else {
            continue;
        };
        if s == "--previous-notify" {
            break;
        }
        if s.contains("codex-speak") {
            return None;
        }
        items.push(s.to_string());
    }
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

fn replace_notify_line(existing: &str, notify_line: &str) -> String {
    let mut replaced = false;
    let mut lines = Vec::new();
    for line in existing.lines() {
        if !replaced && line.trim_start().starts_with("notify =") {
            lines.push(notify_line.to_string());
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.insert(0, notify_line.to_string());
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn install_tts_assets() -> Result<()> {
    install_tts_runtime()?;
    install_melo_model()?;
    Ok(())
}

pub fn install_all_models() -> Result<()> {
    for provider in ["sherpa_melo", "sherpa_kokoro", "piper", "sherpa_zipvoice"] {
        install_model(provider)?;
    }
    Ok(())
}

pub fn install_model(provider: &str) -> Result<()> {
    create_dirs()?;
    match provider {
        "sherpa_melo" => {
            install_tts_runtime()?;
            install_melo_model()
        }
        "sherpa_kokoro" => {
            install_tts_runtime()?;
            install_kokoro_model()
        }
        "sherpa_zipvoice" => {
            install_tts_runtime()?;
            install_zipvoice_model()
        }
        "piper" => {
            install_tts_runtime()?;
            install_piper_model()
        }
        "system" => {
            eprintln!("System speech does not require a model download.");
            Ok(())
        }
        other => anyhow::bail!("unsupported provider: {other}"),
    }
}

fn install_tts_runtime() -> Result<()> {
    if cfg!(target_os = "macos") {
        install_sherpa_macos()?;
    } else {
        eprintln!("Skipping Sherpa download on this platform for now");
    }
    Ok(())
}

fn install_sherpa_macos() -> Result<()> {
    if config::sherpa_bin()?.exists() {
        return Ok(());
    }
    let archive = config::cache_dir()?.join("sherpa-onnx.tar.bz2");
    download(SHERPA_MACOS_URL, &archive)?;
    let extract_dir = config::cache_dir()?.join("sherpa-onnx-extract");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir)?;
    untar_bzip2(&archive, &extract_dir)?;

    let target_dir = config::tools_dir()?.join("sherpa-onnx");
    let _ = fs::remove_dir_all(&target_dir);
    fs::create_dir_all(&target_dir)?;
    let root =
        find_sherpa_root(&extract_dir).context("could not find sherpa-onnx root in archive")?;
    copy_dir_recursive(&root, &target_dir)?;
    make_executable(&target_dir.join("bin/sherpa-onnx-offline-tts"))?;
    Ok(())
}

fn install_melo_model() -> Result<()> {
    if config::model_dir()?.join("model.onnx").exists() {
        return Ok(());
    }
    let archive = config::cache_dir()?.join("vits-melo-tts-zh_en.tar.bz2");
    download(MODEL_URL, &archive)?;
    untar_bzip2(&archive, &config::models_dir()?)?;
    Ok(())
}

fn install_kokoro_model() -> Result<()> {
    let target = config::kokoro_model_dir()?;
    if target.join("model.onnx").exists() && target.join("voices.bin").exists() {
        return Ok(());
    }
    let archive = config::cache_dir()?.join("kokoro-multi-lang-v1_0.tar.bz2");
    download(KOKORO_URL, &archive)?;
    let extract_dir = config::cache_dir()?.join("kokoro-extract");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir)?;
    untar_bzip2(&archive, &extract_dir)?;
    let root = find_child_dir(&extract_dir, "kokoro-multi-lang-v1_0")
        .context("could not find Kokoro model root in archive")?;
    replace_dir(&root, &target)?;
    Ok(())
}

fn install_zipvoice_model() -> Result<()> {
    let target = config::zipvoice_model_dir()?;
    if target.join("encoder.onnx").exists()
        && target.join("decoder.onnx").exists()
        && target.join("vocoder.onnx").exists()
    {
        return Ok(());
    }
    let archive = config::cache_dir()?.join("zipvoice-zh-en.tar.bz2");
    download(ZIPVOICE_URL, &archive)?;
    let extract_dir = config::cache_dir()?.join("zipvoice-extract");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir)?;
    untar_bzip2(&archive, &extract_dir)?;
    let root = find_child_dir(
        &extract_dir,
        "sherpa-onnx-zipvoice-distill-int8-zh-en-emilia",
    )
    .context("could not find ZipVoice model root in archive")?;
    replace_dir(&root, &target)?;
    normalize_zipvoice_files(&target)?;
    download(ZIPVOICE_VOCODER_URL, &target.join("vocoder.onnx"))?;
    ensure_zipvoice_reference(&target)?;
    Ok(())
}

fn install_piper_model() -> Result<()> {
    let target = config::piper_model_dir()?;
    if target.join("model.onnx").exists()
        && target.join("tokens.txt").exists()
        && target.join("lexicon.txt").exists()
    {
        return Ok(());
    }
    fs::create_dir_all(&target)?;
    download(PIPER_ONNX_URL, &target.join("model.onnx"))?;
    download(PIPER_JSON_URL, &target.join("model.onnx.json"))?;
    download(PIPER_TOKENS_URL, &target.join("tokens.txt"))?;
    download(PIPER_LEXICON_URL, &target.join("lexicon.txt"))?;
    Ok(())
}

fn download(url: &str, dest: &Path) -> Result<()> {
    eprintln!("Downloading {url}");
    let status = Command::new("curl")
        .args(["-L", "--fail", "--progress-bar", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .with_context(|| format!("failed to run curl for {url}"))?;
    if !status.success() {
        anyhow::bail!("download failed: {url}");
    }
    Ok(())
}

fn untar_bzip2(archive: &Path, dest: &Path) -> Result<()> {
    let status = Command::new("tar")
        .arg("xjf")
        .arg(archive)
        .arg("-C")
        .arg(dest)
        .status()
        .with_context(|| format!("failed to extract {}", archive.display()))?;
    if !status.success() {
        anyhow::bail!("tar failed for {}", archive.display());
    }
    Ok(())
}

fn find_sherpa_root(root: &Path) -> Option<PathBuf> {
    for entry in walkdir::WalkDir::new(root)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.join("bin/sherpa-onnx-offline-tts").exists() && path.join("lib").is_dir() {
            return Some(path.to_path_buf());
        }
    }
    None
}

fn find_child_dir(root: &Path, name: &str) -> Option<PathBuf> {
    for entry in walkdir::WalkDir::new(root)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_dir() && entry.file_name() == name {
            return Some(entry.path().to_path_buf());
        }
    }
    None
}

fn replace_dir(src: &Path, dst: &Path) -> Result<()> {
    let _ = fs::remove_dir_all(dst);
    fs::create_dir_all(dst)?;
    copy_dir_recursive(src, dst)
}

fn normalize_zipvoice_files(dir: &Path) -> Result<()> {
    copy_first_existing(
        &[
            dir.join("encoder.onnx"),
            dir.join("encoder.int8.onnx"),
            dir.join("model.int8.onnx"),
        ],
        &dir.join("encoder.onnx"),
    )?;
    copy_first_existing(
        &[dir.join("decoder.onnx"), dir.join("decoder.int8.onnx")],
        &dir.join("decoder.onnx"),
    )?;
    copy_first_existing(
        &[
            dir.join("tokens.txt"),
            dir.join("tokens_en.txt"),
            dir.join("tokens_zh.txt"),
        ],
        &dir.join("tokens.txt"),
    )?;
    Ok(())
}

fn ensure_zipvoice_reference(dir: &Path) -> Result<()> {
    let reference_wav = dir.join("reference.wav");
    if !reference_wav.exists() {
        copy_first_existing(
            &[
                dir.join("test_wavs/leijun-1.wav"),
                dir.join("test_wavs/en-1.wav"),
                dir.join("test_wavs/0.wav"),
            ],
            &reference_wav,
        )?;
    }
    let reference_txt = dir.join("reference.txt");
    if !reference_txt.exists() {
        fs::write(
            reference_txt,
            "小米汽车正式发布会现在开始，今天我们要给大家介绍一个全新的产品。",
        )?;
    }
    Ok(())
}

fn copy_first_existing(candidates: &[PathBuf], dest: &Path) -> Result<()> {
    if dest.exists() {
        return Ok(());
    }
    let Some(src) = candidates.iter().find(|path| path.exists()) else {
        anyhow::bail!("could not find any candidate file for {}", dest.display());
    };
    fs::copy(src, dest)?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let rel = path.strip_prefix(src)?;
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)?;
        }
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
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

fn toml_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_existing_notify() {
        let out = replace_notify_line("model = \"x\"\nnotify = [\"old\"]\n", "notify = [\"new\"]");
        assert!(out.contains("notify = [\"new\"]"));
        assert!(!out.contains("[\"old\"]"));
    }

    #[test]
    fn parses_previous_notify_without_nested_codex_speak() {
        let raw = r#"notify = ["/a/SkyComputerUseClient", "turn-ended", "--previous-notify", "[\"/old/codex-speak-notify\"]"]"#;
        let previous = parse_previous_notify(raw).unwrap();
        assert_eq!(previous, vec!["/a/SkyComputerUseClient", "turn-ended"]);
    }
}
