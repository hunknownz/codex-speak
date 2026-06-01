use regex::Regex;

pub fn normalize_for_tts(text: &str) -> String {
    let mut result = text.to_string();
    for (pattern, replacement) in TERM_REPLACEMENTS {
        result = replace_word_case_insensitive(&result, pattern, replacement);
    }
    result
}

const TERM_REPLACEMENTS: &[(&str, &str)] = &[
    ("Sherpa-ONNX", "Sherpa 语音引擎"),
    ("MeloTTS", "Melo 朗读模型"),
    ("Kokoro", "Kokoro 语音模型"),
    ("Piper", "Piper 语音模型"),
    ("ZipVoice", "ZipVoice 语音模型"),
    ("GitHub Actions", "自动构建检查"),
    ("GitHub", "代码托管平台"),
    ("PowerShell", "PowerShell 命令窗口"),
    ("Terminal", "命令窗口"),
    ("terminal", "命令窗口"),
    ("Tauri", "Tauri 桌面应用框架"),
    ("Rust", "Rust 语言"),
    ("Hook", "自动触发器"),
    ("hook", "自动触发器"),
    ("Plugin", "插件"),
    ("plugin", "插件"),
    ("MCP", "插件通道"),
    ("TTS", "朗读工具"),
    ("CLI", "命令行工具"),
    ("API", "接口"),
    ("JSON", "数据格式"),
    ("HTML", "网页标记"),
    ("XML", "结构化标记"),
    ("URL", "链接"),
    ("HTTPS", "安全网页协议"),
    ("HTTP", "网页协议"),
    ("UI", "界面"),
    ("UX", "使用体验"),
    ("CI", "自动检查"),
    ("QA", "验收测试"),
    ("PR", "合并请求"),
    ("config.toml", "配置文件"),
];

fn replace_word_case_insensitive(text: &str, pattern: &str, replacement: &str) -> String {
    if pattern
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        let re = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(pattern)))
            .expect("term replacement regex should compile");
        re.replace_all(text, replacement).into_owned()
    } else {
        let re = Regex::new(&format!(r"(?i){}", regex::escape(pattern)))
            .expect("term replacement regex should compile");
        re.replace_all(text, replacement).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_for_tts;

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
    fn does_not_replace_inside_longer_words() {
        assert_eq!(normalize_for_tts("capital"), "capital");
    }
}
