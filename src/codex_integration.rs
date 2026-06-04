use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use serde::Serialize;
use serde_json::{json, Value};

use crate::config::{self, Config};
use crate::{mcp, session};

#[derive(Debug, Serialize)]
pub struct CodexIntegrationReport {
    pub ok: bool,
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub generated_at: String,
    pub checks: Vec<CodexIntegrationCheck>,
}

#[derive(Debug, Serialize)]
pub struct CodexIntegrationCheck {
    pub id: &'static str,
    pub label: &'static str,
    pub status: &'static str,
    pub detail: String,
}

impl CodexIntegrationCheck {
    fn ok(id: &'static str, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            id,
            label,
            status: "ok",
            detail: detail.into(),
        }
    }

    fn fail(id: &'static str, label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            id,
            label,
            status: "fail",
            detail: detail.into(),
        }
    }
}

pub fn run(json_output: bool) -> Result<()> {
    let report = collect()?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text(&report);
    }

    if !report.ok {
        anyhow::bail!("Codex integration verification failed");
    }
    Ok(())
}

pub fn collect() -> Result<CodexIntegrationReport> {
    let mut cfg = Config::load_or_default()?;
    cfg.max_read_chars = cfg.max_read_chars.max(300);
    let backup = SpoolBackup::capture()?;
    let mut checks = Vec::new();

    let prepared_text = match prepare_guide_via_mcp(&cfg) {
        Ok(text) => {
            checks.push(CodexIntegrationCheck::ok(
                "mcp_prepare",
                "MCP side-channel prepare",
                "codex_speak_prepare wrote latest.json",
            ));
            Some(text)
        }
        Err(err) => {
            checks.push(CodexIntegrationCheck::fail(
                "mcp_prepare",
                "MCP side-channel prepare",
                format!("{err:#}"),
            ));
            None
        }
    };

    if let Some(expected) = prepared_text {
        match consume_guide_like_hook(&cfg, &expected) {
            Ok(detail) => checks.push(CodexIntegrationCheck::ok(
                "hook_consume_side_channel",
                "Hook consumes side-channel",
                detail,
            )),
            Err(err) => checks.push(CodexIntegrationCheck::fail(
                "hook_consume_side_channel",
                "Hook consumes side-channel",
                format!("{err:#}"),
            )),
        }
    } else {
        checks.push(CodexIntegrationCheck::fail(
            "hook_consume_side_channel",
            "Hook consumes side-channel",
            "skipped because MCP prepare failed",
        ));
    }

    match verify_fallback_cleaner(&cfg) {
        Ok(detail) => checks.push(CodexIntegrationCheck::ok(
            "text_cleaner",
            "Text cleaner",
            detail,
        )),
        Err(err) => checks.push(CodexIntegrationCheck::fail(
            "text_cleaner",
            "Text cleaner",
            format!("{err:#}"),
        )),
    }

    match verify_no_side_channel_hook_notice() {
        Ok(detail) => checks.push(CodexIntegrationCheck::ok(
            "no_side_channel_hook_notice",
            "No side-channel hook notice",
            detail,
        )),
        Err(err) => checks.push(CodexIntegrationCheck::fail(
            "no_side_channel_hook_notice",
            "No side-channel hook notice",
            format!("{err:#}"),
        )),
    }

    match verify_mixed_english_normalization(&cfg) {
        Ok(detail) => checks.push(CodexIntegrationCheck::ok(
            "mixed_english_normalization",
            "Mixed English speech normalization",
            detail,
        )),
        Err(err) => checks.push(CodexIntegrationCheck::fail(
            "mixed_english_normalization",
            "Mixed English speech normalization",
            format!("{err:#}"),
        )),
    }

    match verify_pronunciation_dictionary_roundtrip(&cfg) {
        Ok(detail) => checks.push(CodexIntegrationCheck::ok(
            "pronunciation_dictionary",
            "Pronunciation dictionary",
            detail,
        )),
        Err(err) => checks.push(CodexIntegrationCheck::fail(
            "pronunciation_dictionary",
            "Pronunciation dictionary",
            format!("{err:#}"),
        )),
    }

    if let Err(err) = backup.restore() {
        checks.push(CodexIntegrationCheck::fail(
            "spool_restore",
            "Restore previous spool files",
            format!("{err:#}"),
        ));
    }

    let ok = checks.iter().all(|check| check.status == "ok");
    Ok(CodexIntegrationReport {
        ok,
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        generated_at: Local::now().to_rfc3339(),
        checks,
    })
}

