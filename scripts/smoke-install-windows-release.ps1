param(
  [string]$Archive = "dist\codex-speak-windows.zip"
)

$ErrorActionPreference = "Stop"

$Smoke = Join-Path $env:TEMP "codex-speak-release-smoke"
Remove-Item -Recurse -Force $Smoke -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Smoke | Out-Null
Expand-Archive -Force -Path $Archive -DestinationPath $Smoke

& (Join-Path $Smoke "codex-speak-windows\installers\install-windows.ps1") -SkipTtsDownload

$InstalledCli = Join-Path $env:USERPROFILE ".codex\codex-speak\bin\codex-speak.exe"
$InstalledApp = Join-Path $env:USERPROFILE ".codex\codex-speak\apps\codex-speak-control.exe"
if (-not (Test-Path $InstalledCli)) {
  throw "Installed CLI missing: $InstalledCli"
}
if (-not (Test-Path $InstalledApp)) {
  throw "Installed control app missing: $InstalledApp"
}

& $InstalledCli models list | Out-File -Encoding utf8 (Join-Path $Smoke "models.json")
Write-Host "Windows release smoke install OK"
