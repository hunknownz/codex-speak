#!/usr/bin/env node
import { accessSync, constants, existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";

const [platform, packageDir] = process.argv.slice(2);

if (!platform || !packageDir) {
  fail("Usage: check-release-package.mjs <macos|windows> <package-dir>");
}

const root = path.resolve(packageDir);
if (!existsSync(root) || !statSync(root).isDirectory()) {
  fail(`Package directory does not exist: ${root}`);
}

const commonFiles = [
  "README.md",
  "release-manifest.json",
  "docs/installation.md",
  "docs/release-qa.md",
  "docs/requirements.md",
  "docs/technical-design.md",
  "docs/tauri-control-app.md"
];

for (const file of commonFiles) {
  requireFile(file);
}
checkReleaseManifest(platform);

if (platform === "macos") {
  requireExecutable("bin/codex-speak");
  requireExecutable("bin/codex-speak-pet-macos");
  requireDirectory("apps/Codex Speak.app");
  requireExecutable("installers/install-macos.sh");
  requireExecutable("installers/uninstall-macos.sh");
  requireExecutable("scripts/manual-qa-macos.sh");
  requireExecutable("scripts/check-manual-qa-report.mjs");
  requireFile("assets/pet/codex-agent.mov");
  requireFile("assets/pet/codex-agent-source-spritesheet.png");
  requireFile("assets/pet/codex-agent-hit.png");
  requireFile("assets/pet/codex-agent-preview.png");
  requireFile("assets/pet/ASSET-NOTICE.txt");
} else if (platform === "windows") {
  requireFile("bin/codex-speak.exe");
  requireFile("apps/codex-speak-control.exe");
  requireFile("installers/install-windows.ps1");
  requireFile("installers/uninstall-windows.ps1");
  requireFile("scripts/manual-qa-windows.ps1");
  requireFile("scripts/check-manual-qa-report.mjs");
} else {
  fail(`Unsupported platform: ${platform}`);
}

console.log(`Release package layout OK: ${platform} ${root}`);

function requireFile(relativePath) {
  const file = path.join(root, relativePath);
  if (!existsSync(file) || !statSync(file).isFile()) {
    fail(`Missing file: ${relativePath}`);
  }
}

function requireDirectory(relativePath) {
  const dir = path.join(root, relativePath);
  if (!existsSync(dir) || !statSync(dir).isDirectory()) {
    fail(`Missing directory: ${relativePath}`);
  }
}

function requireExecutable(relativePath) {
  requireFile(relativePath);
  if (process.platform !== "win32") {
    try {
      accessSync(path.join(root, relativePath), constants.X_OK);
    } catch {
      fail(`File is not executable: ${relativePath}`);
    }
  }
}

function checkReleaseManifest(expectedPlatform) {
  const manifestPath = path.join(root, "release-manifest.json");
  let manifest;
  try {
    manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  } catch (error) {
    fail(`Invalid release-manifest.json: ${error.message}`);
  }

  if (manifest.product !== "codex-speak") {
    fail("release-manifest.json product must be codex-speak");
  }
  if (manifest.platform !== expectedPlatform) {
    fail(`release-manifest.json platform must be ${expectedPlatform}`);
  }
  if (typeof manifest.version !== "string" || manifest.version.length === 0) {
    fail("release-manifest.json version is missing");
  }
  if (typeof manifest.git?.commit !== "string" || !/^[0-9a-f]{40}$/.test(manifest.git.commit)) {
    fail("release-manifest.json git.commit must be a full 40-character SHA");
  }
  if (!Array.isArray(manifest.files) || manifest.files.length === 0) {
    fail("release-manifest.json files list is missing");
  }
  for (const file of manifest.files) {
    if (typeof file.path !== "string" || !file.path) {
      fail("release-manifest.json file entry is missing path");
    }
    if (!existsSync(path.join(root, file.path))) {
      fail(`release-manifest.json file does not exist: ${file.path}`);
    }
    if (!Number.isInteger(file.bytes) || file.bytes < 0) {
      fail(`release-manifest.json file has invalid byte size: ${file.path}`);
    }
    if (typeof file.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(file.sha256)) {
      fail(`release-manifest.json file has invalid sha256: ${file.path}`);
    }
  }
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
