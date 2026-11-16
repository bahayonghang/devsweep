//! `winget upgrade` probe and one-package update execution.
//!
//! The check probe is read-only. Plan, preview, and execute keep program and
//! argv separate. They never accept source or package agreements. Refusals
//! return before the upgrade process starts.

use std::{
    ffi::{OsStr, OsString},
    fmt,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::digest_encoding::hex_digest;

use super::audit::{
    SoftwareSupportJournal, SoftwareSupportLifecycleRecordV1, SoftwareSupportOperation,
    SoftwareSupportTerminalResult, SupportAppendFault,
};
use super::file_authority::{BoundFile, digest as authority_digest};
use super::{SoftwareInventoryV1, unix_ms};
use crate::process::{
    CancelObserver, CwdPolicy, DEFAULT_EXECUTOR_COMMAND_TIMEOUT, DEFAULT_PROVIDER_PHASE_DEADLINE,
    FlagCancelObserver, NoopCancelObserver, ProcessRequest, ProcessResult, ProcessRunner,
    ProcessStatus,
};

/// Software update-check wire version.
pub const SOFTWARE_UPDATES_VERSION: u32 = 1;

const WINGET_PROGRAM: &str = "winget";
const WINGET_UPGRADE_ARGS: [&str; 6] = [
    "upgrade",
    "--disable-interactivity",
    "--scope",
    "user",
    "--source",
    "winget",
];
const TABLE_COLUMNS: usize = 5;
const TRUNCATION_MARK: char = '\u{2026}';

/// Exact winget messages for an empty upgrade table (English and zh-CN).
const NO_UPDATE_MESSAGES: &[&str] = &[
    "No installed package found matching input criteria.",
    "No available upgrade found.",
    "No applicable upgrade found.",
    "找不到与输入条件匹配的已安装程序包。",
    "找不到可用的升级。",
    "找不到适用的升级。",
];

/// Locale-neutral update-check result. Unknown output is never an empty success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoftwareUpdatesV1 {
    Available {
        version: u32,
        observed_at_unix_ms: u64,
        rows: Vec<SoftwareUpdateRowV1>,
    },
    Unavailable {
        version: u32,
        observed_at_unix_ms: u64,
        reason_code: SoftwareUpdatesReason,
    },
}

/// One parsed `winget upgrade` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareUpdateRowV1 {
    pub id: String,
    pub name: String,
    /// True when winget shortened the name with a trailing ellipsis.
    pub name_truncated: bool,
    pub installed_version: String,
    pub available_version: String,
    pub source: String,
    /// Inventory entries whose display name equals `name` (case-insensitive).
    pub matched_software_ids: Vec<String>,
}

/// Stable reason for an unavailable update check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareUpdatesReason {
    WingetMissing,
    ProviderUnsupported,
    SourceAgreementPending,
    NonZeroExit,
    TimedOut,
    Canceled,
    OutputTruncated,
    UnrecognizedOutput,
    ProcessFailed,
}

/// Update plan wire version.
pub const SOFTWARE_UPDATE_PLAN_VERSION: u32 = 1;

const SUPPORTED_UPDATE_SOURCE: &str = "winget";
const UPDATE_SCOPE: &str = "user";

/// One explicit package update. It stores no installer argv.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareUpdatePlanV1 {
    pub version: u32,
    pub package_id: String,
    pub source: String,
    pub installed_version: String,
    pub available_version: String,
    pub provider_fingerprint: String,
}

/// Live preview of one update plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareUpdatePreviewV1 {
    pub version: u32,
    pub package_id: String,
    pub source: String,
    pub installed_version: String,
    pub available_version: String,
    pub digest: String,
}

/// Provider result. `updated` is true only for [`SoftwareUpdateOutcome::Updated`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareUpdateOutcome {
    Updated,
    RestartRequired,
    Failed,
    Unknown,
}

/// Result of one update execute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareUpdateReportV1 {
    pub version: u32,
    pub package_id: String,
    pub source: String,
    pub available_version: String,
    pub outcome: SoftwareUpdateOutcome,
    pub updated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

/// Failures before the upgrade process starts, plus a failed upgrade.
#[derive(Debug)]
pub enum SoftwareUpdateError {
    ConfirmationRequired,
    CanceledBeforeStart,
    UnsupportedPlanVersion(u32),
    InvalidPlan,
    AmbiguousPackage,
    UnsupportedSource,
    NetworkUnavailable,
    VersionChanged,
    PackageNotFound,
    StaleConfirmation,
    ProviderChanged,
    ProbeFailed,
    ProviderMissing,
    UnsupportedProvider,
    AuditUnavailable,
}

impl fmt::Display for SoftwareUpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfirmationRequired => {
                formatter.write_str("package update requires explicit confirmation")
            }
            Self::CanceledBeforeStart => {
                formatter.write_str("package update was canceled before dispatch")
            }
            Self::UnsupportedPlanVersion(version) => {
                write!(formatter, "unsupported update plan version {version}")
            }
            Self::InvalidPlan => formatter.write_str("update plan is invalid"),
            Self::AmbiguousPackage => formatter.write_str("package id matches more than one row"),
            Self::UnsupportedSource => formatter.write_str("update source is not supported"),
            Self::NetworkUnavailable => {
                formatter.write_str("update provider network is unavailable")
            }
            Self::VersionChanged => formatter.write_str("offered package version changed"),
            Self::PackageNotFound => formatter.write_str("package id is not in the update table"),
            Self::StaleConfirmation => formatter.write_str("update preview digest is stale"),
            Self::ProviderChanged => formatter.write_str("update provider fingerprint changed"),
            Self::ProbeFailed => formatter.write_str("update provider probe failed"),
            Self::ProviderMissing => {
                formatter.write_str("update provider is not installed or cannot be resolved")
            }
            Self::UnsupportedProvider => formatter.write_str(
                "update provider version, capabilities, or native file layout is unsupported",
            ),
            Self::AuditUnavailable => {
                formatter.write_str("update audit is unavailable; no update was dispatched")
            }
        }
    }
}

impl std::error::Error for SoftwareUpdateError {}

impl SoftwareUpdateError {
    /// Stable machine code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::ConfirmationRequired => "confirmation_required",
            Self::CanceledBeforeStart => "canceled_before_start",
            Self::UnsupportedPlanVersion(_) => "unsupported_plan_version",
            Self::InvalidPlan => "invalid_plan",
            Self::AmbiguousPackage => "ambiguous_package",
            Self::UnsupportedSource => "unsupported_source",
            Self::NetworkUnavailable => "network_unavailable",
            Self::VersionChanged => "version_changed",
            Self::PackageNotFound => "package_not_found",
            Self::StaleConfirmation => "stale_confirmation",
            Self::ProviderChanged => "provider_changed",
            Self::ProbeFailed => "probe_failed",
            Self::ProviderMissing => "provider_missing",
            Self::UnsupportedProvider => "unsupported_provider",
            Self::AuditUnavailable => "audit_unavailable",
        }
    }
}

/// Selects one package from a read-only provider probe. It does not upgrade.
pub fn plan_software_update(
    program: &OsStr,
    package_id: &str,
    source: &str,
    available_version: &str,
) -> Result<SoftwareUpdatePlanV1, SoftwareUpdateError> {
    let runner = ProcessRunner::default();
    validate_update_selector(package_id, source, available_version)?;
    let mut run = |request: &ProcessRequest<'_>| runner.run(request);
    let binding = bind_update_provider(program, &mut run)?;
    let row = select_update_row(
        &mut run,
        binding.file.path.as_os_str(),
        package_id,
        source,
        available_version,
        None,
    )?;
    Ok(SoftwareUpdatePlanV1 {
        version: SOFTWARE_UPDATE_PLAN_VERSION,
        package_id: row.id,
        source: row.source,
        installed_version: row.installed_version,
        available_version: row.available_version,
        provider_fingerprint: binding.fingerprint,
    })
}

/// Re-runs the read-only probe and binds the live row to `plan`.
pub fn preview_software_update(
    program: &OsStr,
    plan: &SoftwareUpdatePlanV1,
) -> Result<SoftwareUpdatePreviewV1, SoftwareUpdateError> {
    let runner = ProcessRunner::default();
    validate_update_plan(plan)?;
    let mut run = |request: &ProcessRequest<'_>| runner.run(request);
    let binding = bind_update_provider(program, &mut run)?;
    let row = bound_update_row(&mut run, &binding, plan)?;
    Ok(SoftwareUpdatePreviewV1 {
        version: SOFTWARE_UPDATE_PLAN_VERSION,
        digest: update_digest(plan, &row)?,
        package_id: row.id,
        source: row.source,
        installed_version: row.installed_version,
        available_version: row.available_version,
    })
}

