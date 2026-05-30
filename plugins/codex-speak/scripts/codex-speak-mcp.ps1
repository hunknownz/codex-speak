$ErrorActionPreference = "Stop"

$cmd = Get-Command codex-speak -ErrorAction SilentlyContinue
if ($cmd) {
  & $cmd.Source mcp
  exit $LASTEXITCODE
}

$local = Join-Path $HOME ".codex\codex-speak\bin\codex-speak.exe"
if (Test-Path $local) {
  & $local mcp
  exit $LASTEXITCODE
}

Write-Error "codex-speak CLI is not installed. Run the Codex Speak installer first."
exit 127
