//! Counts of durable per-item outcomes from the three primary journals.
use super::*;
use crate::{
    optimize::{MaintenanceActionClass, MaintenanceExecutionOutcome, OptimizeAuditTransition},
    software::{SoftwareAuditTransition, SoftwareExecutionOutcome},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessTotalV1 {
    pub succeeded: u64,
    pub lower_bound: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryTotalsV1 {
    pub clean: CleanMovedTotalsV1,
    pub software: SuccessTotalV1,
    pub optimize: SuccessTotalV1,
}

impl HistoryTotalsV1 {
    pub fn partial(&self) -> bool {
        self.clean.lower_bound || self.software.lower_bound || self.optimize.lower_bound
    }
}

pub fn history_totals() -> Result<HistoryTotalsV1, HistoryError> {
    Ok(history_totals_at(&history_store_paths()?))
}

// Read to the byte length observed at open. An in-flight tail never becomes a
// completed action. Bound each line individually so a long-lived valid journal
// is still fully counted. Keep only the per-item accumulator across lines.
fn visit_primary(
    domain: HistoryDomain,
    path: &Path,
    mut accept: impl FnMut(HistoryRecordV1),
) -> Result<bool, HistoryError> {
    use std::io::{BufRead, BufReader};
    let io_error = |source| HistoryError::Io {
        stage: "read_totals",
        path: path.to_owned(),
        source,
    };
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(io_error(e)),
    };
    let length = file.metadata().map_err(io_error)?.len();
    let mut reader = BufReader::new(file.take(length));
    let mut line = Vec::new();
    let mut incomplete = false;
    loop {
        line.clear();
        let count = (&mut reader)
            .take(MAX_NEW_AUDIT_BYTES + 1)
            .read_until(b'\n', &mut line)
            .map_err(io_error)?;
        if count == 0 {
            break;
        }
        if count as u64 > MAX_NEW_AUDIT_BYTES || line.last() != Some(&b'\n') {
            incomplete = true;
            if line.last() != Some(&b'\n') {
                reader.skip_until(b'\n').map_err(io_error)?;
            }
            continue;
        }
        let Ok(text) = std::str::from_utf8(&line) else {
            incomplete = true;
            continue;
        };
        let text = text.trim_end_matches(['\r', '\n']);
        if text.is_empty() {
            continue;
        }
        match decode_line(domain, text) {
            LineDecode::Record(record) => accept(record),
            _ => incomplete = true,
        }
    }
    Ok(incomplete)
}

// A duplicate terminal can repeat a durable outcome. Conflicting terminals,
// invalid evidence and started-only items cannot establish a completion.
struct Item<T> {
    terminal: Option<T>,
    invalid: bool,
}

impl<T: Copy + Eq> Item<T> {
    fn new() -> Self {
        Self {
            terminal: None,
            invalid: false,
        }
    }

    fn observe(&mut self, valid: bool, outcome: Option<T>) {
        self.invalid |= !valid;
        if let Some(outcome) = outcome {
            self.invalid |= self.terminal.is_some_and(|old| old != outcome);
            self.terminal = Some(outcome);
        }
    }
}

fn counts<K, T: Eq>(
    items: BTreeMap<K, Item<T>>,
    success: T,
    mut lower_bound: bool,
) -> SuccessTotalV1 {
    let mut succeeded = 0;
    for item in items.into_values() {
        if item.invalid || item.terminal.is_none() {
            lower_bound = true;
        } else if item.terminal.as_ref() == Some(&success) {
            succeeded += 1;
        }
    }
    SuccessTotalV1 {
        succeeded,
        lower_bound,
    }
}

pub(super) fn clean_at(path: &Path) -> Result<CleanMovedTotalsV1, HistoryError> {
    let mut moves = BTreeMap::new();
    let incomplete = visit_primary(HistoryDomain::Clean, path, |record| {
        let HistoryRecordV1::Clean {
            record:
                CleanAuditRecordV1::ExecutionTransition {
                    domain,
                    operation_id,
                    outcome_code,
                    evidence,
                    error_code,
                    ..
                },
        } = record
        else {
            return;
        };
        if evidence.action_kind != RedactedActionKind::MoveToTrash {
            return;
        }
        let valid = domain == "clean"
            && !operation_id.is_empty()
            && !evidence.target_identity_sha256.is_empty()
            && (outcome_code != ExecutionOutcomeCode::Succeeded || error_code.is_none());
        let item = moves
            .entry((operation_id, evidence.target_identity_sha256))
            .or_insert_with(Item::new);
        let terminal = (outcome_code != ExecutionOutcomeCode::Started).then_some((
            outcome_code,
            evidence.capacity_class,
            evidence.estimated_bytes,
        ));
        item.observe(valid, terminal);
    })?;
    let mut totals = CleanMovedTotalsV1 {
        known_bytes: 0,
        unknown_records: 0,
        lower_bound: incomplete,
    };
    for item in moves.into_values() {
        if item.invalid || item.terminal.is_none() {
            totals.lower_bound = true;
            continue;
        }
        let Some((ExecutionOutcomeCode::Succeeded, capacity, size)) = item.terminal else {
            continue;
        };
        match size {
            Some(bytes) if capacity != CapacityClass::Unknown => {
                let (sum, overflow) = totals.known_bytes.overflowing_add(bytes);
                totals.known_bytes = if overflow { u64::MAX } else { sum };
                totals.lower_bound |= overflow || capacity == CapacityClass::Partial;
            }
            _ => {
                totals.unknown_records = totals.unknown_records.saturating_add(1);
                totals.lower_bound = true;
            }
        }
    }
    Ok(totals)
}

fn history_totals_at(paths: &HistoryStorePaths) -> HistoryTotalsV1 {
    let clean = clean_at(&paths.clean).unwrap_or(CleanMovedTotalsV1 {
        known_bytes: 0,
        unknown_records: 0,
        lower_bound: true,
    });
    let mut items = BTreeMap::new();
    let incomplete = visit_primary(HistoryDomain::Software, &paths.software, |record| {
        if let HistoryRecordV1::Software { record } = record {
            let valid = record.valid_for_history();
            let outcome = match record.transition {
                SoftwareAuditTransition::Terminal { outcome } => Some(outcome),
                _ => None,
            };
            items
                .entry((record.operation_id, record.identity))
                .or_insert_with(Item::new)
                .observe(valid, outcome);
        }
    })
    .unwrap_or(true);
    let software = counts(items, SoftwareExecutionOutcome::Removed, incomplete);
    let mut items = BTreeMap::new();
    let incomplete = visit_primary(HistoryDomain::Optimize, &paths.optimize, |record| {
        if let HistoryRecordV1::Optimize { record } = record {
            let valid = record.valid_for_history();
            if record.action_class != MaintenanceActionClass::Execute && valid {
                return;
            }
            let outcome = match record.transition {
                OptimizeAuditTransition::Terminal { outcome } => Some(outcome),
                OptimizeAuditTransition::Skipped { .. } => {
                    Some(MaintenanceExecutionOutcome::CanceledBeforeStart)
                }
                _ => None,
            };
            items
                .entry((record.operation_id, record.catalogue_id))
                .or_insert_with(Item::new)
                .observe(valid, outcome);
        }
    })
    .unwrap_or(true);
    let optimize = counts(items, MaintenanceExecutionOutcome::Succeeded, incomplete);
    HistoryTotalsV1 {
        clean,
        software,
        optimize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn write_rows(path: &Path, rows: &[Value]) {
        fs::write(
            path,
            rows.iter()
                .map(|row| format!("{row}\n"))
                .collect::<String>(),
        )
        .unwrap();
    }

    #[test]
    fn totals_use_valid_per_item_terminals_with_conflicts_and_incomplete_stores() {
        let root = tempfile::tempdir().unwrap();
        let p = HistoryStorePaths {
            clean: root.path().join("clean"),
            software: root.path().join("software"),
            optimize: root.path().join("optimize"),
            status: root.path().join("status"),
        };
        let digest = format!("sha256:{}", "a".repeat(64));
        let item = json!({"schema_version":1,"domain":"software","operation_id":"batch","timestamp_unix_ms":12,
          "identity":{"source":"msix","package_full_name":"Alpha.Fixture_1.0.0.0_x64__8wekyb3d8bbwe"},
          "inventory_fingerprint":digest,"preview_digest":digest,"irreversible":true,"status_code":"removed",
          "transition":{"kind":"terminal","outcome":"removed"},"adapter_outcome":"success","installed_state":"absent",
          "requery_result":"absent","reboot_evidence":"none"});
        let mut other = item.clone();
        other["identity"]["package_full_name"] = "Beta.Fixture_1.0.0.0_x64__8wekyb3d8bbwe".into();
        let mut present = item.clone();
        for key in ["installed_state", "requery_result"] {
            present[key] = "present".into();
        }
        present["status_code"] = "still_present".into();
        present["transition"]["outcome"] = "still_present".into();
        write_rows(&p.software, &[item.clone(), item.clone(), other.clone()]);
        let optimized = json!({"schema_version":1,"domain":"optimize","operation_id":"fixture-low","timestamp_unix_ms":21,
            "catalogue_version":1,"catalogue_id":"dns.flush","action_class":"execute","preview_digest":digest,
            "transition":{"kind":"terminal","outcome":"succeeded"},"status_code":"succeeded","adapter_outcome":"success"});
        let mut settings = optimized.clone();
        settings["catalogue_id"] = "settings.search".into();
        settings["action_class"] = "settings_handoff".into();
        settings["transition"]["outcome"] = "launched".into();
        settings["status_code"] = "launched".into();
        let mut skipped = optimized.clone();
        skipped["operation_id"] = "fixture-skip".into();
        skipped["batch_id"] = "batch-skip".into();
        skipped["status_code"] = "skipped".into();
        skipped["transition"] = json!({"kind":"skipped","reason":{"kind":"canceled"}});
        skipped.as_object_mut().unwrap().remove("adapter_outcome");
        write_rows(
            &p.optimize,
            &[optimized.clone(), optimized, settings, skipped],
        );
        let totals = history_totals_at(&p);
        assert_eq!(
            totals.software,
            SuccessTotalV1 {
                succeeded: 2,
                lower_bound: false
            }
        );
        assert_eq!(
            totals.optimize,
            SuccessTotalV1 {
                succeeded: 1,
                lower_bound: false
            }
        );
        assert!(!totals.partial());
        write_rows(&p.software, &[item, present, other]);
        fs::write(&p.optimize, b"{\"schema_version\":2}\ntruncated").unwrap();
        let totals = history_totals_at(&p);
        assert_eq!(
            totals.software,
            SuccessTotalV1 {
                succeeded: 1,
                lower_bound: true
            }
        );
        assert_eq!(
            totals.optimize,
            SuccessTotalV1 {
                succeeded: 0,
                lower_bound: true
            }
        );
        assert!(!totals.clean.lower_bound);
    }
}