/// Upgrades one planned package. Refusals return before the upgrade process.
pub fn execute_software_update(
    program: &OsStr,
    plan: &SoftwareUpdatePlanV1,
    expected_preview_digest: &str,
    confirmed: bool,
) -> Result<SoftwareUpdateReportV1, SoftwareUpdateError> {
    execute_software_update_cancellable(
        program,
        plan,
        expected_preview_digest,
        confirmed,
        &NoopCancelObserver,
    )
}

/// Cancels read-only preflight and refuses before targeted dispatch. Once the
/// target process starts, this observer cannot terminate it or hide its result.
pub fn execute_software_update_cancellable(
    program: &OsStr,
    plan: &SoftwareUpdatePlanV1,
    expected_preview_digest: &str,
    confirmed: bool,
    cancel: &dyn CancelObserver,
) -> Result<SoftwareUpdateReportV1, SoftwareUpdateError> {
    if !confirmed {
        return Err(SoftwareUpdateError::ConfirmationRequired);
    }
    if !is_preview_digest(expected_preview_digest) {
        return Err(SoftwareUpdateError::InvalidPlan);
    }
    let runner = ProcessRunner::default();
    let audit_path =
        super::software_audit_v1_path().map_err(|_| SoftwareUpdateError::AuditUnavailable)?;
    execute_software_update_with(
        |request| runner.run(request),
        program,
        plan,
        expected_preview_digest,
        &audit_path,
        SupportAppendFault::NONE,
        cancel,
    )
}

fn execute_software_update_with(
    mut run: impl FnMut(&ProcessRequest<'_>) -> ProcessResult,
    program: &OsStr,
    plan: &SoftwareUpdatePlanV1,
    expected_preview_digest: &str,
    audit_path: &Path,
    fault: SupportAppendFault,
    cancel: &dyn CancelObserver,
) -> Result<SoftwareUpdateReportV1, SoftwareUpdateError> {
    if !is_preview_digest(expected_preview_digest) {
        return Err(SoftwareUpdateError::InvalidPlan);
    }
    validate_update_plan(plan)?;
    if cancel.is_cancel_requested() {
        return Err(SoftwareUpdateError::CanceledBeforeStart);
    }
    let cancellation_error = |error| {
        if cancel.is_cancel_requested() {
            SoftwareUpdateError::CanceledBeforeStart
        } else {
            error
        }
    };
    let (binding, row) = {
        let mut preflight =
            |request: &ProcessRequest<'_>| update_preflight_probe(&mut run, request, cancel);
        let binding = bind_update_provider(program, &mut preflight).map_err(cancellation_error)?;
        let row = bound_update_row(&mut preflight, &binding, plan).map_err(cancellation_error)?;
        (binding, row)
    };
    let digest = update_digest(plan, &row)?;
    if digest != expected_preview_digest {
        return Err(SoftwareUpdateError::StaleConfirmation);
    }
    if cancel.is_cancel_requested() {
        return Err(SoftwareUpdateError::CanceledBeforeStart);
    }
    let plan_identity = authority_digest(&("devsweep.software.update.plan.v2", plan, UPDATE_SCOPE))
        .map_err(|_| SoftwareUpdateError::InvalidPlan)?;
    let mut journal = SoftwareSupportJournal::open(audit_path)
        .map_err(|_| SoftwareUpdateError::AuditUnavailable)?;
    let now = unix_ms(SystemTime::now());
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let operation_id = format!(
        "software-update-{now:016x}-{:08x}-{sequence:016x}",
        std::process::id()
    );
    journal
        .append_lifecycle(
            SoftwareSupportLifecycleRecordV1::started(
                SoftwareSupportOperation::Update,
                &operation_id,
                now,
                &plan_identity,
                &digest,
            ),
            fault,
        )
        .map_err(|_| SoftwareUpdateError::AuditUnavailable)?;
    // Bind again after the durable start. The held file handles prevent a
    // same-name replacement while the trusted process is dispatched.
    let late = {
        let mut preflight =
            |request: &ProcessRequest<'_>| update_preflight_probe(&mut run, request, cancel);
        bind_update_provider(program, &mut preflight).and_then(|live| {
            if live.fingerprint != binding.fingerprint {
                return Err(SoftwareUpdateError::ProviderChanged);
            }
            bound_update_row(&mut preflight, &live, plan).map(|_| ())
        })
    }
    .map_err(cancellation_error)
    .and_then(|()| {
        if cancel.is_cancel_requested() {
            Err(SoftwareUpdateError::CanceledBeforeStart)
        } else {
            Ok(())
        }
    });
    if let Err(error) = late {
        journal
            .append_lifecycle(
                SoftwareSupportLifecycleRecordV1::terminal(
                    SoftwareSupportOperation::Update,
                    &operation_id,
                    unix_ms(SystemTime::now()),
                    &plan_identity,
                    &digest,
                    SoftwareSupportTerminalResult::Refused,
                    Some(error.code()),
                ),
                fault,
            )
            .map_err(|_| SoftwareUpdateError::AuditUnavailable)?;
        return Err(error);
    }
    let noop = NoopCancelObserver;
    let request = upgrade_request(binding.file.path.as_os_str(), plan, &noop);
    let result = run(&request);
    // WinGet's HRESULT for RebootRequiredToFinish. Localized output is not
    // execution evidence, including an injected `restart_required` line.
    let restart_required = matches!(result.status, ProcessStatus::Exit { code: Some(code) } if code == 0x8A15_0109_u32 as i32);
    let requery = bind_update_provider(program, &mut run).and_then(|live| {
        if live.fingerprint != binding.fingerprint {
            return Err(SoftwareUpdateError::ProviderChanged);
        }
        let rows = probe_rows(&mut run, live.file.path.as_os_str())?;
        let mut selected = rows.iter().filter(|row| row.id == plan.package_id);
        let current = selected.next();
        if selected.next().is_some() {
            return Err(SoftwareUpdateError::AmbiguousPackage);
        }
        if current.is_some_and(|row| row.source != plan.source) {
            return Err(SoftwareUpdateError::UnsupportedSource);
        }
        Ok(rows)
    });
    let observation_error = requery.as_ref().err().map(|error| error.code().to_string());
    let outcome = if restart_required {
        SoftwareUpdateOutcome::RestartRequired
    } else if matches!(result.status, ProcessStatus::Success) {
        match requery {
            Ok(rows) => {
                let current = rows.iter().find(|row| row.id == plan.package_id);
                if current.is_none()
                    || current.is_some_and(|row| row.installed_version == plan.available_version)
                {
                    SoftwareUpdateOutcome::Updated
                } else {
                    SoftwareUpdateOutcome::Failed
                }
            }
            Err(_) => SoftwareUpdateOutcome::Unknown,
        }
    } else if matches!(result.status, ProcessStatus::Exit { code: Some(_) }) {
        SoftwareUpdateOutcome::Failed
    } else {
        SoftwareUpdateOutcome::Unknown
    };
    let error_code = match result.status {
        ProcessStatus::Exit { code: Some(code) } => Some(format!("winget_exit_{}", code as u32)),
        ProcessStatus::Success => observation_error.or_else(|| {
            (outcome == SoftwareUpdateOutcome::Failed).then(|| "update_not_observed".to_string())
        }),
        _ => Some("unknown_after_dispatch".to_string()),
    };
    let terminal = match outcome {
        SoftwareUpdateOutcome::Updated => SoftwareSupportTerminalResult::Succeeded,
        SoftwareUpdateOutcome::RestartRequired => SoftwareSupportTerminalResult::RebootRequired,
        SoftwareUpdateOutcome::Failed => SoftwareSupportTerminalResult::Failed,
        SoftwareUpdateOutcome::Unknown => SoftwareSupportTerminalResult::Unknown,
    };
    let durable = journal
        .append_lifecycle(
            SoftwareSupportLifecycleRecordV1::terminal(
                SoftwareSupportOperation::Update,
                &operation_id,
                unix_ms(SystemTime::now()),
                &plan_identity,
                &digest,
                terminal,
                error_code.as_deref(),
            ),
            fault,
        )
        .is_ok();
    let outcome = if durable {
        outcome
    } else {
        SoftwareUpdateOutcome::Unknown
    };
    Ok(SoftwareUpdateReportV1 {
        version: SOFTWARE_UPDATE_PLAN_VERSION,
        package_id: plan.package_id.clone(),
        source: plan.source.clone(),
        available_version: plan.available_version.clone(),
        outcome,
        updated: outcome == SoftwareUpdateOutcome::Updated,
        operation_id: Some(operation_id),
        error_code: if durable {
            error_code
        } else {
            Some("unknown_after_dispatch".to_string())
        },
    })
}

fn update_preflight_probe(
    run: &mut impl FnMut(&ProcessRequest<'_>) -> ProcessResult,
    request: &ProcessRequest<'_>,
    cancel: &dyn CancelObserver,
) -> ProcessResult {
    if cancel.is_cancel_requested() {
        return ProcessResult {
            status: ProcessStatus::Canceled,
            output: Default::default(),
        };
    }
    run(&ProcessRequest {
        program: request.program.clone(),
        args: request.args.clone(),
        cwd: request.cwd.clone(),
        timeout: request.timeout,
        job_deadline: request.job_deadline,
        cancel,
    })
}

fn bound_update_row(
    run: &mut impl FnMut(&ProcessRequest<'_>) -> ProcessResult,
    binding: &UpdateBinding,
    plan: &SoftwareUpdatePlanV1,
) -> Result<ParsedRow, SoftwareUpdateError> {
    validate_update_plan(plan)?;
    if binding.fingerprint != plan.provider_fingerprint {
        return Err(SoftwareUpdateError::ProviderChanged);
    }
    select_update_row(
        run,
        binding.file.path.as_os_str(),
        &plan.package_id,
        &plan.source,
        &plan.available_version,
        Some(plan.installed_version.as_str()),
    )
}

fn select_update_row(
    run: &mut impl FnMut(&ProcessRequest<'_>) -> ProcessResult,
    program: &OsStr,
    package_id: &str,
    source: &str,
    available_version: &str,
    installed_version: Option<&str>,
) -> Result<ParsedRow, SoftwareUpdateError> {
    if source != SUPPORTED_UPDATE_SOURCE {
        return Err(SoftwareUpdateError::UnsupportedSource);
    }
    let rows = probe_rows(run, program)?;
    let matches = rows
        .into_iter()
        .filter(|row| row.id == package_id)
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(SoftwareUpdateError::AmbiguousPackage);
    }
    let Some(row) = matches.into_iter().next() else {
        return Err(SoftwareUpdateError::PackageNotFound);
    };
    if row.source != SUPPORTED_UPDATE_SOURCE || row.source != source {
        return Err(SoftwareUpdateError::UnsupportedSource);
    }
    if row.available_version != available_version
        || installed_version.is_some_and(|installed| row.installed_version != installed)
    {
        return Err(SoftwareUpdateError::VersionChanged);
    }
    Ok(row)
}

fn probe_rows(
    run: &mut impl FnMut(&ProcessRequest<'_>) -> ProcessResult,
    program: &OsStr,
) -> Result<Vec<ParsedRow>, SoftwareUpdateError> {
    let noop = NoopCancelObserver;
    let request = read_only_request(program, &noop);
    let result = run(&request);
    if output_line(&result, "network_unavailable") {
        return Err(SoftwareUpdateError::NetworkUnavailable);
    }
    classify(&result).map_err(|_| SoftwareUpdateError::ProbeFailed)
}

fn validate_update_selector(
    package_id: &str,
    source: &str,
    available_version: &str,
) -> Result<(), SoftwareUpdateError> {
    if !update_token(package_id) || !update_token(source) || !update_token(available_version) {
        return Err(SoftwareUpdateError::InvalidPlan);
    }
    if source != SUPPORTED_UPDATE_SOURCE {
        return Err(SoftwareUpdateError::UnsupportedSource);
    }
    Ok(())
}

fn validate_update_plan(plan: &SoftwareUpdatePlanV1) -> Result<(), SoftwareUpdateError> {
    if plan.version != SOFTWARE_UPDATE_PLAN_VERSION {
        return Err(SoftwareUpdateError::UnsupportedPlanVersion(plan.version));
    }
    validate_update_selector(&plan.package_id, &plan.source, &plan.available_version)?;
    if !update_token(&plan.installed_version) || !is_preview_digest(&plan.provider_fingerprint) {
        return Err(SoftwareUpdateError::InvalidPlan);
    }
    Ok(())
}

fn update_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('-')
        && !value.chars().any(char::is_whitespace)
}

fn update_digest(
    plan: &SoftwareUpdatePlanV1,
    row: &ParsedRow,
) -> Result<String, SoftwareUpdateError> {
    let canonical = serde_json::to_vec(&(
        "devsweep.software.update.preview.v1",
        plan.version,
        &plan.package_id,
        &plan.source,
        &plan.installed_version,
        &plan.available_version,
        &plan.provider_fingerprint,
        UPDATE_SCOPE,
        &row.id,
        &row.source,
        &row.installed_version,
        &row.available_version,
    ))
    .map_err(|_| SoftwareUpdateError::InvalidPlan)?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(canonical))))
}

