use regex::Regex;

pub fn extract_speak_block(text: &str) -> Option<String> {
    let re = Regex::new(r"(?s)<!--\s*codex-speak\s*(.*?)\s*-->").ok()?;
    re.captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| normalize_space(m.as_str()))
        .filter(|s| !s.is_empty())
}

pub fn clean_for_speech(text: &str, max_chars: usize) -> String {
    if let Some(block) = extract_speak_block(text) {
        return truncate_chars(&block, max_chars);
    }

    let without_code = strip_fenced_code(text);
    let mut lines = Vec::new();
    for raw in without_code.lines() {
        let mut line = raw.trim().to_string();
        if line.is_empty() {
            continue;
        }
        if should_skip_line(&line) {
            continue;
        }
        line = strip_markdown(&line);
        line = simplify_terms(&line);
        if !line.is_empty() {
            lines.push(line);
        }
    }

    let joined = normalize_space(&lines.join("。"));
    truncate_chars(&joined, max_chars)
}

fn strip_fenced_code(text: &str) -> String {
    let mut output = String::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn should_skip_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|')
        || trimmed.starts_with("diff --git")
        || trimmed.starts_with("@@")
        || trimmed.starts_with("+++")
        || trimmed.starts_with("---")
        || trimmed.starts_with("::")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.contains("/Users/")
        || trimmed.contains("\\Users\\")
        || trimmed.len() > 220
}

fn strip_markdown(line: &str) -> String {
    let mut s = line.trim().to_string();
    while s.starts_with('#') || s.starts_with('-') || s.starts_with('*') || s.starts_with('>') {
        s = s[1..].trim_start().to_string();
    }
    let replacements = [
        ("**", ""),
        ("__", ""),
        ("`", ""),
        ("[", ""),
        ("]", ""),
        ("(", " "),
        (")", " "),
        ("|", " "),
    ];
    for (from, to) in replacements {
        s = s.replace(from, to);
    }
    s
}

fn simplify_terms(line: &str) -> String {
    let replacements = [
        ("Hook", "自动触发器"),
        ("hook", "自动触发器"),
        ("Plugin", "插件"),
        ("plugin", "插件"),
        ("TTS", "朗读工具"),
        ("API", "接口"),
        ("config.toml", "配置文件"),
        ("terminal", "命令窗口"),
        ("Terminal", "命令窗口"),
        ("CLI", "命令行工具"),
    ];
    let mut s = line.to_string();
    for (from, to) in replacements {
        s = s.replace(from, to);
    }
    s
}

pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let mut s: String = text.chars().take(max_chars).collect();
    s.push('…');
    s
}

fn normalize_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_hidden_speak_block() {
        let text = "hello\n<!-- codex-speak\n我简单说一下：做好了。\n-->";
        assert_eq!(extract_speak_block(text).unwrap(), "我简单说一下：做好了。");
    }

    #[test]
    fn removes_code_and_markdown() {
        let text = "# 标题\n```rust\nfn main() {}\n```\n- Hook 已经配置。";
        let cleaned = clean_for_speech(text, 300);
        assert!(!cleaned.contains("fn main"));
        assert!(cleaned.contains("自动触发器"));
    }

    #[test]
    fn truncates_by_chars() {
        assert_eq!(truncate_chars("你好世界", 2), "你好…");
    }
}
