use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use chrono::Local;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::bundled;
use crate::config::{self, Config};

const PLUGIN_NAME: &str = "codex-speak";
const AGENTS_BLOCK_START: &str = "<!-- BEGIN CODEX SPEAK GROWTH MODE -->";
const AGENTS_BLOCK_END: &str = "<!-- END CODEX SPEAK GROWTH MODE -->";
const AGENTS_BLOCK: &str = r#"<!-- BEGIN CODEX SPEAK GROWTH MODE -->
## Codex Speak Growth Mode

When answering, write in a gentle growth-mode style suitable for a child to read and hear. Keep the visible final answer concise, warm, concrete, and naturally speakable.

Put the most important result first. Avoid long code blocks, raw logs, long paths, and command dumps unless the user specifically needs them. When technical detail is necessary, keep a short child-friendly summary before the detail so the local speech hook can read the answer naturally.

Do not add hidden protocol blocks, HTML speech blocks, or separate "朗读导览" sections for normal replies.
<!-- END CODEX SPEAK GROWTH MODE -->
"#;

struct DownloadAsset {
    url: &'static str,
    sha256: Option<&'static str>,
}

const MELO_MODEL: DownloadAsset = DownloadAsset {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-melo-tts-zh_en.tar.bz2",
    sha256: Some("e58351ed7149f290a54534538badd4077cdbe6fddc964b24d0bee870415d1514"),
};
const KOKORO_MODEL: DownloadAsset = DownloadAsset {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/kokoro-multi-lang-v1_0.tar.bz2",
    sha256: Some("c133d26353d776da730870dac7da07dbfc9a5e3bc80cc5e8e83ab6e823be7046"),
};
const ZIPVOICE_MODEL: DownloadAsset = DownloadAsset {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/sherpa-onnx-zipvoice-distill-int8-zh-en-emilia.tar.bz2",
    sha256: Some("77219c8b40f4ee8d73a7f902305ff6c1128ef9b54461c41b4ca6ed890b6c2803"),
};
const ZIPVOICE_VOCODER: DownloadAsset = DownloadAsset {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/vocoder-models/vocos_24khz.onnx",
    sha256: Some("bcb3b970e384161c4d634f0bb9e999ff1c471b34c9bc0b1049a5014065ed3cc0"),
};
const PIPER_ONNX: DownloadAsset = DownloadAsset {
    url: "https://huggingface.co/csukuangfj/vits-piper-zh_CN-huayan-x_low/resolve/main/zh_CN-huayan-x_low.onnx",
    sha256: Some("74ab713ba7c6d5e8b0b690a85d468c8fe7a4bc531759dbada9a47ced6007af71"),
};
const PIPER_JSON: DownloadAsset = DownloadAsset {
    url: "https://huggingface.co/csukuangfj/vits-piper-zh_CN-huayan-x_low/resolve/main/zh_CN-huayan-x_low.onnx.json",
    sha256: Some("5521dcb09adf68a9bee289032f7f5af18d29bff020953429b5d223ec1f881816"),
};
const PIPER_TOKENS: DownloadAsset = DownloadAsset {
    url: "https://huggingface.co/csukuangfj/vits-piper-zh_CN-huayan-x_low/resolve/main/tokens.txt",
    sha256: Some("42d1a69ed2b91a51928a711aa228ed9f3dc021c6d359a3e9c4f37eb1d20f80bd"),
};
const PIPER_LEXICON: DownloadAsset = DownloadAsset {
    url: "https://huggingface.co/csukuangfj/vits-piper-zh_CN-huayan-x_low/resolve/main/lexicon.txt",
    sha256: Some("13e5192297791d93f8d63c353a0568f8ebb57a3a229dfb041043bcd230b3059d"),
};
const SHERPA_MACOS: DownloadAsset = DownloadAsset {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.2/sherpa-onnx-v1.13.2-osx-universal2-shared.tar.bz2",
    sha256: Some("5cb77e97dabff363e9cb5a94e20b98de642fff7cb07b0062082ae254e1edfa4c"),
};
const SHERPA_WINDOWS: DownloadAsset = DownloadAsset {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.2/sherpa-onnx-v1.13.2-win-x64-shared-MD-Release.tar.bz2",
    sha256: Some("f91f488186e797dd9e9bc2a3dcbe18ddd244627af5d9fa3707f7a2f3bc4032ce"),
};

