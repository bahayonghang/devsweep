//! Versioned, non-replayable Clean/Software/Optimize audit history.

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    execution::{
        CapacityClass, CleanAuditError, CleanAuditRecordV1, ExecutionOutcomeCode,
        ProtectionOutcomeCode, RedactedActionKind, clean_audit_v1_path,
    },
    optimize::{OptimizeAuditError, OptimizeAuditRecordV1, optimize_audit_v1_path},
    software::{SoftwareAuditError, SoftwareAuditRecordV1, software_audit_v1_path},
};

const FORBIDDEN_KEYS: &[&str] = &[
    "argv",
    "args",
    "program",
    "cwd",
    "env",
    "environment",
    "action_path",
    "plan",
    "message",
    "command",
    "path",
];

/// The three fixed V1 audit domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryDomain {
    Clean,
    Software,
    Optimize,
}

impl HistoryDomain {
    const ALL: [Self; 3] = [Self::Clean, Self::Software, Self::Optimize];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Software => "software",
            Self::Optimize => "optimize",
        }
    }
}

/// Availability of one fixed store. Unknown versions are never executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryStoreState {
    Available,
    Empty,
    Unavailable,
    Unsupported,
}

/// One store's truthful availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryStoreStatusV1 {
    pub domain: HistoryDomain,
    pub state: HistoryStoreState,
    pub reason_code: Option<String>,
}

/// One operation row for `history list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryOperationSummaryV1 {
    pub domain: HistoryDomain,
    pub operation_id: String,
    pub first_timestamp_unix_ms: u64,
    pub last_timestamp_unix_ms: u64,
    pub outcome_code: String,
    pub stable_codes: Vec<String>,
    pub partial: bool,
    pub unknown: bool,
    pub unsupported: bool,
}

/// One redacted history record. It never carries executable argv or raw paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "history_kind", rename_all = "snake_case")]
pub enum HistoryRecordV1 {
    Clean {
        record: CleanAuditRecordV1,
    },
    Software {
        record: SoftwareAuditRecordV1,
    },
    Optimize {
        record: OptimizeAuditRecordV1,
    },
    Unsupported {
        domain: HistoryDomain,
        operation_id: Option<String>,
        schema_version: Option<u64>,
        reason_code: &'static str,
    },
}

/// `history list` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistoryListV1 {
    pub operations: Vec<HistoryOperationSummaryV1>,
    pub stores: Vec<HistoryStoreStatusV1>,
}

/// `history show` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistoryDetailV1 {
    pub summary: HistoryOperationSummaryV1,
    pub records: Vec<HistoryRecordV1>,
}

/// Cumulative bytes that Clean moved to the Recycle Bin, from the Clean V1 store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanMovedTotalsV1 {
    /// Sum of recorded `estimated_bytes` on succeeded trash moves.
    pub known_bytes: u64,
    /// Succeeded trash moves without a recorded size.
    pub unknown_records: u32,
    /// True when `known_bytes` is a lower bound: an unknown record, a partial
    /// size, or an unreadable store line exists.
    pub lower_bound: bool,
}

/// List filters. The reader never walks directories or legacy paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryListOptions {
    pub domain: Option<HistoryDomain>,
    pub limit: u32,
}

impl Default for HistoryListOptions {
    fn default() -> Self {
        Self {
            domain: None,
            limit: 100,
        }
    }
}

/// Fail-closed history errors.
#[derive(Debug)]
pub enum HistoryError {
    LocalAppDataUnavailable,
    NotFound {
        operation_id: String,
        stores_incomplete: bool,
    },
    Io {
        stage: &'static str,
        path: PathBuf,
        source: io::Error,
    },
}