struct UpdateBinding {
    file: BoundFile,
    fingerprint: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
struct UpdateSource {
    arg: String,
    data: String,
    explicit: bool,
    identifier: String,
    name: String,
    trust_level: Vec<String>,
    #[serde(rename = "Type")]
    kind: String,
}

fn source_identity(text: &str) -> Result<UpdateSource, SoftwareUpdateError> {
    let mut source: UpdateSource =
        serde_json::from_str(text).map_err(|_| SoftwareUpdateError::UnsupportedSource)?;
    source.trust_level.sort();
    if source.arg != "https://cdn.winget.microsoft.com/cache"
        || source.name != "winget"
        || source.kind != "Microsoft.PreIndexed.Package"
        || source.identifier != "Microsoft.Winget.Source_8wekyb3d8bbwe"
        || source.data != source.identifier
        || source.trust_level != ["StoreOrigin", "Trusted"]
    {
        return Err(SoftwareUpdateError::UnsupportedSource);
    }
    Ok(source)
}

fn bind_update_provider(
    program: &OsStr,
    run: &mut impl FnMut(&ProcessRequest<'_>) -> ProcessResult,
) -> Result<UpdateBinding, SoftwareUpdateError> {
    let path = resolve_update_program(program)?;
    if !path
        .try_exists()
        .map_err(|_| SoftwareUpdateError::UnsupportedProvider)?
    {
        return Err(SoftwareUpdateError::ProviderMissing);
    }
    let file = BoundFile::open(&path).map_err(|_| SoftwareUpdateError::UnsupportedProvider)?;
    let mut header = [0_u8; 4];
    std::fs::File::open(&file.path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|_| SoftwareUpdateError::UnsupportedProvider)?;
    let native = if cfg!(windows) {
        header.starts_with(b"MZ")
    } else {
        header == *b"\x7fELF"
            || matches!(header, [0xcf, 0xfa, 0xed, 0xfe] | [0xfe, 0xed, 0xfa, 0xcf])
    };
    if !native {
        return Err(SoftwareUpdateError::UnsupportedProvider);
    }
    let noop = NoopCancelObserver;
    let request = |args: Vec<OsString>| ProcessRequest {
        program: file.path.as_os_str().to_os_string(),
        args,
        cwd: CwdPolicy::Neutral,
        timeout: Some(DEFAULT_PROVIDER_PHASE_DEADLINE),
        job_deadline: None,
        cancel: &noop,
    };
    let version = run(&request(vec![OsString::from("--version")]));
    let version = bounded_provider_text(&version)?;
    // Stable WinGet 1.x plus an explicit capability probe. A guessed minimum
    // minor version cannot substitute for verifying the flags we actually use.
    let Some(numbers) = version.trim().strip_prefix('v') else {
        return Err(SoftwareUpdateError::UnsupportedProvider);
    };
    let components = numbers.split('.').collect::<Vec<_>>();
    if !(3..=4).contains(&components.len())
        || components[0] != "1"
        || !components.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && part.parse::<u32>().is_ok()
        })
    {
        return Err(SoftwareUpdateError::UnsupportedProvider);
    }
    let help = run(&request(vec![
        OsString::from("upgrade"),
        OsString::from("--help"),
    ]));
    let help = bounded_provider_text(&help)?;
    let flags = [
        "--id",
        "--version",
        "--source",
        "--exact",
        "--scope",
        "--silent",
        "--disable-interactivity",
    ];
    let tokens = help
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '|' | '[' | ']' | '(' | ')'))
        .collect::<Vec<_>>();
    if flags.iter().any(|flag| !tokens.contains(flag)) {
        return Err(SoftwareUpdateError::UnsupportedProvider);
    }
    let help_digest = hex_digest(Sha256::digest(help.as_bytes()));
    let source = run(&request(
        [
            "source",
            "export",
            "--name",
            "winget",
            "--disable-interactivity",
        ]
        .map(OsString::from)
        .into(),
    ));
    let source = source_identity(&bounded_provider_text(&source)?)?;
    let fingerprint = authority_digest(&(
        "devsweep.software.update.provider.v2",
        &file.digest,
        version.trim(),
        flags,
        help_digest,
        UPDATE_SCOPE,
        source,
    ))
    .map_err(|_| SoftwareUpdateError::InvalidPlan)?;
    Ok(UpdateBinding { file, fingerprint })
}

fn bounded_provider_text(result: &ProcessResult) -> Result<String, SoftwareUpdateError> {
    if !matches!(result.status, ProcessStatus::Success)
        || result.output.stdout_truncated
        || result.output.stderr_truncated
        || result.output.stdout.len() > 128 * 1024
    {
        return Err(SoftwareUpdateError::UnsupportedProvider);
    }
    String::from_utf8(result.output.stdout.clone())
        .map_err(|_| SoftwareUpdateError::UnsupportedProvider)
}

