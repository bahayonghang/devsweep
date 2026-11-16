//! Process actions share the durable Status store, not the power action meaning.

use std::path::PathBuf;

use super::keep_awake_audit::{append_status_record, status_audit_v1_path};
use super::{ProcessActionError, StatusActionAuditRecordV1};

/// Explicit injection boundary for isolated test journals.
pub trait ProcessActionAudit {
    fn append(&mut self, record: StatusActionAuditRecordV1) -> Result<(), ProcessActionError>;
}

/// Production fixed Status journal. No environment override installs a killer.
pub struct FileProcessActionAudit {
    path: Result<PathBuf, ProcessActionError>,
}

impl FileProcessActionAudit {
    #[must_use]
    pub fn production() -> Self {
        Self {
            path: status_audit_v1_path().map_err(|_| ProcessActionError::AuditFailed {
                after_dispatch: false,
            }),
        }
    }

    #[cfg(test)]
    pub(super) fn at(path: PathBuf) -> Self {
        Self { path: Ok(path) }
    }
}

impl ProcessActionAudit for FileProcessActionAudit {
    fn append(&mut self, record: StatusActionAuditRecordV1) -> Result<(), ProcessActionError> {
        append_status_record(self.path.as_ref().map_err(Clone::clone)?, &record).map_err(|_| {
            ProcessActionError::AuditFailed {
                after_dispatch: false,
            }
        })
    }
}

/// Tests explicitly supply this journal along with a fake process controller.
#[derive(Default)]
pub struct RecordingProcessActionAudit(pub Vec<StatusActionAuditRecordV1>);