impl HistoryError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::LocalAppDataUnavailable => "history_store_unavailable",
            Self::NotFound {
                stores_incomplete: true,
                ..
            } => "history_store_unavailable",
            Self::NotFound { .. } => "history_not_found",
            Self::Io { .. } => "history_store_unavailable",
        }
    }
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalAppDataUnavailable => formatter
                .write_str("LOCALAPPDATA is unavailable; history cannot open the V1 stores"),
            Self::NotFound {
                operation_id,
                stores_incomplete,
            } => {
                if *stores_incomplete {
                    write!(
                        formatter,
                        "operation {operation_id} was not found and at least one history store is unavailable"
                    )
                } else {
                    write!(formatter, "operation {operation_id} was not found")
                }
            }
            Self::Io {
                stage,
                path,
                source,
            } => write!(
                formatter,
                "history {stage} failed for {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for HistoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// The three fixed store paths from the shared application-data resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryStorePaths {
    pub clean: PathBuf,
    pub software: PathBuf,
    pub optimize: PathBuf,
}

impl HistoryStorePaths {
    fn for_domain(&self, domain: HistoryDomain) -> &Path {
        match domain {
            HistoryDomain::Clean => &self.clean,
            HistoryDomain::Software => &self.software,
            HistoryDomain::Optimize => &self.optimize,
        }
    }
}

/// Resolve the three fixed V1 stores. This never probes legacy audit files.
pub fn history_store_paths() -> Result<HistoryStorePaths, HistoryError> {
    let clean = clean_audit_v1_path().map_err(map_clean_path_error)?;
    let software = software_audit_v1_path().map_err(map_software_path_error)?;
    let optimize = optimize_audit_v1_path().map_err(map_optimize_path_error)?;
    Ok(HistoryStorePaths {
        clean,
        software,
        optimize,
    })
}

fn map_clean_path_error(error: CleanAuditError) -> HistoryError {
    match error {
        CleanAuditError::LocalAppDataUnavailable => HistoryError::LocalAppDataUnavailable,
        other => HistoryError::Io {
            stage: "resolve_clean_store",
            path: PathBuf::from("<clean>"),
            source: io::Error::other(other.to_string()),
        },
    }
}

fn map_software_path_error(error: SoftwareAuditError) -> HistoryError {
    match error {
        SoftwareAuditError::LocalAppDataUnavailable => HistoryError::LocalAppDataUnavailable,
        other => HistoryError::Io {
            stage: "resolve_software_store",
            path: PathBuf::from("<software>"),
            source: io::Error::other(other.to_string()),
        },
    }
}

fn map_optimize_path_error(error: OptimizeAuditError) -> HistoryError {
    match error {
        OptimizeAuditError::LocalAppDataUnavailable => HistoryError::LocalAppDataUnavailable,
        other => HistoryError::Io {
            stage: "resolve_optimize_store",
            path: PathBuf::from("<optimize>"),
            source: io::Error::other(other.to_string()),
        },
    }
}

/// List redacted operations from the three fixed stores.
pub fn list_history(options: HistoryListOptions) -> Result<HistoryListV1, HistoryError> {
    list_history_at(&history_store_paths()?, options)
}

/// Show one redacted operation. History never retries or replays the action.
pub fn show_history(operation_id: &str) -> Result<HistoryDetailV1, HistoryError> {
    show_history_at(&history_store_paths()?, operation_id)
}

/// Sum the bytes that succeeded Clean trash moves recorded. Read-only.
pub fn clean_moved_totals() -> Result<CleanMovedTotalsV1, HistoryError> {
    clean_moved_totals_at(&history_store_paths()?.clean)
}

pub(crate) fn clean_moved_totals_at(path: &Path) -> Result<CleanMovedTotalsV1, HistoryError> {
    let (status, records) = read_store(HistoryDomain::Clean, path)?;
    let mut totals = CleanMovedTotalsV1 {
        known_bytes: 0,
        unknown_records: 0,
        lower_bound: status.reason_code.is_some(),
    };
    for record in &records {
        let HistoryRecordV1::Clean {
            record:
                CleanAuditRecordV1::ExecutionTransition {
                    outcome_code: ExecutionOutcomeCode::Succeeded,
                    evidence,
                    ..
                },
        } = record
        else {
            continue;
        };
        if evidence.action_kind != RedactedActionKind::MoveToTrash {
            continue;
        }
        match evidence.estimated_bytes {
            Some(bytes) => {
                totals.known_bytes = totals.known_bytes.saturating_add(bytes);
                if evidence.capacity_class == CapacityClass::Partial {
                    totals.lower_bound = true;
                }
            }
            None => {
                totals.unknown_records = totals.unknown_records.saturating_add(1);
                totals.lower_bound = true;
            }
        }
    }
    Ok(totals)
}

