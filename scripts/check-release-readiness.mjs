#!/usr/bin/env node
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";

const args = new Set(process.argv.slice(2));
const tag = valueAfter("--tag");
const macosQaDir = valueAfter("--macos-qa-dir");
const windowsQaDir = valueAfter("--windows-qa-dir");
const allowDirty = args.has("--allow-dirty");
const offline = args.has("--offline");
const requireSigningEnv = args.has("--require-signing-env");
const requireManualQa = args.has("--require-manual-qa");
const allowNonInteractiveQa = args.has("--allow-non-interactive-qa");
const help = args.has("--help") || args.has("-h");

const results = [];
let repo;
let head;

if (help) {
  printUsage();
  process.exit(0);
}

main().catch((error) => {
  fail("release readiness script", error.message);
  printResults();
  process.exit(1);
});

async function main() {
  const root = git(["rev-parse", "--show-toplevel"]);
  process.chdir(root);
  head = git(["rev-parse", "HEAD"]);
  const branch = git(["branch", "--show-current"]);
  const status = git(["status", "--porcelain"]);
  repo = parseGitHubRepo(git(["remote", "get-url", "origin"]));

  checkNodeRuntime();
  check(branch === "main", "local branch", branch || "(detached)");
  check(allowDirty || status.length === 0, "working tree clean", allowDirty && status ? "dirty allowed" : "clean");
  checkRemoteMain();
  check(Boolean(repo), "GitHub origin", repo ?? "could not parse origin remote");

  checkRequiredFiles();
  checkLocalReleaseArtifactsIfPresent();
  checkManualQaMixedEnglishCoverage();
  checkWorkflowRuntime();
  checkStableReleaseSigningPolicy();
  checkCiSmokeArtifactPolicy();
  checkSigningEnv();
  checkManualQaEvidence();

  if (!offline && repo) {
    await checkCiRun();
    if (tag) {
      await checkReleaseRun(tag);
    }
  } else {
    warn("GitHub Actions status", offline ? "skipped by --offline" : "skipped: no GitHub origin");
  }

  printResults();
  if (results.some((item) => item.status === "FAIL")) {
    process.exit(1);
  }
}

function valueAfter(flag) {
  const index = process.argv.indexOf(flag);
  if (index === -1) return null;
  return process.argv[index + 1] ?? null;
}

function printUsage() {
  console.log(`Usage: node scripts/check-release-readiness.mjs [options]

Options:
  --tag <name>             Also verify the release workflow for a pushed tag.
  --allow-dirty            Allow local uncommitted changes.
  --offline                Skip GitHub remote and Actions API checks.
  --require-signing-env    Fail if signing environment variables are missing.
  --require-manual-qa      Fail unless both macOS and Windows QA evidence directories are supplied and pass.
  --macos-qa-dir <dir>     Validate a macOS manual QA output directory or qa-report.json.
  --windows-qa-dir <dir>   Validate a Windows manual QA output directory or qa-report.json.
  --allow-non-interactive-qa
                           Accept non-interactive QA reports, for CI smoke only.
  --help                   Show this help.

Examples:
  node scripts/check-release-readiness.mjs
  node scripts/check-release-readiness.mjs --tag v0.1.0-rc.1
  node scripts/check-release-readiness.mjs --tag v0.1.0 --require-signing-env
  node scripts/check-release-readiness.mjs --tag v0.1.0 --require-signing-env --require-manual-qa --macos-qa-dir /path/to/macos-qa --windows-qa-dir /path/to/windows-qa
`);
}

function checkNodeRuntime() {
  const [major] = process.versions.node.split(".").map(Number);
  check(major >= 18, "Node runtime", `v${process.versions.node}`);
}

function checkRemoteMain() {
  if (offline) {
    warn("origin/main matches HEAD", "skipped by --offline");
    return;
  }
  const remoteMain = git(["ls-remote", "origin", "refs/heads/main"]).split(/\s+/)[0] ?? "";
  check(remoteMain === head, "origin/main matches HEAD", `${head.slice(0, 7)} / ${remoteMain.slice(0, 7)}`);
}

