#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const reportArg = args.find((arg) => !arg.startsWith("--"));
const allowNonInteractive = args.includes("--allow-non-interactive");
const summaryJson = args.includes("--summary-json");
const help = args.includes("--help") || args.includes("-h");

const requiredAutoChecks = [
  "cli exists",
  "cli version",
  "release package manifest",
  "verify install",
  "doctor json",
  "doctor json parse",
  "status",
  "models list",
  "verify codex integration",
  "verify controls",
  "mixed english extract",
  "mixed english normalization",
  "support bundle",
  "support doctor.json",
  "support status.json",
  "support models.json",
  "support pronunciation-dictionary.json",
  "support support-bundle-metadata.json",
  "support release manifest",
  "app path"
];

const commonManualChecks = [
  "control app visible",
  "control settings adjustable",
  "pronunciation dictionary adjustable",
  "speech audible",
  "mixed english speech clear",
  "speech stopped"
];

if (help || !reportArg) {
  printUsage();
  process.exit(help ? 0 : 2);
}

const reportPath = resolveReportPath(reportArg);
const reportDir = path.dirname(reportPath);
const report = readJson(reportPath);
const results = [];
const supportDir = path.join(reportDir, "support-bundle");

check(typeof report.generatedAt === "string" && report.generatedAt.length > 0, "generatedAt", report.generatedAt ?? "missing");
check(typeof report.cliPath === "string" && report.cliPath.length > 0, "cliPath", report.cliPath ?? "missing");
check(report.machine && typeof report.machine === "object", "machine metadata", report.machine ? "present" : "missing");
check(Array.isArray(report.checks), "checks array", Array.isArray(report.checks) ? `${report.checks.length} checks` : "missing");

const checks = Array.isArray(report.checks) ? report.checks : [];
const byName = new Map(checks.map((item) => [item.name, item]));

for (const name of requiredAutoChecks) {
  const item = byName.get(name);
  check(item?.status === "pass", `required check ${name}`, item ? item.status : "missing");
}

const failedChecks = checks.filter((item) => item.status === "fail");
for (const item of failedChecks) {
  fail(`reported failure ${item.name}`, item.detail ?? "failed");
}

if (report.nonInteractive && !allowNonInteractive) {
  fail("interactive QA", "report was generated with nonInteractive=true");
} else if (report.nonInteractive && allowNonInteractive) {
  warn("interactive QA", "non-interactive report allowed for CI or smoke validation");
} else {
  const requiredManualChecks = expectedManualChecks(report);
  check(requiredManualChecks.length > 0, "manual checks expected", requiredManualChecks.join(", "));
  for (const name of requiredManualChecks) {
    const item = byName.get(name);
    check(item?.status === "pass", `manual check ${name}`, item ? item.status : "missing");
  }
}

for (const file of ["doctor.json", "status.json", "models.json", "pronunciation-dictionary.json", "support-bundle-metadata.json"]) {
  const supportPath = path.join(supportDir, file);
  check(existsSync(supportPath) && statSync(supportPath).isFile(), `support file ${file}`, existsSync(supportPath) ? "present" : "missing");
}
const manifestPath = path.join(supportDir, "release-manifest.json");
const missingManifestPath = path.join(supportDir, "release-manifest-missing.txt");
check(
  existsSync(manifestPath) || existsSync(missingManifestPath),
  "support file release manifest",
  existsSync(manifestPath) ? "present" : existsSync(missingManifestPath) ? "missing marker present" : "missing"
);
validateSupportBundlePrivacy(supportDir, report);

const summary = buildSummary(report, checks);
printResults(summary);
if (summaryJson) {
  console.log(JSON.stringify(summary, null, 2));
}
if (results.some((item) => item.status === "FAIL")) {
  process.exit(1);
}

function resolveReportPath(input) {
  const resolved = path.resolve(input);
  if (!existsSync(resolved)) {
    failAndExit(`QA report path does not exist: ${resolved}`);
  }
  if (statSync(resolved).isDirectory()) {
    return path.join(resolved, "qa-report.json");
  }
  return resolved;
}

function readJson(file) {
  if (!existsSync(file) || !statSync(file).isFile()) {
    failAndExit(`QA report file does not exist: ${file}`);
  }
  try {
    return JSON.parse(readFileSync(file, "utf8"));
  } catch (error) {
    failAndExit(`Could not parse QA report JSON: ${error.message}`);
  }
}