pub(crate) fn list_history_at(
    paths: &HistoryStorePaths,
    options: HistoryListOptions,
) -> Result<HistoryListV1, HistoryError> {
    let mut operations = Vec::new();
    let mut stores = Vec::new();
    let domains = options
        .domain
        .map(|domain| vec![domain])
        .unwrap_or_else(|| HistoryDomain::ALL.to_vec());
    for domain in domains {
        let (status, records) = read_store(domain, paths.for_domain(domain))?;
        stores.push(status);
        operations.extend(summaries_from(domain, &records));
    }
    operations.sort_by(|left, right| {
        right
            .last_timestamp_unix_ms
            .cmp(&left.last_timestamp_unix_ms)
            .then_with(|| left.operation_id.cmp(&right.operation_id))
    });
    let limit = options.limit.max(1) as usize;
    if operations.len() > limit {
        operations.truncate(limit);
    }
    Ok(HistoryListV1 { operations, stores })
}

pub(crate) fn show_history_at(
    paths: &HistoryStorePaths,
    operation_id: &str,
) -> Result<HistoryDetailV1, HistoryError> {
    let mut found = None;
    let mut stores_incomplete = false;
    for domain in HistoryDomain::ALL {
        let (status, records) = read_store(domain, paths.for_domain(domain))?;
        if matches!(
            status.state,
            HistoryStoreState::Unavailable | HistoryStoreState::Unsupported
        ) {
            stores_incomplete = true;
        }
        let matched: Vec<HistoryRecordV1> = records
            .into_iter()
            .filter(|record| record_operation_id(record).as_deref() == Some(operation_id))
            .collect();
        if !matched.is_empty() {
            let summaries = summaries_from(domain, &matched);
            found = Some((
                summaries
                    .into_iter()
                    .next()
                    .expect("matching records form one summary"),
                matched,
            ));
            break;
        }
    }
    match found {
        Some((summary, records)) => Ok(HistoryDetailV1 { summary, records }),
        None => Err(HistoryError::NotFound {
            operation_id: operation_id.to_string(),
            stores_incomplete,
        }),
    }
}

fn read_store(
    domain: HistoryDomain,
    path: &Path,
) -> Result<(HistoryStoreStatusV1, Vec<HistoryRecordV1>), HistoryError> {
    if !path.exists() {
        return Ok((
            HistoryStoreStatusV1 {
                domain,
                state: HistoryStoreState::Empty,
                reason_code: None,
            },
            Vec::new(),
        ));
    }
    let bytes = fs::read(path).map_err(|source| HistoryError::Io {
        stage: "read_store",
        path: path.to_path_buf(),
        source,
    })?;
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Ok((
            HistoryStoreStatusV1 {
                domain,
                state: HistoryStoreState::Unavailable,
                reason_code: Some("invalid_utf8".to_string()),
            },
            Vec::new(),
        ));
    };
    let mut records = Vec::new();
    let mut saw_unsupported = false;
    let mut saw_corrupt = false;
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        match decode_line(domain, line) {
            LineDecode::Record(record) => records.push(redact_record(record)),
            LineDecode::Unsupported(record) => {
                saw_unsupported = true;
                records.push(record);
            }
            LineDecode::Corrupt => saw_corrupt = true,
        }
    }
    let state = if saw_corrupt && records.is_empty() {
        HistoryStoreState::Unavailable
    } else if saw_unsupported
        && records
            .iter()
            .all(|record| matches!(record, HistoryRecordV1::Unsupported { .. }))
    {
        HistoryStoreState::Unsupported
    } else if records.is_empty() {
        HistoryStoreState::Empty
    } else {
        HistoryStoreState::Available
    };
    let reason_code = if saw_corrupt {
        Some("corrupt_record".to_string())
    } else if saw_unsupported {
        Some("unknown_schema".to_string())
    } else {
        None
    };
    Ok((
        HistoryStoreStatusV1 {
            domain,
            state,
            reason_code,
        },
        records,
    ))
}