fn resolve_update_program(program: &OsStr) -> Result<PathBuf, SoftwareUpdateError> {
    let path = Path::new(program);
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    if program != OsStr::new("winget") && program != OsStr::new("winget.exe") {
        return Err(SoftwareUpdateError::UnsupportedProvider);
    }
    registered_winget()
}

#[cfg(not(windows))]
fn registered_winget() -> Result<PathBuf, SoftwareUpdateError> {
    Err(SoftwareUpdateError::ProviderMissing)
}

/// Current-user registered Microsoft App Installer, without following the
/// WindowsApps execution-alias reparse point or trusting PATH text.
#[cfg(windows)]
fn registered_winget() -> Result<PathBuf, SoftwareUpdateError> {
    super::msix::run_joined_mta(|| {
        use windows::{
            ApplicationModel::PackageSignatureKind, Management::Deployment::PackageManager,
            core::HSTRING,
        };
        let sid =
            super::msix::current_process_sid().map_err(|_| SoftwareUpdateError::ProviderMissing)?;
        let manager = PackageManager::new().map_err(|_| SoftwareUpdateError::ProviderMissing)?;
        let packages = manager
            .FindPackagesByUserSecurityId(&HSTRING::from(sid))
            .map_err(|_| SoftwareUpdateError::ProviderMissing)?;
        let iterator = packages
            .First()
            .map_err(|_| SoftwareUpdateError::ProviderMissing)?;
        let mut found = Vec::new();
        let mut count = 0;
        while iterator
            .HasCurrent()
            .map_err(|_| SoftwareUpdateError::ProviderMissing)?
        {
            count += 1;
            if count > 4_096 {
                return Err(SoftwareUpdateError::UnsupportedProvider);
            }
            let package = iterator
                .Current()
                .map_err(|_| SoftwareUpdateError::ProviderMissing)?;
            let id = package
                .Id()
                .map_err(|_| SoftwareUpdateError::ProviderMissing)?;
            if id
                .FamilyName()
                .map_err(|_| SoftwareUpdateError::ProviderMissing)?
                == "Microsoft.DesktopAppInstaller_8wekyb3d8bbwe"
            {
                if package.SignatureKind().ok() != Some(PackageSignatureKind::Store)
                    || !package
                        .Status()
                        .and_then(|status| status.VerifyIsOK())
                        .unwrap_or(false)
                    || package.IsFramework().unwrap_or(true)
                    || package.IsResourcePackage().unwrap_or(true)
                {
                    return Err(SoftwareUpdateError::UnsupportedProvider);
                }
                let root = package
                    .InstalledPath()
                    .map_err(|_| SoftwareUpdateError::ProviderMissing)?
                    .to_string();
                if root.is_empty() || root.len() > 32 * 1024 {
                    return Err(SoftwareUpdateError::UnsupportedProvider);
                }
                found.push(PathBuf::from(root).join("winget.exe"));
            }
            iterator
                .MoveNext()
                .map_err(|_| SoftwareUpdateError::ProviderMissing)?;
        }
        if found.len() != 1 {
            return Err(SoftwareUpdateError::ProviderMissing);
        }
        found.pop().ok_or(SoftwareUpdateError::ProviderMissing)
    })
    .map_err(|_| SoftwareUpdateError::ProviderMissing)?
}

fn is_preview_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn output_line(result: &ProcessResult, token: &str) -> bool {
    let stdout = String::from_utf8_lossy(&result.output.stdout);
    let stderr = String::from_utf8_lossy(&result.output.stderr);
    stdout.lines().any(|line| line.trim() == token)
        || stderr.lines().any(|line| line.trim() == token)
}

fn read_only_request<'a>(program: &OsStr, cancel: &'a dyn CancelObserver) -> ProcessRequest<'a> {
    ProcessRequest {
        program: program.to_os_string(),
        args: [
            OsString::from("upgrade"),
            OsString::from("--disable-interactivity"),
            OsString::from("--scope"),
            OsString::from(UPDATE_SCOPE),
            OsString::from("--source"),
            OsString::from(SUPPORTED_UPDATE_SOURCE),
        ]
        .into(),
        cwd: CwdPolicy::Neutral,
        timeout: Some(DEFAULT_PROVIDER_PHASE_DEADLINE),
        job_deadline: None,
        cancel,
    }
}

fn upgrade_request<'a>(
    program: &OsStr,
    plan: &SoftwareUpdatePlanV1,
    cancel: &'a dyn CancelObserver,
) -> ProcessRequest<'a> {
    ProcessRequest {
        program: program.to_os_string(),
        args: [
            OsString::from("upgrade"),
            OsString::from("--id"),
            OsString::from(&plan.package_id),
            OsString::from("--version"),
            OsString::from(&plan.available_version),
            OsString::from("--source"),
            OsString::from(&plan.source),
            OsString::from("--disable-interactivity"),
            OsString::from("--exact"),
            OsString::from("--silent"),
            OsString::from("--scope"),
            OsString::from(UPDATE_SCOPE),
        ]
        .into(),
        cwd: CwdPolicy::Neutral,
        timeout: Some(DEFAULT_EXECUTOR_COMMAND_TIMEOUT),
        job_deadline: None,
        cancel,
    }
}

