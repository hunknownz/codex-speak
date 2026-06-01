param(
  [switch]$RemoveModels
)

$Cli = Join-Path $env:USERPROFILE ".codex\codex-speak\bin\codex-speak.exe"
$App = Join-Path $env:USERPROFILE ".codex\codex-speak\apps\codex-speak-control.exe"
if (Test-Path $Cli) {
  if ($RemoveModels) {
    & $Cli uninstall --remove-models
  } else {
    & $Cli uninstall
  }
  if (Test-Path $App) {
    Remove-Item -Force $App
  }
} else {
  Write-Host "Codex Speak CLI not found at $Cli"
  exit 1
}