enum LineDecode {
    Record(HistoryRecordV1),
    Unsupported(HistoryRecordV1),
    Corrupt,
}

fn decode_line(domain: HistoryDomain, line: &str) -> LineDecode {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return LineDecode::Corrupt;
    };
    if json_contains_forbidden_key(&value) {
        return LineDecode::Unsupported(HistoryRecordV1::Unsupported {
            domain,
            operation_id: value
                .get("operation_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            schema_version: value
                .get("schema_version")
                .and_then(serde_json::Value::as_u64),
            reason_code: "redacted_forbidden_fields",
        });
    }
    let schema_version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64);
    if schema_version != Some(1) {
        return LineDecode::Unsupported(HistoryRecordV1::Unsupported {
            domain,
            operation_id: value
                .get("operation_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            schema_version,
            reason_code: "unknown_schema",
        });
    }
    let decoded = match domain {
        HistoryDomain::Clean => serde_json::from_str::<CleanAuditRecordV1>(line)
            .ok()
            .map(|record| HistoryRecordV1::Clean { record }),
        HistoryDomain::Software => serde_json::from_str::<SoftwareAuditRecordV1>(line)
            .ok()
            .map(|record| HistoryRecordV1::Software { record }),
        HistoryDomain::Optimize => serde_json::from_str::<OptimizeAuditRecordV1>(line)
            .ok()
            .map(|record| HistoryRecordV1::Optimize { record }),
    };
    match decoded {
        Some(record) => LineDecode::Record(record),
        None => LineDecode::Unsupported(HistoryRecordV1::Unsupported {
            domain,
            operation_id: value
                .get("operation_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            schema_version,
            reason_code: "unknown_schema",
        }),
    }
}

fn redact_record(record: HistoryRecordV1) -> HistoryRecordV1 {
    let Ok(value) = serde_json::to_value(&record) else {
        return HistoryRecordV1::Unsupported {
            domain: record_domain(&record),
            operation_id: record_operation_id(&record),
            schema_version: Some(1),
            reason_code: "redacted_forbidden_fields",
        };
    };
    if json_contains_forbidden_key(&value) {
        return HistoryRecordV1::Unsupported {
            domain: record_domain(&record),
            operation_id: record_operation_id(&record),
            schema_version: Some(1),
            reason_code: "redacted_forbidden_fields",
        };
    }
    record
}

fn json_contains_forbidden_key(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, child)| {
            FORBIDDEN_KEYS.contains(&key.as_str()) || json_contains_forbidden_key(child)
        }),
        serde_json::Value::Array(items) => items.iter().any(json_contains_forbidden_key),
        _ => false,
    }
}

fn record_domain(record: &HistoryRecordV1) -> HistoryDomain {
    match record {
        HistoryRecordV1::Clean { .. } => HistoryDomain::Clean,
        HistoryRecordV1::Software { .. } => HistoryDomain::Software,
        HistoryRecordV1::Optimize { .. } => HistoryDomain::Optimize,
        HistoryRecordV1::Unsupported { domain, .. } => *domain,
    }
}

fn record_operation_id(record: &HistoryRecordV1) -> Option<String> {
    match record {
        HistoryRecordV1::Clean { record } => Some(match record {
            CleanAuditRecordV1::ExecutionTransition { operation_id, .. }
            | CleanAuditRecordV1::ProtectionMutation { operation_id, .. } => operation_id.clone(),
        }),
        HistoryRecordV1::Software { record } => Some(record.operation_id.clone()),
        HistoryRecordV1::Optimize { record } => Some(record.operation_id.clone()),
        HistoryRecordV1::Unsupported { operation_id, .. } => operation_id.clone(),
    }
}

