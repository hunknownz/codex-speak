use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

use crate::bundled;

const SOURCES_FILE: &str = "data/speech-style/source-candidates.jsonl";
const PRINCIPLES_FILE: &str = "data/speech-style/distilled-principles.jsonl";
const EXAMPLES_FILE: &str = "data/speech-style/style-examples.jsonl";

#[derive(Debug, Clone, Copy)]
struct ValidationOptions {
    enforce_min_counts: bool,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            enforce_min_counts: true,
        }
    }
}

#[derive(Debug, PartialEq)]
struct StyleSummary {
    sources: usize,
    principles: usize,
    examples: usize,
    child: usize,
    adult: usize,
    visual: usize,
}

#[derive(Debug, Deserialize)]
struct SourceCandidate {
    id: String,
    name: String,
    url: String,
    platform: String,
    language: Vec<String>,
    kind: Vec<String>,
    license: String,
    use_level: String,
    risks: Vec<String>,
    status: String,
    license_status: String,
    ingest_allowed: bool,
    attribution_required: bool,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DistilledPrinciple {
    id: String,
    mode: String,
    principle: String,
    zh_rule: String,
    source_ids: Vec<String>,
    quality_check: String,
}

#[derive(Debug, Deserialize)]
struct StyleExample {
    id: String,
    mode: String,
    scene: String,
    label: String,
    language: String,
    text: String,
    source: String,
    source_ids: Vec<String>,
    tags: Vec<String>,
    quality: Value,
}

pub fn validate() -> Result<()> {
    let summary = validate_all(
        bundled::SPEECH_STYLE_SOURCES,
        bundled::SPEECH_STYLE_PRINCIPLES,
        bundled::SPEECH_STYLE_EXAMPLES,
        ValidationOptions::default(),
    )?;

    println!("Codex Speak style data OK");
    println!("  sources: {}", summary.sources);
    println!("  principles: {}", summary.principles);
    println!("  examples: {}", summary.examples);
    println!("  child: {}", summary.child);
    println!("  adult: {}", summary.adult);
    println!("  visual: {}", summary.visual);
    Ok(())
}

pub fn print_sources() -> Result<()> {
    let values = parse_jsonl_values(SOURCES_FILE, bundled::SPEECH_STYLE_SOURCES)?;
    let _ = validate_sources(&values)?;
    let source_values: Vec<&Value> = values.iter().map(|item| &item.value).collect();
    println!("{}", serde_json::to_string_pretty(&source_values)?);
    Ok(())
}

fn validate_all(
    sources_raw: &str,
    principles_raw: &str,
    examples_raw: &str,
    options: ValidationOptions,
) -> Result<StyleSummary> {
    let source_values = parse_jsonl_values(SOURCES_FILE, sources_raw)?;
    let sources = validate_sources(&source_values)?;
    let source_by_id: HashMap<String, SourceCandidate> = sources
        .into_iter()
        .map(|source| (source.id.clone(), source))
        .collect();

    let principle_values = parse_jsonl_values(PRINCIPLES_FILE, principles_raw)?;
    let principles = validate_principles(&principle_values, &source_by_id)?;

    let example_values = parse_jsonl_values(EXAMPLES_FILE, examples_raw)?;
    let summary = validate_examples(&example_values, &source_by_id, options)?;

    Ok(StyleSummary {
        sources: source_by_id.len(),
        principles,
        ..summary
    })
}

fn parse_jsonl_values(file: &str, raw: &str) -> Result<Vec<LineValue>> {
    raw.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some((index + 1, trimmed))
            }
        })
        .map(|(line, text)| {
            let value: Value = serde_json::from_str(text)
                .map_err(|err| anyhow::anyhow!("{file}:{line}: invalid JSON: {err}"))?;
            if !value.is_object() {
                bail!("{file}:{line}: expected a JSON object");
            }
            Ok(LineValue { line, value })
        })
        .collect()
}

#[derive(Debug)]
struct LineValue {
    line: usize,
    value: Value,
}

