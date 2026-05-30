param(
  [switch]$SkipTtsDownload
)

Write-Host "Codex Speak Windows installer skeleton"
Write-Host "Build the Rust CLI on Windows, then run:"
Write-Host "  .\\target\\release\\codex-speak.exe install"

if ($SkipTtsDownload) {
  Write-Host "TTS download will be skipped."
}