fn record_timestamp(record: &HistoryRecordV1) -> u64 {
    match record {
        HistoryRecordV1::Clean { record } => match record {
            CleanAuditRecordV1::ExecutionTransition {
                timestamp_epoch_ms, ..
            }
            | CleanAuditRecordV1::ProtectionMutation {
                timestamp_epoch_ms, ..
            } => *timestamp_epoch_ms,
        },
        HistoryRecordV1::Software { record } => record.timestamp_unix_ms,
        HistoryRecordV1::Optimize { record } => record.timestamp_unix_ms,
        HistoryRecordV1::Unsupported { .. } => 0,
    }
}

fn json_code<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn record_codes(record: &HistoryRecordV1) -> Vec<String> {
    match record {
        HistoryRecordV1::Clean { record } => match record {
            CleanAuditRecordV1::ExecutionTransition { outcome_code, .. } => {
                vec![json_code(*outcome_code)]
            }
            CleanAuditRecordV1::ProtectionMutation { outcome_code, .. } => {
                vec![protection_outcome(*outcome_code).to_string()]
            }
        },
        HistoryRecordV1::Software { record } => vec![json_code(record.status_code)],
        HistoryRecordV1::Optimize { record } => vec![json_code(record.status_code)],
        HistoryRecordV1::Unsupported { reason_code, .. } => vec![(*reason_code).to_string()],
    }
}

fn protection_outcome(code: ProtectionOutcomeCode) -> &'static str {
    match code {
        ProtectionOutcomeCode::Requested => "requested",
        ProtectionOutcomeCode::Committed => "committed",
        ProtectionOutcomeCode::Failed => "failed",
    }
}