fn validate_sources(values: &[LineValue]) -> Result<Vec<SourceCandidate>> {
    let mut seen = HashSet::new();
    let mut sources = Vec::with_capacity(values.len());
    for item in values {
        require_fields(
            SOURCES_FILE,
            item.line,
            &item.value,
            &[
                "id",
                "name",
                "url",
                "platform",
                "language",
                "kind",
                "license",
                "use_level",
                "risks",
                "status",
                "license_status",
                "ingest_allowed",
                "attribution_required",
            ],
        )?;
        let source: SourceCandidate =
            serde_json::from_value(item.value.clone()).map_err(|err| {
                anyhow::anyhow!(
                    "{}:{}: invalid source schema: {err}",
                    SOURCES_FILE,
                    item.line
                )
            })?;
        if !seen.insert(source.id.clone()) {
            bail!(
                "{}:{}: duplicate source id `{}`",
                SOURCES_FILE,
                item.line,
                source.id
            );
        }
        if source.id.trim().is_empty()
            || source.name.trim().is_empty()
            || source.url.trim().is_empty()
            || source.platform.trim().is_empty()
            || source.language.is_empty()
            || source.kind.is_empty()
            || source.use_level.trim().is_empty()
            || source.status.trim().is_empty()
            || source.license_status.trim().is_empty()
            || source.risks.iter().any(|risk| risk.trim().is_empty())
            || source.notes.as_deref().unwrap_or("ok").trim().is_empty()
        {
            bail!(
                "{}:{}: source `{}` has an empty required value",
                SOURCES_FILE,
                item.line,
                source.id
            );
        }
        if source.attribution_required && source.url.trim().is_empty() {
            bail!(
                "{}:{}: source `{}` requires attribution but has no URL",
                SOURCES_FILE,
                item.line,
                source.id
            );
        }
        if source.ingest_allowed && !is_commercial_friendly_license(&source.license) {
            bail!(
                "{}:{}: source `{}` has ingest_allowed=true but license `{}` is blocked",
                SOURCES_FILE,
                item.line,
                source.id,
                source.license
            );
        }
        sources.push(source);
    }
    Ok(sources)
}

fn validate_principles(
    values: &[LineValue],
    source_by_id: &HashMap<String, SourceCandidate>,
) -> Result<usize> {
    let mut seen = HashSet::new();
    for item in values {
        require_fields(
            PRINCIPLES_FILE,
            item.line,
            &item.value,
            &[
                "id",
                "mode",
                "principle",
                "zh_rule",
                "source_ids",
                "quality_check",
            ],
        )?;
        let principle: DistilledPrinciple =
            serde_json::from_value(item.value.clone()).map_err(|err| {
                anyhow::anyhow!(
                    "{}:{}: invalid principle schema: {err}",
                    PRINCIPLES_FILE,
                    item.line
                )
            })?;
        if !seen.insert(principle.id.clone()) {
            bail!(
                "{}:{}: duplicate principle id `{}`",
                PRINCIPLES_FILE,
                item.line,
                principle.id
            );
        }
        if !matches!(principle.mode.as_str(), "child" | "adult" | "both") {
            bail!(
                "{}:{}: principle `{}` has invalid mode `{}`",
                PRINCIPLES_FILE,
                item.line,
                principle.id,
                principle.mode
            );
        }
        if principle.principle.trim().is_empty()
            || principle.zh_rule.trim().is_empty()
            || principle.quality_check.trim().is_empty()
            || principle.source_ids.is_empty()
        {
            bail!(
                "{}:{}: principle `{}` has an empty required value",
                PRINCIPLES_FILE,
                item.line,
                principle.id
            );
        }
        validate_source_ids(
            PRINCIPLES_FILE,
            item.line,
            &principle.id,
            &principle.source_ids,
            source_by_id,
        )?;
    }
    Ok(values.len())
}

