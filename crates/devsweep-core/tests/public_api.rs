use devsweep_core::{
    analysis::{ANALYZE_SNAPSHOT_VERSION, AnalyzeSnapshotV1, analyze_path},
    execution::{ExecutionRequest, Executor, confirmation_digest},
    model::{CLEANUP_PLAN_VERSION, UntrustedPlan},
    plan::validate_plan,
    process::FlagCancelObserver,
    scan::{ScanOptions, Sweeper},
    services::{
        CleanService, ExecutorCleanService, InventoryService, LocalInventoryService, ScanService,
        SweepScanService,
    },
};

#[test]
fn external_crate_can_use_the_supported_core_surface() {
    let plan = UntrustedPlan::empty();
    assert_eq!(plan.version, CLEANUP_PLAN_VERSION);
    let validated = validate_plan(&plan).expect("empty plan validates");
    assert!(validated.default_selected_ids().is_empty());
    let digest = confirmation_digest(&validated, &[]).expect("empty selection digest");
    assert_eq!(digest.as_str().len(), 64);
    let report = Executor::default()
        .run_plan(
            &validated,
            ExecutionRequest {
                execute: false,
                audit_log: None,
                selected: Vec::new(),
                expected_digest: None,
                cancel: None,
            },
        )
        .expect("empty dry-run succeeds");
    assert_eq!(report.confirmation_digest, digest);
    serde_json::to_value(report).expect("execution report serializes externally");

    let options = ScanOptions {
        include_projects: true,
        include_global: false,
        roots: Vec::new(),
    };
    assert!(options.include_projects);
    serde_json::to_value(options).expect("scan options serialize externally");

    let cancel = FlagCancelObserver::new();
    cancel.request_cancel();
    assert_eq!(ANALYZE_SNAPSHOT_VERSION, 1);
    let _ = analyze_path;
    serde_json::to_value(AnalyzeSnapshotV1 {
        version: ANALYZE_SNAPSHOT_VERSION,
        root: devsweep_core::analysis::AnalyzeRootIdentity {
            input: "C:/root".into(),
            normalized: "C:/root".into(),
            volume: "vol".into(),
        },
        nodes: Vec::new(),
        warnings: Vec::new(),
        completeness: devsweep_core::analysis::AnalyzeCompleteness::Complete,
        accounted_owned_bytes: 0,
    })
    .expect("analyze snapshot serializes externally");
    assert_default::<Sweeper>();
    assert_default::<Executor>();
    assert_scan_service::<SweepScanService>();
    assert_inventory_service::<LocalInventoryService>();
    assert_clean_service::<ExecutorCleanService>();
}

fn assert_default<T: Default>() {}

fn assert_scan_service<T: ScanService>() {}

fn assert_inventory_service<T: InventoryService>() {}

fn assert_clean_service<T: CleanService>() {}
