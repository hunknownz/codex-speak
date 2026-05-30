use regex::Regex;

pub fn extract_speak_block(text: &str) -> Option<String> {
    let re = Regex::new(r"(?s)<!--\s*codex-speak\s*(.*?)\s*-->").ok()?;
    re.captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| normalize_space(m.as_str()))
        .filter(|s| !s.is_empty())
}

pub fn clean_for_speech(text: &str, max_chars: usize) -> String {
    if let Some(guide) = extract_html_protocol_guide(text) {
        return truncate_chars(&guide, max_chars);
    }
    if let Some(guide) = extract_spoken_guide(text) {
        return truncate_chars(&guide, max_chars);
    }
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

pub fn extract_html_protocol_guide(text: &str) -> Option<String> {
    let aside_re =
        Regex::new(r#"(?is)<aside\b[^>]*data-codex-speak\s*=\s*["']guide["'][^>]*>(.*?)</aside>"#)
            .ok()?;
    let p_re =
        Regex::new(r#"(?is)<p\b[^>]*(?:data-role\s*=\s*["']([^"']+)["'])?[^>]*>(.*?)</p>"#).ok()?;
    let tag_re = Regex::new(r"(?is)<[^>]+>").ok()?;

    let aside = aside_re
        .captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())?;

    let mut parts = Vec::new();
    for caps in p_re.captures_iter(aside) {
        let role = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        if !is_allowed_protocol_role(role) {
            continue;
        }
        let raw = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
        let without_tags = tag_re.replace_all(raw, "");
        let text = html_unescape(&without_tags);
        let text = normalize_space(&text);
        if !text.is_empty() && !should_skip_line(&text) {
            parts.push(text);
        }
    }

    let guide = normalize_space(&parts.join(""));
    if guide.is_empty() {
        None
    } else {
        Some(guide)
    }
}

fn is_allowed_protocol_role(role: &str) -> bool {
    matches!(
        role,
        "" | "did" | "why" | "code-summary" | "command-summary" | "result" | "next" | "warning"
    )
}

pub fn extract_spoken_guide(text: &str) -> Option<String> {
    let heading =
        Regex::new(r"^\s*(?:#{1,6}\s*)?\*\*朗读导览\*\*\s*$|^\s*#{1,6}\s*朗读导览\s*$").ok()?;
    let next_heading = Regex::new(r"^\s*(?:#{1,6}\s+|\*\*[^*]+?\*\*\s*$)").ok()?;
    let mut in_guide = false;
    let mut lines = Vec::new();

    for line in text.lines() {
        if !in_guide && heading.is_match(line) {
            in_guide = true;
            continue;
        }
        if in_guide {
            if next_heading.is_match(line) && !line.trim().is_empty() {
                break;
            }
            if line.trim_start().starts_with("<!--") {
                break;
            }
            let cleaned = strip_markdown(line);
            if !cleaned.trim().is_empty() && !should_skip_line(&cleaned) {
                lines.push(simplify_terms(&cleaned));
            }
        }
    }

    let guide = normalize_space(&lines.join(""));
    if guide.is_empty() {
        None
    } else {
        Some(guide)
    }
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

fn html_unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
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
    fn extracts_visible_spoken_guide() {
        let text = "**朗读导览**\n\n我改了规则。\n现在不会朗读代码。\n\n**验证**\n测试通过。";
        assert_eq!(
            extract_spoken_guide(text).unwrap(),
            "我改了规则。现在不会朗读代码。"
        );
    }

    #[test]
    fn extracts_html_protocol_guide() {
        let text = r#"
<aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" lang="zh-CN">
  <p class="codex-speak-did" data-role="did">我改了规则。</p>
  <p class="codex-speak-code-summary" data-role="code-summary">代码会跳过长路径。</p>
  <p class="codex-speak-next" data-role="next">下一步可以接插件。</p>
</aside>
"#;
        assert_eq!(
            extract_html_protocol_guide(text).unwrap(),
            "我改了规则。代码会跳过长路径。下一步可以接插件。"
        );
    }

    #[test]
    fn prefers_visible_guide_before_legacy_hidden_block() {
        let text = r#"
**朗读导览**

我会读这一段。

<!-- codex-speak
不要优先读这里。
-->
"#;
        assert_eq!(clean_for_speech(text, 300), "我会读这一段。");
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