fn validate_examples(
    values: &[LineValue],
    source_by_id: &HashMap<String, SourceCandidate>,
    options: ValidationOptions,
) -> Result<StyleSummary> {
    let mut seen = HashSet::new();
    let mut child = 0;
    let mut adult = 0;
    let mut visual = 0;
    for item in values {
        require_fields(
            EXAMPLES_FILE,
            item.line,
            &item.value,
            &[
                "id",
                "mode",
                "scene",
                "label",
                "language",
                "text",
                "source",
                "source_ids",
                "tags",
                "quality",
            ],
        )?;
        let example: StyleExample = serde_json::from_value(item.value.clone()).map_err(|err| {
            anyhow::anyhow!(
                "{}:{}: invalid style example schema: {err}",
                EXAMPLES_FILE,
                item.line
            )
        })?;
        if !seen.insert(example.id.clone()) {
            bail!(
                "{}:{}: duplicate example id `{}`",
                EXAMPLES_FILE,
                item.line,
                example.id
            );
        }
        if !matches!(example.mode.as_str(), "child" | "adult") {
            bail!(
                "{}:{}: example `{}` has invalid mode `{}`",
                EXAMPLES_FILE,
                item.line,
                example.id,
                example.mode
            );
        }
        if !matches!(example.label.as_str(), "good" | "bad" | "rewrite") {
            bail!(
                "{}:{}: example `{}` has invalid label `{}`",
                EXAMPLES_FILE,
                item.line,
                example.id,
                example.label
            );
        }
        if example.scene.trim().is_empty()
            || example.language.trim().is_empty()
            || example.text.trim().is_empty()
            || example.source.trim().is_empty()
            || example.source_ids.is_empty()
            || example.tags.is_empty()
            || !example.quality.is_object()
        {
            bail!(
                "{}:{}: example `{}` has an empty required value",
                EXAMPLES_FILE,
                item.line,
                example.id
            );
        }
        validate_source_ids(
            EXAMPLES_FILE,
            item.line,
            &example.id,
            &example.source_ids,
            source_by_id,
        )?;
        if example.source != "original" {
            for source_id in &example.source_ids {
                let source = source_by_id.get(source_id).expect("source id checked");
                if !source.ingest_allowed {
                    bail!(
                        "{}:{}: example `{}` directly uses non-ingestable source `{}`",
                        EXAMPLES_FILE,
                        item.line,
                        example.id,
                        source_id
                    );
                }
            }
        }
        validate_speech_text(&example, item.line)?;

        if example.mode == "child" {
            child += 1;
        } else {
            adult += 1;
        }
        if example.scene == "visual" || example.tags.iter().any(|tag| tag == "visual") {
            visual += 1;
        }
    }

    let summary = StyleSummary {
        sources: 0,
        principles: 0,
        examples: values.len(),
        child,
        adult,
        visual,
    };
    if options.enforce_min_counts {
        if summary.examples < 50 {
            bail!(
                "{}: expected at least 50 examples, found {}",
                EXAMPLES_FILE,
                summary.examples
            );
        }
        if summary.child < 30 {
            bail!(
                "{}: expected at least 30 child examples, found {}",
                EXAMPLES_FILE,
                summary.child
            );
        }
        if summary.adult < 10 {
            bail!(
                "{}: expected at least 10 adult examples, found {}",
                EXAMPLES_FILE,
                summary.adult
            );
        }
        if summary.visual < 10 {
            bail!(
                "{}: expected at least 10 visual examples, found {}",
                EXAMPLES_FILE,
                summary.visual
            );
        }
    }
    Ok(summary)
}

fn require_fields(file: &str, line: usize, value: &Value, fields: &[&str]) -> Result<()> {
    for field in fields {
        if value.get(*field).is_none() {
            bail!("{file}:{line}: missing required field `{field}`");
        }
    }
    Ok(())
}

