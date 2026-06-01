param(
  [switch]$SkipTtsDownload,
  [switch]$SkipControlApp
)

$ErrorActionPreference = "Stop"

$RootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$AppTargetDir = Join-Path $env:USERPROFILE ".codex\codex-speak\apps"
$AppTarget = Join-Path $AppTargetDir "codex-speak-control.exe"

function Copy-ControlApp {
  param([string]$Source)

  New-Item -ItemType Directory -Force -Path $AppTargetDir | Out-Null
  Copy-Item -Force $Source $AppTarget
  Write-Host "Codex Speak control app installed at $AppTarget"
}

function First-ExistingPath {
  param([string[]]$Candidates)

  foreach ($Candidate in $Candidates) {
    if (Test-Path $Candidate) {
      return $Candidate
    }
  }
  return $null
}

function Write-NextSteps {
  $CliPath = Join-Path $env:USERPROFILE ".codex\codex-speak\bin\codex-speak.exe"
  $IncludeControlApp = (-not $SkipControlApp) -and (Test-Path $AppTarget)
  Write-Host ""
  Write-Host "Next steps:"
  Write-Host "  1. Run self-check:"
  Write-Host "     $CliPath doctor"
  if ($IncludeControlApp) {
    Write-Host "  2. Open the control app:"
    Write-Host "     $CliPath app open"
    Write-Host "  3. If you need help, create a support bundle:"
  } else {
    Write-Host "  2. If you need help, create a support bundle:"
  }
  Write-Host "     $CliPath support-bundle"
  if ($SkipTtsDownload) {
    if ($IncludeControlApp) {
      Write-Host "  4. Install the default local Chinese voice model when ready:"
    } else {
      Write-Host "  3. Install the default local Chinese voice model when ready:"
    }
    Write-Host "     $CliPath models install --provider sherpa_melo"
  }
}

Push-Location $RootDir
try {
  $Cli = First-ExistingPath -Candidates @(
    (Join-Path $RootDir "bin\codex-speak.exe"),
    (Join-Path $RootDir "target\release\codex-speak.exe")
  )

  if (-not $Cli) {
    $Cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if ($Cargo) {
      cargo build --release
      if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
      }
      $Cli = Join-Path $RootDir "target\release\codex-speak.exe"
    } else {
      throw "Codex Speak CLI was not found. Use a release package that contains bin\codex-speak.exe, or install Rust from https://rustup.rs and run this script from the source checkout."
    }
  }

  if (-not (Test-Path $Cli)) {
    throw "Codex Speak CLI was not found at $Cli"
  }

  $Args = @("install", "--no-summary")
  if ($SkipTtsDownload) {
    $Args += "--skip-tts-download"
  }

  & $Cli @Args
  if ($LASTEXITCODE -ne 0) {
    throw "codex-speak install failed with exit code $LASTEXITCODE"
  }

  if ($SkipControlApp) {
    Write-Host "Skipping Codex Speak control app install."
    Write-NextSteps
    return
  }

  $AppSource = First-ExistingPath -Candidates @(
    (Join-Path $RootDir "apps\codex-speak-control.exe"),
    (Join-Path $RootDir "apps\Codex Speak.exe"),
    (Join-Path $RootDir "apps\codex-speak-control\src-tauri\target\release\codex-speak-control.exe")
  )

  if ($AppSource) {
    Copy-ControlApp $AppSource
    Write-NextSteps
    return
  }

  $PackageJson = Join-Path $RootDir "apps\codex-speak-control\package.json"
  $Npm = Get-Command npm -ErrorAction SilentlyContinue
  if ($Npm -and (Test-Path $PackageJson)) {
    Push-Location (Join-Path $RootDir "apps\codex-speak-control")
    try {
      npm install
      if ($LASTEXITCODE -ne 0) {
        throw "npm install failed with exit code $LASTEXITCODE"
      }
      npm run build
      if ($LASTEXITCODE -ne 0) {
        throw "npm run build failed with exit code $LASTEXITCODE"
      }
    } finally {
      Pop-Location
    }
    $AppSource = Join-Path $RootDir "apps\codex-speak-control\src-tauri\target\release\codex-speak-control.exe"
  }

  if ($AppSource -and (Test-Path $AppSource)) {
    Copy-ControlApp $AppSource
  } else {
    Write-Host "Codex Speak control app was not installed because no prebuilt app or npm build path is available."
  }
  Write-NextSteps
} finally {
  Pop-Location
}
