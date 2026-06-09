use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::config;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PronunciationDictionary {
    pub terms: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PronunciationTermUpdate {
    pub path: String,
    pub term: String,
    pub spoken: Option<String>,
    pub terms: usize,
}

pub fn normalize_for_tts(text: &str) -> String {
    let dictionary = load_user_dictionary().unwrap_or_default();
    normalize_for_tts_with_dictionary(text, &dictionary)
}

pub fn normalize_for_tts_with_dictionary(
    text: &str,
    dictionary: &PronunciationDictionary,
) -> String {
    let (mut result, protected_terms) = protect_user_dictionary_terms(text, dictionary);
    if contains_cjk(&result) {
        for (pattern, replacement) in CHINESE_CONTEXT_REPLACEMENTS {
            result = replace_word_case_insensitive(&result, pattern, replacement);
        }
    }
    for (pattern, replacement) in TERM_REPLACEMENTS {
        result = replace_word_case_insensitive(&result, pattern, replacement);
    }
    result = replace_spelled_acronyms(&result);
    result = replace_file_like_tokens(&result);
    result = replace_command_flags(&result);
    result = replace_code_identifiers(&result);
    result = replace_unknown_uppercase_acronyms(&result);
    result = restore_protected_terms(&result, &protected_terms);
    normalize_spacing(&result)
}

pub fn dictionary_path() -> Result<PathBuf> {
    config::pronunciation_dictionary_path()
}

pub fn load_user_dictionary() -> Result<PronunciationDictionary> {
    let path = dictionary_path()?;
    if !path.exists() {
        return Ok(PronunciationDictionary::default());
    }
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let dictionary: PronunciationDictionary =
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
    validate_dictionary(&dictionary).with_context(|| format!("invalid {}", path.display()))?;
    Ok(dictionary)
}

pub fn save_user_dictionary(dictionary: &PronunciationDictionary) -> Result<()> {
    let path = dictionary_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = toml::to_string_pretty(dictionary)?;
    fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))
}

pub fn set_user_term(term: &str, spoken: &str) -> Result<PronunciationTermUpdate> {
    validate_term(term)?;
    validate_spoken(spoken)?;
    let mut dictionary = load_user_dictionary()?;
    dictionary
        .terms
        .insert(term.trim().to_string(), spoken.trim().to_string());
    save_user_dictionary(&dictionary)?;
    let path = dictionary_path()?;
    Ok(PronunciationTermUpdate {
        path: path.display().to_string(),
        term: term.trim().to_string(),
        spoken: Some(spoken.trim().to_string()),
        terms: dictionary.terms.len(),
    })
}

pub fn remove_user_term(term: &str) -> Result<PronunciationTermUpdate> {
    validate_term(term)?;
    let mut dictionary = load_user_dictionary()?;
    dictionary.terms.remove(term.trim());
    save_user_dictionary(&dictionary)?;
    let path = dictionary_path()?;
    Ok(PronunciationTermUpdate {
        path: path.display().to_string(),
        term: term.trim().to_string(),
        spoken: None,
        terms: dictionary.terms.len(),
    })
}

fn validate_term(term: &str) -> Result<()> {
    let term = term.trim();
    if term.is_empty() {
        bail!("pronunciation term must not be empty");
    }
    if term.chars().count() > 80 {
        bail!("pronunciation term must be 80 characters or fewer");
    }
    Ok(())
}

fn validate_dictionary(dictionary: &PronunciationDictionary) -> Result<()> {
    for (term, spoken) in &dictionary.terms {
        validate_term(term)?;
        validate_spoken(spoken)?;
    }
    Ok(())
}

fn validate_spoken(spoken: &str) -> Result<()> {
    let spoken = spoken.trim();
    if spoken.is_empty() {
        bail!("pronunciation spoken value must not be empty");
    }
    if spoken.chars().count() > 120 {
        bail!("pronunciation spoken value must be 120 characters or fewer");
    }
    Ok(())
}

