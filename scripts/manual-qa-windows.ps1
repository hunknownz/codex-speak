param(
  [string]$CliPath = (Join-Path $env:USERPROFILE ".codex\codex-speak\bin\codex-speak.exe"),
  [string]$OutputDir = "",
  [switch]$AllowMissingModels,
  [switch]$NonInteractive,
  [switch]$SkipAppOpen,
  [switch]$SkipSpeak
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $PSCommandPath
$PackageDir = Split-Path -Parent $ScriptDir
$PackageCli = Join-Path $PackageDir "bin\codex-speak.exe"

if (-not $OutputDir) {
  $Stamp = Get-Date -Format "yyyyMMdd-HHmmss"
  $OutputDir = Join-Path $env:TEMP "codex-speak-windows-qa-$Stamp"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$Checks = New-Object System.Collections.Generic.List[object]
$HadFailure = $false

function Add-QaCheck {
  param(
    [string]$Name,
    [string]$Status,
    [string]$Detail,
    [object]$ExitCode = $null,
    [string]$Stdout = "",
    [string]$Stderr = ""
  )

  $script:Checks.Add([ordered]@{
    name = $Name
    status = $Status
    detail = $Detail
    exitCode = $ExitCode
    stdout = $Stdout
    stderr = $Stderr
  })

  if ($Status -eq "fail") {
    $script:HadFailure = $true
  }
}

function Invoke-QaExecutable {
  param(
    [string]$Name,
    [string]$Executable,
    [string[]]$Arguments,
    [switch]$AllowFailure
  )

  $SafeName = $Name -replace "[^A-Za-z0-9_.-]", "-"
  $Stdout = Join-Path $OutputDir "$SafeName.stdout.txt"
  $Stderr = Join-Path $OutputDir "$SafeName.stderr.txt"

  & $Executable @Arguments > $Stdout 2> $Stderr
  $ExitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
  $Status = if ($ExitCode -eq 0 -or $AllowFailure) { "pass" } else { "fail" }
  $Detail = if ($ExitCode -eq 0) { "exit 0" } else { "exit $ExitCode" }
  Add-QaCheck $Name $Status $Detail $ExitCode $Stdout $Stderr
  return $ExitCode
}

function Invoke-QaCommand {
  param(
    [string]$Name,
    [string[]]$Arguments,
    [switch]$AllowFailure
  )
  return Invoke-QaExecutable -Name $Name -Executable $CliPath -Arguments $Arguments -AllowFailure:$AllowFailure
}

function Add-ManualCheck {
  param(
    [string]$Name,
    [string]$Question
  )

  if ($NonInteractive) {
    Add-QaCheck $Name "skip" "skipped by -NonInteractive"
    return
  }

  while ($true) {
    $Answer = (Read-Host "$Question [y/n/s]").Trim().ToLowerInvariant()
    if ($Answer -in @("y", "yes")) {
      Add-QaCheck $Name "pass" "tester confirmed"
      return
    }
    if ($Answer -in @("n", "no")) {
      Add-QaCheck $Name "fail" "tester reported failure"
      return
    }
    if ($Answer -in @("s", "skip")) {
      Add-QaCheck $Name "skip" "tester skipped"
      return
    }
    Write-Host "Please answer y, n, or s."
  }
}

function Get-OsCaption {
  try {
    return (Get-CimInstance Win32_OperatingSystem).Caption
  } catch {
    return [Environment]::OSVersion.VersionString
  }
}

function Write-QaReport {
  $Report = [ordered]@{
    generatedAt = (Get-Date).ToString("o")
    cliPath = $CliPath
    outputDir = $OutputDir
    nonInteractive = [bool]$NonInteractive
    allowMissingModels = [bool]$AllowMissingModels
    machine = [ordered]@{
      os = Get-OsCaption
      osVersion = [Environment]::OSVersion.VersionString
      architecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
      powerShell = $PSVersionTable.PSVersion.ToString()
      user = [Environment]::UserName
    }
    checks = $Checks
  }
  $ReportPath = Join-Path $OutputDir "qa-report.json"
  $Report | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $ReportPath
  Write-Host "QA report: $ReportPath"
}

if (-not (Test-Path $CliPath)) {
  Add-QaCheck "cli exists" "fail" "missing CLI at $CliPath"
  Write-QaReport
  exit 1
}

Add-QaCheck "cli exists" "pass" $CliPath

Invoke-QaCommand "cli version" @("--version") | Out-Null
if ((Test-Path $PackageCli) -and (Test-Path (Join-Path $PackageDir "release-manifest.json"))) {
  Invoke-QaExecutable "release package manifest" $PackageCli @("verify-package", "--package-dir", $PackageDir) | Out-Null
} else {
  Add-QaCheck "release package manifest" "fail" "missing release package manifest or package CLI near $ScriptDir"
}

$VerifyArgs = @("verify-install")
if ($AllowMissingModels) {
  $VerifyArgs += "--allow-missing-models"
}
Invoke-QaCommand "verify install" $VerifyArgs | Out-Null

Invoke-QaCommand "doctor json" @("doctor", "--json") -AllowFailure | Out-Null
$DoctorStdout = Join-Path $OutputDir "doctor-json.stdout.txt"
try {
  Get-Content -Raw -Encoding utf8 $DoctorStdout | ConvertFrom-Json | Out-Null
  Add-QaCheck "doctor json parse" "pass" "doctor JSON parsed"
} catch {
  Add-QaCheck "doctor json parse" "fail" $_.Exception.Message
}

Invoke-QaCommand "status" @("status") | Out-Null
Invoke-QaCommand "models list" @("models", "list") | Out-Null
Invoke-QaCommand "verify codex integration" @("verify-codex") | Out-Null
Invoke-QaCommand "verify controls" @("verify-controls") | Out-Null

$SupportDir = Join-Path $OutputDir "support-bundle"
Invoke-QaCommand "support bundle" @("support-bundle", "--output", $SupportDir) | Out-Null
foreach ($File in @("doctor.json", "status.json", "models.json")) {
  $Path = Join-Path $SupportDir $File
  if (Test-Path $Path) {
    Add-QaCheck "support $File" "pass" $Path
  } else {
    Add-QaCheck "support $File" "fail" "missing $Path"
  }
}
$ManifestPath = Join-Path $SupportDir "release-manifest.json"
$MissingManifestPath = Join-Path $SupportDir "release-manifest-missing.txt"
if (Test-Path $ManifestPath) {
  Add-QaCheck "support release manifest" "pass" $ManifestPath
} elseif (Test-Path $MissingManifestPath) {
  Add-QaCheck "support release manifest" "pass" $MissingManifestPath
} else {
  Add-QaCheck "support release manifest" "fail" "missing release manifest or missing marker"
}

Invoke-QaCommand "app path" @("app", "path") | Out-Null

if (-not $SkipAppOpen -and -not $NonInteractive) {
  Invoke-QaCommand "app open" @("app", "open") | Out-Null
  Add-ManualCheck "control app visible" "Did the Codex Speak control panel open?"
  Add-ManualCheck "control settings adjustable" "Can you toggle child mode and change speed, voice profile, and TTS provider in the control panel?"
}

if (-not $SkipSpeak -and -not $NonInteractive) {
  Invoke-QaCommand "speak sample" @("speak", "--text", "你好，这是CodexSpeak的Windows真机朗读验收。") -AllowFailure | Out-Null
  Add-ManualCheck "speech audible" "Did you hear the test voice clearly?"

  $StopStdout = Join-Path $OutputDir "stop-background-speak.stdout.txt"
  $StopStderr = Join-Path $OutputDir "stop-background-speak.stderr.txt"
  $LongText = "这是CodexSpeak的停止按钮测试。我会读得稍微久一点，方便你确认停止命令有没有打断朗读。"
  $Process = Start-Process -FilePath $CliPath -ArgumentList @("speak", "--text", $LongText) -NoNewWindow -PassThru -RedirectStandardOutput $StopStdout -RedirectStandardError $StopStderr
  Start-Sleep -Seconds 2
  Invoke-QaCommand "stop speech" @("stop") | Out-Null
  if (-not $Process.HasExited) {
    try {
      Wait-Process -Id $Process.Id -Timeout 10
    } catch {
      Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
    }
  }
  Add-QaCheck "background speak process" "pass" "started pid $($Process.Id)" $Process.ExitCode $StopStdout $StopStderr
  Add-ManualCheck "speech stopped" "Did the stop command interrupt the voice?"
}

Write-QaReport

if ($HadFailure) {
  Write-Host "Windows manual QA found failures."
  exit 1
}

Write-Host "Windows manual QA completed."