pub fn install(skip_tts_download: bool, no_summary: bool) -> Result<()> {
    create_dirs()?;
    install_self_binary()?;
    install_release_manifest()?;
    install_skill()?;
    remove_global_mcp_config()?;
    install_hook()?;
    install_stop_hook()?;
    install_agents_block()?;
    remove_notify_if_codex_speak()?;
    let mut cfg = Config::load_or_default()?;
    cfg.previous_notify = None;
    cfg.save()?;

    if !skip_tts_download {
        install_tts_assets()?;
    }

    println!("Codex Speak installed at {}", config::app_home()?.display());
    if !no_summary {
        print_install_summary(skip_tts_download, config::control_app_path()?.exists())?;
    }
    Ok(())
}

pub fn uninstall(remove_models: bool) -> Result<()> {
    remove_notify_if_codex_speak()?;
    remove_stop_hook()?;
    remove_agents_block()?;
    remove_global_mcp_config()?;
    let _ = fs::remove_file(config::codex_home()?.join("hooks/codex-speak-notify"));
    let _ = fs::remove_file(config::codex_home()?.join("hooks/codex-speak-notify.ps1"));
    let _ = fs::remove_file(config::codex_home()?.join("hooks/codex-speak-stop-hook"));
    let _ = fs::remove_file(config::codex_home()?.join("hooks/codex-speak-stop-hook.ps1"));
    let _ = fs::remove_dir_all(config::codex_home()?.join("skills/codex-speak"));
    let _ = cleanup_legacy_plugin_files();
    let _ = fs::remove_dir_all(config::queue_pending_dir()?);
    let _ = fs::remove_dir_all(config::queue_running_dir()?);
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
        config::apps_dir()?,
        config::tools_dir()?,
        config::models_dir()?,
        config::cache_dir()?,
        config::logs_dir()?,
        config::state_dir()?,
        config::queue_pending_dir()?,
        config::queue_running_dir()?,
        config::queue_done_dir()?,
        config::queue_failed_dir()?,
        config::app_home()?.join("backups"),
        config::codex_home()?.join("hooks"),
        config::codex_home()?.join("skills"),
    ] {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

pub fn print_install_summary(skip_tts_download: bool, include_control_app: bool) -> Result<()> {
    let cli = config::bin_dir()?.join(binary_name());
    println!();
    println!("Next steps:");
    println!("  1. Run self-check:");
    println!("     {} doctor", cli.display());
    if include_control_app {
        println!("  2. Open the control app:");
        println!("     {} app open", cli.display());
        println!("  3. If you need help, create a support bundle:");
    } else {
        println!("  2. If you need help, create a support bundle:");
    }
    println!("     {} support-bundle", cli.display());
    if skip_tts_download {
        let step = if include_control_app { 4 } else { 3 };
        println!("  {step}. Install the default local Chinese voice model when ready:");
        println!(
            "     {} models install --provider sherpa_melo",
            cli.display()
        );
    }
    Ok(())
}

fn install_self_binary() -> Result<()> {
    let current = std::env::current_exe()?;
    let target = config::bin_dir()?.join(binary_name());
    let temp = target.with_extension(format!("tmp.{}", std::process::id()));
    fs::copy(&current, &temp)
        .with_context(|| format!("failed to copy {} to {}", current.display(), temp.display()))?;
    make_executable(&temp)?;
    replace_file(&temp, &target)?;
    Ok(())
}

fn install_release_manifest() -> Result<()> {
    let current = std::env::current_exe()?;
    let Some(package_root) = current.parent().and_then(Path::parent) else {
        return Ok(());
    };
    let source = package_root.join("release-manifest.json");
    if !source.is_file() {
        return Ok(());
    }

    let target = config::release_manifest_path()?;
    let source_canonical = source.canonicalize().ok();
    let target_canonical = target.canonicalize().ok();
    if source_canonical.is_some() && source_canonical == target_canonical {
        return Ok(());
    }

    let temp = target.with_extension(format!("tmp.{}", std::process::id()));
    fs::copy(&source, &temp).with_context(|| {
        format!(
            "failed to copy release manifest {} to {}",
            source.display(),
            temp.display()
        )
    })?;
    replace_data_file(&temp, &target)
}

fn replace_file(source: &Path, target: &Path) -> Result<()> {
    #[cfg(windows)]
    if target.exists() {
        fs::remove_file(target)
            .with_context(|| format!("failed to remove {}", target.display()))?;
    }

    fs::rename(source, target).with_context(|| {
        format!(
            "failed to move {} to {}",
            source.display(),
            target.display()
        )
    })?;
    make_executable(&target)?;
    Ok(())
}

fn replace_data_file(source: &Path, target: &Path) -> Result<()> {
    #[cfg(windows)]
    if target.exists() {
        fs::remove_file(target)
            .with_context(|| format!("failed to remove {}", target.display()))?;
    }

    fs::rename(source, target).with_context(|| {
        format!(
            "failed to move {} to {}",
            source.display(),
            target.display()
        )
    })
}

fn install_skill() -> Result<()> {
    let dir = config::codex_home()?.join("skills/codex-speak");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("SKILL.md"), bundled::CODEX_SKILL)?;
    fs::write(
        dir.join("speech-style-examples.jsonl"),
        bundled::CODEX_SPEECH_STYLE_EXAMPLES,
    )?;
    Ok(())
}