const TERM_REPLACEMENTS: &[(&str, &str)] = &[
    ("Codex Speak", "朗读助手"),
    ("Codex", "代码助手"),
    ("Sherpa-ONNX", "本地语音引擎"),
    ("MeloTTS", "默认中文朗读模型"),
    ("Kokoro", "可选朗读模型"),
    ("Piper", "可选朗读模型"),
    ("ZipVoice", "可选朗读模型"),
    ("OpenAI", "人工智能公司"),
    ("OpenRouter", "开放路由平台"),
    ("Open Router", "开放路由"),
    ("OAuth", "授权登录协议"),
    ("ChatGPT", "聊天机器人"),
    ("GitHub Actions", "自动构建检查"),
    ("GitHub", "代码托管平台"),
    ("GitLab", "代码托管平台"),
    ("Git", "代码版本管理工具"),
    ("VS Code", "代码编辑器"),
    ("Xcode", "苹果开发工具"),
    ("Docker", "容器工具"),
    ("Kubernetes", "容器编排工具"),
    ("WebSocket", "网页实时通信协议"),
    ("GraphQL", "接口查询语言"),
    ("REST", "接口设计风格"),
    ("hello world", "你好世界示例"),
    ("PowerShell", "命令窗口"),
    ("Terminal", "命令窗口"),
    ("terminal", "命令窗口"),
    ("Tauri", "桌面应用框架"),
    ("Rust", "系统编程语言"),
    ("Hook", "自动触发器"),
    ("hook", "自动触发器"),
    ("Plugin", "插件"),
    ("plugin", "插件"),
    ("Skill", "技能规则"),
    ("skill", "技能规则"),
    ("MCP", "插件通道"),
    ("TTS", "朗读工具"),
    ("CLI", "命令行工具"),
    ("API", "接口"),
    ("SDK", "开发工具包"),
    ("IDE", "代码编辑环境"),
    ("LLM", "大语言模型"),
    ("GPT", "大语言模型"),
    ("AI", "人工智能"),
    ("JSON", "数据格式"),
    ("HTML", "网页标记"),
    ("XML", "结构化标记"),
    ("CSS", "样式文件"),
    ("DOM", "网页结构"),
    ("SQL", "数据库查询语言"),
    ("URL", "链接"),
    ("SSH", "远程连接"),
    ("DNS", "域名解析服务"),
    ("SSL", "安全连接证书"),
    ("HTTPS", "安全网页协议"),
    ("HTTP", "网页协议"),
    ("UI", "界面"),
    ("UX", "使用体验"),
    ("CI", "自动检查"),
    ("QA", "验收测试"),
    ("PR", "合并请求"),
    ("CPU", "处理器"),
    ("GPU", "显卡"),
    ("RAM", "内存"),
    ("OS", "操作系统"),
    ("PDF", "文档文件"),
    ("ID", "编号"),
    ("OK", "好的"),
    ("stdout", "标准输出"),
    ("stderr", "错误输出"),
    ("stdin", "标准输入"),
    ("Node.js", "前端运行环境"),
    ("JavaScript", "网页脚本语言"),
    ("TypeScript", "类型脚本语言"),
    ("React", "前端框架"),
    ("Vue", "前端框架"),
    ("Vite", "前端构建工具"),
    ("Next.js", "前端框架"),
    ("Playwright", "浏览器自动化测试工具"),
    ("npm", "包管理工具"),
    ("pnpm", "包管理工具"),
    ("yarn", "包管理工具"),
    ("npx", "命令工具"),
    ("macOS", "苹果电脑系统"),
    ("mac", "苹果电脑"),
    ("Windows", "微软电脑系统"),
    ("Linux", "开源系统"),
    ("README.md", "说明文件"),
    ("config.toml", "配置文件"),
    ("Cargo.toml", "项目配置文件"),
    ("package.json", "前端项目配置文件"),
    ("install-macos.sh", "苹果电脑安装脚本"),
    ("manual-qa-macos.sh", "苹果电脑手工验收脚本"),
    ("manual-qa-windows.ps1", "微软电脑手工验收脚本"),
    ("package-macos-release.sh", "苹果电脑发布打包脚本"),
    ("smoke-install-macos-release.sh", "苹果电脑安装烟测脚本"),
    ("verify-codex", "集成自检"),
    ("verify-install", "安装自检"),
    ("verify-package", "安装包自检"),
    ("support-bundle", "排障支持包"),
    ("clear-bright", "清楚明亮"),
    ("slow-clear", "慢速清晰"),
    ("quick-preview", "快速预览"),
    ("codex_speak_prepare", "准备朗读导览的插件工具"),
    ("codex_speak_speak_text", "播放朗读的插件工具"),
    ("codex_speak_stop", "停止朗读的插件工具"),
    ("sherpa_melo", "默认中文朗读引擎"),
    ("sherpa_kokoro", "可选朗读引擎"),
    ("sherpa_zipvoice", "可选朗读引擎"),
    ("voice_profile", "声音档位"),
    ("child_mode", "儿童模式"),
    ("max_read_chars", "最大朗读长度"),
    ("tts_silence_scale", "朗读停顿设置"),
];

