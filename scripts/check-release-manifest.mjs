#!/usr/bin/env node
import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const help = args.includes("--help") || args.includes("-h");
const packageDirArg = args.find((arg) => !arg.startsWith("--")) ?? ".";

if (help) {
  printUsage();
  process.exit(0);
}

const root = path.resolve(packageDirArg);
const manifestPath = path.join(root, "release-manifest.json");
const results = [];

if (!existsSync(root) || !statSync(root).isDirectory()) {
  failAndExit(`Package directory does not exist: ${root}`);
}

const manifest = readManifest(manifestPath);
check(manifest.schemaVersion === 1, "schemaVersion", manifest.schemaVersion ?? "missing");
check(manifest.product === "codex-speak", "product", manifest.product ?? "missing");
check(["macos", "windows"].includes(manifest.platform), "platform", manifest.platform ?? "missing");
check(typeof manifest.version === "string" && manifest.version.length > 0, "version", manifest.version ?? "missing");
check(/^[0-9a-f]{40}$/.test(manifest.git?.commit ?? ""), "git commit", manifest.git?.commit ?? "missing");
check(Array.isArray(manifest.files) && manifest.files.length > 0, "file entries", Array.isArray(manifest.files) ? `${manifest.files.length} files` : "missing");
checkVerificationCommands(manifest);

if (Array.isArray(manifest.files)) {
  for (const file of manifest.files) {
    checkFileEntry(file);
  }
}

printResults();
if (results.some((item) => item.status === "FAIL")) {
  process.exit(1);
}

function readManifest(file) {
  if (!existsSync(file) || !statSync(file).isFile()) {
    failAndExit(`release-manifest.json not found in ${root}`);
  }
  try {
    return JSON.parse(readFileSync(file, "utf8"));
  } catch (error) {
    failAndExit(`Could not parse release-manifest.json: ${error.message}`);
  }
}

function checkFileEntry(file) {
  if (!file || typeof file !== "object") {
    fail("file entry", "entry is not an object");
    return;
  }
  if (typeof file.path !== "string" || !file.path) {
    fail("file path", "missing");
    return;
  }
  if (path.isAbsolute(file.path) || file.path.split(/[\\/]/).includes("..")) {
    fail(`file ${file.path}`, "path must stay inside the release package");
    return;
  }

  const absolutePath = path.join(root, file.path);
  if (!existsSync(absolutePath) || !statSync(absolutePath).isFile()) {
    fail(`file ${file.path}`, "missing");
    return;
  }

  const stats = statSync(absolutePath);
  check(stats.size === file.bytes, `file ${file.path} bytes`, `${stats.size}/${file.bytes}`);
  check(sha256(absolutePath) === file.sha256, `file ${file.path} sha256`, "matched");
}

function checkVerificationCommands(manifest) {
  const commands = manifest.verification ?? {};
  for (const key of ["install", "verifyInstall", "verifyCodex", "manualQa", "checkQaReport", "verifyManifest"]) {
    check(typeof commands[key] === "string" && commands[key].length > 0, `verification.${key}`, commands[key] ?? "missing");
  }
}

function sha256(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

function ok(label, detail) {
  results.push({ status: "OK", label, detail });
}

function fail(label, detail) {
  results.push({ status: "FAIL", label, detail });
}

function check(condition, label, detail) {
  if (condition) {
    ok(label, detail);
  } else {
    fail(label, detail);
  }
}

function printResults() {
  for (const item of results) {
    console.log(`${item.status.padEnd(4)} ${item.label}: ${item.detail}`);
  }
}

function printUsage() {
  console.log(`Usage: node scripts/check-release-manifest.mjs [package-dir]

Validates release-manifest.json and all file sha256 entries in an unpacked Codex Speak release package.

Examples:
  node scripts/check-release-manifest.mjs .
  node scripts/check-release-manifest.mjs /tmp/codex-speak-macos
`);
}

function failAndExit(message) {
  console.error(message);
  process.exit(1);
}