fn prepare_guide_via_mcp(cfg: &Config) -> Result<String> {
    let result = mcp::call_tool_by_name(
        "codex_speak_prepare",
        json!({
            "version": 1,
            "audience": "beginner",
            "style": "clear-bright",
            "lang": "zh-CN",
            "items": [
                {
                    "role": "did",
                    "text": "我刚才帮你检查了 Codex Speak 的集成。"
                },
                {
                    "role": "code-summary",
                    "text": "插件会写入适合朗读的导览，自动触发器会优先读这段内容。"
                },
                {
                    "role": "next",
                    "text": "下一步可以继续做真实机器验收。"
                },
                {
                    "role": "debug",
                    "text": "不要朗读这一段。"
                }
            ]
        }),
        cfg.clone(),
    )?;
    let value: Value = serde_json::from_str(&result).context("MCP prepare result was not JSON")?;
    let text = value
        .get("text")
        .and_then(Value::as_str)
        .context("MCP prepare result did not include text")?;
    if !text.contains("Codex Speak") || text.contains("不要朗读") {
        anyhow::bail!("unexpected side-channel text: {text}");
    }
    Ok(text.to_string())
}

fn consume_guide_like_hook(cfg: &Config, expected: &str) -> Result<String> {
    let consumed = session::resolve_text_for_speech(None, None, cfg)?;
    if consumed != expected {
        anyhow::bail!("consumed text did not match prepared guide");
    }

    let spool_dir = config::spool_dir()?;
    if spool_dir.join("latest.json").exists() {
        anyhow::bail!("latest.json still exists after hook-style consume");
    }
    if !spool_dir.join("last-consumed.json").is_file() {
        anyhow::bail!("last-consumed.json was not written");
    }

    Ok("prepared guide was read and moved to last-consumed.json".to_string())
}

fn verify_fallback_cleaner(cfg: &Config) -> Result<String> {
    let raw = r#"
我改好了。
```rust
fn main() {
    println!("不要逐字读代码");
}
```
命令和长路径：/Users/someone/project/target/release/codex-speak models install --provider sherpa_melo
代码的作用是：让孩子听到摘要，而不是逐字听代码、命令和路径。
"#;
    let cleaned = session::resolve_text(Some(raw.to_string()), None, cfg)?;
    for forbidden in ["fn main", "/Users/", "models install"] {
        if cleaned.contains(forbidden) {
            anyhow::bail!("fallback cleaner leaked raw technical text: {forbidden}");
        }
    }
    if !cleaned.contains("孩子听到摘要") {
        anyhow::bail!("fallback cleaner lost the useful summary: {cleaned}");
    }
    Ok("code blocks, commands, and long paths were skipped".to_string())
}

fn verify_no_side_channel_hook_notice() -> Result<String> {
    let text = session::missing_side_channel_notice();
    if !text.contains("没有收到") || !text.contains("不乱读") {
        anyhow::bail!("missing side-channel notice is not explicit enough: {text}");
    }
    for forbidden in [
        "cargo",
        "HTML",
        "final answer",
        "产品变化优先于本地构建步骤",
    ] {
        if text.contains(forbidden) {
            anyhow::bail!("missing side-channel notice leaked technical detail: {forbidden}");
        }
    }
    Ok("without side-channel, hook reports the missing plugin guide instead of guessing the final answer".to_string())
}

fn verify_mixed_english_normalization(cfg: &Config) -> Result<String> {
    let raw =
        "我运行 hello world，并检查 README.md、codex_speak_prepare、--provider sherpa_melo、OpenRouter、OAuth、M C P、J.S.O.N、CLI、API、CPU、XYZ、build failed because timeout 和 ProjectAlpha42。";
    let cleaned = session::resolve_text(Some(raw.to_string()), None, cfg)?;
    for required in [
        "你好世界示例",
        "说明文件",
        "准备朗读导览的插件工具",
        "命令参数",
        "默认中文朗读引擎",
        "开放路由平台",
        "授权登录协议",
        "插件通道",
        "数据格式",
        "命令行工具",
        "接口",
        "处理器",
        "英文缩写",
        "构建失败，因为超时",
        "英文编号",
    ] {
        if !cleaned.contains(required) {
            anyhow::bail!("mixed English normalization lost expected term {required}: {cleaned}");
        }
    }
    for forbidden in [
        "README.md",
        "codex_speak_prepare",
        "--provider",
        "sherpa_melo",
        "OpenRouter",
        "OAuth",
        "M C P",
        "J.S.O.N",
        "hello world",
        "MCP",
        "JSON",
        "CLI",
        "API",
        "CPU",
        "XYZ",
        "build",
        "failed",
        "timeout",
        "ProjectAlpha42",
    ] {
        if cleaned.contains(forbidden) {
            anyhow::bail!("mixed English normalization leaked raw term {forbidden}: {cleaned}");
        }
    }
    Ok("English technical terms, file names, flags, identifiers, and acronyms are converted before speech".to_string())
}

