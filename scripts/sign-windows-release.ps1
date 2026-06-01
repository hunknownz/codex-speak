param(
  [Parameter(Mandatory = $true)]
  [string]$PackageDir
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $PackageDir)) {
  throw "Package directory does not exist: $PackageDir"
}

$CertificatePath = $env:WINDOWS_SIGN_CERT_PATH
$CertificatePassword = $env:WINDOWS_SIGN_CERT_PASSWORD

if (-not $CertificatePath) {
  Write-Host "Skipping Windows signing: WINDOWS_SIGN_CERT_PATH is not set."
  return
}

if (-not (Test-Path $CertificatePath)) {
  throw "Windows signing certificate not found: $CertificatePath"
}

function Find-SignTool {
  $Tool = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($Tool) {
    return $Tool.Source
  }

  $KitRoot = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
  if (Test-Path $KitRoot) {
    $Candidate = Get-ChildItem -Path $KitRoot -Recurse -Filter signtool.exe |
      Sort-Object FullName -Descending |
      Select-Object -First 1
    if ($Candidate) {
      return $Candidate.FullName
    }
  }

  throw "signtool.exe was not found."
}

$SignTool = Find-SignTool
$Targets = @(
  (Join-Path $PackageDir "bin\codex-speak.exe"),
  (Join-Path $PackageDir "apps\codex-speak-control.exe")
)

foreach ($Target in $Targets) {
  if (-not (Test-Path $Target)) {
    throw "Missing sign target: $Target"
  }

  $Args = @(
    "sign",
    "/fd", "SHA256",
    "/td", "SHA256",
    "/tr", "http://timestamp.digicert.com",
    "/f", $CertificatePath
  )
  if ($CertificatePassword) {
    $Args += @("/p", $CertificatePassword)
  }
  $Args += $Target

  & $SignTool @Args
  if ($LASTEXITCODE -ne 0) {
    throw "signtool sign failed for $Target with exit code $LASTEXITCODE"
  }

  & $SignTool verify /pa $Target
  if ($LASTEXITCODE -ne 0) {
    throw "signtool verify failed for $Target with exit code $LASTEXITCODE"
  }
}
