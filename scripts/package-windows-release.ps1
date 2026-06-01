param(
  [string]$PackageDir = "dist\codex-speak-windows"
)

$ErrorActionPreference = "Stop"

$RootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Package = Join-Path $RootDir $PackageDir
$ZipPath = Join-Path $RootDir "dist\codex-speak-windows.zip"
$ShaPath = Join-Path $RootDir "dist\codex-speak-windows.zip.sha256"

Remove-Item -Recurse -Force $Package -ErrorAction SilentlyContinue
Remove-Item -Force $ZipPath, $ShaPath -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path "$Package\bin", "$Package\apps", "$Package\installers", "$Package\scripts", "$Package\docs" | Out-Null

Copy-Item -Force "$RootDir\target\release\codex-speak.exe" "$Package\bin\codex-speak.exe"
Copy-Item -Force "$RootDir\apps\codex-speak-control\src-tauri\target\release\codex-speak-control.exe" "$Package\apps\codex-speak-control.exe"
Copy-Item -Force -Path @("$RootDir\installers\install-windows.ps1", "$RootDir\installers\uninstall-windows.ps1") -Destination "$Package\installers\"
Copy-Item -Force "$RootDir\scripts\manual-qa-windows.ps1" "$Package\scripts\manual-qa-windows.ps1"
Copy-Item -Force "$RootDir\README.md" "$Package\README.md"
Copy-Item -Force -Path @(
  "$RootDir\docs\installation.md",
  "$RootDir\docs\release-qa.md",
  "$RootDir\docs\requirements.md",
  "$RootDir\docs\technical-design.md",
  "$RootDir\docs\tauri-control-app.md"
) -Destination "$Package\docs\"

& "$RootDir\scripts\sign-windows-release.ps1" -PackageDir $Package

node "$RootDir\scripts\check-release-package.mjs" windows $Package
if ($LASTEXITCODE -ne 0) {
  throw "check-release-package.mjs failed with exit code $LASTEXITCODE"
}

Compress-Archive -Force -Path $Package -DestinationPath $ZipPath
$Hash = (Get-FileHash -Algorithm SHA256 $ZipPath).Hash.ToLower()
"$Hash  codex-speak-windows.zip" | Set-Content -NoNewline $ShaPath

Write-Host "Created $ZipPath"