fn validate_source_ids(
    file: &str,
    line: usize,
    owner_id: &str,
    source_ids: &[String],
    source_by_id: &HashMap<String, SourceCandidate>,
) -> Result<()> {
    for source_id in source_ids {
        if !source_by_id.contains_key(source_id) {
            bail!("{file}:{line}: `{owner_id}` references unknown source id `{source_id}`");
        }
    }
    Ok(())
}

fn validate_speech_text(example: &StyleExample, line: usize) -> Result<()> {
    let text = example.text.as_str();
    let banned_meta = [
        "儿童模式",
        "生成给小孩",
        "小孩能听懂",
        "儿童友好",
        "要生成",
        "我将输出",
        "Mermaid 图表源码",
    ];
    for phrase in banned_meta {
        if text.contains(phrase) {
            bail!(
                "{}:{}: example `{}` contains meta talk `{}`",
                EXAMPLES_FILE,
                line,
                example.id,
                phrase
            );
        }
    }

    let over_teaching = [
        "今天我们来学习",
        "你需要理解",
        "重要概念",
        "定义是",
        "软件工程中的",
    ];
    for phrase in over_teaching {
        if text.contains(phrase) {
            bail!(
                "{}:{}: example `{}` over-teaches with `{}`",
                EXAMPLES_FILE,
                line,
                example.id,
                phrase
            );
        }
    }

    let technical_noise = Regex::new(
        r"(?s)```|</?[a-zA-Z][^>]*>|(?:^|\s)--[A-Za-z0-9][A-Za-z0-9_-]*|/Users/[^\s，。；]+|[A-Za-z]:\\[^\s，。；]+|\b(?:flowchart|graph)\s+(?:LR|TD|TB|RL)|-->|^\s*(?:cargo|git|npm|node|python|rustc)\s+",
    )
    .expect("technical noise regex should compile");
    if technical_noise.is_match(text) {
        bail!(
            "{}:{}: example `{}` leaks code, command, path, HTML, or chart source",
            EXAMPLES_FILE,
            line,
            example.id
        );
    }

    if example.mode == "child" {
        if text.chars().count() > 280 {
            bail!(
                "{}:{}: child example `{}` is too long for speech",
                EXAMPLES_FILE,
                line,
                example.id
            );
        }
        for sentence in split_sentences(text) {
            if sentence.chars().count() > 70 {
                bail!(
                    "{}:{}: child example `{}` has an overlong sentence",
                    EXAMPLES_FILE,
                    line,
                    example.id
                );
            }
        }
    } else if text.chars().count() > 420 {
        bail!(
            "{}:{}: adult example `{}` is too long for speech",
            EXAMPLES_FILE,
            line,
            example.id
        );
    }

    if example.mode == "adult" {
        for phrase in ["小朋友", "姐姐", "很棒哦", "小脑袋"] {
            if text.contains(phrase) {
                bail!(
                    "{}:{}: adult example `{}` uses childlike phrase `{}`",
                    EXAMPLES_FILE,
                    line,
                    example.id,
                    phrase
                );
            }
        }
    }

    Ok(())
}

fn split_sentences(text: &str) -> Vec<&str> {
    text.split(['。', '！', '？', '；', '\n'])
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .collect()
}

