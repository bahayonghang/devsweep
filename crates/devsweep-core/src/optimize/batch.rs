//! Ordered maintenance batches. Untrusted plans name catalogue rows only.
use std::{collections::HashSet, time::SystemTime};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    MaintenanceActionClass, MaintenanceExecutionError, MaintenanceExecutionOutcome,
    MaintenanceExecutor, OptimizeAuditErrorCode, OptimizeAuditRecordV1, OptimizeAuditStatusCode,
    OptimizeAuditTransition, OptimizePlanError, PrecheckReason, ValidatedMaintenanceAction,
    audit::{OptimizeAuditEvent, OptimizeAuditJournal},
    catalogue::OPTIMIZE_CATALOGUE_V2,
    catalogue_entry,
    execution::{next_operation_id, unix_ms},
    plan::construct_validated_action,
    plan_operation,
};
use crate::{digest_encoding::hex_digest, process::CancelObserver};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenanceBatchOperation {
    pub operation_id: String,
    pub action_class: MaintenanceActionClass,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaintenancePlanV2 {
    pub version: u32,
    pub catalogue_version: u32,
    pub operations: Vec<MaintenanceBatchOperation>,
}

impl MaintenancePlanV2 {
    pub fn new(ids: &[String]) -> Result<Self, OptimizePlanError> {
        let plan = Self {
            version: 2,
            catalogue_version: OPTIMIZE_CATALOGUE_V2,
            operations: ids
                .iter()
                .map(|id| MaintenanceBatchOperation {
                    operation_id: id.clone(),
                    action_class: MaintenanceActionClass::Execute,
                })
                .collect(),
        };
        plan.validate()?;
        Ok(plan)
    }

    fn validate(&self) -> Result<(), OptimizePlanError> {
        if self.version != 2 {
            return Err(OptimizePlanError::UnsupportedPlanVersion(self.version));
        }
        if self.catalogue_version != OPTIMIZE_CATALOGUE_V2 {
            return Err(OptimizePlanError::UnsupportedCatalogueVersion(
                self.catalogue_version,
            ));
        }
        let mut seen = HashSet::new();
        if self.operations.is_empty() {
            return Err(OptimizePlanError::InvalidBatchSelection);
        }
        for item in &self.operations {
            let entry = catalogue_entry(&item.operation_id)
                .ok_or_else(|| OptimizePlanError::UnknownOperation(item.operation_id.clone()))?;
            if entry.action_class != MaintenanceActionClass::Execute
                || item.action_class != entry.action_class
                || !seen.insert(entry.id)
            {
                return Err(OptimizePlanError::InvalidBatchSelection);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "detail",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum BatchSkipReason {
    Precheck(PrecheckReason),
    PlatformUnsupported,
    OsBuildUnavailable,
    BuildUnsupported,
    ResolverUnavailable,
    IdentityChanged,
    Canceled,
}

impl BatchSkipReason {
    /// Frontends present the same preflight reason in the two supported languages.
    pub fn label(self, chinese: bool) -> &'static str {
        let (en, zh) = match self {
            Self::Precheck(reason) => match reason {
                PrecheckReason::UacDeclined => {
                    ("Administrator privileges required", "需要管理员权限")
                }
                PrecheckReason::NotSsd => ("No eligible fixed SSD", "没有可用的固定固态硬盘"),
                PrecheckReason::LowBattery => ("Battery charge is low", "电池电量过低"),
                PrecheckReason::DiskProblem => ("Volume health check failed", "卷健康检查未通过"),
                PrecheckReason::RebootPending => ("Restart pending", "正在等待重启"),
                PrecheckReason::Cooldown => ("Recently completed", "近期已完成"),
                PrecheckReason::ProgramMissing => ("Required program is missing", "缺少所需程序"),
                PrecheckReason::ImageNotRepairable => (
                    "No evidence that the image can be repaired",
                    "缺少映像可修复的证据",
                ),
                PrecheckReason::BackupWriteFailed => {
                    ("Backup could not be written", "无法写入备份")
                }
            },
            Self::PlatformUnsupported => ("Requires Windows", "需要 Windows"),
            Self::OsBuildUnavailable => ("Windows build unavailable", "无法读取 Windows 版本"),
            Self::BuildUnsupported => ("Windows build is too old", "Windows 版本过低"),
            Self::ResolverUnavailable => (
                "Required program identity unavailable",
                "无法核实所需程序的身份",
            ),
            Self::IdentityChanged => ("Identity changed since preview", "预览后身份已变更"),
            Self::Canceled => ("Canceled before this item started", "已在此项开始前取消"),
        };
        if chinese { zh } else { en }
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::Precheck(reason) => reason.code(),
            Self::PlatformUnsupported => "platform_unsupported",
            Self::OsBuildUnavailable => "os_build_unavailable",
            Self::BuildUnsupported => "build_unsupported",
            Self::ResolverUnavailable => "resolver_unavailable",
            Self::IdentityChanged => "identity_changed",
            Self::Canceled => "canceled",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum BatchReadiness {
    Ready { digest: String },
    Skipped { reason: BatchSkipReason },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchPreviewItem {
    pub operation_id: String,
    #[serde(flatten)]
    pub state: BatchReadiness,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceBatchPreview {
    pub version: u32,
    pub operations: Vec<BatchPreviewItem>,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum BatchOutcome {
    Succeeded,
    Skipped {
        reason: BatchSkipReason,
    },
    Failed {
        reason: Option<OptimizeAuditErrorCode>,
    },
    UnknownAfterDispatch {
        reason: Option<OptimizeAuditErrorCode>,
    },
}

impl BatchOutcome {
    pub fn label(&self, chinese: bool) -> String {
        let pick = |en, zh| if chinese { zh } else { en };
        match self {
            Self::Succeeded => pick("Succeeded", "已完成").to_string(),
            Self::Skipped { reason } => {
                format!("{} · {}", pick("Skipped", "已跳过"), reason.label(chinese))
            }
            Self::Failed { reason } | Self::UnknownAfterDispatch { reason } => {
                let status = if matches!(self, Self::Failed { .. }) {
                    pick("Failed", "失败")
                } else {
                    pick("Outcome unknown", "结果未知")
                };
                let detail = match reason {
                    Some(OptimizeAuditErrorCode::AdapterDispatchFailed) => {
                        pick("Could not start", "无法启动")
                    }
                    Some(OptimizeAuditErrorCode::AdapterOperationFailed) => {
                        pick("Operation reported failure", "操作报告失败")
                    }
                    Some(OptimizeAuditErrorCode::AdapterTimedOut) => pick("Timed out", "已超时"),
                    Some(OptimizeAuditErrorCode::AdapterCanceledAfterDispatch) => {
                        pick("Canceled after dispatch", "派发后取消")
                    }
                    Some(OptimizeAuditErrorCode::RecoveredAfterCrash) => {
                        pick("Recovered after interruption", "中断后恢复")
                    }
                    None => pick("No further evidence", "无更多证据"),
                };
                format!("{status} · {detail}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchResultItem {
    pub catalogue_id: String,
    pub operation_id: String,
    #[serde(flatten)]
    pub outcome: BatchOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceBatchReport {
    pub version: u32,
    pub batch_id: String,
    pub outcomes: Vec<BatchResultItem>,
}

impl MaintenanceBatchReport {
    pub fn has_failures(&self) -> bool {
        self.outcomes.iter().any(|item| {
            matches!(
                item.outcome,
                BatchOutcome::Failed { .. } | BatchOutcome::UnknownAfterDispatch { .. }
            )
        })
    }
}

impl MaintenanceExecutor {
    fn resolve_batch_item(
        &self,
        id: &str,
    ) -> Result<Result<ValidatedMaintenanceAction, BatchSkipReason>, OptimizePlanError> {
        let one = plan_operation(id)?;
        match construct_validated_action(&one, self.preflight.as_ref()) {
            Ok(mut action) => {
                if let super::ResolvedMaintenanceAction::DeadStartup {
                    backup_body,
                    deletions,
                    ..
                } = &action.resolved
                {
                    // The legacy digest includes the names and backup path. A
                    // batch also binds the removal kind, targets and backup contents.
                    let identity = serde_json::to_vec(&(
                        "devsweep.optimize.batch.startup.v2",
                        &action.preview_digest,
                        backup_body,
                        deletions,
                    ))
                    .map_err(|_| OptimizePlanError::SerializationFailed)?;
                    action.preview_digest =
                        format!("sha256:{}", hex_digest(Sha256::digest(identity)));
                }
                Ok(Ok(action))
            }
            Err(error) => Ok(Err(match error {
                OptimizePlanError::PrecheckRefused { reason, .. } => {
                    BatchSkipReason::Precheck(reason)
                }
                OptimizePlanError::PlatformUnsupported => BatchSkipReason::PlatformUnsupported,
                OptimizePlanError::OsBuildUnavailable => BatchSkipReason::OsBuildUnavailable,
                OptimizePlanError::BuildUnsupported { .. } => BatchSkipReason::BuildUnsupported,
                OptimizePlanError::ResolverUnavailable => BatchSkipReason::ResolverUnavailable,
                other => return Err(other),
            })),
        }
    }

    pub fn preview_batch(
        &self,
        plan: &MaintenancePlanV2,
    ) -> Result<MaintenanceBatchPreview, OptimizePlanError> {
        plan.validate()?;
        let mut operations = Vec::with_capacity(plan.operations.len());
        for item in &plan.operations {
            let state = match self.resolve_batch_item(&item.operation_id)? {
                Ok(action) => BatchReadiness::Ready {
                    digest: action.preview_digest,
                },
                Err(reason) => BatchReadiness::Skipped { reason },
            };
            operations.push(BatchPreviewItem {
                operation_id: item.operation_id.clone(),
                state,
            });
        }
        let bytes = serde_json::to_vec(&("devsweep.optimize.batch.v2", plan, &operations))
            .map_err(|_| OptimizePlanError::SerializationFailed)?;
        Ok(MaintenanceBatchPreview {
            version: 2,
            operations,
            digest: format!("sha256:{}", hex_digest(Sha256::digest(bytes))),
        })
    }

    /// Hold both execution locks through the batch. Re-resolve each ready item
    /// after preceding actions and sample cancellation only at item boundaries.
    pub fn execute_batch(
        &self,
        plan: &MaintenancePlanV2,
        expected_digest: &str,
        confirmed: bool,
        cancel: Option<&dyn CancelObserver>,
        mut progress: impl FnMut(usize, &str),
    ) -> Result<MaintenanceBatchReport, MaintenanceExecutionError> {
        if !confirmed {
            return Err(MaintenanceExecutionError::ConfirmationRequired);
        }
        let (_permit, mut journal) = self.begin_execution()?;
        let preview = self.preview_batch(plan)?;
        if preview.digest != expected_digest {
            return Err(OptimizePlanError::DigestMismatch.into());
        }
        let batch_id = next_operation_id();
        journal.batch_id = Some(batch_id.clone());
        journal.batch_size = Some(preview.operations.len());
        let mut outcomes = Vec::with_capacity(preview.operations.len());
        for (index, item) in preview.operations.iter().enumerate() {
            progress(index, &item.operation_id);
            let live = if cancel.is_some_and(|flag| flag.is_cancel_requested()) {
                Err(BatchSkipReason::Canceled)
            } else {
                match &item.state {
                    BatchReadiness::Skipped { reason } => Err(*reason),
                    BatchReadiness::Ready { digest } => self
                        .resolve_batch_item(&item.operation_id)?
                        .and_then(|action| {
                            if &action.preview_digest == digest {
                                Ok(action)
                            } else {
                                Err(BatchSkipReason::IdentityChanged)
                            }
                        }),
                }
            };
            let result = match live {
                Ok(action) => {
                    let done = self.execute_one(&mut journal, &action, None)?;
                    let outcome = match done.outcome {
                        MaintenanceExecutionOutcome::Succeeded => BatchOutcome::Succeeded,
                        MaintenanceExecutionOutcome::Failed => BatchOutcome::Failed {
                            reason: done.error_code,
                        },
                        _ => BatchOutcome::UnknownAfterDispatch {
                            reason: done.error_code,
                        },
                    };
                    BatchResultItem {
                        catalogue_id: item.operation_id.clone(),
                        operation_id: done.operation_id,
                        outcome,
                    }
                }
                Err(reason) => {
                    record_skip(&mut journal, &item.operation_id, &preview.digest, reason)?
                }
            };
            outcomes.push(result);
        }
        Ok(MaintenanceBatchReport {
            version: 2,
            batch_id,
            outcomes,
        })
    }
}

fn record_skip(
    journal: &mut OptimizeAuditJournal,
    id: &str,
    digest: &str,
    reason: BatchSkipReason,
) -> Result<BatchResultItem, MaintenanceExecutionError> {
    let operation_id = next_operation_id();
    journal.append(OptimizeAuditRecordV1::new(
        &operation_id,
        unix_ms(SystemTime::now()),
        id,
        MaintenanceActionClass::Execute,
        digest,
        OptimizeAuditEvent {
            transition: OptimizeAuditTransition::Skipped { reason },
            status_code: OptimizeAuditStatusCode::Skipped,
            error_code: None,
            adapter_outcome: None,
        },
    ))?;
    Ok(BatchResultItem {
        catalogue_id: id.to_string(),
        operation_id,
        outcome: BatchOutcome::Skipped { reason },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::FlagCancelObserver;
    use tempfile::TempDir;

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn batch_rejects_invalid_shape_and_binding_order_and_checks_confirmation() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("audit.jsonl");
        let executor = MaintenanceExecutor::isolated_batch_fixture(&path);
        for bad in [
            &[][..],
            &["dns.flush", "dns.flush"],
            &["settings.search"],
            &["guidance.drive_optimize"],
            &["maintenance.disk_health"],
        ] {
            assert!(MaintenancePlanV2::new(&ids(bad)).is_err());
        }
        let original =
            MaintenancePlanV2::new(&ids(&["dns.flush", "maintenance.clock_probe"])).unwrap();
        let preview = executor.preview_batch(&original).unwrap();
        for changed in [
            &["maintenance.clock_probe", "dns.flush"][..],
            &["dns.flush"],
        ] {
            let new = MaintenancePlanV2::new(&ids(changed)).unwrap();
            assert_ne!(executor.preview_batch(&new).unwrap().digest, preview.digest);
            assert!(matches!(
                executor.execute_batch(&new, &preview.digest, true, None, |_, _| {}),
                Err(MaintenanceExecutionError::Plan(
                    OptimizePlanError::DigestMismatch
                ))
            ));
        }
        assert!(matches!(
            executor.execute_batch(&original, &preview.digest, false, None, |_, _| {}),
            Err(MaintenanceExecutionError::ConfirmationRequired)
        ));
        assert!(std::fs::read(path).unwrap().is_empty());
    }

    #[test]
    fn changed_prerequisites_never_upgrade_a_skip_or_dispatch_a_changed_identity() {
        use super::super::{
            LivePreflight, MaintenanceCatalogueEntryV1, ResolvedMaintenanceAction,
            execution::{AdapterEvidence, DispatchTiming, MaintenanceDispatch},
            maintenance::{DeadStartupDeletion, DeadStartupKind},
        };
        use std::sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        };
        struct Changing {
            changed: AtomicBool,
            calls: AtomicUsize,
        }
        impl LivePreflight for Changing {
            fn resolve(
                &self,
                entry: &MaintenanceCatalogueEntryV1,
            ) -> Result<ResolvedMaintenanceAction, OptimizePlanError> {
                let changed = self.changed.load(Ordering::SeqCst);
                if entry.id == "maintenance.dead_startup" {
                    return Ok(ResolvedMaintenanceAction::DeadStartup {
                        backup_path: "backup.reg".into(),
                        backup_body: "fixture".to_string(),
                        deletions: vec![DeadStartupDeletion {
                            name: "Same name".to_string(),
                            kind: DeadStartupKind::StartupLink,
                            target: if changed { "new.exe" } else { "old.exe" }.into(),
                            link_path: Some("unchanged.lnk".into()),
                        }],
                        skip_names: Vec::new(),
                    });
                }
                if entry.id == "maintenance.retrim" && !changed
                    || entry.id == "dns.flush" && changed
                {
                    return Err(OptimizePlanError::PrecheckRefused {
                        operation_id: entry.id.to_string(),
                        reason: PrecheckReason::UacDeclined,
                    });
                }
                Ok(ResolvedMaintenanceAction::NativeCommand {
                    program: if changed { "changed" } else { "reviewed" }.into(),
                    argv: Vec::new(),
                })
            }
        }
        impl MaintenanceDispatch for Changing {
            fn dispatch(
                &self,
                _: &ValidatedMaintenanceAction,
                _: Option<&Arc<FlagCancelObserver>>,
                _: DispatchTiming,
            ) -> AdapterEvidence {
                self.calls.fetch_add(1, Ordering::SeqCst);
                AdapterEvidence::success()
            }
        }
        let temp = TempDir::new().unwrap();
        let adapter = Arc::new(Changing {
            changed: AtomicBool::new(false),
            calls: AtomicUsize::new(0),
        });
        let executor = MaintenanceExecutor::for_test(
            temp.path().join("audit.jsonl"),
            adapter.clone(),
            adapter.clone(),
            DispatchTiming::default(),
        );
        let plan = MaintenancePlanV2::new(&ids(&[
            "maintenance.retrim",
            "dns.flush",
            "maintenance.clock_probe",
            "maintenance.dead_startup",
        ]))
        .unwrap();
        let preview = executor.preview_batch(&plan).unwrap();
        let startup_plan = MaintenancePlanV2::new(&ids(&["maintenance.dead_startup"])).unwrap();
        let startup_before = executor.preview_batch(&startup_plan).unwrap();
        let report = executor
            .execute_batch(&plan, &preview.digest, true, None, |_, _| {
                adapter.changed.store(true, Ordering::SeqCst)
            })
            .unwrap();
        assert_eq!(
            report.outcomes[0].outcome,
            BatchOutcome::Skipped {
                reason: BatchSkipReason::Precheck(PrecheckReason::UacDeclined)
            }
        );
        assert_eq!(report.outcomes[1].outcome, report.outcomes[0].outcome);
        assert_eq!(
            report.outcomes[2].outcome,
            BatchOutcome::Skipped {
                reason: BatchSkipReason::IdentityChanged
            }
        );
        assert_eq!(report.outcomes[3].outcome, report.outcomes[2].outcome);
        assert!(matches!(
            executor.execute_batch(&startup_plan, &startup_before.digest, true, None, |_, _| {}),
            Err(MaintenanceExecutionError::Plan(
                OptimizePlanError::DigestMismatch
            ))
        ));
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn unreadable_audit_stops_the_batch_before_the_first_item() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("audit.jsonl");
        std::fs::write(&path, b"truncated").unwrap();
        let executor = MaintenanceExecutor::isolated_batch_fixture(&path);
        let plan = MaintenancePlanV2::new(&ids(&["dns.flush", "maintenance.clock_probe"])).unwrap();
        let preview = executor.preview_batch(&plan).unwrap();
        assert!(matches!(
            executor.execute_batch(&plan, &preview.digest, true, None, |_, _| panic!(
                "item started"
            )),
            Err(MaintenanceExecutionError::Audit(_))
        ));
        assert_eq!(std::fs::read(path).unwrap(), b"truncated");
    }

    #[test]
    fn batch_audits_every_item_continues_after_failure_and_cancels_between_items() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("audit.jsonl");
        let executor = MaintenanceExecutor::isolated_batch_fixture(&path);
        let plan = MaintenancePlanV2::new(&ids(&[
            "maintenance.clock_probe",
            "maintenance.retrim",
            "dns.flush",
        ]))
        .unwrap();
        let preview = executor.preview_batch(&plan).unwrap();
        let result = executor
            .execute_batch(&plan, &preview.digest, true, None, |_, _| {})
            .unwrap();
        assert!(matches!(
            result.outcomes[0].outcome,
            BatchOutcome::Failed { .. }
        ));
        assert_eq!(
            result.outcomes[1].outcome,
            BatchOutcome::Skipped {
                reason: BatchSkipReason::Precheck(PrecheckReason::UacDeclined)
            }
        );
        assert_eq!(result.outcomes[2].outcome, BatchOutcome::Succeeded);
        let canceled = FlagCancelObserver::new();
        let second = executor
            .execute_batch(&plan, &preview.digest, true, Some(&canceled), |index, _| {
                if index == 1 {
                    canceled.request_cancel()
                }
            })
            .unwrap();
        assert_ne!(result.batch_id, second.batch_id);
        assert!(second.outcomes[1..].iter().all(|item| item.outcome
            == BatchOutcome::Skipped {
                reason: BatchSkipReason::Canceled
            }));
        let text = std::fs::read_to_string(path).unwrap();
        for report in [&result, &second] {
            for item in &report.outcomes {
                let history: Vec<OptimizeAuditRecordV1> = text
                    .lines()
                    .map(|line| serde_json::from_str(line).unwrap())
                    .filter(|record: &OptimizeAuditRecordV1| {
                        record.operation_id == item.operation_id
                    })
                    .collect();
                assert!(
                    history
                        .iter()
                        .all(|entry| entry.batch_id.as_ref() == Some(&report.batch_id))
                );
                assert_eq!(
                    history.len(),
                    if matches!(item.outcome, BatchOutcome::Skipped { .. }) {
                        1
                    } else {
                        4
                    }
                );
            }
        }
    }
}
