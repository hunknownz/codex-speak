use regex::Regex;

use crate::pronunciation;

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
        line = strip_markdown(&line);
        line = replace_inline_technical_noise(&line);
        if should_skip_line(&line) {
            continue;
        }
        line = simplify_terms(&line);
        for segment in speakable_segments(&line) {
            if !segment.is_empty() && !should_skip_line(&segment) {
                lines.push(segment);
            }
        }
    }

    let joined = normalize_space(&lines.join("。"));
    truncate_chars(&joined, max_chars)
}

pub fn fallback_reply_guide(text: &str, max_chars: usize) -> String {
    if let Some(guide) = extract_html_protocol_guide(text) {
        return truncate_chars(&guide, max_chars);
    }
    if let Some(guide) = extract_spoken_guide(text) {
        return truncate_chars(&guide, max_chars);
    }
    if let Some(block) = extract_speak_block(text) {
        return truncate_chars(&block, max_chars);
    }

    let cleaned = clean_for_speech(text, max_chars);
    let guide_limit = max_chars.min(360);
    let sentences = split_sentences(&cleaned);
    if sentences.is_empty() {
        return String::new();
    }

    let mut chosen: Vec<String> = Vec::new();
    push_unique(&mut chosen, &sentences[0]);

    for sentence in scored_sentences(&sentences) {
        if chosen.len() >= 4 {
            break;
        }
        push_unique(&mut chosen, sentence);
    }

    if let Some(next) = sentences
        .iter()
        .rev()
        .find(|sentence| is_next_step_sentence(sentence))
    {
        push_unique(&mut chosen, next);
    }

    truncate_chars(&normalize_space(&chosen.join("。")), guide_limit)
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
        || looks_like_raw_code(trimmed)
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
    pronunciation::normalize_for_tts(line)
}

fn replace_inline_technical_noise(line: &str) -> String {
    let mut output = line.to_string();
    let replacements = [
        (
            r#"(?i)代码片段[:：]\s*[^。！？\n]*?\{[^。！？\n]*?\}"#,
            "代码片段已经跳过",
        ),
        (r#"/Users/[^\s，。；；、！？)]+"#, "项目里的文件"),
        (r#"[A-Za-z]:\\Users\\[^\s，。；；、！？)]+"#, "项目里的文件"),
    ];
    for (pattern, replacement) in replacements {
        if let Ok(re) = Regex::new(pattern) {
            output = re.replace_all(&output, replacement).to_string();
        }
    }
    output
}

fn speakable_segments(line: &str) -> Vec<String> {
    if line.chars().count() <= 220 {
        return vec![line.trim().to_string()];
    }

    line.split_inclusive(['。', '！', '？', '；'])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| truncate_chars(segment, 220))
        .collect()
}

fn split_sentences(text: &str) -> Vec<String> {
    let normalized = normalize_space(text);
    normalized
        .split_inclusive(['。', '！', '？'])
        .flat_map(|chunk| chunk.split('；'))
        .map(|sentence| sentence.trim_matches(['。', '！', '？', '；', ' ', '\n', '\t']))
        .filter(|sentence| {
            !sentence.is_empty()
                && !is_heading_like_sentence(sentence)
                && !is_detail_list_sentence(sentence)
        })
        .map(str::to_string)
        .collect()
}

fn scored_sentences(sentences: &[String]) -> Vec<&String> {
    let mut scored = sentences
        .iter()
        .enumerate()
        .map(|(index, sentence)| (sentence_score(sentence, index), sentence))
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| right.0.cmp(&left.0));
    scored.into_iter().map(|(_, sentence)| sentence).collect()
}

fn sentence_score(sentence: &str, index: usize) -> i32 {
    let mut score = 0;
    for word in [
        "完成",
        "通过",
        "成功",
        "已经",
        "修",
        "更新",
        "检查",
        "验证",
        "跑通",
        "全绿",
        "失败",
        "问题",
        "风险",
        "下一步",
        "建议",
        "可以",
    ] {
        if sentence.contains(word) {
            score += 3;
        }
    }
    for word in ["代码", "命令", "路径", "日志", "参数", "文件"] {
        if sentence.contains(word) {
            score -= 1;
        }
    }
    if index == 0 {
        score += 2;
    }
    if sentence.chars().count() > 120 {
        score -= 2;
    }
    score
}

