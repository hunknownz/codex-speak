use regex::Regex;

use crate::pronunciation;

pub fn extract_speak_block(text: &str) -> Option<String> {
    let re = Regex::new(r"(?s)<!--\s*codex-speak\s*(.*?)\s*-->").ok()?;
    re.captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| normalize_space(m.as_str()))
        .filter(|s| !s.is_empty())
}

pub fn has_explicit_spoken_source(text: &str) -> bool {
    extract_html_protocol_guide(text).is_some()
        || extract_spoken_guide(text).is_some()
        || extract_speak_block(text).is_some()
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
        if should_skip_structural_line(&line) {
            continue;
        }
        line = strip_markdown(&line);
        line = replace_inline_technical_noise(&line);
        if should_skip_line(&line) {
            continue;
        }
        line = simplify_terms(&line);
        if is_low_value_speech_sentence(&line) {
            continue;
        }
        for segment in speakable_segments(&line) {
            if !segment.is_empty()
                && !should_skip_line(&segment)
                && !is_low_value_speech_sentence(&segment)
            {
                lines.push(segment);
            }
        }
    }

    let joined = normalize_space(&lines.join("。"));
    truncate_chars(&joined, max_chars)
}

pub fn conservative_reply_notice(text: &str, max_chars: usize) -> String {
    let source = prepare_reply_for_fallback(text);
    let cleaned = clean_for_speech(&source, max_chars.min(240));
    let sentences = split_sentences(&cleaned);
    let mut chosen: Vec<String> = Vec::new();

    if let Some(first) = sentences.iter().find(|sentence| {
        !is_heading_like_sentence(sentence) && !is_too_technical_for_fallback(sentence)
    }) {
        push_unique(&mut chosen, first);
    }

    if let Some(result) = sentences.iter().find(|sentence| {
        sentence.contains("测试和构建都通过")
            || sentence.contains("测试通过")
            || sentence.contains("构建通过")
            || sentence.contains("验证通过")
    }) {
        push_unique(&mut chosen, result);
    }

    if let Some(next) = sentences
        .iter()
        .find(|sentence| is_next_step_sentence(sentence) && sentence.chars().count() <= 80)
    {
        push_unique(&mut chosen, next);
    }

    let text = if chosen.is_empty() {
        "这次回答已经完成了。你可以看一下屏幕上的结果。".to_string()
    } else {
        normalize_space(&chosen.join("。"))
    };
    truncate_chars(&ensure_terminal_punctuation(&text), max_chars.min(220))
}

pub fn fallback_reply_guide(text: &str, max_chars: usize) -> String {
    if let Some(guide) = extract_html_protocol_guide(text) {
        return truncate_chars(&ensure_terminal_punctuation(&guide), max_chars);
    }
    if let Some(guide) = extract_spoken_guide(text) {
        return truncate_chars(&ensure_terminal_punctuation(&guide), max_chars);
    }
    if let Some(block) = extract_speak_block(text) {
        return truncate_chars(&ensure_terminal_punctuation(&block), max_chars);
    }

    let source = prepare_reply_for_fallback(text);
    let cleaned = clean_for_speech(&source, max_chars);
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

    truncate_chars(
        &ensure_terminal_punctuation(&normalize_space(&chosen.join("。"))),
        guide_limit,
    )
}

