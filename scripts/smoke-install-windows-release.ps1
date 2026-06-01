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
$DoctorJson = Join-Path $Smoke "doctor.json"
$DoctorErr = Join-Path $Smoke "doctor.stderr.txt"
$DoctorProcess = Start-Process `
  -FilePath $InstalledCli `
  -ArgumentList @("doctor", "--json") `
  -NoNewWindow `
  -Wait `
  -PassThru `
  -RedirectStandardOutput $DoctorJson `
  -RedirectStandardError $DoctorErr
if (-not (Test-Path $DoctorJson) -or (Get-Item $DoctorJson).Length -eq 0) {
  $DoctorError = if (Test-Path $DoctorErr) { Get-Content -Raw -Encoding utf8 $DoctorErr } else { "" }
  throw "doctor --json produced no output. exit=$($DoctorProcess.ExitCode) $DoctorError"
}
$Doctor = Get-Content -Raw -Encoding utf8 $DoctorJson | ConvertFrom-Json
$Checks = @{}
foreach ($Check in $Doctor.checks) {
  $Checks[$Check.id] = $Check
}
foreach ($Id in @(
  "codex_home",
  "config",
  "cli",
  "control_app",
  "codex_notify",
  "plugin",
  "plugin_skill",
  "plugin_mcp_config",
  "plugin_mcp_script",
  "plugin_marketplace",
  "player"
)) {
  if (-not $Checks.ContainsKey($Id) -or $Checks[$Id].status -ne "ok") {
    $Status = if ($Checks.ContainsKey($Id)) { $Checks[$Id].status } else { "missing" }
    throw "doctor core check failed: $Id status=$Status"
  }
}
Write-Host "Windows release smoke install OK"