const CHINESE_CONTEXT_REPLACEMENTS: &[(&str, &str)] = &[
    ("cargo test", "测试命令"),
    ("cargo build", "构建命令"),
    ("cargo run", "运行命令"),
    ("cargo", "代码构建工具"),
    ("build failed because timeout", "构建失败，因为超时"),
];

fn protect_user_dictionary_terms(
    text: &str,
    dictionary: &PronunciationDictionary,
) -> (String, Vec<String>) {
    let mut result = text.to_string();
    let mut protected_terms = Vec::new();
    for (pattern, replacement) in &dictionary.terms {
        let placeholder = protected_placeholder(protected_terms.len());
        result = replace_word_case_insensitive(&result, pattern, &placeholder);
        protected_terms.push(replacement.clone());
    }
    (result, protected_terms)
}

fn protected_placeholder(index: usize) -> String {
    format!("\u{E000}{index}\u{E001}")
}

fn restore_protected_terms(text: &str, protected_terms: &[String]) -> String {
    let mut result = text.to_string();
    for (index, replacement) in protected_terms.iter().enumerate() {
        result = result.replace(&protected_placeholder(index), replacement);
    }
    result
}

fn replace_word_case_insensitive(text: &str, pattern: &str, replacement: &str) -> String {
    if pattern
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        let re = Regex::new(&format!(
            r"(?i)(^|[^A-Za-z0-9_-]){}([^A-Za-z0-9_-]|$)",
            regex::escape(pattern)
        ))
        .expect("term replacement regex should compile");
        re.replace_all(text, |caps: &regex::Captures| {
            format!(
                "{}{}{}",
                caps.get(1).map(|m| m.as_str()).unwrap_or(""),
                replacement,
                caps.get(2).map(|m| m.as_str()).unwrap_or("")
            )
        })
        .into_owned()
    } else {
        let re = Regex::new(&format!(r"(?i){}", regex::escape(pattern)))
            .expect("term replacement regex should compile");
        re.replace_all(text, replacement).into_owned()
    }
}

fn replace_spelled_acronyms(text: &str) -> String {
    let re = Regex::new(r"\b[A-Za-z](?:[ .-]+[A-Za-z]){1,7}\b")
        .expect("spelled acronym regex should compile");
    re.replace_all(text, |caps: &regex::Captures| {
        let raw = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
        let acronym = raw
            .chars()
            .filter(|ch| ch.is_ascii_alphabetic())
            .collect::<String>()
            .to_ascii_uppercase();
        acronym_replacement(&acronym).unwrap_or("英文缩写")
    })
    .into_owned()
}

fn acronym_replacement(acronym: &str) -> Option<&'static str> {
    TERM_REPLACEMENTS.iter().find_map(|(pattern, replacement)| {
        let pattern_is_acronym = pattern
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit());
        if pattern_is_acronym && pattern.eq_ignore_ascii_case(acronym) {
            Some(*replacement)
        } else {
            None
        }
    })
}

