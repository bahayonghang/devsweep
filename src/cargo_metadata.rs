use std::{
    collections::{BTreeSet, HashMap},
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{
    model::{ScanDiagnosticOutcome, ScanProcessOutput, ScanProcessProbe, ScanProcessStatus},
    process_runner::{
        CwdPolicy, DEFAULT_PROVIDER_PROBE_TIMEOUT, NoopCancelObserver, ProcessRequest,
        ProcessResult, ProcessRunner, ProcessStatus,
    },
};

/// Resolved cargo metadata fields used by scan and live revalidation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CargoMetadataScope {
    pub workspace_root: PathBuf,
    pub target_directory: PathBuf,
    /// Manifest paths explicitly named by Cargo as workspace members. Scanner
    /// cache entries may only reuse this scope for one of these manifests.
    pub member_manifests: Vec<PathBuf>,
}

/// Typed, display-safe metadata probe failure for scan diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CargoMetadataFailure {
    pub outcome: ScanDiagnosticOutcome,
    pub detail: String,
    pub process: Option<ScanProcessProbe>,
}

/// Result at the Cargo metadata probe boundary. This keeps scan diagnostics
/// structured while [`query_cargo_metadata`] remains the uncached execution
/// recheck wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CargoMetadataProbeResult {
    Resolved(CargoMetadataScope),
    Failed(CargoMetadataFailure),
}