#[allow(dead_code)]
fn install_global_mcp_config() -> Result<()> {
    let codex_config = config::codex_home()?.join("config.toml");
    let existing = fs::read_to_string(&codex_config).unwrap_or_default();
    backup_codex_config(&codex_config, &existing)?;

    let cli_path = config::bin_dir()?.join(binary_name());
    let table = format_global_mcp_table(&cli_path);
    let updated = upsert_global_mcp_table(&existing, &table);
    fs::write(&codex_config, updated)?;
    Ok(())
}

fn remove_global_mcp_config() -> Result<()> {
    let codex_config = config::codex_home()?.join("config.toml");
    let existing = fs::read_to_string(&codex_config).unwrap_or_default();
    let updated = remove_global_mcp_table(&existing);
    fs::write(&codex_config, updated)?;
    Ok(())
}

fn cleanup_legacy_plugin_files() -> Result<()> {
    let _ = fs::remove_dir_all(config::installed_plugin_dir()?);
    let _ = fs::remove_dir_all(legacy_installed_plugin_dir()?);
    remove_personal_marketplace_entry()
}

fn install_hook() -> Result<()> {
    let hook_path = hook_path()?;
    let cli_path = config::bin_dir()?.join(binary_name());
    fs::write(&hook_path, bundled::hook_content(&cli_path))?;
    make_executable(&hook_path)?;
    Ok(())
}

fn install_stop_hook() -> Result<()> {
    let path = hooks_json_path()?;
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut root = read_hooks_json(&existing);
    remove_codex_speak_hooks_from_value(&mut root);

    let hooks = root
        .as_object_mut()
        .expect("hooks root is object")
        .entry("hooks")
        .or_insert_with(|| json!({}));
    let hooks_obj = hooks.as_object_mut().context("hooks must be an object")?;
    let stop = hooks_obj.entry("Stop").or_insert_with(|| json!([]));
    let stop_arr = stop.as_array_mut().context("hooks.Stop must be an array")?;
    stop_arr.push(json!({
        "hooks": [{
            "type": "command",
            "command": stop_hook_command(&hook_path()?),
            "timeout": 5,
            "statusMessage": "Queueing Codex Speak growth-mode audio"
        }]
    }));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(&root)?)?;
    Ok(())
}

