#!/usr/bin/env node
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";

const [platform, packageDir] = process.argv.slice(2);

if (!platform || !packageDir) {
  fail("Usage: write-release-manifest.mjs <macos|windows> <package-dir>");
}

const root = path.resolve(packageDir);
if (!existsSync(root) || !statSync(root).isDirectory()) {
  fail(`Package directory does not exist: ${root}`);
}

const version = readCargoVersion();
const gitCommit = git(["rev-parse", "HEAD"]);
const gitDescribe = git(["describe", "--tags", "--always", "--dirty"], true);
const gitDirty = git(["status", "--porcelain"]).length > 0;

const manifest = {
  schemaVersion: 1,
  product: "codex-speak",
  version,
  platform,
  generatedAt: new Date().toISOString(),
  git: {
    commit: gitCommit,
    describe: gitDescribe || gitCommit.slice(0, 7),
    dirty: gitDirty
  },
  package: {
    name: platform === "macos" ? "codex-speak-macos" : "codex-speak-windows",
    format: platform === "macos" ? "tar.gz" : "zip"
  },
  files: manifestFiles(platform).map((file) => describeFile(file)),
  verification: verificationCommands(platform)
};

writeFileSync(
  path.join(root, "release-manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`
);

console.log(`Wrote release manifest: ${path.join(root, "release-manifest.json")}`);

function manifestFiles(targetPlatform) {
  if (targetPlatform === "macos") {
    return [
      "bin/codex-speak",
      "bin/codex-speak-pet-macos",
      "assets/pet/codex-agent.mov",
      "assets/pet/codex-agent-source-spritesheet.png",
      "scripts/manual-qa-macos.sh",
      "scripts/check-manual-qa-report.mjs",
      "scripts/check-release-manifest.mjs"
    ];
  }
  if (targetPlatform === "windows") {
    return [
      "bin/codex-speak.exe",
      "apps/codex-speak-control.exe",
      "scripts/manual-qa-windows.ps1",
      "scripts/check-manual-qa-report.mjs",
      "scripts/check-release-manifest.mjs"
    ];
  }
  fail(`Unsupported platform: ${targetPlatform}`);
}

function describeFile(relativePath) {
  const file = path.join(root, relativePath);
  if (!existsSync(file) || !statSync(file).isFile()) {
    fail(`Manifest target file is missing: ${relativePath}`);
  }
  return {
    path: relativePath,
    bytes: statSync(file).size,
    sha256: sha256(file)
  };
}

function verificationCommands(targetPlatform) {
  if (targetPlatform === "macos") {
    return {
      install: "./installers/install-macos.sh --skip-tts-download",
      verifyInstall: "~/.codex/codex-speak/bin/codex-speak verify-install --allow-missing-models",
      verifyCodex: "~/.codex/codex-speak/bin/codex-speak verify-codex",
      manualQa: "./scripts/manual-qa-macos.sh --allow-missing-models",
      checkQaReport: "node scripts/check-manual-qa-report.mjs <qa-output-dir>",
      verifyManifest: "./bin/codex-speak verify-package --package-dir ."
    };
  }
  if (targetPlatform === "windows") {
    return {
      install: ".\\installers\\install-windows.ps1 -SkipTtsDownload",
      verifyInstall: "%USERPROFILE%\\.codex\\codex-speak\\bin\\codex-speak.exe verify-install --allow-missing-models",
      verifyCodex: "%USERPROFILE%\\.codex\\codex-speak\\bin\\codex-speak.exe verify-codex",
      manualQa: ".\\scripts\\manual-qa-windows.ps1 -AllowMissingModels",
      checkQaReport: "node .\\scripts\\check-manual-qa-report.mjs <qa-output-dir>",
      verifyManifest: ".\\bin\\codex-speak.exe verify-package --package-dir ."
    };
  }
  fail(`Unsupported platform: ${targetPlatform}`);
}

function readCargoVersion() {
  const raw = readFileSync("Cargo.toml", "utf8");
  const match = raw.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    fail("Could not read package version from Cargo.toml");
  }
  return match[1];
}

function sha256(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

function git(args, optional = false) {
  try {
    return execFileSync("git", args, { encoding: "utf8" }).trim();
  } catch (error) {
    if (optional) {
      return "";
    }
    throw error;
  }
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