fn is_commercial_friendly_license(license: &str) -> bool {
    matches!(
        license.to_ascii_lowercase().as_str(),
        "apache-2.0" | "mit" | "cc-by-4.0" | "cc0-1.0" | "project_original"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const VALID_SOURCES: &str = r#"{"id":"internal_project_case","name":"Project original cases","url":"local://codex-speak/original","platform":"local","language":["zh"],"kind":["project_example"],"license":"project_original","use_level":"sample_reference","risks":[],"status":"approved","license_status":"approved_original","ingest_allowed":true,"attribution_required":false}
{"id":"blocked_nc","name":"Non-commercial sample","url":"https://example.test/nc","platform":"web","language":["en"],"kind":["dialogue"],"license":"cc-by-nc-4.0","use_level":"research_only","risks":["non_commercial"],"status":"candidate","license_status":"research_only","ingest_allowed":false,"attribution_required":true}"#;

    const VALID_PRINCIPLES: &str = r#"{"id":"plain_short_active","mode":"child","principle":"Use short sentences.","zh_rule":"一句话只讲一件事。","source_ids":["internal_project_case"],"quality_check":"Short and useful."}"#;

    fn example_line(id: &str, source: &str, source_ids: &str, text: &str) -> String {
        json!({
            "id": id,
            "mode": "child",
            "scene": "debugging",
            "label": "good",
            "language": "zh-CN",
            "text": text,
            "source": source,
            "source_ids": source_ids.split(',').collect::<Vec<_>>(),
            "tags": ["debugging", "next_action"],
            "quality": {
                "meta_talk": false,
                "over_teaching": false,
                "visual_source_leak": false,
                "has_next_action": true
            }
        })
        .to_string()
    }

    fn validate_fixture(examples: &str) -> Result<StyleSummary> {
        validate_all(
            VALID_SOURCES,
            VALID_PRINCIPLES,
            examples,
            ValidationOptions {
                enforce_min_counts: false,
            },
        )
    }

    #[test]
    fn valid_embedded_style_data_passes() {
        let summary = validate_all(
            bundled::SPEECH_STYLE_SOURCES,
            bundled::SPEECH_STYLE_PRINCIPLES,
            bundled::SPEECH_STYLE_EXAMPLES,
            ValidationOptions::default(),
        )
        .expect("embedded style data should validate");
        assert!(summary.examples >= 50);
        assert!(summary.child >= 30);
        assert!(summary.adult >= 10);
        assert!(summary.visual >= 10);
    }

    #[test]
    fn duplicate_source_id_fails() {
        let sources = format!("{VALID_SOURCES}\n{}", VALID_SOURCES.lines().next().unwrap());
        let err = validate_all(
            &sources,
            VALID_PRINCIPLES,
            &example_line(
                "ok",
                "original",
                "internal_project_case",
                "我修好了刷新。现在你可以看新时间。",
            ),
            ValidationOptions {
                enforce_min_counts: false,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("duplicate source id"));
    }

    #[test]
    fn missing_example_field_fails() {
        let examples = r#"{"id":"missing_text","mode":"child","scene":"debugging","label":"good","language":"zh-CN","source":"original","source_ids":["internal_project_case"],"tags":["debugging"],"quality":{}}"#;
        let err = validate_fixture(examples).unwrap_err().to_string();
        assert!(err.contains("missing required field `text`"));
    }

    #[test]
    fn unknown_source_id_fails() {
        let examples = example_line(
            "unknown",
            "original",
            "nope",
            "我修好了刷新。现在你可以看新时间。",
        );
        let err = validate_fixture(&examples).unwrap_err().to_string();
        assert!(err.contains("unknown source id"));
    }

    #[test]
    fn blocked_source_ingest_fails() {
        let sources =
            VALID_SOURCES.replace(r#""ingest_allowed":false"#, r#""ingest_allowed":true"#);
        let err = validate_all(
            &sources,
            VALID_PRINCIPLES,
            &example_line(
                "ok",
                "original",
                "internal_project_case",
                "我修好了刷新。现在你可以看新时间。",
            ),
            ValidationOptions {
                enforce_min_counts: false,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("license `cc-by-nc-4.0` is blocked"));
    }

    #[test]
    fn child_meta_talk_fails() {
        let examples = example_line(
            "meta",
            "original",
            "internal_project_case",
            "要生成给小孩听的总结，所以我先说明规则。",
        );
        let err = validate_fixture(&examples).unwrap_err().to_string();
        assert!(err.contains("contains meta talk"));
    }

    #[test]
    fn raw_visual_source_leak_fails() {
        let examples = example_line(
            "mermaid",
            "original",
            "internal_project_case",
            "我会画图。flowchart LR A-->B",
        );
        let err = validate_fixture(&examples).unwrap_err().to_string();
        assert!(err.contains("leaks code"));
    }
}