/// Injectable Cargo metadata probe used by the scanner's scan-lifetime cache.
pub(crate) trait CargoMetadataProbe: Send + Sync {
    fn probe(&self, manifest_dir: &Path) -> CargoMetadataProbeResult;
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SystemCargoMetadataProbe {
    runner: ProcessRunner,
}

impl CargoMetadataProbe for SystemCargoMetadataProbe {
    fn probe(&self, manifest_dir: &Path) -> CargoMetadataProbeResult {
        probe_cargo_metadata(&self.runner, manifest_dir)
    }
}

pub(crate) fn query_cargo_metadata(
    runner: &ProcessRunner,
    manifest_dir: &Path,
) -> Result<CargoMetadataScope> {
    match probe_cargo_metadata(runner, manifest_dir) {
        CargoMetadataProbeResult::Resolved(scope) => Ok(scope),
        CargoMetadataProbeResult::Failed(failure) => Err(anyhow::anyhow!(failure.detail)),
    }
}

fn probe_cargo_metadata(runner: &ProcessRunner, manifest_dir: &Path) -> CargoMetadataProbeResult {
    let manifest = manifest_dir.join("Cargo.toml");
    let cancel = NoopCancelObserver;
    let request = ProcessRequest {
        program: OsString::from("cargo"),
        args: vec![
            OsString::from("metadata"),
            OsString::from("--format-version"),
            OsString::from("1"),
            OsString::from("--no-deps"),
            OsString::from("--manifest-path"),
            OsString::from(manifest.as_os_str()),
        ],
        cwd: CwdPolicy::Explicit {
            path: manifest_dir.to_path_buf(),
            reason: "cargo metadata for rust target scope".to_string(),
        },
        timeout: Some(DEFAULT_PROVIDER_PROBE_TIMEOUT),
        job_deadline: None,
        cancel: &cancel,
    };
    classify_cargo_metadata_result(runner.run(&request))
}

fn classify_cargo_metadata_result(result: ProcessResult) -> CargoMetadataProbeResult {
    let process = scan_process_probe(&result);
    if result.output.stdout_truncated || result.output.stderr_truncated {
        return CargoMetadataProbeResult::Failed(CargoMetadataFailure {
            outcome: ScanDiagnosticOutcome::OutputTruncated,
            detail: "cargo metadata output was truncated before JSON parsing".to_string(),
            process: Some(process),
        });
    }

    match result.status {
        ProcessStatus::Success => {
            let stdout = String::from_utf8_lossy(&result.output.stdout);
            match parse_cargo_metadata_json(&stdout) {
                Ok(scope) => CargoMetadataProbeResult::Resolved(scope),
                Err(error) => CargoMetadataProbeResult::Failed(CargoMetadataFailure {
                    outcome: ScanDiagnosticOutcome::Failed,
                    detail: format!("cargo metadata returned invalid JSON: {error}"),
                    process: Some(process),
                }),
            }
        }
        ProcessStatus::Canceled => CargoMetadataProbeResult::Failed(CargoMetadataFailure {
            outcome: ScanDiagnosticOutcome::Canceled,
            detail: "cargo metadata probe was canceled".to_string(),
            process: Some(process),
        }),
        status => CargoMetadataProbeResult::Failed(CargoMetadataFailure {
            outcome: ScanDiagnosticOutcome::Failed,
            detail: format!("cargo metadata failed with status {status:?}"),
            process: Some(process),
        }),
    }
}

fn scan_process_probe(result: &ProcessResult) -> ScanProcessProbe {
    ScanProcessProbe {
        status: scan_process_status(&result.status),
        stdout: ScanProcessOutput {
            truncated: result.output.stdout_truncated,
            retained_bytes: result.output.stdout.len() as u64,
            total_bytes: result.output.total_stdout_bytes,
        },
        stderr: ScanProcessOutput {
            truncated: result.output.stderr_truncated,
            retained_bytes: result.output.stderr.len() as u64,
            total_bytes: result.output.total_stderr_bytes,
        },
    }
}

fn scan_process_status(status: &ProcessStatus) -> ScanProcessStatus {
    match status {
        ProcessStatus::Success => ScanProcessStatus::Success,
        ProcessStatus::NotFound => ScanProcessStatus::NotFound,
        ProcessStatus::Timeout => ScanProcessStatus::Timeout,
        ProcessStatus::Exit { code } => ScanProcessStatus::Exit { code: *code },
        ProcessStatus::InvalidOutput => ScanProcessStatus::InvalidOutput,
        ProcessStatus::Canceled => ScanProcessStatus::Canceled,
    }
}

fn parse_cargo_metadata_json(stdout: &str) -> Result<CargoMetadataScope> {
    let value: serde_json::Value =
        serde_json::from_str(stdout).context("cargo metadata returned invalid JSON")?;
    let workspace_root = value
        .get("workspace_root")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow::anyhow!("cargo metadata missing workspace_root"))?;
    let target_directory = value
        .get("target_directory")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow::anyhow!("cargo metadata missing target_directory"))?;
    let mut manifests_by_package_id = HashMap::new();
    if let Some(packages) = value.get("packages").and_then(|value| value.as_array()) {
        for package in packages {
            let Some(package_id) = package.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(manifest_path) = package
                .get("manifest_path")
                .and_then(|value| value.as_str())
            else {
                continue;
            };
            manifests_by_package_id.insert(package_id, PathBuf::from(manifest_path));
        }
    }

    let member_manifests = value
        .get("workspace_members")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|member| member.as_str())
        .filter_map(|member| manifests_by_package_id.get(member))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(CargoMetadataScope {
        workspace_root: PathBuf::from(workspace_root),
        target_directory: PathBuf::from(target_directory),
        member_manifests,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_runner::ProcessOutput;

    #[test]
    fn cargo_metadata_parser_reads_workspace_and_target() {
        let json = r#"{
            "workspace_root": "C:/code/app",
            "target_directory": "C:/code/app/target"
        }"#;
        let scope = parse_cargo_metadata_json(json).expect("parse");
        assert_eq!(scope.workspace_root, PathBuf::from("C:/code/app"));
        assert_eq!(scope.target_directory, PathBuf::from("C:/code/app/target"));
        assert!(scope.member_manifests.is_empty());
    }

    #[test]
    fn cargo_metadata_parser_collects_only_explicit_workspace_member_manifests() {
        let json = r#"{
            "workspace_root": "C:/code/workspace",
            "target_directory": "C:/code/workspace/target",
            "workspace_members": ["path+file:///C:/code/workspace/member-a#0.1.0"],
            "packages": [
                {
                    "id": "path+file:///C:/code/workspace/member-a#0.1.0",
                    "manifest_path": "C:/code/workspace/member-a/Cargo.toml"
                },
                {
                    "id": "path+file:///C:/code/workspace/member-b#0.1.0",
                    "manifest_path": "C:/code/workspace/member-b/Cargo.toml"
                }
            ]
        }"#;

        let scope = parse_cargo_metadata_json(json).expect("parse");

        assert_eq!(
            scope.member_manifests,
            vec![PathBuf::from("C:/code/workspace/member-a/Cargo.toml")]
        );
    }

    #[test]
    fn truncated_cargo_metadata_is_classified_before_json_parsing() {
        let result = ProcessResult {
            status: ProcessStatus::Success,
            output: ProcessOutput {
                stdout: b"{".to_vec(),
                stdout_truncated: true,
                total_stdout_bytes: 2_048,
                ..ProcessOutput::default()
            },
        };

        let CargoMetadataProbeResult::Failed(failure) = classify_cargo_metadata_result(result)
        else {
            panic!("truncated output must not be parsed as successful metadata");
        };

        assert_eq!(failure.outcome, ScanDiagnosticOutcome::OutputTruncated);
        assert!(failure.detail.contains("truncated"));
        let process = failure.process.expect("process metadata is retained");
        assert!(process.stdout.truncated);
        assert_eq!(process.stdout.retained_bytes, 1);
        assert_eq!(process.stdout.total_bytes, 2_048);
    }
}