fn replace_file_like_tokens(text: &str) -> String {
    let re = Regex::new(
        r"(?i)\b(?:[A-Za-z0-9_.-]+/)*[A-Za-z0-9_.-]+\.(rs|tsx|ts|jsx|js|json|toml|md|yaml|yml|sh|ps1|exe|onnx|wav|txt|log|html|css|lock)\b",
    )
    .expect("file-like token regex should compile");
    re.replace_all(text, |caps: &regex::Captures| {
        describe_extension(caps.get(1).map(|m| m.as_str()).unwrap_or_default())
    })
    .into_owned()
}

fn describe_extension(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "md" => "说明文件",
        "toml" | "json" | "yaml" | "yml" => "配置文件",
        "sh" => "命令脚本",
        "ps1" => "微软系统命令脚本",
        "rs" => "源码文件",
        "ts" | "tsx" | "js" | "jsx" => "源码文件",
        "html" => "页面文件",
        "css" => "样式文件",
        "exe" => "程序文件",
        "onnx" => "语音模型文件",
        "wav" => "音频文件",
        "txt" => "文本文件",
        "log" => "日志文件",
        "lock" => "依赖锁定文件",
        _ => "文件",
    }
}

fn replace_command_flags(text: &str) -> String {
    let re = Regex::new(r"(^|[\s，,、。；;:：])--[A-Za-z0-9][A-Za-z0-9_-]*")
        .expect("command flag regex should compile");
    re.replace_all(text, |caps: &regex::Captures| {
        format!("{}命令参数", caps.get(1).map(|m| m.as_str()).unwrap_or(""))
    })
    .into_owned()
}

fn replace_code_identifiers(text: &str) -> String {
    let snake_re = Regex::new(r"\b[a-z][a-z0-9]*(?:_[a-z0-9]+)+\b")
        .expect("snake-case identifier regex should compile");
    let result = snake_re.replace_all(text, "代码里的名称").into_owned();

    let kebab_re = Regex::new(r"\b[a-z][a-z0-9]*(?:-[a-z0-9]+)+\b")
        .expect("kebab-case identifier regex should compile");
    kebab_re.replace_all(&result, "命令名").into_owned()
}

fn replace_unknown_uppercase_acronyms(text: &str) -> String {
    let re = Regex::new(r"\b[A-Z]{2,8}\b").expect("uppercase acronym regex should compile");
    re.replace_all(text, "英文缩写").into_owned()
}

fn contains_cjk(text: &str) -> bool {
    text.chars().any(|ch| {
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
    })
}

