#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";

const args = new Set(process.argv.slice(2));
const tag = valueAfter("--tag");
const allowDirty = args.has("--allow-dirty");
const offline = args.has("--offline");
const requireSigningEnv = args.has("--require-signing-env");
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
  const remoteMain = git(["ls-remote", "origin", "refs/heads/main"]).split(/\s+/)[0] ?? "";
  repo = parseGitHubRepo(git(["remote", "get-url", "origin"]));

  checkNodeRuntime();
  check(branch === "main", "local branch", branch || "(detached)");
  check(allowDirty || status.length === 0, "working tree clean", allowDirty && status ? "dirty allowed" : "clean");
  check(remoteMain === head, "origin/main matches HEAD", `${head.slice(0, 7)} / ${remoteMain.slice(0, 7)}`);
  check(Boolean(repo), "GitHub origin", repo ?? "could not parse origin remote");

  checkRequiredFiles();
  checkWorkflowRuntime();
  checkSigningEnv();

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
  --offline                Skip GitHub Actions API checks.
  --require-signing-env    Fail if signing environment variables are missing.
  --help                   Show this help.

Examples:
  node scripts/check-release-readiness.mjs
  node scripts/check-release-readiness.mjs --tag v0.1.0-rc.1
  node scripts/check-release-readiness.mjs --tag v0.1.0 --require-signing-env
`);
}

function checkNodeRuntime() {
  const [major] = process.versions.node.split(".").map(Number);
  check(major >= 18, "Node runtime", `v${process.versions.node}`);
}

function checkRequiredFiles() {
  const files = [
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    "scripts/package-macos-release.sh",
    "scripts/package-windows-release.ps1",
    "scripts/smoke-install-macos-release.sh",
    "scripts/smoke-install-windows-release.ps1",
    "scripts/check-release-package.mjs",
    "scripts/check-release-readiness.mjs",
    "scripts/build-pet-assets-from-spritesheet.swift",
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
    "scripts/check-release-package.mjs",
    "scripts/check-release-readiness.mjs",
    "scripts/verify-local.sh"
  ];
  for (const file of executableFiles) {
    const executable = existsSync(file) && (statSync(file).mode & 0o111) !== 0;
    check(executable, `executable bit ${file}`, executable ? "set" : "missing");
  }
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

async function checkCiRun() {
  const runs = await githubJson(`/repos/${repo}/actions/runs?per_page=50`);
  const run = runs.workflow_runs?.find((item) => item.name === "CI" && item.head_sha === head);
  check(run?.conclusion === "success", "latest HEAD CI", run ? `${run.status}/${run.conclusion} ${run.html_url}` : "not found");
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
    throw new Error(`GitHub API ${response.status} for ${apiPath}: ${json.message ?? body}`);
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
