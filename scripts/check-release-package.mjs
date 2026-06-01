#!/usr/bin/env node
import { accessSync, constants, existsSync, statSync } from "node:fs";
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
  "docs/installation.md",
  "docs/release-qa.md",
  "docs/requirements.md",
  "docs/technical-design.md",
  "docs/tauri-control-app.md"
];

for (const file of commonFiles) {
  requireFile(file);
}

if (platform === "macos") {
  requireExecutable("bin/codex-speak");
  requireExecutable("bin/codex-speak-pet-macos");
  requireDirectory("apps/Codex Speak.app");
  requireExecutable("installers/install-macos.sh");
  requireExecutable("installers/uninstall-macos.sh");
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

function fail(message) {
  console.error(message);
  process.exit(1);
}