fn remove_stop_hook() -> Result<()> {
    let path = hooks_json_path()?;
    if !path.exists() {
        return Ok(());
    }
    let existing = fs::read_to_string(&path)?;
    let mut root = read_hooks_json(&existing);
    remove_codex_speak_hooks_from_value(&mut root);
    fs::write(&path, serde_json::to_string_pretty(&root)?)?;
    Ok(())
}

fn install_agents_block() -> Result<()> {
    let path = agents_path()?;
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let cleaned = remove_agents_block_from_text(&existing);
    let mut updated = cleaned.trim_end().to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(AGENTS_BLOCK);
    fs::write(path, updated)?;
    Ok(())
}

fn remove_agents_block() -> Result<()> {
    let path = agents_path()?;
    if !path.exists() {
        return Ok(());
    }
    let existing = fs::read_to_string(&path)?;
    let updated = remove_agents_block_from_text(&existing);
    fs::write(path, updated)?;
    Ok(())
}

pub(crate) fn agents_block_present(raw: &str) -> bool {
    raw.contains(AGENTS_BLOCK_START) && raw.contains(AGENTS_BLOCK_END)
}

fn remove_agents_block_from_text(raw: &str) -> String {
    let Some(start) = raw.find(AGENTS_BLOCK_START) else {
        return raw.to_string();
    };
    let Some(end_rel) = raw[start..].find(AGENTS_BLOCK_END) else {
        return raw.to_string();
    };
    let end = start + end_rel + AGENTS_BLOCK_END.len();
    let mut out = String::new();
    out.push_str(raw[..start].trim_end());
    let tail = raw[end..].trim_start_matches(['\r', '\n']);
    if !out.is_empty() && !tail.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(tail);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn hooks_json_path() -> Result<PathBuf> {
    Ok(config::codex_home()?.join("hooks.json"))
}

fn agents_path() -> Result<PathBuf> {
    Ok(config::codex_home()?.join("AGENTS.md"))
}

fn read_hooks_json(raw: &str) -> Value {
    let value = serde_json::from_str(raw).unwrap_or_else(|_| json!({ "hooks": {} }));
    if value.is_object() {
        value
    } else {
        json!({ "hooks": {} })
    }
}

fn remove_codex_speak_hooks_from_value(root: &mut Value) {
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else {
        return;
    };
    for groups in hooks.values_mut() {
        let Some(groups) = groups.as_array_mut() else {
            continue;
        };
        for group in groups.iter_mut() {
            let Some(commands) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
                continue;
            };
            commands.retain(|item| {
                !item
                    .get("command")
                    .and_then(Value::as_str)
                    .is_some_and(is_codex_speak_hook_command)
            });
        }
        groups.retain(|group| {
            group
                .get("hooks")
                .and_then(Value::as_array)
                .is_none_or(|commands| !commands.is_empty())
        });
    }
}

fn is_codex_speak_hook_command(command: &str) -> bool {
    command.contains("codex-speak-notify")
        || command.contains("codex-speak-stop-hook")
        || (command.contains("codex-speak") && command.contains("hook --stdin"))
}

fn stop_hook_command(hook: &Path) -> String {
    if cfg!(windows) {
        format!(
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"{}\"",
            shell_escape(hook)
        )
    } else {
        format!("\"{}\"", shell_escape(hook))
    }
}

fn shell_escape(path: &Path) -> String {
    path.display().to_string().replace('"', "\\\"")
}

