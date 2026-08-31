use super::*;
use crate::{
    i18n::Locale,
    tui::{app::App, shell::ShellComposition, test_support::render_text_with_size},
};
use devsweep_core::status::{
    AvailabilityV1, StatusStartedV1, StatusTerminalV1, TerminalReason, TickSkipReason,
    TickSkippedV1,
};

fn canonical_snapshot() -> StatusSnapshotV1 {
    let envelope: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/cli/status-snapshot.json"
    ))
    .expect("canonical snapshot envelope");
    serde_json::from_value(envelope["data"].clone()).expect("canonical StatusSnapshotV1")
}

fn live_events() -> Vec<StatusEventV1> {
    include_str!("../../../../tests/fixtures/cli/status-live.ndjson")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("canonical live event"))
        .collect()
}

fn started(operation_id: &str, sequence: u64) -> StatusEventV1 {
    StatusEventV1::Started {
        schema_version: 1,
        operation_id: operation_id.to_string(),
        sequence,
        emitted_at_unix_ms: 1,
        data: StatusStartedV1 {
            interval_ms: 2_000,
            process_limit: 15,
        },
    }
}

fn snapshot_event(operation_id: &str, sequence: u64, snapshot: StatusSnapshotV1) -> StatusEventV1 {
    StatusEventV1::Snapshot {
        schema_version: 1,
        operation_id: operation_id.to_string(),
        sequence,
        emitted_at_unix_ms: 2,
        data: Box::new(snapshot),
    }
}

fn skipped(operation_id: &str, sequence: u64, total: u64) -> StatusEventV1 {
    StatusEventV1::TickSkipped {
        schema_version: 1,
        operation_id: operation_id.to_string(),
        sequence,
        emitted_at_unix_ms: 3,
        data: TickSkippedV1 {
            reason: TickSkipReason::SampleInFlight,
            skipped_total: total,
        },
    }
}

fn terminal(operation_id: &str, sequence: u64, reason: TerminalReason) -> StatusEventV1 {
    StatusEventV1::Terminal {
        schema_version: 1,
        operation_id: operation_id.to_string(),
        sequence,
        emitted_at_unix_ms: 4,
        data: StatusTerminalV1 {
            reason,
            error_code: None,
        },
    }
}

fn drive_live(state: &mut StatusModeState, events: impl IntoIterator<Item = StatusEventV1>) {
    state.reduce(StatusAction::LiveStarted(7));
    for event in events {
        state.reduce(StatusAction::LiveEvent { job_id: 7, event });
    }
}

#[test]
fn canonical_snapshot_keeps_privacy_partial_and_no_battery() {
    let snapshot = canonical_snapshot();
    let mut state = StatusModeState::default();
    state.reduce(StatusAction::SnapshotStarted(1));
    state.reduce(StatusAction::SnapshotFinished {
        job_id: 1,
        snapshot: Box::new(snapshot.clone()),
    });
    assert_eq!(state.phase, StatusPhase::Ready);
    assert_eq!(
        state.snapshot.as_ref().unwrap().snapshot_id,
        "snapshot-fixture"
    );
    let processes = available_value(&snapshot.processes).expect("process group");
    assert!(processes.truncated_by_limit);
    assert!(processes.budget_exhausted);
    assert_eq!(processes.enumerated_count, 4096);
    assert_eq!(processes.returned_count, 1);
    let power = available_value(&snapshot.power).expect("power");
    assert!(!power.battery_present);
    assert!(power.charge_basis_points.is_none());
    for process in &processes.items {
        let encoded = serde_json::to_value(process).expect("process row");
        let object = encoded.as_object().expect("object");
        let mut keys: Vec<_> = object.keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "cpu_basis_points_of_one_logical_core",
                "name",
                "pid",
                "private_bytes",
                "read_bytes_per_second",
                "write_bytes_per_second"
            ]
        );
        assert!(!object.contains_key("path"));
        assert!(!object.contains_key("cmdline"));
        assert!(!object.contains_key("user"));
    }
}

