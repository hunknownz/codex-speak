param(
  [switch]$RemoveModels
)

$Cli = Join-Path $env:USERPROFILE ".codex\codex-speak\bin\codex-speak.exe"
if (Test-Path $Cli) {
  if ($RemoveModels) {
    & $Cli uninstall --remove-models
  } else {
    & $Cli uninstall
  }
} else {
  Write-Host "Codex Speak CLI not found at $Cli"
  exit 1
}