function validateSupportBundlePrivacy(dir, report) {
  if (!existsSync(dir) || !statSync(dir).isDirectory()) {
    fail("support privacy", "support-bundle directory missing");
    return;
  }

  const metadataPath = path.join(dir, "support-bundle-metadata.json");
  const metadata = existsSync(metadataPath) ? readJson(metadataPath) : null;
  if (metadata) {
    check(metadata.redacted === true, "support metadata redacted", metadata.redacted);
    check(metadata.includePrivate === false, "support metadata includePrivate", metadata.includePrivate);
    check(metadata.redaction?.localPaths === true, "support metadata local path redaction", metadata.redaction?.localPaths);
    check(metadata.redaction?.recentSpokenText === true, "support metadata spoken text redaction", metadata.redaction?.recentSpokenText);
  }

  const pronunciationPath = path.join(dir, "pronunciation-dictionary.json");
  if (existsSync(pronunciationPath)) {
    const pronunciation = readJson(pronunciationPath);
    check(pronunciation.redacted === true, "support privacy pronunciation redacted", pronunciation.redacted);
    check(
      typeof pronunciation.terms === "number",
      "support privacy pronunciation terms",
      typeof pronunciation.terms === "number" ? `${pronunciation.terms} term(s)` : "raw terms exposed"
    );
  }

  const files = listFiles(dir);
  const pathLeaks = [];
  const homePathPatterns = [
    /\/Users\/[^/\s:]+(?:\/[^\s:]+)*/g,
    /\/home\/[^/\s:]+(?:\/[^\s:]+)*/g,
    /[A-Z]:\\Users\\[^\\\s:]+(?:\\[^\s:]+)*/gi
  ];
  const username = typeof report.machine?.user === "string" ? report.machine.user.trim() : "";
  for (const file of files) {
    const text = readTextIfSmall(file);
    if (text === null) continue;
    for (const pattern of homePathPatterns) {
      for (const match of text.matchAll(pattern)) {
        pathLeaks.push(`${path.relative(dir, file)}: ${match[0]}`);
      }
    }
    if (username && username.length >= 3 && text.includes(username) && !safeUsernameOccurrence(username, text)) {
      pathLeaks.push(`${path.relative(dir, file)}: contains machine username ${username}`);
    }
  }
  check(pathLeaks.length === 0, "support privacy local paths", pathLeaks.length === 0 ? "none found" : pathLeaks.slice(0, 3).join("; "));

  const lastSpokenPath = path.join(dir, "logs", "last-spoken.txt");
  if (existsSync(lastSpokenPath)) {
    const text = readFileSync(lastSpokenPath, "utf8");
    check(
      text.includes("[redacted by codex-speak support-bundle]"),
      "support privacy last-spoken",
      text.includes("[redacted by codex-speak support-bundle]") ? "redacted" : "not redacted"
    );
  } else {
    ok("support privacy last-spoken", "no recent spoken text log present");
  }
}

function listFiles(dir) {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...listFiles(file));
    } else if (entry.isFile()) {
      out.push(file);
    }
  }
  return out;
}

function readTextIfSmall(file) {
  const stats = statSync(file);
  if (stats.size > 512 * 1024) return null;
  try {
    return readFileSync(file, "utf8");
  } catch {
    return null;
  }
}

function safeUsernameOccurrence(username, text) {
  return username === "runner" && !/\/Users\/runner|\\Users\\runner|\/home\/runner/i.test(text);
}

function expectedManualChecks(report) {
  const os = `${report.machine?.os ?? ""} ${report.machine?.osVersion ?? ""}`.toLowerCase();
  const checks = [...commonManualChecks];
  if (os.includes("mac") || os.includes("darwin")) {
    checks.splice(1, 0, "desktop pet transparent");
  }
  return checks;
}