#[test]
fn live_opt_in_skip_stale_terminal_and_broken_pipe() {
    let events = live_events();
    assert_eq!(events.len(), 4);
    let mut state = StatusModeState::default();
    drive_live(&mut state, events.clone());
    assert_eq!(state.phase, StatusPhase::Ready);
    assert_eq!(state.chart.len(), 2);
    assert!(state.chart[0].cpu_bp.is_none());
    assert!(state.chart[1].is_gap());
    assert_eq!(state.skipped_total, 1);
    assert!(state.operation_id.is_none());

    let mut stale = StatusModeState::default();
    stale.reduce(StatusAction::LiveStarted(3));
    stale.reduce(StatusAction::LiveEvent {
        job_id: 3,
        event: started("op-fixture", 0),
    });
    stale.reduce(StatusAction::LiveEvent {
        job_id: 3,
        event: skipped("other-op", 1, 1),
    });
    assert_eq!(stale.chart.len(), 0);
    assert_eq!(stale.phase, StatusPhase::Live);
    stale.reduce(StatusAction::LiveEvent {
        job_id: 99,
        event: snapshot_event("op-fixture", 1, canonical_snapshot()),
    });
    assert!(stale.snapshot.is_none());

    let mut pipe = StatusModeState::default();
    drive_live(
        &mut pipe,
        [
            started("op-pipe", 0),
            terminal("op-pipe", 1, TerminalReason::BrokenPipe),
        ],
    );
    assert_eq!(pipe.phase, StatusPhase::Idle);
    assert!(pipe.chart.is_empty());
}

#[test]
fn exact_sequence_rejects_duplicate_and_nonmonotonic_events() {
    let mut duplicate = StatusModeState::default();
    drive_live(&mut duplicate, [started("op", 0), started("op", 0)]);
    assert_eq!(duplicate.phase, StatusPhase::Failed);
    assert_eq!(
        duplicate.error.as_deref(),
        Some("status_started sequence is not exact")
    );

    let mut skipped_seq = StatusModeState::default();
    drive_live(
        &mut skipped_seq,
        [
            started("op", 0),
            snapshot_event("op", 2, canonical_snapshot()),
        ],
    );
    assert_eq!(skipped_seq.phase, StatusPhase::Failed);
    assert!(skipped_seq.needs_cancel_after_decode_failure());
}

#[test]
fn unknown_event_and_unknown_fields_are_rejected_at_the_wire() {
    let unknown_event = serde_json::json!({
        "schema_version": 1,
        "event": "status_health",
        "operation_id": "op",
        "sequence": 0,
        "emitted_at_unix_ms": 1,
        "data": {}
    });
    assert!(serde_json::from_value::<StatusEventV1>(unknown_event).is_err());

    let mut snapshot = serde_json::to_value(canonical_snapshot()).expect("snapshot json");
    snapshot["processes"]["value"]["items"][0]["cmdline"] = serde_json::json!("secret");
    let object = snapshot["processes"]["value"]["items"][0]
        .as_object()
        .expect("process");
    assert!(object.contains_key("cmdline"));
    let closed = [
        "pid",
        "name",
        "cpu_basis_points_of_one_logical_core",
        "private_bytes",
        "read_bytes_per_second",
        "write_bytes_per_second",
    ];
    assert!(object.keys().any(|key| !closed.contains(&key.as_str())));
}

#[test]
fn chart_caps_at_sixty_gaps_missing_samples_and_drops_on_leave() {
    let mut state = StatusModeState::default();
    state.reduce(StatusAction::LiveStarted(4));
    state.reduce(StatusAction::LiveEvent {
        job_id: 4,
        event: started("op-cap", 0),
    });
    for sequence in 1..=61 {
        if sequence % 5 == 0 {
            state.reduce(StatusAction::LiveEvent {
                job_id: 4,
                event: skipped("op-cap", sequence, sequence / 5),
            });
        } else {
            state.reduce(StatusAction::LiveEvent {
                job_id: 4,
                event: snapshot_event("op-cap", sequence, canonical_snapshot()),
            });
        }
    }
    assert_eq!(state.chart.len(), MAX_CHART_POINTS);
    assert_eq!(state.chart.first().map(|point| point.sequence), Some(2));
    assert!(state.chart.iter().any(ChartPoint::is_gap));
    assert!(state.chart.iter().any(|point| point.cpu_bp == Some(4321)));
    assert!(
        state
            .chart
            .iter()
            .filter(|point| point.is_gap())
            .all(|point| point.cpu_bp.is_none() && point.memory_used_bytes.is_none())
    );
    state.reduce(StatusAction::Released);
    assert!(state.chart.is_empty());
    assert!(state.snapshot.is_none());
    assert_eq!(state.phase, StatusPhase::Idle);
}

#[test]
fn interval_change_is_local_until_a_new_live_operation() {
    let mut state = StatusModeState::default();
    state.reduce(StatusAction::IntervalStep(1));
    assert_eq!(state.interval_ms, 5_000);
    state.reduce(StatusAction::LiveStarted(2));
    state.reduce(StatusAction::IntervalStep(-1));
    assert_eq!(state.interval_ms, 2_000);
    assert!(state.pending_live_restart);
    state.reduce(StatusAction::Canceled(2));
    state.reduce(StatusAction::SetIntervalMs(1_000));
    assert_eq!(state.interval_ms, 1_000);
}