impl ProcessActionAudit for RecordingProcessActionAudit {
    fn append(&mut self, record: StatusActionAuditRecordV1) -> Result<(), ProcessActionError> {
        self.0.push(record);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{
        ObservedProcess, ProcessKiller, ProcessProbe, StatusActionAuditAction,
        StatusActionAuditCode, bind_process_termination_plan, preview_bound_process,
        terminate_bound_process_audited,
    };
    use std::cell::RefCell;
    use std::collections::VecDeque;

    struct Probe(RefCell<VecDeque<Option<ObservedProcess>>>);
    impl ProcessProbe for Probe {
        fn lookup(&self, _: u32) -> Result<Option<ObservedProcess>, ProcessActionError> {
            let mut samples = self.0.borrow_mut();
            Ok(if samples.len() > 1 {
                samples.pop_front().flatten()
            } else {
                samples.front().cloned().flatten()
            })
        }
    }
    #[derive(Default)]
    struct Killer {
        calls: usize,
        fail: bool,
    }
    impl ProcessKiller for Killer {
        fn terminate(&mut self, pid: u32, creation: u64) -> Result<(), ProcessActionError> {
            assert_eq!((pid, creation), (42, 1000));
            self.calls += 1;
            if self.fail {
                Err(ProcessActionError::PermissionDenied)
            } else {
                Ok(())
            }
        }
    }
    fn live() -> ObservedProcess {
        ObservedProcess {
            pid: 42,
            creation_time_filetime: 1000,
            name: "fixture.exe".into(),
        }
    }
    fn probe(samples: Vec<Option<ObservedProcess>>) -> Probe {
        Probe(RefCell::new(samples.into()))
    }
    struct FailingAudit {
        records: Vec<StatusActionAuditRecordV1>,
        fail_at: usize,
        calls: usize,
    }
    impl ProcessActionAudit for FailingAudit {
        fn append(&mut self, record: StatusActionAuditRecordV1) -> Result<(), ProcessActionError> {
            self.calls += 1;
            if self.calls == self.fail_at {
                return Err(ProcessActionError::AuditFailed {
                    after_dispatch: false,
                });
            }
            self.records.push(record);
            Ok(())
        }
    }

    #[test]
    fn process_audited_success_and_backend_failure_record_the_bound_identity() {
        let plan = bind_process_termination_plan(42, 1000, "fixture.exe").expect("plan");
        for fail in [false, true] {
            let probe = probe(vec![Some(live())]);
            let preview = preview_bound_process(&plan, &probe).expect("preview");
            let mut killer = Killer {
                fail,
                ..Killer::default()
            };
            let mut audit = RecordingProcessActionAudit::default();
            let result = terminate_bound_process_audited(
                &plan,
                &preview.digest,
                true,
                &probe,
                &mut killer,
                &mut audit,
            );
            assert_eq!(result.is_ok(), !fail);
            assert_eq!(killer.calls, 1);
            assert_eq!(audit.0.len(), 2);
            assert_eq!(audit.0[0].action, StatusActionAuditAction::ProcessTerminate);
            assert_eq!(
                audit.0[0].status_code,
                StatusActionAuditCode::DispatchStarted
            );
            assert_eq!(
                audit.0[1].status_code,
                if fail {
                    StatusActionAuditCode::BackendFailed
                } else {
                    StatusActionAuditCode::Succeeded
                }
            );
            assert_eq!(audit.0[0].operation_id, audit.0[1].operation_id);
            assert!(audit.0.iter().all(StatusActionAuditRecordV1::is_valid));
            assert_eq!(
                audit.0[1]
                    .process
                    .as_ref()
                    .expect("process")
                    .error_code
                    .as_deref(),
                fail.then_some("identity_unavailable")
            );
        }
    }

    #[test]
    fn process_no_confirmation_or_digest_or_protected_identity_never_opens_audit_or_calls_killer() {
        let plan = bind_process_termination_plan(42, 1000, "fixture.exe").expect("plan");
        let probe = probe(vec![Some(live())]);
        let mut killer = Killer::default();
        let mut audit = RecordingProcessActionAudit::default();
        for (confirmed, digest) in [(false, ""), (true, ""), (true, "sha256:bad")] {
            assert!(
                terminate_bound_process_audited(
                    &plan,
                    digest,
                    confirmed,
                    &probe,
                    &mut killer,
                    &mut audit
                )
                .is_err()
            );
        }
        for (name, _) in crate::status::protected_process_names() {
            let forged = crate::status::ProcessTerminationPlanV1 {
                name: (*name).into(),
                ..plan.clone()
            };
            assert!(matches!(
                terminate_bound_process_audited(&forged, "", true, &probe, &mut killer, &mut audit),
                Err(ProcessActionError::Protected { .. })
            ));
        }
        assert!(audit.0.is_empty());
        assert_eq!(killer.calls, 0);
    }

    #[test]
    fn process_live_drift_after_started_is_refused_without_calling_killer() {
        let plan = bind_process_termination_plan(42, 1000, "fixture.exe").expect("plan");
        let digest = preview_bound_process(&plan, &probe(vec![Some(live())]))
            .expect("preview")
            .digest;
        let reused = ObservedProcess {
            creation_time_filetime: 1001,
            ..live()
        };
        let renamed = ObservedProcess {
            name: "replacement.exe".into(),
            ..live()
        };
        let protected = ObservedProcess {
            name: "lsass.exe".into(),
            ..live()
        };
        for final_state in [None, Some(reused), Some(renamed), Some(protected)] {
            for fail_terminal in [false, true] {
                let probe = probe(vec![Some(live()), final_state.clone()]);
                let mut killer = Killer::default();
                let mut audit = FailingAudit {
                    records: Vec::new(),
                    fail_at: if fail_terminal { 2 } else { 0 },
                    calls: 0,
                };
                let result = terminate_bound_process_audited(
                    &plan,
                    &digest,
                    true,
                    &probe,
                    &mut killer,
                    &mut audit,
                );
                assert!(result.is_err());
                assert_eq!(killer.calls, 0);
                if fail_terminal {
                    assert_eq!(
                        result,
                        Err(ProcessActionError::AuditFailed {
                            after_dispatch: false
                        })
                    );
                    assert_eq!(audit.records.len(), 1, "only Started remains");
                } else {
                    assert_eq!(audit.records[1].status_code, StatusActionAuditCode::Refused);
                }
                assert!(
                    audit
                        .records
                        .iter()
                        .all(StatusActionAuditRecordV1::is_valid)
                );
            }
        }
    }

    #[test]
    fn process_started_failure_blocks_kill_and_terminal_failure_stays_unknown() {
        let plan = bind_process_termination_plan(42, 1000, "fixture.exe").expect("plan");
        let probe = probe(vec![Some(live())]);
        let digest = preview_bound_process(&plan, &probe)
            .expect("preview")
            .digest;
        for fail_at in [1, 2] {
            let mut killer = Killer::default();
            let mut audit = FailingAudit {
                records: Vec::new(),
                fail_at,
                calls: 0,
            };
            assert_eq!(
                terminate_bound_process_audited(
                    &plan,
                    &digest,
                    true,
                    &probe,
                    &mut killer,
                    &mut audit
                ),
                Err(ProcessActionError::AuditFailed {
                    after_dispatch: fail_at == 2
                })
            );
            assert_eq!(killer.calls, usize::from(fail_at == 2));
            assert_eq!(audit.records.len(), usize::from(fail_at == 2));
        }
    }

    #[test]
    fn process_cancel_during_started_append_refuses_before_actual_killer() {
        use crate::process::FlagCancelObserver;
        use crate::status::terminate_bound_process_audited_cancellable;
        struct CancelOnStart<'a> {
            cancel: &'a FlagCancelObserver,
            records: Vec<StatusActionAuditRecordV1>,
        }
        impl ProcessActionAudit for CancelOnStart<'_> {
            fn append(
                &mut self,
                record: StatusActionAuditRecordV1,
            ) -> Result<(), ProcessActionError> {
                self.records.push(record);
                self.cancel.request_cancel();
                Ok(())
            }
        }
        let cancel = FlagCancelObserver::new();
        let mut audit = CancelOnStart {
            cancel: &cancel,
            records: Vec::new(),
        };
        let plan = bind_process_termination_plan(42, 1000, "fixture.exe").expect("plan");
        let probe = probe(vec![Some(live())]);
        let digest = preview_bound_process(&plan, &probe)
            .expect("preview")
            .digest;
        let mut killer = Killer::default();
        assert_eq!(
            terminate_bound_process_audited_cancellable(
                &plan,
                &digest,
                true,
                &probe,
                &mut killer,
                &mut audit,
                &cancel
            ),
            Err(ProcessActionError::CanceledBeforeDispatch)
        );
        assert_eq!(killer.calls, 0);
        assert_eq!(audit.records[1].status_code, StatusActionAuditCode::Refused);
        assert_eq!(
            audit.records[1]
                .process
                .as_ref()
                .expect("process")
                .error_code
                .as_deref(),
            Some("canceled_before_start")
        );
        assert!(
            audit
                .records
                .iter()
                .all(StatusActionAuditRecordV1::is_valid)
        );
    }

    #[test]
    fn process_cancellation_at_final_probe_refuses_but_after_killer_commit_preserves_result() {
        use crate::process::FlagCancelObserver;
        use crate::status::terminate_bound_process_audited_cancellable;
        use std::cell::Cell;
        struct CancelProbe<'a> {
            calls: Cell<usize>,
            cancel: &'a FlagCancelObserver,
            cancel_before: bool,
        }
        impl ProcessProbe for CancelProbe<'_> {
            fn lookup(&self, _: u32) -> Result<Option<ObservedProcess>, ProcessActionError> {
                self.calls.set(self.calls.get() + 1);
                if self.calls.get() == 2 && self.cancel_before {
                    self.cancel.request_cancel();
                }
                Ok(Some(live()))
            }
        }
        struct CancelKiller<'a> {
            calls: usize,
            cancel: &'a FlagCancelObserver,
        }
        impl ProcessKiller for CancelKiller<'_> {
            fn terminate(&mut self, _: u32, _: u64) -> Result<(), ProcessActionError> {
                self.calls += 1;
                self.cancel.request_cancel();
                Ok(())
            }
        }
        let plan = bind_process_termination_plan(42, 1000, "fixture.exe").expect("plan");
        let digest = preview_bound_process(&plan, &probe(vec![Some(live())]))
            .expect("preview")
            .digest;
        for cancel_before in [true, false] {
            let cancel = FlagCancelObserver::new();
            let probe = CancelProbe {
                calls: Cell::new(0),
                cancel: &cancel,
                cancel_before,
            };
            let mut killer = CancelKiller {
                calls: 0,
                cancel: &cancel,
            };
            let mut audit = RecordingProcessActionAudit::default();
            let result = terminate_bound_process_audited_cancellable(
                &plan,
                &digest,
                true,
                &probe,
                &mut killer,
                &mut audit,
                &cancel,
            );
            assert_eq!(probe.calls.get(), 2);
            assert_eq!(killer.calls, usize::from(!cancel_before));
            assert_eq!(audit.0.len(), 2);
            assert_eq!(
                audit.0[1].status_code,
                if cancel_before {
                    StatusActionAuditCode::Refused
                } else {
                    StatusActionAuditCode::Succeeded
                }
            );
            if cancel_before {
                assert_eq!(result, Err(ProcessActionError::CanceledBeforeDispatch));
            } else {
                assert_eq!(result.expect("committed result").outcome, "terminated");
            }
        }
    }

    #[test]
    fn process_file_audit_coexists_with_legacy_power_history_and_validates_action_fields() {
        use crate::history::{HistoryStorePaths, show_history_at};
        let root = tempfile::TempDir::new().expect("isolated root");
        let status = root.path().join("status.jsonl");
        let legacy = serde_json::json!({"schema_version":1,"domain":"status","operation_id":"legacy-awake","timestamp_unix_ms":1,"action":"start","status_code":"succeeded","duration_ms":10,"preview_digest":null});
        std::fs::write(&status, format!("{legacy}\n")).expect("legacy journal");
        let plan = bind_process_termination_plan(42, 1000, "fixture.exe").expect("plan");
        let probe = probe(vec![Some(live())]);
        let digest = preview_bound_process(&plan, &probe)
            .expect("preview")
            .digest;
        let mut audit = FileProcessActionAudit::at(status.clone());
        terminate_bound_process_audited(
            &plan,
            &digest,
            true,
            &probe,
            &mut Killer::default(),
            &mut audit,
        )
        .expect("audited fixture");
        let text = std::fs::read_to_string(&status).expect("file");
        assert!(
            text.starts_with(&format!("{legacy}\n")),
            "legacy bytes unchanged"
        );
        let entries: Vec<StatusActionAuditRecordV1> = text
            .lines()
            .map(|line| serde_json::from_str(line).expect("V1"))
            .collect();
        let paths = HistoryStorePaths {
            clean: root.path().join("clean.jsonl"),
            software: root.path().join("software.jsonl"),
            optimize: root.path().join("optimize.jsonl"),
            status: status.clone(),
        };
        let operation = &entries[1].operation_id;
        let history = show_history_at(&paths, operation).expect("process history");
        assert_eq!(history.summary.outcome_code, "succeeded");
        assert!(!history.summary.unknown);
        assert!(show_history_at(&paths, "legacy-awake").is_ok());
        let mut invalid = entries[1].clone();
        invalid.action = StatusActionAuditAction::Start;
        assert!(audit.append(invalid).is_err());
        let mut invalid = entries[1].clone();
        invalid.process = None;
        assert!(audit.append(invalid).is_err());
        let mut invalid = entries[1].clone();
        invalid.process.as_mut().expect("process").name = "secret\nargs".into();
        assert!(audit.append(invalid).is_err());
        std::fs::write(
            &status,
            format!("{}\n", serde_json::to_string(&entries[1]).expect("started")),
        )
        .expect("interrupted journal");
        let pending = show_history_at(&paths, operation).expect("pending");
        assert!(pending.summary.unknown);
        assert_eq!(pending.summary.outcome_code, "unknown_after_dispatch");
        // A complete JSON terminal without its newline is not durable completion.
        std::fs::write(
            &status,
            format!(
                "{}\n{}",
                serde_json::to_string(&entries[1]).expect("started"),
                serde_json::to_string(&entries[2]).expect("terminal")
            ),
        )
        .expect("partial append");
        assert!(
            show_history_at(&paths, operation)
                .expect("partial history")
                .summary
                .unknown
        );
        let mut killer = Killer::default();
        assert!(
            terminate_bound_process_audited(&plan, &digest, true, &probe, &mut killer, &mut audit)
                .is_err()
        );
        assert_eq!(killer.calls, 0);
    }
}
