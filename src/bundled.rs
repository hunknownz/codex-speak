use std::path::Path;

pub const CODEX_SKILL: &str = include_str!("../skills/codex-speak/SKILL.md");
pub const PLUGIN_MANIFEST: &str = include_str!("../plugins/codex-speak/.codex-plugin/plugin.json");
pub const PLUGIN_MCP_CONFIG: &str = include_str!("../plugins/codex-speak/.mcp.json");
pub const PLUGIN_README: &str = include_str!("../plugins/codex-speak/README.md");
pub const PLUGIN_SKILL: &str = include_str!("../plugins/codex-speak/skills/codex-speak/SKILL.md");
pub const PLUGIN_MCP_SCRIPT_UNIX: &str =
    include_str!("../plugins/codex-speak/scripts/codex-speak-mcp");
pub const PLUGIN_MCP_SCRIPT_WINDOWS: &str =
    include_str!("../plugins/codex-speak/scripts/codex-speak-mcp.ps1");

pub fn hook_content(cli_path: &Path) -> String {
    if cfg!(windows) {
        format!(
            r#"$ErrorActionPreference = "SilentlyContinue"
& "{}" speak *> $null
exit 0
"#,
            cli_path.display()
        )
    } else {
        format!(
            r#"#!/usr/bin/env bash
set -u
"{}" speak >/dev/null 2>&1 || true
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
        assert!(content.contains("speak"));
    }
}