#[test]
fn process_sort_and_churn_keep_privacy_and_truncation_flags() {
    let mut first = canonical_snapshot();
    if let AvailabilityV1::Partial { value, .. } = &mut first.processes {
        value.items[0].pid = 7;
        value.items[0].name = "alpha.exe".into();
        value.items.push(devsweep_core::status::ProcessV1 {
            pid: 9,
            name: "beta.exe".into(),
            cpu_basis_points_of_one_logical_core: 100,
            private_bytes: 8_192,
            read_bytes_per_second: 0,
            write_bytes_per_second: 0,
        });
        value.returned_count = 2;
    }
    let mut second = first.clone();
    if let AvailabilityV1::Partial { value, .. } = &mut second.processes {
        value.items[0].pid = 11;
        value.items[0].name = "gamma.exe".into();
        value.items.pop();
        value.returned_count = 1;
        value.truncated_by_limit = true;
        value.budget_exhausted = true;
    }
    let mut state = StatusModeState::default();
    state.reduce(StatusAction::SnapshotStarted(1));
    state.reduce(StatusAction::SnapshotFinished {
        job_id: 1,
        snapshot: Box::new(first),
    });
    state.reduce(StatusAction::CycleSort);
    assert_eq!(state.process_sort, ProcessSort::Memory);
    assert_eq!(state.sorted_processes()[0].name, "alpha.exe");
    state.reduce(StatusAction::LiveStarted(2));
    state.reduce(StatusAction::LiveEvent {
        job_id: 2,
        event: started("op-churn", 0),
    });
    state.reduce(StatusAction::LiveEvent {
        job_id: 2,
        event: snapshot_event("op-churn", 1, second),
    });
    let rows = state.sorted_processes();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "gamma.exe");
    let processes = available_value(&state.snapshot.as_ref().unwrap().processes).unwrap();
    assert!(processes.truncated_by_limit);
    assert!(processes.budget_exhausted);
}

#[test]
fn leave_close_and_pause_clear_ephemeral_live_state() {
    let mut state = StatusModeState::default();
    drive_live(
        &mut state,
        [
            started("op", 0),
            snapshot_event("op", 1, canonical_snapshot()),
        ],
    );
    assert_eq!(state.phase, StatusPhase::Live);
    state.reduce(StatusAction::CancelRequested(7));
    assert_eq!(state.phase, StatusPhase::Canceling);
    state.reduce(StatusAction::Canceled(7));
    assert_eq!(state.phase, StatusPhase::Ready);
    assert!(state.operation_id.is_none());
    state.reduce(StatusAction::Released);
    assert!(state.chart.is_empty());
    assert!(state.snapshot.is_none());
}

#[test]
fn tui_renders_snapshot_cards_text_chart_alternative_and_chinese() {
    let snapshot = canonical_snapshot();
    for locale in [Locale::En, Locale::ZhCn] {
        let mut shell = ShellComposition::for_locale(locale).expect("shell");
        assert!(shell.activate(crate::tui::shell::ModeId::Status));
        let mut app = App::with_shell(shell);
        app.status.reduce(StatusAction::SnapshotStarted(1));
        app.status.reduce(StatusAction::SnapshotFinished {
            job_id: 1,
            snapshot: Box::new(snapshot.clone()),
        });
        app.status.reduce(StatusAction::LiveStarted(2));
        app.status.reduce(StatusAction::LiveEvent {
            job_id: 2,
            event: started("op-render", 0),
        });
        app.status.reduce(StatusAction::LiveEvent {
            job_id: 2,
            event: skipped("op-render", 1, 1),
        });
        app.status.reduce(StatusAction::LiveEvent {
            job_id: 2,
            event: snapshot_event("op-render", 2, snapshot.clone()),
        });
        let rendered = render_text_with_size(&app, 100, 36);
        assert!(rendered.contains("43.21"));
        assert!(rendered.contains("fixture.exe"));
        assert!(!rendered.contains("cmdline"));
        assert!(!rendered.to_lowercase().contains("gpu 0"));
        if locale == Locale::En {
            assert!(rendered.contains("No battery present"));
            assert!(rendered.contains("gap"));
            assert!(rendered.contains("Start live") || rendered.contains("Stop live"));
        } else {
            assert!(rendered.contains("无电池"));
            assert!(rendered.contains("缺口"));
        }
        let narrow = render_text_with_size(&app, 60, 24);
        assert!(narrow.contains("43.21"));
    }
}

#[test]
fn serde_rejects_unknown_status_event_variant_from_fixture() {
    let source = include_str!("../../../../tests/fixtures/status/unknown-event.json");
    let parsed: Result<StatusEventV1, _> = serde_json::from_str(source);
    assert!(parsed.is_err());
}