#[allow(dead_code)]
fn install_notify() -> Result<Option<Vec<String>>> {
    let codex_config = config::codex_home()?.join("config.toml");
    let existing = fs::read_to_string(&codex_config).unwrap_or_default();
    backup_codex_config(&codex_config, &existing)?;

    let previous = parse_previous_notify(&existing).or_else(read_saved_previous_notify);
    fs::write(
        config::state_dir()?.join("previous-notify.json"),
        serde_json::to_string_pretty(&previous)?,
    )?;

    if notify_already_routes_to_codex_speak(&existing) {
        return Ok(previous);
    }

    let hook = hook_path()?;
    let notify_line = format_notify_line(&notify_command(&hook));
    let updated = replace_notify_line(&existing, &notify_line);
    fs::write(&codex_config, updated)?;
    Ok(previous)
}

fn remove_notify_if_codex_speak() -> Result<()> {
    let codex_config = config::codex_home()?.join("config.toml");
    let existing = fs::read_to_string(&codex_config).unwrap_or_default();
    if notify_already_routes_to_codex_speak(&existing) {
        restore_notify()?;
    }
    Ok(())
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

#[allow(dead_code)]
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

fn remove_personal_marketplace_entry() -> Result<()> {
    let path = config::personal_marketplace_path()?;
    if !path.exists() {
        return Ok(());
    }
    let marketplace = read_marketplace_json(&path)?;
    let updated = remove_plugin_entry(marketplace);
    fs::write(&path, serde_json::to_string_pretty(&updated)?)?;
    Ok(())
}

fn read_marketplace_json(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(seed_marketplace());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn seed_marketplace() -> Value {
    json!({
        "name": "personal",
        "interface": {
            "displayName": "Personal"
        },
        "plugins": []
    })
}

fn remove_plugin_entry(value: Value) -> Value {
    let mut marketplace = normalize_marketplace(value);
    let plugins = marketplace
        .as_object_mut()
        .expect("marketplace is normalized object")
        .get_mut("plugins")
        .and_then(Value::as_array_mut)
        .expect("marketplace plugins is normalized array");
    plugins.retain(|item| item.get("name").and_then(Value::as_str) != Some(PLUGIN_NAME));
    marketplace
}

fn normalize_marketplace(value: Value) -> Value {
    let mut marketplace = if value.is_object() {
        value
    } else {
        seed_marketplace()
    };
    let object = marketplace
        .as_object_mut()
        .expect("seeded marketplace must be an object");
    object
        .entry("name")
        .or_insert_with(|| Value::String("personal".to_string()));
    object.entry("interface").or_insert_with(|| {
        json!({
            "displayName": "Personal"
        })
    });
    if !object.get("plugins").is_some_and(Value::is_array) {
        object.insert("plugins".to_string(), Value::Array(Vec::new()));
    }
    marketplace
}

fn legacy_installed_plugin_dir() -> Result<PathBuf> {
    Ok(config::personal_plugins_root()?
        .join("plugins")
        .join(PLUGIN_NAME))
}

fn hook_path() -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "codex-speak-stop-hook.ps1"
    } else {
        "codex-speak-stop-hook"
    };
    Ok(config::codex_home()?.join("hooks").join(name))
}

#[allow(dead_code)]
fn notify_command(hook: &Path) -> Vec<String> {
    if cfg!(windows) {
        vec![
            "powershell.exe".to_string(),
            "-NoProfile".to_string(),
            "-ExecutionPolicy".to_string(),
            "Bypass".to_string(),
            "-File".to_string(),
            hook.display().to_string(),
        ]
    } else {
        vec![hook.display().to_string()]
    }
}