function printUsage() {
  console.log(`Usage: node scripts/check-manual-qa-report.mjs <qa-output-dir|qa-report.json> [options]

Options:
  --allow-non-interactive   Accept reports generated by CI smoke mode.
  --summary-json            Print a machine-readable reviewer summary after checks.
  --help                    Show this help.

Examples:
  node scripts/check-manual-qa-report.mjs /tmp/codex-speak-macos-qa-20260601-120000
  node scripts/check-manual-qa-report.mjs ./qa-report.json --allow-non-interactive
  node scripts/check-manual-qa-report.mjs ./qa-report.json --summary-json
`);
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

function buildSummary(report, checks) {
  const requiredManualChecks = report.nonInteractive ? [] : expectedManualChecks(report);
  const autoCheckNames = new Set(requiredAutoChecks);
  const manualCheckNames = new Set(requiredManualChecks);
  const counts = {
    reportChecks: countStatuses(checks),
    reviewerChecks: countStatuses(results),
    auto: countStatuses(checks.filter((item) => autoCheckNames.has(item.name))),
    manual: countStatuses(checks.filter((item) => manualCheckNames.has(item.name)))
  };
  const failedReportChecks = checks
    .filter((item) => item.status === "fail")
    .map((item) => item.name);
  const skippedManualChecks = checks
    .filter((item) => item.status === "skip" && manualCheckNames.has(item.name))
    .map((item) => item.name);
  const failedReviewerChecks = results
    .filter((item) => item.status === "FAIL")
    .map((item) => item.label);
  const warningReviewerChecks = results
    .filter((item) => item.status === "WARN")
    .map((item) => item.label);

  return {
    ok: failedReviewerChecks.length === 0,
    report: reportPath,
    generatedAt: report.generatedAt ?? null,
    platform: report.machine?.os ?? null,
    nonInteractive: Boolean(report.nonInteractive),
    counts,
    failedReportChecks,
    skippedManualChecks,
    failedReviewerChecks,
    warningReviewerChecks,
    nextAction: nextAction({
      failedReviewerChecks,
      failedReportChecks,
      skippedManualChecks,
      nonInteractive: Boolean(report.nonInteractive)
    })
  };
}

function countStatuses(items) {
  return {
    total: items.length,
    pass: items.filter((item) => item.status === "pass").length,
    fail: items.filter((item) => item.status === "fail").length,
    skip: items.filter((item) => item.status === "skip").length,
    ok: items.filter((item) => item.status === "OK").length,
    warn: items.filter((item) => item.status === "WARN").length,
    reviewerFail: items.filter((item) => item.status === "FAIL").length
  };
}

function nextAction({ failedReviewerChecks, failedReportChecks, skippedManualChecks, nonInteractive }) {
  if (failedReviewerChecks.length > 0 || failedReportChecks.length > 0) {
    return "Fix failed checks or inspect the named stdout/stderr files in the QA output directory.";
  }
  if (nonInteractive) {
    return "This is a non-interactive smoke report; collect real macOS/Windows tester confirmations before release.";
  }
  if (skippedManualChecks.length > 0) {
    return "Ask the tester to rerun skipped manual checks or explicitly accept the skips before release.";
  }
  return "QA report is ready for release-readiness evidence.";
}

function printResults(summary) {
  for (const item of results) {
    console.log(`${item.status.padEnd(4)} ${item.label}: ${item.detail}`);
  }
  console.log(
    `SUMMARY ${summary.ok ? "PASS" : "FAIL"} ${summary.report}: ` +
      `report checks pass=${summary.counts.reportChecks.pass} fail=${summary.counts.reportChecks.fail} skip=${summary.counts.reportChecks.skip}; ` +
      `reviewer checks ok=${summary.counts.reviewerChecks.ok} warn=${summary.counts.reviewerChecks.warn} fail=${summary.counts.reviewerChecks.reviewerFail}`
  );
  if (summary.failedReportChecks.length > 0) {
    console.log(`SUMMARY failed report checks: ${summary.failedReportChecks.join(", ")}`);
  }
  if (summary.skippedManualChecks.length > 0) {
    console.log(`SUMMARY skipped manual checks: ${summary.skippedManualChecks.join(", ")}`);
  }
  if (summary.failedReviewerChecks.length > 0) {
    console.log(`SUMMARY failed reviewer checks: ${summary.failedReviewerChecks.join(", ")}`);
  }
  if (summary.warningReviewerChecks.length > 0) {
    console.log(`SUMMARY warnings: ${summary.warningReviewerChecks.join(", ")}`);
  }
  console.log(`SUMMARY next action: ${summary.nextAction}`);
}

function failAndExit(message) {
  console.error(message);
  process.exit(1);
}
