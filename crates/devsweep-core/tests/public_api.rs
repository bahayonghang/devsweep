use devsweep_core::{
    execution::Executor,
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
    assert!(
        validate_plan(&plan)
            .expect("empty plan validates")
            .default_selected_ids()
            .is_empty()
    );

    let options = ScanOptions {
        include_projects: true,
        include_global: false,
        roots: Vec::new(),
    };
    assert!(options.include_projects);

    let cancel = FlagCancelObserver::new();
    cancel.request_cancel();

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