fn format_notify_line(command: &[String]) -> String {
    format!(
        "notify = [{}]",
        command
            .iter()
            .map(|item| format!("\"{}\"", toml_escape(item)))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn parse_previous_notify(config_text: &str) -> Option<Vec<String>> {
    let value = toml::from_str::<toml::Value>(config_text)
        .ok()
        .or_else(|| parse_notify_value_from_line(config_text))?;
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

fn parse_notify_value_from_line(config_text: &str) -> Option<toml::Value> {
    let line = config_text
        .lines()
        .find(|line| line.trim_start().starts_with("notify ="))?;
    let (_, array) = line.split_once('=')?;
    toml::from_str::<toml::Value>(&format!("notify = {}", array.trim())).ok()
}

fn notify_already_routes_to_codex_speak(config_text: &str) -> bool {
    config_text
        .lines()
        .find(|line| line.trim_start().starts_with("notify ="))
        .is_some_and(|line| line.contains("codex-speak-notify"))
}

#[allow(dead_code)]
fn read_saved_previous_notify() -> Option<Vec<String>> {
    let path = config::state_dir().ok()?.join("previous-notify.json");
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Option<Vec<String>>>(&raw)
        .ok()
        .flatten()
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

fn format_global_mcp_table(cli_path: &Path) -> String {
    format!(
        "[mcp_servers.codex_speak]\nargs = [\"mcp\"]\ncommand = \"{}\"\nstartup_timeout_sec = 120\n",
        toml_escape(&cli_path.display().to_string())
    )
}

fn upsert_global_mcp_table(existing: &str, table: &str) -> String {
    let without_existing = remove_global_mcp_table(existing);
    let mut lines: Vec<String> = without_existing.lines().map(ToString::to_string).collect();
    let insert_at = lines
        .iter()
        .position(|line| line.trim() == "[mcp_servers.node_repl]")
        .unwrap_or(lines.len());

    let mut table_lines = table.lines().map(ToString::to_string).collect::<Vec<_>>();
    if insert_at > 0 && !lines[insert_at.saturating_sub(1)].trim().is_empty() {
        table_lines.insert(0, String::new());
    }
    if insert_at < lines.len()
        && !table_lines
            .last()
            .is_some_and(|line| line.trim().is_empty())
    {
        table_lines.push(String::new());
    }
    lines.splice(insert_at..insert_at, table_lines);

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn remove_global_mcp_table(existing: &str) -> String {
    let mut lines = Vec::new();
    let mut skipping = false;

    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed == "[mcp_servers.codex_speak]" {
            skipping = true;
            continue;
        }
        if skipping && trimmed.starts_with('[') {
            skipping = false;
        }
        if !skipping {
            lines.push(line.to_string());
        }
    }

    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
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
    } else if cfg!(windows) {
        install_sherpa_windows()?;
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
    download(&SHERPA_MACOS, &archive)?;
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

fn install_sherpa_windows() -> Result<()> {
    if config::sherpa_bin()?.exists() {
        return Ok(());
    }
    let archive = config::cache_dir()?.join("sherpa-onnx-win-x64.tar.bz2");
    download(&SHERPA_WINDOWS, &archive)?;
    let extract_dir = config::cache_dir()?.join("sherpa-onnx-win-extract");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir)?;
    untar_bzip2(&archive, &extract_dir)?;

    let target_dir = config::tools_dir()?.join("sherpa-onnx");
    let _ = fs::remove_dir_all(&target_dir);
    fs::create_dir_all(&target_dir)?;
    let root =
        find_sherpa_root(&extract_dir).context("could not find sherpa-onnx root in archive")?;
    copy_dir_recursive(&root, &target_dir)?;
    Ok(())
}

fn install_melo_model() -> Result<()> {
    if config::model_dir()?.join("model.onnx").exists() {
        return Ok(());
    }
    let archive = config::cache_dir()?.join("vits-melo-tts-zh_en.tar.bz2");
    download(&MELO_MODEL, &archive)?;
    untar_bzip2(&archive, &config::models_dir()?)?;
    Ok(())
}

fn install_kokoro_model() -> Result<()> {
    let target = config::kokoro_model_dir()?;
    if target.join("model.onnx").exists() && target.join("voices.bin").exists() {
        return Ok(());
    }
    let archive = config::cache_dir()?.join("kokoro-multi-lang-v1_0.tar.bz2");
    download(&KOKORO_MODEL, &archive)?;
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
    download(&ZIPVOICE_MODEL, &archive)?;
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
    download(&ZIPVOICE_VOCODER, &target.join("vocoder.onnx"))?;
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
    download(&PIPER_ONNX, &target.join("model.onnx"))?;
    download(&PIPER_JSON, &target.join("model.onnx.json"))?;
    download(&PIPER_TOKENS, &target.join("tokens.txt"))?;
    download(&PIPER_LEXICON, &target.join("lexicon.txt"))?;
    Ok(())
}

fn download(asset: &DownloadAsset, dest: &Path) -> Result<()> {
    if dest.exists() && verify_download(dest, asset)? {
        eprintln!("Using cached {}", dest.display());
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = dest.with_extension(format!("download.{}", std::process::id()));
    let _ = fs::remove_file(&temp);
    eprintln!("Downloading {}", asset.url);
    let status = Command::new("curl")
        .args([
            "-L",
            "--fail",
            "--retry",
            "3",
            "--connect-timeout",
            "20",
            "--progress-bar",
            "-o",
        ])
        .arg(&temp)
        .arg(asset.url)
        .status()
        .with_context(|| format!("failed to run curl for {}", asset.url))?;
    if !status.success() {
        let _ = fs::remove_file(&temp);
        anyhow::bail!("download failed: {}", asset.url);
    }
    if !verify_download(&temp, asset)? {
        let _ = fs::remove_file(&temp);
        anyhow::bail!("download checksum mismatch: {}", asset.url);
    }
    replace_download(&temp, dest)?;
    Ok(())
}

fn verify_download(path: &Path, asset: &DownloadAsset) -> Result<bool> {
    let Some(expected) = asset.sha256 else {
        return Ok(path.exists());
    };
    let actual = sha256_file(path)?;
    Ok(actual.eq_ignore_ascii_case(expected))
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed to open {} for checksum", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 64];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let digest = hasher.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn replace_download(source: &Path, target: &Path) -> Result<()> {
    #[cfg(windows)]
    if target.exists() {
        fs::remove_file(target)
            .with_context(|| format!("failed to remove {}", target.display()))?;
    }

    fs::rename(source, target).with_context(|| {
        format!(
            "failed to move {} to {}",
            source.display(),
            target.display()
        )
    })
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
        if path.join("bin").join(sherpa_tts_binary_name()).exists() && path.join("lib").is_dir() {
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

fn sherpa_tts_binary_name() -> &'static str {
    if cfg!(windows) {
        "sherpa-onnx-offline-tts.exe"
    } else {
        "sherpa-onnx-offline-tts"
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

    #[test]
    fn parses_previous_notify_when_unrelated_config_is_unknown() {
        let raw = r#"
service_tier = "priority"
notify = ["/a/SkyComputerUseClient", "turn-ended", "--previous-notify", "[\"/old/codex-speak-notify\"]"]
"#;
        let previous = parse_previous_notify(raw).unwrap();
        assert_eq!(previous, vec!["/a/SkyComputerUseClient", "turn-ended"]);
    }

    #[test]
    fn detects_existing_codex_speak_notify_route() {
        let raw = r#"notify = ["/a/SkyComputerUseClient", "turn-ended", "--previous-notify", "[\"/Users/me/.codex/hooks/codex-speak-notify\"]"]"#;
        assert!(notify_already_routes_to_codex_speak(raw));
        assert!(!notify_already_routes_to_codex_speak(
            "notify = [\"/other\"]"
        ));
    }

    #[test]
    fn formats_notify_command_array() {
        let line = format_notify_line(&[
            "powershell.exe".to_string(),
            "-File".to_string(),
            "C:\\Users\\me\\.codex\\hooks\\codex-speak-notify.ps1".to_string(),
        ]);
        assert!(line.contains("\"powershell.exe\""));
        assert!(line.contains("C:\\\\Users\\\\me"));
    }

    #[test]
    fn upserts_global_mcp_before_existing_mcp_servers() {
        let table = format_global_mcp_table(Path::new("/tmp/codex-speak"));
        let out = upsert_global_mcp_table(
            "model = \"x\"\n\n[mcp_servers.node_repl]\ncommand = \"node\"\n",
            &table,
        );
        assert!(out.contains("[mcp_servers.codex_speak]\nargs = [\"mcp\"]"));
        assert!(out.contains("command = \"/tmp/codex-speak\""));
        assert!(
            out.find("[mcp_servers.codex_speak]").unwrap()
                < out.find("[mcp_servers.node_repl]").unwrap()
        );
    }

    #[test]
    fn replaces_existing_global_mcp_without_touching_others() {
        let table = format_global_mcp_table(Path::new("/new/codex-speak"));
        let out = upsert_global_mcp_table(
            "[mcp_servers.codex_speak]\nargs = [\"old\"]\ncommand = \"/old\"\n\n[mcp_servers.node_repl]\ncommand = \"node\"\n",
            &table,
        );
        assert!(out.contains("command = \"/new/codex-speak\""));
        assert!(!out.contains("command = \"/old\""));
        assert!(out.contains("[mcp_servers.node_repl]\ncommand = \"node\""));
    }

    #[test]
    fn removes_global_mcp_table_only() {
        let out = remove_global_mcp_table(
            "model = \"x\"\n\n[mcp_servers.codex_speak]\nargs = [\"mcp\"]\ncommand = \"/tmp/codex-speak\"\n\n[mcp_servers.node_repl]\ncommand = \"node\"\n",
        );
        assert!(!out.contains("[mcp_servers.codex_speak]"));
        assert!(out.contains("[mcp_servers.node_repl]"));
        assert!(out.contains("model = \"x\""));
    }

    #[test]
    fn verifies_download_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("asset.txt");
        fs::write(&file, b"codex-speak").unwrap();
        let asset = DownloadAsset {
            url: "https://example.invalid/asset.txt",
            sha256: Some("5859d5fa93e61d7200b470d310972a2c3b2907a96339f9de03f47807e4641b4c"),
        };
        assert!(verify_download(&file, &asset).unwrap());
        let wrong = DownloadAsset {
            url: "https://example.invalid/asset.txt",
            sha256: Some("0000000000000000000000000000000000000000000000000000000000000000"),
        };
        assert!(!verify_download(&file, &wrong).unwrap());
    }

    #[test]
    fn all_download_assets_have_fixed_checksums() {
        let assets = [
            &MELO_MODEL,
            &KOKORO_MODEL,
            &ZIPVOICE_MODEL,
            &ZIPVOICE_VOCODER,
            &PIPER_ONNX,
            &PIPER_JSON,
            &PIPER_TOKENS,
            &PIPER_LEXICON,
            &SHERPA_MACOS,
            &SHERPA_WINDOWS,
        ];
        for asset in assets {
            assert_eq!(
                asset.sha256.map(str::len),
                Some(64),
                "missing or invalid sha256 for {}",
                asset.url
            );
        }
    }

    #[test]
    fn removes_only_codex_speak_marketplace_entry() {
        let marketplace = json!({
            "name": "personal",
            "plugins": [
                {
                    "name": "codex-speak",
                    "source": { "source": "local", "path": "./plugins/codex-speak" },
                    "policy": { "installation": "AVAILABLE", "authentication": "ON_INSTALL" },
                    "category": "Productivity"
                },
                {
                    "name": "other",
                    "source": { "source": "local", "path": "./plugins/other" },
                    "policy": { "installation": "AVAILABLE", "authentication": "ON_INSTALL" },
                    "category": "Productivity"
                }
            ]
        });
        let updated = remove_plugin_entry(marketplace);
        let plugins = updated.get("plugins").and_then(Value::as_array).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(
            plugins[0].get("name").and_then(Value::as_str),
            Some("other")
        );
    }

    #[test]
    fn legacy_plugin_dir_is_under_marketplace_home() {
        let path = legacy_installed_plugin_dir().unwrap();
        assert!(path.ends_with(".agents/plugins/plugins/codex-speak"));
    }
}
