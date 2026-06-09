use std::path::Path;

pub const CODEX_SKILL: &str = include_str!("../skills/codex-speak/SKILL.md");
pub const CODEX_SPEECH_STYLE_EXAMPLES: &str =
    include_str!("../skills/codex-speak/speech-style-examples.jsonl");
pub const SPEECH_STYLE_SOURCES: &str = include_str!("../data/speech-style/source-candidates.jsonl");
pub const SPEECH_STYLE_PRINCIPLES: &str =
    include_str!("../data/speech-style/distilled-principles.jsonl");
pub const SPEECH_STYLE_EXAMPLES: &str = include_str!("../data/speech-style/style-examples.jsonl");

pub fn hook_content(cli_path: &Path) -> String {
    if cfg!(windows) {
        format!(
            r#"$ErrorActionPreference = "SilentlyContinue"
$input | & "{}" hook --stdin *> $null
exit 0
"#,
            cli_path.display()
        )
    } else {
        format!(
            r#"#!/usr/bin/env bash
set -u
"{}" hook --stdin >/dev/null 2>&1 || true
exit 0
"#,
            cli_path.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::hook_content;
    use std::path::Path;

    #[test]
    fn hook_content_points_at_cli() {
        let content = hook_content(Path::new("/tmp/codex-speak"));
        assert!(content.contains("/tmp/codex-speak"));
        assert!(content.contains("hook --stdin"));
    }

}