fn is_next_step_sentence(sentence: &str) -> bool {
    ["下一步", "接下来", "可以继续", "你可以", "建议"]
        .iter()
        .any(|word| sentence.contains(word))
}

fn is_heading_like_sentence(sentence: &str) -> bool {
    sentence.chars().count() <= 32 && sentence.ends_with([':', '：'])
}

fn is_detail_list_sentence(sentence: &str) -> bool {
    sentence.matches('、').count() >= 4 || sentence.matches(',').count() >= 4
}

fn push_unique(items: &mut Vec<String>, sentence: &str) {
    let sentence = sentence.trim();
    if sentence.is_empty() || items.iter().any(|item| item == sentence) {
        return;
    }
    items.push(sentence.to_string());
}

fn looks_like_raw_code(line: &str) -> bool {
    let code_markers = [
        "fn ",
        "function ",
        "const ",
        "let ",
        "var ",
        "=>",
        "println!",
        "```",
    ];
    let marker_hits = code_markers
        .iter()
        .filter(|marker| line.contains(**marker))
        .count();
    marker_hits >= 2 || (marker_hits >= 1 && line.contains('{') && line.contains('}'))
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
    fn extracts_html_protocol_inside_details() {
        let text = r#"
<details class="codex-speak-fold">
  <summary>朗读导览</summary>
  <aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" lang="zh-CN">
    <p class="codex-speak-did" data-role="did">我整理了插件。</p>
    <p class="codex-speak-next" data-role="next">下一步可以测试。</p>
  </aside>
</details>
"#;
        assert_eq!(
            extract_html_protocol_guide(text).unwrap(),
            "我整理了插件。下一步可以测试。"
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
    fn keeps_useful_text_when_line_contains_paths_and_inline_code() {
        let text = "我运行 cargo test，并检查 /Users/sherry/Documents/project/src/main.rs、codex_speak_prepare、--provider sherpa_melo、M C P、J.S.O.N、OpenRouter、build failed because timeout。代码片段：fn main() { println!(\"hello world\"); } 下一步要看测试是否通过。";
        let cleaned = clean_for_speech(text, 500);
        assert!(!cleaned.is_empty());
        assert!(cleaned.contains("我运行"));
        assert!(cleaned.contains("项目里的文件"));
        assert!(cleaned.contains("下一步要看测试是否通过"));
        assert!(!cleaned.contains("/Users/"));
        assert!(!cleaned.contains("fn main"));
        assert!(!cleaned.contains("codex_speak_prepare"));
    }

    #[test]
    fn fallback_reply_guide_does_not_read_entire_long_reply() {
        let text = r#"
我已经帮你打开最新安装版控制面板了。

当前做到的程度：

- Mac 本机安装版已跑通。
- doctor 全绿：CLI、Hook、Skill、Plugin、MCP、控制面板、Pet helper、MeloTTS 模型、播放器都 OK。
- 默认朗读引擎：MeloTTS 中文女声。

你现在可以这样测：

```bash
~/.codex/codex-speak/bin/codex-speak doctor
```

如果你想测试自动朗读，可以让 Codex 做一个长任务。下一步可以重点验证过程提示是否会在测试开始时播放。
"#;
        let guide = fallback_reply_guide(text, 800);
        assert!(guide.contains("控制面板"));
        assert!(guide.contains("下一步"));
        assert!(!guide.contains("你现在可以这样测"));
        assert!(!guide.contains("命令行工具、自动触发器、技能规则、插件、插件通道、控制面板"));
        assert!(guide.chars().count() < 260);
    }

    #[test]
    fn normalizes_english_technical_terms() {
        let cleaned = clean_for_speech("MCP 和 JSON 都通过了。", 300);
        assert!(cleaned.contains("插件通道"));
        assert!(cleaned.contains("数据格式"));
        assert!(!cleaned.contains("MCP"));
    }

    #[test]
    fn truncates_by_chars() {
        assert_eq!(truncate_chars("你好世界", 2), "你好…");
    }
}