/// Runs the read-only winget probe and matches rows to `inventory` by name.
pub fn check_software_updates(
    inventory: Option<&SoftwareInventoryV1>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> SoftwareUpdatesV1 {
    let runner = ProcessRunner::default();
    let mut run = |request: &ProcessRequest<'_>| runner.run(request);
    let binding = match bind_update_provider(OsStr::new(WINGET_PROGRAM), &mut run) {
        Ok(binding) => binding,
        Err(error) => {
            return SoftwareUpdatesV1::Unavailable {
                version: SOFTWARE_UPDATES_VERSION,
                observed_at_unix_ms: unix_ms(SystemTime::now()),
                reason_code: if matches!(error, SoftwareUpdateError::ProviderMissing) {
                    SoftwareUpdatesReason::WingetMissing
                } else {
                    SoftwareUpdatesReason::ProviderUnsupported
                },
            };
        }
    };
    check_updates_with(
        |request| {
            let request = ProcessRequest {
                program: binding.file.path.as_os_str().to_os_string(),
                args: request.args.clone(),
                cwd: request.cwd.clone(),
                timeout: request.timeout,
                job_deadline: request.job_deadline,
                cancel: request.cancel,
            };
            runner.run(&request)
        },
        inventory,
        cancel,
    )
}

fn check_updates_with(
    run: impl FnOnce(&ProcessRequest<'_>) -> ProcessResult,
    inventory: Option<&SoftwareInventoryV1>,
    cancel: Option<&Arc<FlagCancelObserver>>,
) -> SoftwareUpdatesV1 {
    let noop = NoopCancelObserver;
    let observer: &dyn CancelObserver = match cancel {
        Some(flag) => flag.as_ref(),
        None => &noop,
    };
    let request = update_request(observer);
    let result = run(&request);
    let observed_at_unix_ms = unix_ms(SystemTime::now());
    match classify(&result) {
        Ok(rows) => SoftwareUpdatesV1::Available {
            version: SOFTWARE_UPDATES_VERSION,
            observed_at_unix_ms,
            rows: rows
                .into_iter()
                .map(|row| match_row(row, inventory))
                .collect(),
        },
        Err(reason_code) => SoftwareUpdatesV1::Unavailable {
            version: SOFTWARE_UPDATES_VERSION,
            observed_at_unix_ms,
            reason_code,
        },
    }
}

fn update_request(cancel: &dyn CancelObserver) -> ProcessRequest<'_> {
    ProcessRequest {
        program: OsString::from(WINGET_PROGRAM),
        args: WINGET_UPGRADE_ARGS.iter().map(OsString::from).collect(),
        cwd: CwdPolicy::Neutral,
        timeout: Some(DEFAULT_PROVIDER_PHASE_DEADLINE),
        job_deadline: None,
        cancel,
    }
}

fn classify(result: &ProcessResult) -> Result<Vec<ParsedRow>, SoftwareUpdatesReason> {
    let agreement_pending = || {
        let stdout = String::from_utf8_lossy(&result.output.stdout);
        let stderr = String::from_utf8_lossy(&result.output.stderr);
        mentions_agreement(&stdout) || mentions_agreement(&stderr)
    };
    match result.status {
        ProcessStatus::NotFound => return Err(SoftwareUpdatesReason::WingetMissing),
        ProcessStatus::Timeout => return Err(SoftwareUpdatesReason::TimedOut),
        ProcessStatus::Canceled => return Err(SoftwareUpdatesReason::Canceled),
        ProcessStatus::InvalidOutput => return Err(SoftwareUpdatesReason::ProcessFailed),
        ProcessStatus::Exit { .. } => {
            return Err(if agreement_pending() {
                SoftwareUpdatesReason::SourceAgreementPending
            } else {
                SoftwareUpdatesReason::NonZeroExit
            });
        }
        ProcessStatus::Success => {}
    }
    if result.output.stdout_truncated || result.output.stderr_truncated {
        return Err(SoftwareUpdatesReason::OutputTruncated);
    }
    let stdout = std::str::from_utf8(&result.output.stdout)
        .map_err(|_| SoftwareUpdatesReason::UnrecognizedOutput)?;
    parse_winget_upgrade(stdout).map_err(|reason| {
        if agreement_pending() {
            SoftwareUpdatesReason::SourceAgreementPending
        } else {
            reason
        }
    })
}

fn mentions_agreement(text: &str) -> bool {
    text.to_ascii_lowercase().contains("agreement") || text.contains("协议")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedRow {
    id: String,
    name: String,
    name_truncated: bool,
    installed_version: String,
    available_version: String,
    source: String,
}

fn match_row(row: ParsedRow, inventory: Option<&SoftwareInventoryV1>) -> SoftwareUpdateRowV1 {
    let wanted = row.name.to_lowercase();
    let matched_software_ids = if row.name_truncated {
        Vec::new()
    } else {
        inventory
            .map(|inventory| {
                inventory
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry
                            .display_name
                            .as_deref()
                            .is_some_and(|name| name.trim().to_lowercase() == wanted)
                    })
                    .map(|entry| entry.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    };
    SoftwareUpdateRowV1 {
        id: row.id,
        name: row.name,
        name_truncated: row.name_truncated,
        installed_version: row.installed_version,
        available_version: row.available_version,
        source: row.source,
        matched_software_ids,
    }
}

/// Parses the first `winget upgrade` table by display-width column offsets.
/// Header words are never interpreted, so localized headers parse the same way.
fn parse_winget_upgrade(stdout: &str) -> Result<Vec<ParsedRow>, SoftwareUpdatesReason> {
    let lines = stdout
        .split('\n')
        .map(|line| {
            let line = line.strip_suffix('\r').unwrap_or(line);
            // Progress output rewrites the line with carriage returns; keep
            // only the text after the last one.
            line.rsplit('\r').next().unwrap_or(line)
        })
        .collect::<Vec<_>>();
    let Some(separator) = lines.iter().position(|line| is_separator(line)) else {
        let no_updates = lines
            .iter()
            .any(|line| NO_UPDATE_MESSAGES.contains(&line.trim()));
        return if no_updates {
            Ok(Vec::new())
        } else {
            Err(SoftwareUpdatesReason::UnrecognizedOutput)
        };
    };
    let header = lines[..separator]
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty())
        .ok_or(SoftwareUpdatesReason::UnrecognizedOutput)?;
    let starts = column_starts(header);
    if starts.len() != TABLE_COLUMNS {
        return Err(SoftwareUpdatesReason::UnrecognizedOutput);
    }
    let mut rows = Vec::new();
    for line in &lines[separator + 1..] {
        match parse_row(line, &starts) {
            Some(row) => rows.push(row),
            None => break,
        }
    }
    if rows.is_empty() {
        return Err(SoftwareUpdatesReason::UnrecognizedOutput);
    }
    Ok(rows)
}

fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 10 && trimmed.chars().all(|character| character == '-')
}

/// Display-width offsets of each whitespace-separated header token.
fn column_starts(header: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut offset = 0;
    let mut previous_space = true;
    for character in header.chars() {
        let space = character == ' ';
        if !space && previous_space {
            starts.push(offset);
        }
        previous_space = space;
        offset += display_width(character);
    }
    starts
}

fn parse_row(line: &str, starts: &[usize]) -> Option<ParsedRow> {
    let mut columns = vec![String::new(); starts.len()];
    let mut offset = 0;
    for character in line.chars() {
        let width = display_width(character);
        // Each column boundary must be preceded by a separating space.
        if starts[1..].contains(&(offset + width)) && character != ' ' {
            return None;
        }
        let column = starts.iter().rposition(|start| *start <= offset)?;
        columns[column].push(character);
        offset += width;
    }
    let mut columns = columns.into_iter().map(|value| value.trim().to_string());
    let name = columns.next()?;
    let id = columns.next()?;
    let installed_version = columns.next()?;
    let available_version = columns.next()?;
    let source = columns.next()?;
    let single_token = |value: &str| !value.is_empty() && !value.contains(char::is_whitespace);
    if name.is_empty()
        || installed_version.is_empty()
        || !single_token(&id)
        || !single_token(&available_version)
        || !single_token(&source)
    {
        return None;
    }
    let (name, name_truncated) = match name.strip_suffix(TRUNCATION_MARK) {
        Some(stripped) => (stripped.trim_end().to_string(), true),
        None => (name, false),
    };
    Some(ParsedRow {
        id,
        name,
        name_truncated,
        installed_version,
        available_version,
        source,
    })
}

/// Terminal column width: East Asian Wide and Fullwidth characters count as 2.
fn display_width(character: char) -> usize {
    let code = u32::from(character);
    let wide = matches!(
        code,
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x1F300..=0x1F64F
            | 0x1F900..=0x1F9FF
            | 0x20000..=0x2FFFD
            | 0x30000..=0x3FFFD
    );
    if wide { 2 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessOutput;

    const ENGLISH: &str = include_str!("fixtures/winget-upgrade-en.txt");
    const CHINESE: &str = include_str!("fixtures/winget-upgrade-zh-CN.txt");

    fn success(stdout: &str) -> ProcessResult {
        ProcessResult {
            status: ProcessStatus::Success,
            output: ProcessOutput {
                stdout: stdout.as_bytes().to_vec(),
                ..ProcessOutput::default()
            },
        }
    }

    fn rows(updates: SoftwareUpdatesV1) -> Vec<SoftwareUpdateRowV1> {
        match updates {
            SoftwareUpdatesV1::Available { rows, .. } => rows,
            SoftwareUpdatesV1::Unavailable { reason_code, .. } => {
                panic!("expected rows, got {reason_code:?}")
            }
        }
    }

    fn reason(updates: SoftwareUpdatesV1) -> SoftwareUpdatesReason {
        match updates {
            SoftwareUpdatesV1::Unavailable { reason_code, .. } => reason_code,
            SoftwareUpdatesV1::Available { rows, .. } => {
                panic!("expected unavailable, got {rows:?}")
            }
        }
    }

    #[test]
    fn english_fixture_parses_every_row_and_ignores_summary_lines() {
        let parsed = parse_winget_upgrade(ENGLISH).expect("english table");
        assert_eq!(parsed.len(), 13);
        assert_eq!(
            parsed[0],
            ParsedRow {
                id: "Bandisoft.Bandizip".into(),
                name: "Bandizip Professional".into(),
                name_truncated: false,
                installed_version: "7.43".into(),
                available_version: "7.46".into(),
                source: "winget".into(),
            }
        );
        let physx = parsed.iter().find(|row| row.id == "Nvidia.PhysX").unwrap();
        assert_eq!(physx.name, "NVIDIA PhysX 系统软件 9.23.1019");
        assert_eq!(physx.installed_version, "9.23.1019");
        assert_eq!(physx.available_version, "9.26.0703");
        let nutstore = parsed
            .iter()
            .find(|row| row.id == "Nutstore.Nutstore")
            .unwrap();
        assert_eq!(nutstore.name, "坚果云");
        assert_eq!(nutstore.available_version, "7.2.12");
        let t3 = parsed
            .iter()
            .find(|row| row.id == "T3Tools.T3Code")
            .unwrap();
        assert_eq!(t3.installed_version, "0.0.38-nightly.20260831.1236");
    }

    #[test]
    fn chinese_fixture_parses_by_display_width_not_header_words() {
        let parsed = parse_winget_upgrade(CHINESE).expect("zh-CN table");
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0].id, "Bandisoft.Bandizip");
        assert_eq!(parsed[1].name, "NVIDIA PhysX 系统软件 9.23.1019");
        assert_eq!(parsed[1].source, "winget");
        assert_eq!(parsed[2].name, "坚果云");
        assert_eq!(parsed[2].installed_version, "7.2.7");
        assert!(parsed[3].name_truncated);
        assert_eq!(parsed[3].name, "Microsoft Visual C++ 2015-2022 Redistr");
        assert_eq!(parsed[3].id, "Microsoft.VCRedist.2015+.x64");
    }

    #[test]
    fn cjk_characters_count_two_columns() {
        assert_eq!(display_width('坚'), 2);
        assert_eq!(display_width('A'), 1);
        assert_eq!(display_width(TRUNCATION_MARK), 1);
        assert_eq!(column_starts("名称  ID  版本"), vec![0, 6, 10]);
    }

    #[test]
    fn truncated_names_lose_the_ellipsis_and_never_match_inventory() {
        let table = "Name                 Id          Version Available Source\n\
                     -----------------------------------------------------------\n\
                     Very Long Product N\u{2026} Vendor.Long 1.0     2.0       winget\n";
        let parsed = parse_winget_upgrade(table).expect("table");
        assert_eq!(parsed[0].name, "Very Long Product N");
        assert!(parsed[0].name_truncated);
        let row = match_row(parsed[0].clone(), None);
        assert!(row.matched_software_ids.is_empty());
    }

    #[test]
    fn no_update_messages_are_an_empty_success_in_both_locales() {
        for message in [
            "No installed package found matching input criteria.\r\n",
            "No available upgrade found.\r\n",
            "  \r找不到与输入条件匹配的已安装程序包。\r\n",
        ] {
            assert!(rows(check_updates_with(|_| success(message), None, None)).is_empty());
        }
    }

    #[test]
    fn unknown_output_and_failures_are_unavailable_never_empty_success() {
        assert_eq!(
            reason(check_updates_with(|_| success(""), None, None)),
            SoftwareUpdatesReason::UnrecognizedOutput
        );
        assert_eq!(
            reason(check_updates_with(
                |_| success("Something changed.\n"),
                None,
                None
            )),
            SoftwareUpdatesReason::UnrecognizedOutput
        );
        let four_columns = "Name   Id   Version   Available\n--------------------------------\n\
                            A      B    1         2\n";
        assert_eq!(
            reason(check_updates_with(|_| success(four_columns), None, None)),
            SoftwareUpdatesReason::UnrecognizedOutput
        );
        let exit = |code| ProcessResult {
            status: ProcessStatus::Exit { code: Some(code) },
            output: ProcessOutput::default(),
        };
        assert_eq!(
            reason(check_updates_with(|_| exit(-1_978_335_212), None, None)),
            SoftwareUpdatesReason::NonZeroExit
        );
        let agreement = ProcessResult {
            status: ProcessStatus::Exit { code: Some(1) },
            output: ProcessOutput {
                stdout: "The `msstore` source requires that you view the following agreements before using.\n"
                    .as_bytes()
                    .to_vec(),
                ..ProcessOutput::default()
            },
        };
        assert_eq!(
            reason(check_updates_with(|_| agreement.clone(), None, None)),
            SoftwareUpdatesReason::SourceAgreementPending
        );
        for (status, expected) in [
            (
                ProcessStatus::NotFound,
                SoftwareUpdatesReason::WingetMissing,
            ),
            (ProcessStatus::Timeout, SoftwareUpdatesReason::TimedOut),
            (ProcessStatus::Canceled, SoftwareUpdatesReason::Canceled),
            (
                ProcessStatus::InvalidOutput,
                SoftwareUpdatesReason::ProcessFailed,
            ),
        ] {
            let result = ProcessResult {
                status,
                output: ProcessOutput::default(),
            };
            assert_eq!(
                reason(check_updates_with(|_| result.clone(), None, None)),
                expected
            );
        }
        let truncated = ProcessResult {
            status: ProcessStatus::Success,
            output: ProcessOutput {
                stdout: ENGLISH.as_bytes().to_vec(),
                stdout_truncated: true,
                ..ProcessOutput::default()
            },
        };
        assert_eq!(
            reason(check_updates_with(|_| truncated.clone(), None, None)),
            SoftwareUpdatesReason::OutputTruncated
        );
    }

    #[test]
    fn update_argv_is_separate_bounded_and_never_accepts_agreements() {
        let mut captured = None;
        let _ = check_updates_with(
            |request| {
                captured = Some((
                    request.program.clone(),
                    request.args.clone(),
                    request.cwd.clone(),
                    request.timeout,
                ));
                success(ENGLISH)
            },
            None,
            None,
        );
        let (program, args, cwd, timeout) = captured.expect("probe went through ProcessRequest");
        assert_eq!(program, OsString::from("winget"));
        assert_eq!(
            args,
            vec![
                OsString::from("upgrade"),
                OsString::from("--disable-interactivity"),
                OsString::from("--scope"),
                OsString::from("user"),
                OsString::from("--source"),
                OsString::from("winget"),
            ]
        );
        assert_eq!(cwd, CwdPolicy::Neutral);
        assert_eq!(timeout, Some(DEFAULT_PROVIDER_PHASE_DEADLINE));
        for arg in &args {
            let arg = arg.to_string_lossy();
            assert!(!arg.starts_with("--accept"), "agreement flag {arg}");
            assert!(!arg.contains(' '), "shell-composed argument {arg}");
        }
        let source = include_str!("updates.rs");
        let production = source.split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            ["--accept-source", "-agreements"].concat(),
            ["--accept-package", "-agreements"].concat(),
            ["Command", "::new"].concat(),
            ["cmd", ".exe"].concat(),
            ["power", "shell"].concat(),
        ] {
            assert!(!production.contains(&forbidden), "{forbidden}");
        }
    }

    const HELP: &str = "--id --version --source --exact --scope --silent --disable-interactivity";
    const SOURCE: &str = r#"{"Arg":"https://cdn.winget.microsoft.com/cache","Data":"Microsoft.Winget.Source_8wekyb3d8bbwe","Explicit":false,"Identifier":"Microsoft.Winget.Source_8wekyb3d8bbwe","Name":"winget","TrustLevel":["Trusted","StoreOrigin"],"Type":"Microsoft.PreIndexed.Package"}"#;

    #[test]
    fn same_name_source_replacement_and_ambiguous_exports_never_acquire_authority() {
        assert!(source_identity(SOURCE).is_ok());
        for (field, value) in [
            (
                "Arg",
                serde_json::json!("https://example.invalid/private?token=secret"),
            ),
            ("Type", serde_json::json!("Microsoft.Rest")),
            ("Identifier", serde_json::json!("replaced")),
            ("Data", serde_json::json!("replaced")),
            ("TrustLevel", serde_json::json!(["Trusted"])),
        ] {
            let mut source: serde_json::Value = serde_json::from_str(SOURCE).expect("source");
            source[field] = value;
            assert!(matches!(
                source_identity(&source.to_string()),
                Err(SoftwareUpdateError::UnsupportedSource)
            ));
        }
        assert!(source_identity(&format!("[{SOURCE}] ")).is_err());
        assert!(source_identity(&format!("{SOURCE}\n{SOURCE}")).is_err());
        let mut source: serde_json::Value = serde_json::from_str(SOURCE).expect("source");
        source["Updated"] = serde_json::json!(123);
        assert!(
            source_identity(&source.to_string()).is_err(),
            "unknown schema is unavailable, not silently fingerprinted"
        );
    }

    fn native_fixture(root: &Path) -> PathBuf {
        let path = root.join("update-tool.exe");
        let header = if cfg!(windows) { b"MZxx" } else { b"\x7fELF" };
        std::fs::write(&path, header).expect("native header fixture");
        path
    }

    fn fake_probe(request: &ProcessRequest<'_>) -> ProcessResult {
        if request.args.first().is_some_and(|arg| arg == "source") {
            success(SOURCE)
        } else if request.args == [OsString::from("--version")] {
            success("v1.8.1911\n")
        } else if request.args == [OsString::from("upgrade"), OsString::from("--help")] {
            success(HELP)
        } else {
            success(ENGLISH)
        }
    }

    fn bound_plan(path: &Path) -> SoftwareUpdatePlanV1 {
        let binding =
            bind_update_provider(path.as_os_str(), &mut fake_probe).expect("fixture binding");
        SoftwareUpdatePlanV1 {
            version: 1,
            package_id: "Bandisoft.Bandizip".into(),
            source: "winget".into(),
            installed_version: "7.43".into(),
            available_version: "7.46".into(),
            provider_fingerprint: binding.fingerprint,
        }
    }

    fn fixture_digest(plan: &SoftwareUpdatePlanV1) -> String {
        update_digest(plan, &parse_winget_upgrade(ENGLISH).expect("table")[0]).expect("digest")
    }

    #[test]
    fn update_binding_rejects_replaced_binary_version_capability_and_legacy_authority() {
        let root = tempfile::tempdir().expect("fixture");
        let path = native_fixture(root.path());
        let plan = bound_plan(&path);
        let binding = bind_update_provider(path.as_os_str(), &mut fake_probe).expect("binding");
        assert!(bound_update_row(&mut fake_probe, &binding, &plan).is_ok());
        drop(binding);
        std::fs::write(
            &path,
            if cfg!(windows) {
                b"MZyy".as_slice()
            } else {
                b"\x7fELFy".as_slice()
            },
        )
        .expect("replace same path");
        let changed = bind_update_provider(path.as_os_str(), &mut fake_probe).expect("binding");
        assert!(matches!(
            bound_update_row(&mut fake_probe, &changed, &plan),
            Err(SoftwareUpdateError::ProviderChanged)
        ));
        drop(changed);
        for version in [
            "v0.9.0",
            "v2.0.0",
            "v1.8.0-preview",
            "v1.+8.0",
            "v1.8.0\nv1.9.0",
            "not winget",
        ] {
            let mut run = |request: &ProcessRequest<'_>| {
                if request.args == [OsString::from("--version")] {
                    success(version)
                } else {
                    fake_probe(request)
                }
            };
            assert!(matches!(
                bind_update_provider(path.as_os_str(), &mut run),
                Err(SoftwareUpdateError::UnsupportedProvider)
            ));
        }
        for help in [
            "--id --version --source --exact --scope --silent",
            "--identity --version --source --exact --scope --silent --disable-interactivity-extra",
        ] {
            let mut run = |request: &ProcessRequest<'_>| {
                if request.args == [OsString::from("upgrade"), OsString::from("--help")] {
                    success(help)
                } else {
                    fake_probe(request)
                }
            };
            assert!(matches!(
                bind_update_provider(path.as_os_str(), &mut run),
                Err(SoftwareUpdateError::UnsupportedProvider)
            ));
        }
        let mut localized = |request: &ProcessRequest<'_>| {
            if request.args == [OsString::from("upgrade"), OsString::from("--help")] {
                success(&format!("选项\n{HELP}\n说明"))
            } else {
                fake_probe(request)
            }
        };
        assert!(bind_update_provider(path.as_os_str(), &mut localized).is_ok());
        let mut legacy = plan;
        legacy.provider_fingerprint = format!(
            "sha256:{}",
            hex_digest(Sha256::digest(path.as_os_str().as_encoded_bytes()))
        );
        let live = bind_update_provider(path.as_os_str(), &mut fake_probe).expect("binding");
        let encoded = serde_json::to_vec(&legacy).expect("legacy plan");
        let decoded = serde_json::from_slice::<SoftwareUpdatePlanV1>(&encoded)
            .expect("legacy remains readable");
        assert!(matches!(
            bound_update_row(&mut fake_probe, &live, &decoded),
            Err(SoftwareUpdateError::ProviderChanged)
        ));
    }

    #[test]
    fn unsuccessful_or_truncated_provider_evidence_refuses_before_targeted_upgrade() {
        for phase in [
            vec![OsString::from("--version")],
            vec![OsString::from("upgrade"), OsString::from("--help")],
            [
                "source",
                "export",
                "--name",
                "winget",
                "--disable-interactivity",
            ]
            .map(OsString::from)
            .into(),
        ] {
            for (status, stdout_truncated, stderr_truncated) in [
                (ProcessStatus::Exit { code: Some(1) }, false, false),
                (ProcessStatus::Success, true, false),
                (ProcessStatus::Success, false, true),
            ] {
                let root = tempfile::tempdir().expect("fixture");
                let path = native_fixture(root.path());
                let plan = bound_plan(&path);
                let digest = fixture_digest(&plan);
                let audit = root.path().join("audit/software.jsonl");
                let mut observed = false;
                let mut run = |request: &ProcessRequest<'_>| {
                    assert!(!request.args.contains(&OsString::from("--id")));
                    assert_eq!(request.timeout, Some(DEFAULT_PROVIDER_PHASE_DEADLINE));
                    let mut result = fake_probe(request);
                    if request.args == phase {
                        observed = true;
                        result.status = status.clone();
                        result.output.stdout_truncated = stdout_truncated;
                        result.output.stderr_truncated = stderr_truncated;
                    }
                    result
                };
                assert!(matches!(
                    execute_software_update_with(
                        &mut run,
                        path.as_os_str(),
                        &plan,
                        &digest,
                        &audit,
                        SupportAppendFault::NONE,
                        &NoopCancelObserver,
                    ),
                    Err(SoftwareUpdateError::UnsupportedProvider)
                ));
                assert!(observed, "the failing provider phase was actually observed");
                assert!(!audit.exists());
            }
        }
    }

    #[test]
    fn cancel_during_initial_or_late_preflight_never_submits_a_targeted_upgrade() {
        // Each binding has version/help/source plus a target-free row query.
        // The second binding follows the durable started record.
        for cancel_after_probe in 1..=8 {
            let root = tempfile::tempdir().expect("fixture");
            let path = native_fixture(root.path());
            let plan = bound_plan(&path);
            let digest = fixture_digest(&plan);
            let cancel = FlagCancelObserver::new();
            let mut probes = 0;
            let mut run = |request: &ProcessRequest<'_>| {
                assert!(!request.args.contains(&OsString::from("--id")));
                probes += 1;
                let result = fake_probe(request);
                if probes == cancel_after_probe {
                    cancel.request_cancel();
                    assert!(
                        request.cancel.is_cancel_requested(),
                        "job observer reaches probes"
                    );
                }
                result
            };
            let result = execute_software_update_with(
                &mut run,
                path.as_os_str(),
                &plan,
                &digest,
                &root.path().join("audit/software.jsonl"),
                SupportAppendFault::NONE,
                &cancel,
            );
            assert!(matches!(
                result,
                Err(SoftwareUpdateError::CanceledBeforeStart)
            ));
            assert_eq!(probes, cancel_after_probe, "no probes after cancellation");
            let support = root.path().join("audit/software-support.jsonl");
            if cancel_after_probe <= 4 {
                assert!(!support.exists(), "initial preflight has no dispatch start");
            } else {
                let records = std::fs::read_to_string(support).expect("cancellation terminal");
                let records = records
                    .lines()
                    .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("audit row"))
                    .collect::<Vec<_>>();
                assert_eq!(records.len(), 2);
                assert_eq!(records[0]["phase"], "started");
                assert_eq!(records[1]["phase"], "terminal");
                assert_eq!(records[1]["result"], "refused");
                assert_eq!(records[1]["error_code"], "canceled_before_start");
            }
        }
    }

    #[test]
    fn cancellation_after_target_commit_does_not_cancel_the_installer_or_observation() {
        let root = tempfile::tempdir().expect("fixture");
        let path = native_fixture(root.path());
        let plan = bound_plan(&path);
        let digest = fixture_digest(&plan);
        let cancel = FlagCancelObserver::new();
        let mut dispatched = false;
        let mut observations = 0;
        let mut run = |request: &ProcessRequest<'_>| {
            if request.args.contains(&OsString::from("--id")) {
                dispatched = true;
                cancel.request_cancel();
                assert!(
                    !request.cancel.is_cancel_requested(),
                    "committed installer is not canceled"
                );
                success("completed")
            } else if dispatched {
                observations += 1;
                assert!(
                    !request.cancel.is_cancel_requested(),
                    "terminal observation is retained"
                );
                if request.args.contains(&OsString::from("--scope")) {
                    success("No available upgrade found.")
                } else {
                    fake_probe(request)
                }
            } else {
                fake_probe(request)
            }
        };
        let report = execute_software_update_with(
            &mut run,
            path.as_os_str(),
            &plan,
            &digest,
            &root.path().join("audit/software.jsonl"),
            SupportAppendFault::NONE,
            &cancel,
        )
        .expect("real terminal survives cancellation");
        assert!(dispatched);
        assert_eq!(observations, 4);
        assert_eq!(report.outcome, SoftwareUpdateOutcome::Updated);
        assert!(report.updated);
    }

    #[test]
    fn update_execute_audits_before_dispatch_and_reports_terminal_write_uncertainty() {
        for fault in [
            SupportAppendFault {
                fail_started: true,
                fail_terminal: false,
            },
            SupportAppendFault {
                fail_started: false,
                fail_terminal: true,
            },
            SupportAppendFault::NONE,
        ] {
            let root = tempfile::tempdir().expect("fixture");
            let path = native_fixture(root.path());
            let plan = bound_plan(&path);
            let digest = fixture_digest(&plan);
            let audit = root.path().join("audit/software.jsonl");
            let mut calls = 0;
            let mut dispatched = false;
            let mut run = |request: &ProcessRequest<'_>| {
                assert!(Path::new(&request.program).is_absolute());
                if request.args.contains(&OsString::from("--id")) {
                    calls += 1;
                    dispatched = true;
                    let records =
                        std::fs::read_to_string(root.path().join("audit/software-support.jsonl"))
                            .expect("durable started before dispatch");
                    assert!(records.contains("started") && !records.contains("terminal"));
                    assert!(request.args.ends_with(&[
                        OsString::from("--exact"),
                        OsString::from("--silent"),
                        OsString::from("--scope"),
                        OsString::from("user")
                    ]));
                    assert_eq!(request.timeout, Some(DEFAULT_EXECUTOR_COMMAND_TIMEOUT));
                    success("updated")
                } else if dispatched && request.args.contains(&OsString::from("--scope")) {
                    success("No available upgrade found.")
                } else {
                    fake_probe(request)
                }
            };
            let result = execute_software_update_with(
                &mut run,
                path.as_os_str(),
                &plan,
                &digest,
                &audit,
                fault,
                &NoopCancelObserver,
            );
            if fault.fail_started {
                assert!(matches!(result, Err(SoftwareUpdateError::AuditUnavailable)));
                assert_eq!(calls, 0);
            } else {
                let report = result.expect("report");
                assert_eq!(calls, 1);
                assert_eq!(
                    report.outcome,
                    if fault.fail_terminal {
                        SoftwareUpdateOutcome::Unknown
                    } else {
                        SoftwareUpdateOutcome::Updated
                    }
                );
                assert_eq!(report.updated, !fault.fail_terminal);
            }
        }
    }

    #[test]
    fn stale_digest_unconfirmed_and_late_row_changes_never_dispatch() {
        let root = tempfile::tempdir().expect("fixture");
        let path = native_fixture(root.path());
        let plan = bound_plan(&path);
        assert!(matches!(
            execute_software_update(path.as_os_str(), &plan, "", false),
            Err(SoftwareUpdateError::ConfirmationRequired)
        ));
        let digest = fixture_digest(&plan);
        let audit = root.path().join("audit/software.jsonl");
        let stale = format!("sha256:{}", "0".repeat(64));
        assert!(matches!(
            execute_software_update_with(
                fake_probe,
                path.as_os_str(),
                &plan,
                &stale,
                &audit,
                SupportAppendFault::NONE,
                &NoopCancelObserver,
            ),
            Err(SoftwareUpdateError::StaleConfirmation)
        ));
        assert!(!audit.exists());
        let mut probes = 0;
        let mut run = |request: &ProcessRequest<'_>| {
            assert!(
                !request.args.contains(&OsString::from("--id")),
                "late changed row must never dispatch"
            );
            if request.args.contains(&OsString::from("--scope")) {
                probes += 1;
                if probes > 1 {
                    return success(&ENGLISH.replace("7.46", "7.99"));
                }
            }
            fake_probe(request)
        };
        assert!(matches!(
            execute_software_update_with(
                &mut run,
                path.as_os_str(),
                &plan,
                &digest,
                &audit,
                SupportAppendFault::NONE,
                &NoopCancelObserver,
            ),
            Err(SoftwareUpdateError::VersionChanged)
        ));
        let audit = std::fs::read_to_string(root.path().join("audit/software-support.jsonl"))
            .expect("refusal terminal");
        assert!(audit.contains("refused") && audit.contains("version_changed"));
    }

    #[test]
    fn update_exit_and_requery_evidence_never_promote_failures_or_unknown_to_success() {
        for (status, expected) in [
            (ProcessStatus::Success, SoftwareUpdateOutcome::Updated),
            (
                ProcessStatus::Exit {
                    code: Some(0x8A15_0109_u32 as i32),
                },
                SoftwareUpdateOutcome::RestartRequired,
            ),
            (
                ProcessStatus::Exit { code: Some(1) },
                SoftwareUpdateOutcome::Failed,
            ),
            (ProcessStatus::Timeout, SoftwareUpdateOutcome::Unknown),
            (ProcessStatus::Canceled, SoftwareUpdateOutcome::Unknown),
            (
                ProcessStatus::Exit { code: None },
                SoftwareUpdateOutcome::Unknown,
            ),
        ] {
            let root = tempfile::tempdir().expect("fixture");
            let path = native_fixture(root.path());
            let plan = bound_plan(&path);
            let digest = fixture_digest(&plan);
            let mut dispatched = false;
            let mut run = |request: &ProcessRequest<'_>| {
                if request.args.contains(&OsString::from("--id")) {
                    dispatched = true;
                    ProcessResult {
                        status: status.clone(),
                        ..success("restart_required")
                    }
                } else if dispatched && request.args.contains(&OsString::from("--scope")) {
                    success("No available upgrade found.")
                } else {
                    fake_probe(request)
                }
            };
            let report = execute_software_update_with(
                &mut run,
                path.as_os_str(),
                &plan,
                &digest,
                &root.path().join("audit/software.jsonl"),
                SupportAppendFault::NONE,
                &NoopCancelObserver,
            )
            .expect("report");
            assert_eq!(report.outcome, expected);
            assert_eq!(report.updated, expected == SoftwareUpdateOutcome::Updated);
        }
    }

    #[test]
    fn post_dispatch_source_configuration_change_makes_success_observation_unknown() {
        let root = tempfile::tempdir().expect("fixture");
        let path = native_fixture(root.path());
        let plan = bound_plan(&path);
        let digest = fixture_digest(&plan);
        let mut dispatched = false;
        let mut run = |request: &ProcessRequest<'_>| {
            if request.args.contains(&OsString::from("--id")) {
                dispatched = true;
                success("completed")
            } else if dispatched && request.args.first().is_some_and(|arg| arg == "source") {
                success(&SOURCE.replace("\"Explicit\":false", "\"Explicit\":true"))
            } else if dispatched && request.args.contains(&OsString::from("--scope")) {
                success("No available upgrade found.")
            } else {
                fake_probe(request)
            }
        };
        let report = execute_software_update_with(
            &mut run,
            path.as_os_str(),
            &plan,
            &digest,
            &root.path().join("audit/software.jsonl"),
            SupportAppendFault::NONE,
            &NoopCancelObserver,
        )
        .expect("report");
        assert_eq!(report.outcome, SoftwareUpdateOutcome::Unknown);
        assert!(!report.updated);
        assert_eq!(report.error_code.as_deref(), Some("provider_changed"));
    }

    #[test]
    fn failed_or_ambiguous_post_dispatch_observations_preserve_unknown_reason() {
        let updated = ENGLISH.replace("7.43", "7.46");
        let target = updated.lines().nth(2).expect("target fixture row");
        for (observation, code) in [
            (
                ProcessResult {
                    status: ProcessStatus::Timeout,
                    ..success("")
                },
                "probe_failed",
            ),
            (
                ProcessResult {
                    output: ProcessOutput {
                        stderr_truncated: true,
                        ..success("No available upgrade found.").output
                    },
                    ..success("")
                },
                "probe_failed",
            ),
            (success("unrecognized output"), "probe_failed"),
            (
                success(&updated.replace("winget", "msstore")),
                "unsupported_source",
            ),
            (
                success(&updated.replace(target, &format!("{target}\n{target}"))),
                "ambiguous_package",
            ),
        ] {
            let root = tempfile::tempdir().expect("fixture");
            let path = native_fixture(root.path());
            let plan = bound_plan(&path);
            let digest = fixture_digest(&plan);
            let mut dispatches = 0;
            let mut run = |request: &ProcessRequest<'_>| {
                if request.args.contains(&OsString::from("--id")) {
                    dispatches += 1;
                    assert_eq!(request.timeout, Some(DEFAULT_EXECUTOR_COMMAND_TIMEOUT));
                    success("completed")
                } else if dispatches > 0 && request.args.contains(&OsString::from("--scope")) {
                    observation.clone()
                } else {
                    fake_probe(request)
                }
            };
            let report = execute_software_update_with(
                &mut run,
                path.as_os_str(),
                &plan,
                &digest,
                &root.path().join("audit/software.jsonl"),
                SupportAppendFault::NONE,
                &NoopCancelObserver,
            )
            .expect("post-dispatch report");
            assert_eq!(dispatches, 1);
            assert_eq!(report.outcome, SoftwareUpdateOutcome::Unknown, "{code}");
            assert!(!report.updated, "{code}");
            assert_eq!(report.error_code.as_deref(), Some(code));
            let records = std::fs::read_to_string(root.path().join("audit/software-support.jsonl"))
                .expect("durable result");
            assert!(
                records.contains("unknown") && records.contains(code),
                "{records}"
            );
        }
    }

    #[test]
    fn rows_match_inventory_by_exact_case_insensitive_name() {
        let inventory: SoftwareInventoryV1 = serde_json::from_value(serde_json::json!({
            "version": 1,
            "observed_at_unix_ms": 1,
            "sources": [],
            "entries": [{
                "id": "software:v1:arp:a",
                "identity": {"source": "arp", "hive": "current_user", "view": "registry64", "subkey": "Obsidian"},
                "scope": "current_user",
                "display_name": "obsidian",
                "provenance": [],
                "eligibility": {"state": "manual", "reason": "registry_only_manual"},
                "size": {"state": "unknown", "reason_code": "not_reported"},
                "last_used": {"state": "unknown", "reason_code": "no_supported_exact_source"}
            }],
            "fingerprint": format!("sha256:{}", "0".repeat(64)),
        }))
        .unwrap();
        let updates = rows(check_updates_with(
            |_| success(ENGLISH),
            Some(&inventory),
            None,
        ));
        let obsidian = updates
            .iter()
            .find(|row| row.id == "Obsidian.Obsidian")
            .unwrap();
        assert_eq!(obsidian.matched_software_ids, vec!["software:v1:arp:a"]);
        assert!(
            updates
                .iter()
                .filter(|row| row.id != "Obsidian.Obsidian")
                .all(|row| row.matched_software_ids.is_empty())
        );
    }
}