fn summaries_from(
    domain: HistoryDomain,
    records: &[HistoryRecordV1],
) -> Vec<HistoryOperationSummaryV1> {
    let mut groups: BTreeMap<String, Vec<&HistoryRecordV1>> = BTreeMap::new();
    for record in records {
        let operation_id = record_operation_id(record).unwrap_or_else(|| "unsupported".to_string());
        groups.entry(operation_id).or_default().push(record);
    }
    groups
        .into_iter()
        .map(|(operation_id, group)| {
            let timestamps: Vec<u64> = group
                .iter()
                .map(|record| record_timestamp(record))
                .collect();
            let first = timestamps.iter().copied().min().unwrap_or(0);
            let last = timestamps.iter().copied().max().unwrap_or(0);
            let mut stable_codes = Vec::new();
            for record in &group {
                for code in record_codes(record) {
                    if !stable_codes.contains(&code) {
                        stable_codes.push(code);
                    }
                }
            }
            let unsupported = group
                .iter()
                .any(|record| matches!(record, HistoryRecordV1::Unsupported { .. }));
            let unknown = stable_codes.iter().any(|code| {
                code.contains("unknown") || code == "unsupported" || code == "unknown_schema"
            });
            let partial = matches!(domain, HistoryDomain::Clean)
                && stable_codes.iter().any(|code| code == "failed")
                && stable_codes
                    .iter()
                    .any(|code| code == "succeeded" || code == "committed" || code == "started");
            let outcome_code = stable_codes
                .last()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            HistoryOperationSummaryV1 {
                domain,
                operation_id,
                first_timestamp_unix_ms: first,
                last_timestamp_unix_ms: last,
                outcome_code,
                stable_codes,
                partial,
                unknown,
                unsupported,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(path, contents).expect("write");
    }

    fn paths(root: &Path) -> HistoryStorePaths {
        HistoryStorePaths {
            clean: root.join("DevSweep/audit/v1/clean.jsonl"),
            software: root.join("DevSweep/audit/v1/software.jsonl"),
            optimize: root.join("DevSweep/audit/v1/optimize.jsonl"),
        }
    }

    #[test]
    fn history_reader_opens_only_the_three_fixed_v1_stores() {
        let fixture = TempDir::new().expect("temp");
        let stores = paths(fixture.path());
        let legacy = fixture.path().join("devsweep/audit.jsonl");
        let explicit = fixture.path().join("explicit-audit.jsonl");
        let legacy_bytes =
            include_bytes!("../../tests/fixtures/history/clean-v1/legacy-non-discovery.jsonl");
        write(&legacy, std::str::from_utf8(legacy_bytes).expect("utf8"));
        write(&explicit, r#"{"command":"rm","argv":["-rf","/secret"]}"#);
        write(
            &stores.clean,
            include_str!("../../tests/fixtures/history/clean-v1/protection-mutation.jsonl"),
        );

        let listed = list_history_at(&stores, HistoryListOptions::default()).expect("list");
        assert_eq!(listed.operations.len(), 1);
        assert_eq!(listed.operations[0].operation_id, "op-protect-v1-fixture");
        let encoded = serde_json::to_string(&listed).expect("json");
        assert!(!encoded.contains("path"));
        assert!(!encoded.contains("argv"));
        assert!(!encoded.contains("/secret"));
        assert_eq!(fs::read(&legacy).expect("legacy"), legacy_bytes);
        assert!(
            fs::read_to_string(&explicit)
                .expect("explicit")
                .contains("argv")
        );
    }

    #[test]
    fn history_reader_redacts_protection_mutations_without_raw_paths() {
        let fixture = TempDir::new().expect("temp");
        let stores = paths(fixture.path());
        write(
            &stores.clean,
            include_str!("../../tests/fixtures/history/clean-v1/protection-mutation.jsonl"),
        );
        let detail = show_history_at(&stores, "op-protect-v1-fixture").expect("show");
        let encoded = serde_json::to_string(&detail).expect("json");
        assert!(encoded.contains("identity_sha256"));
        assert!(!encoded.contains("C:\\"));
        assert!(!encoded.contains("path"));
        assert!(!encoded.contains("argv"));
        match &detail.records[0] {
            HistoryRecordV1::Clean {
                record: CleanAuditRecordV1::ProtectionMutation { evidence, .. },
            } => {
                assert_eq!(evidence.identity_sha256.len(), 64);
            }
            other => panic!("expected protection mutation, got {other:?}"),
        }
    }

    #[test]
    fn history_reader_tolerates_mixed_versions_and_rejects_unknown_schema() {
        let fixture = TempDir::new().expect("temp");
        let stores = paths(fixture.path());
        let mixed = format!(
            "{}{}",
            include_str!("../../tests/fixtures/history/clean-v1/execution-transition.jsonl"),
            include_str!("../../tests/fixtures/history/clean-v1/unknown-version.jsonl")
        );
        write(&stores.clean, &mixed);
        let listed = list_history_at(&stores, HistoryListOptions::default()).expect("list");
        assert!(
            listed
                .operations
                .iter()
                .any(|operation| operation.operation_id == "op-clean-v1-fixture")
        );
        assert!(
            listed
                .operations
                .iter()
                .any(|operation| operation.unsupported)
        );
        let encoded = serde_json::to_string(&listed).expect("json");
        assert!(!encoded.contains("future_kind"));
        assert!(!encoded.contains("preserved-unknown-version"));
    }

    #[test]
    fn history_reader_marks_corrupt_clean_store_unavailable() {
        let fixture = TempDir::new().expect("temp");
        let stores = paths(fixture.path());
        write(
            &stores.clean,
            include_str!("../../tests/fixtures/history/clean-v1/corrupt-line.jsonl"),
        );
        let listed = list_history_at(&stores, HistoryListOptions::default()).expect("list");
        let clean = listed
            .stores
            .iter()
            .find(|store| store.domain == HistoryDomain::Clean)
            .expect("clean store");
        assert_eq!(clean.state, HistoryStoreState::Unavailable);
    }

    #[test]
    fn history_reader_cannot_reconstruct_executable_argv() {
        let fixture = TempDir::new().expect("temp");
        let stores = paths(fixture.path());
        write(
            &stores.clean,
            r#"{"record_kind":"execution_transition","schema_version":1,"domain":"clean","operation_id":"hostile","timestamp_epoch_ms":1,"transition":1,"outcome_code":"started","error_code":null,"evidence":{"target_identity_sha256":"ab","action_kind":"command","irreversible":true,"capacity_class":"verified","duration_ms":null},"argv":["cmd.exe","/c","calc"]}"#,
        );
        let listed = list_history_at(
            &stores,
            HistoryListOptions {
                domain: Some(HistoryDomain::Clean),
                limit: 10,
            },
        )
        .expect("list");
        let encoded = serde_json::to_string(&listed).expect("json");
        assert!(!encoded.contains("cmd.exe"));
        assert!(!encoded.contains("argv"));
        assert!(
            listed
                .operations
                .iter()
                .any(|operation| operation.unsupported)
        );
    }

    fn transition(
        operation: &str,
        outcome: &str,
        action: &str,
        capacity: &str,
        bytes: Option<u64>,
    ) -> String {
        let bytes = bytes
            .map(|value| format!(r#","estimated_bytes":{value}"#))
            .unwrap_or_default();
        format!(
            r#"{{"record_kind":"execution_transition","schema_version":1,"domain":"clean","operation_id":"{operation}","timestamp_epoch_ms":1,"transition":2,"outcome_code":"{outcome}","error_code":null,"evidence":{{"target_identity_sha256":"ab","action_kind":"{action}","irreversible":false,"capacity_class":"{capacity}","duration_ms":3{bytes}}}}}"#
        ) + "\n"
    }

    #[test]
    fn clean_moved_totals_sum_succeeded_trash_moves_and_count_unknown_records() {
        let fixture = TempDir::new().expect("temp");
        let stores = paths(fixture.path());
        assert_eq!(
            clean_moved_totals_at(&stores.clean).expect("empty"),
            CleanMovedTotalsV1 {
                known_bytes: 0,
                unknown_records: 0,
                lower_bound: false,
            }
        );

        let exact = [
            transition("a", "succeeded", "move_to_trash", "verified", Some(1_000)),
            transition("a", "started", "move_to_trash", "verified", None),
            transition("a", "failed", "move_to_trash", "verified", None),
            transition("b", "succeeded", "move_to_trash", "verified", Some(24)),
            transition("b", "succeeded", "command", "verified", None),
        ]
        .concat();
        write(&stores.clean, &exact);
        assert_eq!(
            clean_moved_totals_at(&stores.clean).expect("exact"),
            CleanMovedTotalsV1 {
                known_bytes: 1_024,
                unknown_records: 0,
                lower_bound: false,
            }
        );

        let mixed = format!(
            "{exact}{}{}",
            include_str!("../../tests/fixtures/history/clean-v1/protection-mutation.jsonl"),
            transition("c", "succeeded", "move_to_trash", "verified", None),
        );
        write(&stores.clean, &mixed);
        assert_eq!(
            clean_moved_totals_at(&stores.clean).expect("old record"),
            CleanMovedTotalsV1 {
                known_bytes: 1_024,
                unknown_records: 1,
                lower_bound: true,
            }
        );

        write(
            &stores.clean,
            &transition("d", "succeeded", "move_to_trash", "partial", Some(8)),
        );
        let partial = clean_moved_totals_at(&stores.clean).expect("partial");
        assert_eq!(partial.known_bytes, 8);
        assert_eq!(partial.unknown_records, 0);
        assert!(partial.lower_bound);

        let encoded = serde_json::to_value(partial).expect("json");
        assert_eq!(
            encoded,
            serde_json::json!({"known_bytes": 8, "unknown_records": 0, "lower_bound": true})
        );
    }

    #[test]
    fn clean_moved_totals_mark_corrupt_lines_as_lower_bound() {
        let fixture = TempDir::new().expect("temp");
        let stores = paths(fixture.path());
        let text = format!(
            "{}not json\n",
            transition("a", "succeeded", "move_to_trash", "verified", Some(5))
        );
        write(&stores.clean, &text);
        let totals = clean_moved_totals_at(&stores.clean).expect("corrupt line");
        assert_eq!(totals.known_bytes, 5);
        assert!(totals.lower_bound);
    }

    #[test]
    fn history_show_missing_id_is_not_found_when_stores_are_readable() {
        let fixture = TempDir::new().expect("temp");
        let stores = paths(fixture.path());
        let error = show_history_at(&stores, "missing-op").expect_err("missing");
        assert_eq!(error.code(), "history_not_found");
    }
}