function checkRequiredFiles() {
  const files = [
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    "scripts/package-macos-release.sh",
    "scripts/package-windows-release.ps1",
    "scripts/manual-qa-macos.sh",
    "scripts/manual-qa-windows.ps1",
    "scripts/smoke-install-macos-release.sh",
    "scripts/smoke-install-windows-release.ps1",
    "scripts/check-release-package.mjs",
    "scripts/check-manual-qa-report.mjs",
    "scripts/check-release-manifest.mjs",
    "scripts/write-release-manifest.mjs",
    "scripts/prepare-qa-handoff.mjs",
    "scripts/check-release-readiness.mjs",
    "scripts/build-pet-assets-from-spritesheet.swift",
    "tests/fixtures/ci-artifacts.json",
    "plugins/codex-speak/.codex-plugin/plugin.json",
    "plugins/codex-speak/.mcp.json",
    "plugins/codex-speak/skills/codex-speak/SKILL.md",
    "plugins/codex-speak/scripts/codex-speak-mcp",
    "plugins/codex-speak/scripts/codex-speak-mcp.ps1",
    "docs/installation.md",
    "docs/release-qa.md",
    "docs/signing.md",
    "docs/product-completion-plan.md",
    "apps/codex-speak-pet-macos/assets/codex-agent.mov",
    "apps/codex-speak-pet-macos/assets/codex-agent-source-spritesheet.png",
    "apps/codex-speak-pet-macos/assets/codex-agent-preview.png",
    "apps/codex-speak-control/src-tauri/icons/icon.ico"
  ];
  for (const file of files) {
    check(existsSync(file) && statSync(file).isFile(), `required file ${file}`, existsSync(file) ? "present" : "missing");
  }

  const executableFiles = [
    "scripts/package-macos-release.sh",
    "scripts/smoke-install-macos-release.sh",
    "scripts/manual-qa-macos.sh",
    "scripts/check-release-package.mjs",
    "scripts/check-manual-qa-report.mjs",
    "scripts/check-release-manifest.mjs",
    "scripts/write-release-manifest.mjs",
    "scripts/prepare-qa-handoff.mjs",
    "scripts/check-release-readiness.mjs",
    "scripts/verify-local.sh"
  ];
  for (const file of executableFiles) {
    const executable = existsSync(file) && (statSync(file).mode & 0o111) !== 0;
    check(executable, `executable bit ${file}`, executable ? "set" : "missing");
  }
}

function checkLocalReleaseArtifactsIfPresent() {
  const packages = [
    {
      platform: "macos",
      archive: "dist/codex-speak-macos.tar.gz",
      checksumFile: "dist/codex-speak-macos.tar.gz.sha256",
      manifestPath: "dist/codex-speak-macos/release-manifest.json"
    },
    {
      platform: "windows",
      archive: "dist/codex-speak-windows.zip",
      checksumFile: "dist/codex-speak-windows.zip.sha256",
      manifestPath: "dist/codex-speak-windows/release-manifest.json"
    }
  ];

  for (const item of packages) {
    const hasAnyArtifact = [item.archive, item.checksumFile, item.manifestPath].some((file) =>
      existsSync(file)
    );
    if (!hasAnyArtifact) {
      continue;
    }

    check(
      existsSync(item.manifestPath) && statSync(item.manifestPath).isFile(),
      `local ${item.platform} package manifest`,
      existsSync(item.manifestPath) ? "present" : "missing"
    );
    if (existsSync(item.manifestPath) && statSync(item.manifestPath).isFile()) {
      const manifest = readJsonFile(item.manifestPath);
      check(
        manifest.git?.commit === head,
        `local ${item.platform} package commit`,
        manifest.git?.commit ? `${manifest.git.commit.slice(0, 7)} / ${head.slice(0, 7)}` : "missing"
      );
      check(
        manifest.git?.dirty === false,
        `local ${item.platform} package clean source`,
        manifest.git?.dirty === false ? "clean" : "dirty or missing"
      );
      check(
        manifest.platform === item.platform,
        `local ${item.platform} package platform`,
        manifest.platform ?? "missing"
      );
    }

    if (existsSync(item.archive) || existsSync(item.checksumFile)) {
      check(
        existsSync(item.archive) && statSync(item.archive).isFile(),
        `local ${item.platform} archive`,
        existsSync(item.archive) ? "present" : "missing"
      );
      check(
        existsSync(item.checksumFile) && statSync(item.checksumFile).isFile(),
        `local ${item.platform} archive checksum`,
        existsSync(item.checksumFile) ? "present" : "missing"
      );
      if (existsSync(item.archive) && existsSync(item.checksumFile)) {
        const actual = sha256File(item.archive);
        const expected = readChecksumFile(item.checksumFile);
        check(
          actual === expected,
          `local ${item.platform} archive checksum matches`,
          actual === expected ? actual : `${expected} / ${actual}`
        );
      }
    }
  }

  const handoffPath = "dist/qa-handoff/qa-handoff.json";
  if (existsSync(handoffPath)) {
    const handoff = readJsonFile(handoffPath);
    check(
      handoff.git?.commit === head,
      "local QA handoff commit",
      handoff.git?.commit ? `${handoff.git.commit.slice(0, 7)} / ${head.slice(0, 7)}` : "missing"
    );
    check(
      handoff.git?.dirty === false,
      "local QA handoff clean source",
      handoff.git?.dirty === false ? "clean" : "dirty or missing"
    );
    const coverage = handoff.platformCoverage;
    if (coverage && Array.isArray(coverage.missing)) {
      if (coverage.complete === true) {
        ok("local QA handoff platform coverage", "macOS and Windows included");
      } else {
        warn(
          "local QA handoff platform coverage",
          `partial handoff; missing: ${coverage.missing.join(", ") || "none"}`
        );
      }
    } else {
      warn("local QA handoff platform coverage", "missing platformCoverage; regenerate QA handoff");
    }
  }
}