fn normalize_spacing(text: &str) -> String {
    let re = Regex::new(r"[ \t]+").expect("spacing regex should compile");
    re.replace_all(text, " ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{normalize_for_tts_with_dictionary, PronunciationDictionary};

    fn normalize_for_tts(text: &str) -> String {
        normalize_for_tts_with_dictionary(text, &PronunciationDictionary::default())
    }

    #[test]
    fn normalizes_common_acronyms_for_chinese_speech() {
        let text = normalize_for_tts("API, JSON, MCP, TTS, CLI, HTML");
        assert!(text.contains("接口"));
        assert!(text.contains("数据格式"));
        assert!(text.contains("插件通道"));
        assert!(text.contains("朗读工具"));
        assert!(text.contains("命令行工具"));
        assert!(text.contains("网页标记"));
    }

    #[test]
    fn does_not_replace_known_terms_inside_longer_words() {
        let text = normalize_for_tts("capital 这个词里不应该触发替换。");
        assert!(text.contains("capital"));
        assert!(!text.contains("接口"));
    }

    #[test]
    fn normalizes_file_and_script_names_for_speech() {
        let text = normalize_for_tts("README.md、src/pronunciation.rs、install-macos.sh");
        assert!(text.contains("说明文件"));
        assert!(text.contains("源码文件"));
        assert!(text.contains("苹果电脑安装脚本"));
        assert!(!text.contains("README.md"));
        assert!(!text.contains("pronunciation.rs"));
        assert!(!text.contains("install-macos.sh"));
    }

    #[test]
    fn normalizes_code_identifiers_and_flags_for_speech() {
        let text = normalize_for_tts("调用 codex_speak_prepare，并传入 --provider sherpa_melo。");
        assert!(text.contains("准备朗读导览的插件工具"));
        assert!(text.contains("命令参数"));
        assert!(text.contains("默认中文朗读引擎"));
        assert!(!text.contains("codex_speak_prepare"));
        assert!(!text.contains("--provider"));
    }

    #[test]
    fn normalizes_more_acronyms_and_unknown_uppercase_words() {
        let text = normalize_for_tts("CPU、GPU、SDK 和 XYZ 都不应该逐字母读。");
        assert!(text.contains("处理器"));
        assert!(text.contains("显卡"));
        assert!(text.contains("开发工具包"));
        assert!(text.contains("英文缩写"));
        assert!(!text.contains("XYZ"));
    }

    #[test]
    fn normalizes_spelled_acronyms_that_were_split_into_letters() {
        let text = normalize_for_tts("M C P、J.S.O.N、C-L-I 和 X Y Z 不应该逐字母读。");
        assert!(text.contains("插件通道"));
        assert!(text.contains("数据格式"));
        assert!(text.contains("命令行工具"));
        assert!(text.contains("英文缩写"));
        assert!(!text.contains("M C P"));
        assert!(!text.contains("J.S.O.N"));
        assert!(!text.contains("C-L-I"));
        assert!(!text.contains("X Y Z"));
    }

    #[test]
    fn normalizes_common_english_terms_for_chinese_first_speech() {
        let text = normalize_for_tts("OpenRouter、OAuth、WebSocket 和 hello world。");
        assert!(text.contains("开放路由平台"));
        assert!(text.contains("授权登录协议"));
        assert!(text.contains("网页实时通信协议"));
        assert!(text.contains("你好世界示例"));
        assert!(!text.contains("OpenRouter"));
        assert!(!text.contains("OAuth"));
        assert!(!text.contains("WebSocket"));
        assert!(!text.contains("hello world"));
    }

    #[test]
    fn normalizes_common_developer_commands_and_errors() {
        let text = normalize_for_tts("我运行 cargo test，看到 build failed because timeout。");
        assert!(text.contains("测试命令"));
        assert!(text.contains("构建失败，因为超时"));
        assert!(!text.contains("cargo test"));
        assert!(!text.contains("build failed"));
    }

    #[test]
    fn preserves_remaining_english_spans_in_mixed_chinese_speech() {
        let text = normalize_for_tts("我看到 deploy preview failed，然后处理 ProjectAlpha42。");
        assert!(text.contains("deploy preview failed"));
        assert!(text.contains("ProjectAlpha42"));
        assert!(!text.contains("英文短语"));
        assert!(!text.contains("英文编号"));
        assert!(!text.contains("英文单词"));
    }

    #[test]
    fn preserves_mostly_english_text_for_english_speech_paths() {
        let text = normalize_for_tts("The build failed because timeout.");
        assert_eq!(text, "The build failed because timeout.");
    }

    #[test]
    fn applies_custom_dictionary_before_builtin_terms() {
        let mut dictionary = PronunciationDictionary::default();
        dictionary
            .terms
            .insert("OpenAI".to_string(), "欧盆艾".to_string());
        dictionary
            .terms
            .insert("ProjectX".to_string(), "项目 X".to_string());

        let text = normalize_for_tts_with_dictionary("OpenAI 和 ProjectX 已经配置。", &dictionary);
        assert!(text.contains("欧盆艾"));
        assert!(text.contains("项目 X"));
        assert!(!text.contains("人工智能公司"));
        assert!(!text.contains("ProjectX"));
    }

    #[test]
    fn protects_custom_dictionary_spoken_values_from_generic_cleanup() {
        let mut dictionary = PronunciationDictionary::default();
        dictionary
            .terms
            .insert("OpenRouter".to_string(), "Open Router 平台".to_string());

        let text = normalize_for_tts_with_dictionary("我配置了 OpenRouter。", &dictionary);
        assert!(text.contains("Open Router 平台"));
        assert!(!text.contains("开放路由平台"));
    }

    #[test]
    fn rejects_empty_custom_dictionary_values() {
        let raw = "[terms]\nOpenRouter = \"\"\n";
        let dictionary: PronunciationDictionary = toml::from_str(raw).unwrap();
        assert!(super::validate_dictionary(&dictionary).is_err());
    }
}
