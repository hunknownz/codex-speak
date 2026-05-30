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
    if cfg!(target_os = "macos") {
        install_sherpa_macos()?;
    } else {
        println!("Skipping Sherpa download on this platform for now");
    }
    install_melo_model()?;
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

fn download(url: &str, dest: &Path) -> Result<()> {
    println!("Downloading {url}");
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