function checkManualQaMixedEnglishCoverage() {
  const requiredTerms = [
    "hello world",
    "OpenRouter",
    "OAuth",
    "M C P",
    "J.S.O.N",
    "build failed because timeout",
    "ProjectAlpha42",
    "你好世界示例",
    "开放路由平台",
    "授权登录协议",
    "插件通道",
    "数据格式",
    "英文短语",
    "英文编号"
  ];
  for (const file of ["scripts/manual-qa-macos.sh", "scripts/manual-qa-windows.ps1"]) {
    const content = readFileSync(file, "utf8");
    const missing = requiredTerms.filter((term) => !content.includes(term));
    check(
      missing.length === 0,
      `manual QA mixed English coverage ${file}`,
      missing.length === 0 ? "extended mixed-English sample covered" : `missing: ${missing.join(", ")}`
    );
  }
}

function readJsonFile(file) {
  try {
    return JSON.parse(readFileSync(file, "utf8"));
  } catch (error) {
    fail(`parse ${file}`, error.message);
    return {};
  }
}

function readChecksumFile(file) {
  const raw = readFileSync(file, "utf8").trim();
  const match = raw.match(/^([a-fA-F0-9]{64})(?:\s+|$)/);
  if (!match) {
    fail(`parse ${file}`, "checksum file must start with a sha256 hex digest");
    return "";
  }
  return match[1].toLowerCase();
}