pub fn extract_html_protocol_guide(text: &str) -> Option<String> {
    let aside_re =
        Regex::new(r#"(?is)<aside\b[^>]*data-codex-speak\s*=\s*["']guide["'][^>]*>(.*?)</aside>"#)
            .ok()?;
    let p_re = Regex::new(r#"(?is)<p\b([^>]*)>(.*?)</p>"#).ok()?;
    let role_re = Regex::new(r#"(?is)data-role\s*=\s*["']([^"']+)["']"#).ok()?;
    let tag_re = Regex::new(r"(?is)<[^>]+>").ok()?;

    let aside = aside_re
        .captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())?;

    let mut parts = Vec::new();
    for caps in p_re.captures_iter(aside) {
        let attrs = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let role = role_re
            .captures(attrs)
            .and_then(|role_caps| role_caps.get(1))
            .map(|m| m.as_str())
            .unwrap_or_default();
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
        "" | "did"
            | "why"
            | "code-summary"
            | "command-summary"
            | "visual-summary"
            | "result"
            | "next"
            | "warning"
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
            if !line.trim().is_empty()
                && (next_heading.is_match(line) || is_plain_section_heading(line))
            {
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

fn is_plain_section_heading(line: &str) -> bool {
    let stripped = strip_markdown(line);
    let trimmed = stripped.trim();
    if !trimmed.ends_with([':', '：']) {
        return false;
    }
    let body = trimmed.trim_end_matches([':', '：']).trim();
    !body.is_empty() && body.chars().count() <= 24 && !body.contains(['。', '，', '；', '！', '？'])
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

fn prepare_reply_for_fallback(text: &str) -> String {
    let without_code = strip_fenced_code(text);
    let mut lines = Vec::new();
    let mut skip_test_coverage = false;
    let mut saw_test_pass = false;
    let mut saw_build_pass = false;

    for raw in without_code.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("::") {
            break;
        }
        if is_test_coverage_heading(line) {
            skip_test_coverage = true;
            continue;
        }
        if is_verification_heading(line) {
            skip_test_coverage = false;
            continue;
        }
        if skip_test_coverage {
            continue;
        }

        let stripped = strip_markdown(line);
        if is_verification_line(&stripped) {
            if stripped.contains("test") || stripped.contains("测试") {
                saw_test_pass = true;
            }
            if stripped.contains("build") || stripped.contains("构建") {
                saw_build_pass = true;
            }
            continue;
        }
        if is_final_release_metadata_line(&stripped)
            || is_manual_command_prompt_line(&stripped)
            || is_too_technical_for_fallback(&stripped)
        {
            continue;
        }

        let normalized_heading = normalize_fallback_heading(&stripped);
        if !normalized_heading.trim().is_empty() {
            lines.push(normalized_heading);
        }
    }

    if saw_test_pass && saw_build_pass {
        lines.push("测试和构建都通过了。".to_string());
    } else if saw_test_pass {
        lines.push("测试通过了。".to_string());
    } else if saw_build_pass {
        lines.push("构建通过了。".to_string());
    }

    if lines.is_empty() {
        text.to_string()
    } else {
        lines.join("\n")
    }
}

fn is_test_coverage_heading(line: &str) -> bool {
    let trimmed = line.trim_matches(['*', '#', ' ', '：', ':']);
    trimmed.contains("测试覆盖") || trimmed.contains("新增/加强测试")
}

fn is_verification_heading(line: &str) -> bool {
    let trimmed = line.trim_matches(['*', '#', ' ', '：', ':']);
    trimmed.contains("验证已通过")
        || trimmed == "验证"
        || trimmed == "测试结果"
        || trimmed == "验证结果"
}

fn is_verification_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains("cargo test")
        || lower.contains("cargo build")
        || lower.contains("build --release")
        || line.contains("测试全部通过")
        || line.contains("构建通过"))
        && line.contains("通过")
}

fn is_final_release_metadata_line(line: &str) -> bool {
    line.contains("已提交并推送")
        || line.contains("推送到")
        || line.contains("GitHub main")
        || line.contains("git-stage")
        || line.contains("git-commit")
        || line.contains("git-push")
}

fn is_manual_command_prompt_line(line: &str) -> bool {
    line.contains("你现在可以这样测")
        || line.contains("可以这样测试")
        || line.contains("运行下面")
        || line.contains("执行下面")
}

fn is_too_technical_for_fallback(line: &str) -> bool {
    let keep_product_change = line.contains("儿童模式") || line.contains("声音方案");
    if keep_product_change {
        return false;
    }

    line.contains("`")
        || line.contains("=\"")
        || line.contains("data-")
        || line.contains("HTML")
        || line.contains("Markdown")
        || line.contains("Mermaid")
        || line.contains("directive")
        || line.contains("final answer")
        || line.contains("session")
        || line.contains("role")
}

fn normalize_fallback_heading(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.ends_with([':', '：']) {
        format!("{}。", trimmed.trim_end_matches([':', '：']))
    } else {
        trimmed.to_string()
    }
}

fn should_skip_line(line: &str) -> bool {
    let trimmed = line.trim();
    should_skip_structural_line(trimmed) || looks_like_raw_code(trimmed)
}

fn should_skip_structural_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|')
        || trimmed.starts_with("diff --git")
        || trimmed.starts_with("@@")
        || trimmed.starts_with("+++")
        || trimmed.starts_with("---")
        || trimmed.starts_with("::")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
}

fn strip_markdown(line: &str) -> String {
    let mut s = line.trim().to_string();
    if let Ok(numbered_list_re) = Regex::new(r"^\d+[.)]\s+") {
        s = numbered_list_re.replace(&s, "").to_string();
    }
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
                && !is_low_value_speech_sentence(sentence)
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
        "修复",
        "更新",
        "新增",
        "支持",
        "安装",
        "构建",
        "部署",
        "打开",
        "播放",
        "调整",
        "调节",
        "发音",
        "声音方案",
        "儿童模式",
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
    if contains_speech_placeholder_noise(sentence) {
        score -= 8;
    }
    if is_release_metadata_sentence(sentence) {
        score -= 8;
    }
    if is_local_build_sentence(sentence) {
        score -= 3;
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

fn is_low_value_speech_sentence(sentence: &str) -> bool {
    is_placeholder_only_sentence(sentence) || is_release_metadata_sentence(sentence)
}

fn contains_speech_placeholder_noise(sentence: &str) -> bool {
    [
        "英文单词",
        "英文名称",
        "英文短语",
        "英文编号",
        "英文缩写",
        "命令名",
    ]
    .iter()
    .any(|placeholder| sentence.contains(placeholder))
}

fn is_placeholder_only_sentence(sentence: &str) -> bool {
    let trimmed = sentence.trim_matches(['。', '！', '？', '；', ' ', '\n', '\t']);
    Regex::new(r"^(英文单词|英文名称|英文短语|英文编号|英文缩写|命令名)(\s*(通过|失败|成功|已完成|已打开|已播放))?$")
        .map(|re| re.is_match(trimmed))
        .unwrap_or(false)
}

fn is_release_metadata_sentence(sentence: &str) -> bool {
    let trimmed = sentence.trim();
    if trimmed.starts_with("::") {
        return true;
    }

    let has_revisionish_token = Regex::new(r"\b[0-9a-f]{7,40}\b")
        .map(|re| re.is_match(trimmed))
        .unwrap_or(false);
    let is_repo_update = ["推送到", "提交", "commit", "代码托管平台"]
        .iter()
        .any(|word| trimmed.contains(word));
    if has_revisionish_token && is_repo_update {
        return true;
    }

    let technical_tail = ["自检命令", "doctor", "release", "build", "cargo"]
        .iter()
        .any(|word| trimmed.contains(word));
    technical_tail && trimmed.chars().count() <= 28
}

fn is_local_build_sentence(sentence: &str) -> bool {
    [
        "本地重新构建",
        "重新安装到",
        "构建 命令行工具",
        "构建 桌面应用框架",
    ]
    .iter()
    .any(|word| sentence.contains(word))
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
    if let Some(sentence) = truncate_at_sentence_boundary(text, max_chars) {
        return sentence;
    }
    let mut s: String = text.chars().take(max_chars).collect();
    s.push('…');
    s
}

fn truncate_at_sentence_boundary(text: &str, max_chars: usize) -> Option<String> {
    let boundary_chars = ['。', '！', '？', '.', '!', '?'];
    let min_useful_chars = (max_chars / 3).max(6);
    let mut best = None;
    let mut char_count = 0;
    for (byte_index, ch) in text.char_indices() {
        char_count += 1;
        if char_count > max_chars {
            break;
        }
        if boundary_chars.contains(&ch) {
            let end = byte_index + ch.len_utf8();
            if char_count >= min_useful_chars {
                best = Some(text[..end].trim().to_string());
            }
        }
    }
    best
}

fn normalize_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn ensure_terminal_punctuation(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.ends_with(['。', '！', '？', '.', '!', '?']) {
        trimmed.to_string()
    } else {
        format!("{trimmed}。")
    }
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
    fn visible_spoken_guide_stops_at_plain_section_heading() {
        let text = "**朗读导览**\n\n我改了规则。\n测试通过了。\n\n验证：\n- `cargo test` 通过。";
        assert_eq!(
            extract_spoken_guide(text).unwrap(),
            "我改了规则。测试通过了。"
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
    fn html_protocol_filters_unknown_roles_and_unescapes_entities() {
        let text = r#"
<aside data-codex-speak="guide">
  <p data-role="did">我已经整理好 &lt;重点&gt;。</p>
  <p data-role="debug">不要朗读调试细节。</p>
  <p data-role="result"><strong>测试</strong>通过。</p>
</aside>
"#;
        assert_eq!(
            extract_html_protocol_guide(text).unwrap(),
            "我已经整理好 <重点>。测试通过。"
        );
    }

    #[test]
    fn clean_for_speech_prefers_html_protocol_over_other_guides() {
        let text = r#"
<aside data-codex-speak="guide">
  <p data-role="did">优先读结构化导览。</p>
</aside>

**朗读导览**

不要读 Markdown 导览。

<!-- codex-speak
不要读隐藏导览。
-->
"#;
        assert_eq!(clean_for_speech(text, 300), "优先读结构化导览。");
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
    fn detects_explicit_spoken_sources() {
        assert!(has_explicit_spoken_source("**朗读导览**\n\n我会读这里。"));
        assert!(has_explicit_spoken_source(
            r#"<aside data-codex-speak="guide"><p data-role="did">读这里。</p></aside>"#
        ));
        assert!(has_explicit_spoken_source(
            "<!-- codex-speak\n读这里。\n-->"
        ));
        assert!(!has_explicit_spoken_source(
            "普通最终回答，没有专门朗读导览。"
        ));
    }

    #[test]
    fn removes_code_and_markdown() {
        let text = "# 标题\n```rust\nfn main() {}\n```\n- Hook 已经配置。";
        let cleaned = clean_for_speech(text, 300);
        assert!(!cleaned.contains("fn main"));
        assert!(cleaned.contains("自动触发器"));
    }

    #[test]
    fn removes_tables_links_directives_and_fenced_visual_sources() {
        let text = r#"
我画了一张结构图，帮助理解流程。

| 步骤 | 内容 |
| --- | --- |
| 一 | 不要读表格 |

https://example.com/raw-log
::git-push{cwd="/tmp/project" branch="main"}

```mermaid
graph TD
  A --> B
```

下一步可以打开控制面板测试。
"#;
        let cleaned = clean_for_speech(text, 800);
        assert!(cleaned.contains("结构图"));
        assert!(cleaned.contains("下一步可以打开控制面板测试"));
        assert!(!cleaned.contains("graph TD"));
        assert!(!cleaned.contains("example.com"));
        assert!(!cleaned.contains("::git"));
        assert!(!cleaned.contains("不要读表格"));
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
    fn fallback_reply_guide_summarizes_code_heavy_reply_without_raw_code() {
        let text = r#"
我已经把停止朗读按钮修好了。

这次主要改了 `apps/codex-speak-control/src/main.js`，让按钮点击后调用停止接口。

```javascript
const stopButton = document.querySelector("[data-stop]");
stopButton.addEventListener("click", () => invoke("stop_speech"));
```

验证结果：控制面板能停止正在播放的声音，测试通过。
下一步可以继续测试不同声音方案。
"#;
        let guide = fallback_reply_guide(text, 800);
        assert!(guide.contains("停止朗读按钮"));
        assert!(guide.contains("测试通过"));
        assert!(guide.contains("下一步"));
        assert!(!guide.contains("document.querySelector"));
        assert!(!guide.contains("addEventListener"));
        assert!(!guide.contains("src/main"));
    }

    #[test]
    fn fallback_reply_guide_skips_release_metadata_and_placeholder_noise() {
        let text = r#"
已完成并重新部署打开了。

这次做了两件事：

1. 控制面板新增 MeloTTS 声音方案试听。
2. 配置层支持底层发音参数，包括语速、噪声、韵律和停顿。

已完成：
- 本地重新构建 CLI。
- 本地重新构建 Tauri 控制面板。
- 重新安装到 `~/.codex/codex-speak`。
- 已打开控制面板。
- 已播放测试音。
- `doctor` 通过。
- 推送到 GitHub：`75fc76f`。

::git-stage{cwd="/tmp/codex-speak"}
::git-commit{cwd="/tmp/codex-speak"}
::git-push{cwd="/tmp/codex-speak" branch="main"}
"#;
        let cleaned = clean_for_speech(text, 800);
        assert!(cleaned.contains("控制面板新增"));
        assert!(!cleaned.contains("英文单词"));
        assert!(!cleaned.contains("75fc76f"));
        assert!(!cleaned.contains("::git"));

        let guide = fallback_reply_guide(text, 800);
        assert!(guide.contains("已完成并重新部署打开了"));
        assert!(guide.contains("控制面板新增"));
        assert!(guide.contains("发音参数"));
        assert!(!guide.contains("英文单词"));
        assert!(!guide.contains("英文名称"));
        assert!(!guide.contains("75fc76f"));
        assert!(!guide.contains("::git"));
        assert!(guide.chars().count() > 50);
    }

    #[test]
    fn fallback_reply_guide_prioritizes_product_changes_over_local_build_steps() {
        let text = r#"
已完成这轮优化。

- 本地重新构建 CLI。
- 本地重新构建 Tauri 控制面板。
- 重新安装到 `~/.codex/codex-speak`。
- 儿童模式新增更自然的成果说明，会把代码改动解释成做了什么。
- 声音方案支持更慢、更清楚的默认参数。
- 已打开控制面板。

下一步可以用一个真实长任务测试朗读是否自然。
"#;
        let guide = fallback_reply_guide(text, 800);
        assert!(guide.contains("儿童模式新增"));
        assert!(guide.contains("声音方案支持"));
        assert!(guide.contains("下一步可以用一个真实长任务测试"));
        assert!(!guide.contains("重新安装到"));
    }

    #[test]
    fn fallback_reply_guide_skips_test_coverage_lists_and_command_verification() {
        let text = r#"
补好了，并且这次测试真的抓出了两个现有问题，我也一起修了：

- HTML 协议里的 `data-role="debug"` 之前会误读，现在会正确过滤。
- 表格行在去 Markdown 后之前可能漏进朗读，现在清洗前就先跳过表格/链接/git directive。
- 兜底摘要现在更偏向“儿童模式新增、声音方案支持”这类产品变化，少读“本地重新构建、重新安装”这种流水线信息。
- session 层新增测试，保证只读最新的 final answer，不读过程分析，也不回放旧回答。

新增/加强测试覆盖：

- HTML 协议优先级
- HTML role 过滤和实体反转义
- Markdown/隐藏导览优先级
- 表格、链接、Mermaid 源码、git directive 过滤
- 代码密集回复只读总结，不读源码
- 产品变化优先于本地构建步骤
- session 中只读最新最终回答

验证已通过：

- `cargo test`：79 个测试全部通过
- `cargo build --release`：通过

已提交并推送到 GitHub main：`dd2c46c`

::git-stage{cwd="/tmp/codex-speak"}
::git-commit{cwd="/tmp/codex-speak"}
::git-push{cwd="/tmp/codex-speak" branch="main"}
"#;
        let guide = fallback_reply_guide(text, 800);
        assert!(guide.contains("补好了"));
        assert!(guide.contains("兜底摘要"));
        assert!(guide.contains("测试和构建都通过了"));
        assert!(guide.ends_with('。'));
        assert!(!guide.contains("网页标记"));
        assert!(!guide.contains("命令名"));
        assert!(!guide.contains("英文单词"));
        assert!(!guide.contains("构建命令"));
        assert!(!guide.contains("命令参数"));
        assert!(!guide.contains("产品变化优先于本地构建步骤"));
        assert!(!guide.contains("dd2c46c"));
    }

    #[test]
    fn conservative_notice_does_not_read_visible_technical_report() {
        let text = r#"
补好了，并且这次测试真的抓出了两个现有问题，我也一起修了：

- HTML 协议里的 `data-role="debug"` 之前会误读，现在会正确过滤。
- 表格行在去 Markdown 后之前可能漏进朗读，现在清洗前就先跳过表格/链接/git directive。
- 兜底摘要现在更偏向“儿童模式新增、声音方案支持”这类产品变化，少读“本地重新构建、重新安装”这种流水线信息。
- session 层新增测试，保证只读最新的 final answer，不读过程分析，也不回放旧回答。

新增/加强测试覆盖：
- HTML 协议优先级
- HTML role 过滤和实体反转义
- 产品变化优先于本地构建步骤

验证已通过：
- `cargo test`：80 个测试全部通过
- `cargo build --release`：通过
"#;
        let notice = conservative_reply_notice(text, 800);
        assert!(notice.contains("补好了"));
        assert!(notice.contains("测试和构建都通过了"));
        assert!(notice.ends_with('。'));
        assert!(!notice.contains("网页标记"));
        assert!(!notice.contains("命令名"));
        assert!(!notice.contains("英文单词"));
        assert!(!notice.contains("产品变化优先于本地构建步骤"));
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

    #[test]
    fn truncates_at_sentence_boundary_when_possible() {
        let text = "我已经整理好游戏想法。第二句会继续展开很多内容，里面有角色、场景、玩法和下一步测试安排。";
        assert_eq!(truncate_chars(text, 18), "我已经整理好游戏想法。");
    }
}
