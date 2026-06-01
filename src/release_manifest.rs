use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Serialize)]
pub struct PackageVerificationReport {
    pub ok: bool,
    pub package_dir: String,
    pub checks: Vec<PackageVerificationCheck>,
}

#[derive(Debug, Serialize)]
pub struct PackageVerificationCheck {
    pub label: String,
    pub status: &'static str,
    pub detail: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseManifest {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    product: String,
    platform: String,
    version: String,
    git: GitInfo,
    files: Vec<ManifestFile>,
    verification: VerificationCommands,
}

#[derive(Debug, Deserialize)]
struct GitInfo {
    commit: String,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerificationCommands {
    install: String,
    verify_install: String,
    verify_codex: String,
    manual_qa: String,
    check_qa_report: String,
    verify_manifest: String,
}

pub fn run(package_dir: Option<&Path>, json: bool) -> Result<()> {
    let root = package_dir.unwrap_or_else(|| Path::new("."));
    let report = collect(root);
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text(&report);
    }
    if !report.ok {
        anyhow::bail!("release package verification failed");
    }
    Ok(())
}

fn collect(root: &Path) -> PackageVerificationReport {
    let root = root.to_path_buf();
    let mut checks = Vec::new();
    if !root.is_dir() {
        checks.push(PackageVerificationCheck::fail(
            "package directory",
            format!("missing {}", root.display()),
        ));
        return PackageVerificationReport::new(root, checks);
    }

    let manifest_path = root.join("release-manifest.json");
    let manifest = match read_manifest(&manifest_path) {
        Ok(manifest) => {
            checks.push(PackageVerificationCheck::ok(
                "release-manifest.json",
                manifest_path.display().to_string(),
            ));
            manifest
        }
        Err(err) => {
            checks.push(PackageVerificationCheck::fail(
                "release-manifest.json",
                format!("{err:#}"),
            ));
            return PackageVerificationReport::new(root, checks);
        }
    };

    validate_manifest_fields(&manifest, &mut checks);
    for file in &manifest.files {
        validate_manifest_file(&root, file, &mut checks);
    }

    PackageVerificationReport::new(root, checks)
}

fn read_manifest(path: &Path) -> Result<ReleaseManifest> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn validate_manifest_fields(
    manifest: &ReleaseManifest,
    checks: &mut Vec<PackageVerificationCheck>,
) {
    check(
        manifest.schema_version == 1,
        "schemaVersion",
        manifest.schema_version.to_string(),
        checks,
    );
    check(
        manifest.product == "codex-speak",
        "product",
        manifest.product.clone(),
        checks,
    );
    check(
        matches!(manifest.platform.as_str(), "macos" | "windows"),
        "platform",
        manifest.platform.clone(),
        checks,
    );
    check(
        !manifest.version.trim().is_empty(),
        "version",
        manifest.version.clone(),
        checks,
    );
    check(
        is_hex(&manifest.git.commit, 40),
        "git commit",
        manifest.git.commit.clone(),
        checks,
    );
    check(
        !manifest.files.is_empty(),
        "manifest files",
        format!("{} files", manifest.files.len()),
        checks,
    );
    for (label, command) in [
        ("verification.install", &manifest.verification.install),
        (
            "verification.verifyInstall",
            &manifest.verification.verify_install,
        ),
        (
            "verification.verifyCodex",
            &manifest.verification.verify_codex,
        ),
        ("verification.manualQa", &manifest.verification.manual_qa),
        (
            "verification.checkQaReport",
            &manifest.verification.check_qa_report,
        ),
        (
            "verification.verifyManifest",
            &manifest.verification.verify_manifest,
        ),
    ] {
        check(!command.trim().is_empty(), label, command.clone(), checks);
    }
}

fn validate_manifest_file(
    root: &Path,
    file: &ManifestFile,
    checks: &mut Vec<PackageVerificationCheck>,
) {
    let label = format!("file {}", file.path);
    if !is_safe_relative_path(&file.path) {
        checks.push(PackageVerificationCheck::fail(
            label,
            "path must stay inside the release package",
        ));
        return;
    }
    if !is_hex(&file.sha256, 64) {
        checks.push(PackageVerificationCheck::fail(label, "invalid sha256"));
        return;
    }

    let path = root.join(&file.path);
    let Ok(metadata) = fs::metadata(&path) else {
        checks.push(PackageVerificationCheck::fail(label, "missing"));
        return;
    };
    if !metadata.is_file() {
        checks.push(PackageVerificationCheck::fail(label, "not a file"));
        return;
    }

    let actual_bytes = metadata.len();
    if actual_bytes != file.bytes {
        checks.push(PackageVerificationCheck::fail(
            format!("{label} bytes"),
            format!("{actual_bytes}/{}", file.bytes),
        ));
        return;
    }

    match sha256_file(&path) {
        Ok(actual_sha) if actual_sha == file.sha256 => {
            checks.push(PackageVerificationCheck::ok(label, "sha256 matched"));
        }
        Ok(actual_sha) => {
            checks.push(PackageVerificationCheck::fail(
                format!("{label} sha256"),
                format!("{actual_sha}/{}", file.sha256),
            ));
        }
        Err(err) => {
            checks.push(PackageVerificationCheck::fail(label, format!("{err:#}")));
        }
    }
}

fn check(
    condition: bool,
    label: impl Into<String>,
    detail: impl Into<String>,
    checks: &mut Vec<PackageVerificationCheck>,
) {
    let label = label.into();
    let detail = detail.into();
    if condition {
        checks.push(PackageVerificationCheck::ok(label, detail));
    } else {
        checks.push(PackageVerificationCheck::fail(label, detail));
    }
}

fn is_safe_relative_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn is_hex(value: &str, len: usize) -> bool {
    value.len() == len && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn print_text(report: &PackageVerificationReport) {
    println!("Codex Speak release package verification");
    println!("Package: {}", report.package_dir);
    for check in &report.checks {
        let prefix = if check.status == "ok" { "OK  " } else { "FAIL" };
        println!("{prefix} {}: {}", check.label, check.detail);
    }
}

impl PackageVerificationReport {
    fn new(root: PathBuf, checks: Vec<PackageVerificationCheck>) -> Self {
        Self {
            ok: checks.iter().all(|check| check.status == "ok"),
            package_dir: root.display().to_string(),
            checks,
        }
    }
}

impl PackageVerificationCheck {
    fn ok(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: "ok",
            detail: detail.into(),
        }
    }

    fn fail(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: "fail",
            detail: detail.into(),
        }
    }
}