fn verify_pronunciation_dictionary_roundtrip(cfg: &Config) -> Result<String> {
    let backup = FileBackup::capture(config::pronunciation_dictionary_path()?)?;
    let result = (|| {
        let term = "OpenRouterQaTerm";
        let spoken = "本地发音词典测试";
        mcp::call_tool_by_name(
            "codex_speak_set_pronunciation",
            json!({
                "term": term,
                "spoken": spoken
            }),
            cfg.clone(),
        )?;

        let listed =
            mcp::call_tool_by_name("codex_speak_list_pronunciation", json!({}), cfg.clone())?;
        if !listed.contains(term) || !listed.contains(spoken) {
            anyhow::bail!("pronunciation dictionary did not include new term: {listed}");
        }

        let cleaned = mcp::call_tool_by_name(
            "codex_speak_extract",
            json!({
                "text": format!("我配置了 {term}。")
            }),
            cfg.clone(),
        )?;
        if !cleaned.contains(spoken) || cleaned.contains(term) {
            anyhow::bail!("pronunciation dictionary was not applied: {cleaned}");
        }

        mcp::call_tool_by_name(
            "codex_speak_remove_pronunciation",
            json!({
                "term": term
            }),
            cfg.clone(),
        )?;
        let listed =
            mcp::call_tool_by_name("codex_speak_list_pronunciation", json!({}), cfg.clone())?;
        if listed.contains(term) {
            anyhow::bail!("pronunciation dictionary still included removed term: {listed}");
        }
        Ok("MCP can add, use, list, and remove local pronunciation terms".to_string())
    })();

    let restore = backup.restore();
    match (result, restore) {
        (Ok(detail), Ok(())) => Ok(detail),
        (Err(err), Ok(())) => Err(err),
        (Ok(_), Err(err)) => Err(err),
        (Err(err), Err(restore_err)) => Err(err.context(format!(
            "also failed to restore pronunciation dictionary: {restore_err:#}"
        ))),
    }
}

fn print_text(report: &CodexIntegrationReport) {
    println!(
        "Codex integration verification {} on {} {}",
        report.version, report.os, report.arch
    );
    for check in &report.checks {
        let prefix = if check.status == "ok" { "OK  " } else { "FAIL" };
        println!("{prefix} {}: {}", check.label, check.detail);
    }
}

#[cfg(test)]
mod tests {
    use super::verify_mixed_english_normalization;
    use crate::config::Config;

    #[test]
    fn verifies_mixed_english_normalization() {
        verify_mixed_english_normalization(&Config::default()).unwrap();
    }
}

struct SpoolBackup {
    latest_path: PathBuf,
    latest: Option<Vec<u8>>,
    consumed_path: PathBuf,
    consumed: Option<Vec<u8>>,
    pet_state_path: PathBuf,
    pet_state: Option<Vec<u8>>,
}

struct FileBackup {
    path: PathBuf,
    content: Option<Vec<u8>>,
}

impl FileBackup {
    fn capture(path: PathBuf) -> Result<Self> {
        Ok(Self {
            content: read_optional(&path)?,
            path,
        })
    }

    fn restore(self) -> Result<()> {
        restore_optional(&self.path, self.content.as_deref())
    }
}

impl SpoolBackup {
    fn capture() -> Result<Self> {
        let spool_dir = config::spool_dir()?;
        let latest_path = spool_dir.join("latest.json");
        let consumed_path = spool_dir.join("last-consumed.json");
        let pet_state_path = config::pet_state_path()?;
        Ok(Self {
            latest: read_optional(&latest_path)?,
            consumed: read_optional(&consumed_path)?,
            pet_state: read_optional(&pet_state_path)?,
            latest_path,
            consumed_path,
            pet_state_path,
        })
    }

    fn restore(self) -> Result<()> {
        restore_optional(&self.latest_path, self.latest.as_deref())?;
        restore_optional(&self.consumed_path, self.consumed.as_deref())?;
        restore_optional(&self.pet_state_path, self.pet_state.as_deref())?;
        Ok(())
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    if path.is_file() {
        Ok(Some(fs::read(path).with_context(|| {
            format!("failed to read {}", path.display())
        })?))
    } else {
        Ok(None)
    }
}

fn restore_optional(path: &Path, content: Option<&[u8]>) -> Result<()> {
    if let Some(content) = content {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content).with_context(|| format!("failed to restore {}", path.display()))
    } else {
        if path.exists() {
            fs::remove_file(path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
        Ok(())
    }
}