function sha256File(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

function checkWorkflowRuntime() {
  const workflowFiles = [".github/workflows/ci.yml", ".github/workflows/release.yml"];
  const deprecatedPatterns = [
    /actions\/checkout@v4/,
    /actions\/setup-node@v4/,
    /node-version:\s*22\b/
  ];
  for (const file of workflowFiles) {
    const content = readFileSync(file, "utf8");
    const deprecated = deprecatedPatterns.find((pattern) => pattern.test(content));
    check(
      !deprecated,
      `workflow runtime ${file}`,
      deprecated ? `deprecated pattern ${deprecated.source}` : "Node 24 action runtime"
    );
  }
}

function checkStableReleaseSigningPolicy() {
  const content = readFileSync(".github/workflows/release.yml", "utf8");
  check(
    content.includes("Require macOS signing secrets for stable release"),
    "stable release macOS signing guard",
    content.includes("Require macOS signing secrets for stable release") ? "present" : "missing"
  );
  check(
    content.includes("Require Windows signing secrets for stable release"),
    "stable release Windows signing guard",
    content.includes("Require Windows signing secrets for stable release") ? "present" : "missing"
  );
  check(
    content.includes("!contains(github.ref_name, '-rc')"),
    "stable release RC signing exception",
    content.includes("!contains(github.ref_name, '-rc')") ? "RC tags may remain unsigned test builds" : "missing"
  );
}

function checkCiSmokeArtifactPolicy() {
  const content = readFileSync(".github/workflows/ci.yml", "utf8");
  check(
    content.includes("Upload smoke release package artifacts"),
    "CI smoke artifact upload",
    content.includes("Upload smoke release package artifacts") ? "present" : "missing"
  );
  check(
    content.includes("artifact_name: codex-speak-macos") && content.includes("artifact_name: codex-speak-windows"),
    "CI smoke artifact matrix names",
    content.includes("artifact_name: codex-speak-macos") && content.includes("artifact_name: codex-speak-windows")
      ? "macOS/Windows artifact names configured"
      : "missing"
  );
  check(
    content.includes("dist/${{ matrix.artifact_name }}.*"),
    "CI smoke artifact package glob",
    content.includes("dist/${{ matrix.artifact_name }}.*") ? "macOS/Windows package archives uploaded" : "missing"
  );
}

function checkSigningEnv() {
  const macSecrets = [
    "MACOS_CERTIFICATE_P12_BASE64",
    "MACOS_CERTIFICATE_PASSWORD",
    "MACOS_CODESIGN_IDENTITY",
    "MACOS_KEYCHAIN_PASSWORD",
    "APPLE_ID",
    "APPLE_TEAM_ID",
    "APPLE_APP_SPECIFIC_PASSWORD"
  ];
  const winSecrets = ["WINDOWS_SIGN_CERT_PFX_BASE64", "WINDOWS_SIGN_CERT_PASSWORD"];
  const missing = [...macSecrets, ...winSecrets].filter((name) => !process.env[name]);
  if (missing.length === 0) {
    ok("signing environment", "all signing variables are present in current environment");
  } else if (requireSigningEnv) {
    fail("signing environment", `missing: ${missing.join(", ")}`);
  } else {
    warn("signing environment", `not enforced here; missing in current environment: ${missing.join(", ")}`);
  }
}

function checkManualQaEvidence() {
  const evidence = [
    { platform: "macos", label: "macOS manual QA", dir: macosQaDir },
    { platform: "windows", label: "Windows manual QA", dir: windowsQaDir }
  ];

  for (const item of evidence) {
    if (!item.dir) {
      if (requireManualQa) {
        fail(item.label, "missing QA evidence directory");
      } else {
        warn(item.label, "not supplied; pass QA output with --macos-qa-dir or --windows-qa-dir");
      }
      continue;
    }
    validateManualQaEvidence(item.platform, item.label, item.dir);
  }
}

function validateManualQaEvidence(platform, label, inputPath) {
  const reportPath = resolveQaReportPath(inputPath);
  if (!reportPath) {
    fail(label, `qa-report.json not found at ${inputPath}`);
    return;
  }

  const reportDir = path.dirname(reportPath);
  const checkerArgs = ["scripts/check-manual-qa-report.mjs", reportPath];
  if (allowNonInteractiveQa) {
    checkerArgs.push("--allow-non-interactive");
  }
  try {
    execFileSync(process.execPath, checkerArgs, { encoding: "utf8" });
    ok(label, `${reportPath} passed check-manual-qa-report.mjs`);
  } catch (error) {
    const output = `${error.stdout ?? ""}${error.stderr ?? ""}`.trim();
    fail(label, output || error.message);
    return;
  }

  const manifestPath = path.join(reportDir, "support-bundle", "release-manifest.json");
  if (!existsSync(manifestPath) || !statSync(manifestPath).isFile()) {
    fail(`${label} release manifest`, "support-bundle/release-manifest.json missing");
    return;
  }

  let manifest;
  try {
    manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  } catch (error) {
    fail(`${label} release manifest`, `could not parse: ${error.message}`);
    return;
  }

  check(
    manifest.platform === platform,
    `${label} platform`,
    manifest.platform ?? "missing"
  );
  check(
    manifest.git?.commit === head,
    `${label} git commit`,
    manifest.git?.commit ? `${manifest.git.commit.slice(0, 7)} / ${head.slice(0, 7)}` : "missing"
  );
}

function resolveQaReportPath(inputPath) {
  const resolved = path.resolve(inputPath);
  if (!existsSync(resolved)) {
    return null;
  }
  if (statSync(resolved).isDirectory()) {
    const nested = path.join(resolved, "qa-report.json");
    return existsSync(nested) && statSync(nested).isFile() ? nested : null;
  }
  return statSync(resolved).isFile() ? resolved : null;
}

async function checkCiRun() {
  const runs = await githubJson(`/repos/${repo}/actions/runs?per_page=50`);
  const run = runs.workflow_runs?.find((item) => item.name === "CI" && item.head_sha === head);
  check(run?.conclusion === "success", "latest HEAD CI", run ? `${run.status}/${run.conclusion} ${run.html_url}` : "not found");
  if (run?.conclusion === "success") {
    await checkCiSmokeArtifacts(run.id);
  }
}

async function checkCiSmokeArtifacts(runId) {
  const artifacts = await githubJson(`/repos/${repo}/actions/runs/${runId}/artifacts?per_page=100`);
  const requiredArtifacts = ["codex-speak-macos-ci", "codex-speak-windows-ci"];
  for (const name of requiredArtifacts) {
    const artifact = artifacts.artifacts?.find((item) => item.name === name);
    check(
      artifact && artifact.expired === false && artifact.size_in_bytes > 0,
      `CI smoke artifact ${name}`,
      artifact
        ? `${artifact.expired ? "expired" : "active"} ${artifact.size_in_bytes} bytes`
        : "missing"
    );
  }
}

async function checkReleaseRun(tagName) {
  let localTag = "";
  try {
    localTag = git(["rev-parse", `refs/tags/${tagName}`]);
  } catch {
    localTag = "";
  }
  const tagSha = git(["ls-remote", "origin", `refs/tags/${tagName}`]).split(/\s+/)[0] ?? "";
  check(localTag.length > 0, `local tag ${tagName}`, localTag ? localTag.slice(0, 7) : "not found");
  check(tagSha.length > 0, `remote tag ${tagName}`, tagSha ? tagSha.slice(0, 7) : "not found");

  const runs = await githubJson(`/repos/${repo}/actions/runs?event=push&per_page=100`);
  const run = runs.workflow_runs?.find((item) =>
    item.name === "Release Build" && item.head_branch === tagName && (!tagSha || item.head_sha === tagSha)
  );
  check(run?.conclusion === "success", `release workflow ${tagName}`, run ? `${run.status}/${run.conclusion} ${run.html_url}` : "not found");
}

async function githubJson(apiPath) {
  const headers = {
    Accept: "application/vnd.github+json",
    "User-Agent": "codex-speak-release-readiness"
  };
  if (process.env.GITHUB_TOKEN) {
    headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  }
  const response = await fetch(`https://api.github.com${apiPath}`, { headers });
  const body = await response.text();
  let json;
  try {
    json = JSON.parse(body);
  } catch {
    throw new Error(`GitHub API returned non-JSON for ${apiPath}: ${body.slice(0, 120)}`);
  }
  if (!response.ok) {
    const tokenHint = response.status === 403
      ? " Set GITHUB_TOKEN to a GitHub token with Actions read access and retry."
      : "";
    throw new Error(`GitHub API ${response.status} for ${apiPath}: ${json.message ?? body}${tokenHint}`);
  }
  return json;
}

function parseGitHubRepo(remote) {
  const ssh = remote.match(/^git@github\.com:([^/]+\/[^/]+?)(?:\.git)?$/);
  if (ssh) return ssh[1];
  const https = remote.match(/^https:\/\/github\.com\/([^/]+\/[^/]+?)(?:\.git)?$/);
  if (https) return https[1];
  return null;
}

function git(argsList) {
  return execFileSync("git", argsList, { encoding: "utf8" }).trim();
}

function ok(label, detail) {
  results.push({ status: "OK", label, detail });
}

function warn(label, detail) {
  results.push({ status: "WARN", label, detail });
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
