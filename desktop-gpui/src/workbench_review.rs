//! Keyboard review for Software and Optimize.
//!
//! Plans and previews call the accepted core functions. A mutation runs only
//! against an isolated startup hive file. Host defrag, DISM, clock sync, real
//! uninstall, and real startup keys are not dispatched. Guidance and observe
//! rows have no schedule path.

use std::{
    fs,
    path::{Path, PathBuf},
};

use devsweep_core::optimize::{
    MaintenanceActionClass, MaintenancePlanV1, OptimizePlanError, PrecheckReason,
    catalogue_entry, plan_operation, preview_maintenance_plan_live, published_entries,
};
use devsweep_core::software::{
    SOFTWARE_PLAN_VERSION, SoftwareExecutionError, SoftwareExecutionRequest, SoftwareExecutor,
    SoftwareSelectionPlanV1, SoftwareStartupError, SoftwareStartupLocation, StartupFixtureClass,
    StartupTogglePlanV1, execute_startup_hive, plan_software_update, plan_startup_hive,
    preview_startup_hive, startup_fixture_entry_id,
};

use crate::copy::{self, Locale};
use crate::workbench::{self, SoftwareSegment, WorkbenchAction, WorkbenchDecision};

const HIVE_ENV: &str = "DEVSWEEP_SOFTWARE_STARTUP_HIVE";
const UPDATE_ENV: &str = "DEVSWEEP_SOFTWARE_UPDATE_PROGRAM";
const STALE_ENV: &str = "DEVSWEEP_SOFTWARE_REVIEW_STALE";
const ROW_DIR_ENV: &str = "DEVSWEEP_WORKBENCH_ROW_DIR";

const HOST_NOT_DISPOSABLE: &[&str] = &[
    "guidance.drive_optimize",
    "maintenance.retrim",
    "maintenance.image_scan",
    "maintenance.image_repair",
    "maintenance.clock_sync",
];

/// Isolated hive confirm that a second Enter may dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmedHive {
    pub path: PathBuf,
    pub plan: StartupTogglePlanV1,
    pub digest: String,
}

/// What the keyboard path may hold between Enter presses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingReview {
    pub id: String,
    pub hive: Option<ArmedHive>,
}

/// Domain lines plus an optional privilege notice. Never a handwritten success.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewEffect {
    pub lines: Vec<String>,
    pub precheck: Option<PrecheckReason>,
    pub pending: Option<PendingReview>,
}

/// Review state painted under the catalogue rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkbenchReview {
    lines: Vec<String>,
    pending: Option<PendingReview>,
    precheck: Option<PrecheckReason>,
}

impl WorkbenchReview {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn pending_id(&self) -> Option<&str> {
        self.pending.as_ref().map(|pending| pending.id.as_str())
    }

    /// Opens or confirms the selected identity. Host tools are not executed.
    pub fn activate(&mut self, id: &str, locale: Locale) {
        let effect = if self.pending.as_ref().is_some_and(|pending| pending.id == id) {
            confirm(self.pending.as_ref().expect("pending checked"), locale)
        } else {
            open_review(id, locale)
        };
        self.lines = effect.lines;
        self.pending = effect.pending;
        self.precheck = effect.precheck;
    }

    #[must_use]
    pub fn precheck(&self) -> Option<PrecheckReason> {
        self.precheck
    }
}

/// Catalogue ids the shell can select. Isolated row files are appended, not
/// substituted for the four software segments.
#[must_use]
pub fn software_row_ids() -> Vec<String> {
    let mut ids = vec![
        "uninstall".to_string(),
        "updates".to_string(),
        "startup".to_string(),
        "leftovers".to_string(),
    ];
    if let Some(dir) = env_path(ROW_DIR_ENV) {
        ids.extend(isolated_row_ids(&dir));
    }
    ids
}

#[must_use]
pub fn optimize_row_ids() -> Vec<String> {
    published_entries()
        .iter()
        .map(|entry| entry.id.to_string())
        .collect()
}

/// Names of files in an isolated directory. Directories are skipped.
#[must_use]
pub fn isolated_row_ids(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(read) = fs::read_dir(dir) else {
        return names;
    };
    for item in read.flatten() {
        let Ok(file_type) = item.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        if let Some(name) = item.file_name().to_str() {
            names.push(name.to_string());
        }
    }
    names.sort();
    names
}

fn open_review(id: &str, locale: Locale) -> ReviewEffect {
    if let Some(segment) = software_segment(id) {
        return open_software(segment, locale);
    }
    open_optimize(id, locale)
}

fn open_software(segment: SoftwareSegment, locale: Locale) -> ReviewEffect {
    match segment {
        SoftwareSegment::Leftovers => ReviewEffect {
            lines: vec![copy::software_leftovers_unavailable(locale).to_string()],
            precheck: None,
            pending: None,
        },
        SoftwareSegment::Uninstall => refuse_unconfirmed_software(locale),
        SoftwareSegment::Updates => open_updates(locale),
        SoftwareSegment::Startup => open_startup(locale),
    }
}

fn refuse_unconfirmed_software(locale: Locale) -> ReviewEffect {
    let plan = dummy_software_plan();
    let display = match SoftwareExecutor::default().execute(SoftwareExecutionRequest {
        plan: &plan,
        expected_preview_digest: "",
        confirmed: false,
        cancel: None,
    }) {
        Err(error) => error.to_string(),
        Ok(_) => SoftwareExecutionError::ConfirmationRequired.to_string(),
    };
    ReviewEffect {
        lines: vec![domain_line(locale, "confirmation_required", &display)],
        precheck: None,
        pending: Some(PendingReview {
            id: "uninstall".to_string(),
            hive: None,
        }),
    }
}

fn open_updates(locale: Locale) -> ReviewEffect {
    let Some(program) = env_path(UPDATE_ENV) else {
        return ReviewEffect {
            lines: vec!["updates | review | argv=software updates check".to_string()],
            precheck: None,
            pending: None,
        };
    };
    // A missing or failing provider is the isolated error path. A successful
    // probe still does not call execute_software_update.
    let line = match plan_software_update(
        program.as_os_str(),
        "devsweep.fixture.package",
        "winget",
        "1.0.0",
    ) {
        Ok(plan) => format!(
            "updates | {} | {}",
            plan.package_id,
            copy::workbench_not_run(locale)
        ),
        Err(error) => domain_line(locale, error.code(), &error.to_string()),
    };
    ReviewEffect {
        lines: vec![line],
        precheck: None,
        pending: Some(PendingReview {
            id: "updates".to_string(),
            hive: None,
        }),
    }
}

fn open_startup(locale: Locale) -> ReviewEffect {
    let Some(path) = env_path(HIVE_ENV) else {
        let missing = std::env::temp_dir().join("devsweep-gpui-startup-hive-missing.json");
        let line = match plan_startup_hive(&missing, "startup:v1:missing", false) {
            Ok(_) => domain_line(
                locale,
                "startup_source_unavailable",
                &SoftwareStartupError::StartupSourceUnavailable.to_string(),
            ),
            Err(error) => domain_line(locale, error.code(), &error.to_string()),
        };
        return ReviewEffect {
            lines: vec![line],
            precheck: None,
            pending: None,
        };
    };
    let hive = match fs::read(&path).and_then(|bytes| {
        serde_json::from_slice::<devsweep_core::software::StartupFixtureHiveV1>(&bytes)
            .map_err(std::io::Error::other)
    }) {
        Ok(hive) => hive,
        Err(_) => {
            let line = match plan_startup_hive(&path, "startup:v1:unreadable", false) {
                Ok(_) => domain_line(
                    locale,
                    "startup_source_unavailable",
                    &SoftwareStartupError::StartupSourceUnavailable.to_string(),
                ),
                Err(error) => domain_line(locale, error.code(), &error.to_string()),
            };
            return ReviewEffect {
                lines: vec![line],
                precheck: None,
                pending: None,
            };
        }
    };
    let mut lines = Vec::new();
    let mut armed = None;
    for entry in &hive.entries {
        let id = startup_fixture_entry_id(entry.location, &entry.name);
        match plan_startup_hive(&path, &id, false) {
            Ok(plan) => match preview_startup_hive(&path, &plan) {
                Ok(preview) => {
                    lines.push(format!("{id} | {}", preview.digest));
                    if armed.is_none()
                        && entry.class == StartupFixtureClass::Ordinary
                        && entry.location == SoftwareStartupLocation::CurrentUserRun
                    {
                        armed = Some(ArmedHive {
                            path: path.clone(),
                            plan,
                            digest: preview.digest,
                        });
                    }
                }
                Err(error) => lines.push(domain_line(locale, error.code(), &error.to_string())),
            },
            Err(error) => lines.push(domain_line(locale, error.code(), &error.to_string())),
        }
    }
    if lines.is_empty() {
        lines.push(domain_line(
            locale,
            "startup_source_unavailable",
            &SoftwareStartupError::StartupSourceUnavailable.to_string(),
        ));
    }
    ReviewEffect {
        lines,
        precheck: None,
        pending: Some(PendingReview {
            id: "startup".to_string(),
            hive: armed,
        }),
    }
}

fn open_optimize(id: &str, locale: Locale) -> ReviewEffect {
    let class = catalogue_entry(id).map(|entry| entry.action_class);
    if class.is_none() {
        let decision = workbench::resolve(WorkbenchAction::Catalogue(id));
        let explanation = match decision {
            WorkbenchDecision::Unavailable { explanation } => explanation.to_string(),
            WorkbenchDecision::Review { .. } => {
                OptimizePlanError::UnknownOperation(id.to_string()).to_string()
            }
        };
        return ReviewEffect {
            lines: vec![explanation],
            precheck: None,
            pending: None,
        };
    }
    if matches!(
        class,
        Some(MaintenanceActionClass::Guidance | MaintenanceActionClass::Observe)
    ) {
        let line = match plan_operation(id) {
            Ok(_) => copy::workbench_not_run(locale).to_string(),
            Err(error) => domain_line(locale, optimize_code(&error), &error.to_string()),
        };
        return ReviewEffect {
            lines: vec![line],
            precheck: None,
            pending: None,
        };
    }
    let plan = match plan_operation(id) {
        Ok(plan) => plan,
        Err(error) => {
            return ReviewEffect {
                lines: vec![domain_line(locale, optimize_code(&error), &error.to_string())],
                precheck: None,
                pending: None,
            };
        }
    };
    match preview_maintenance_plan_live(&plan) {
        Ok(preview) => ReviewEffect {
            lines: vec![format!(
                "{} | {} | {}",
                preview.operation_id, preview.digest, host_last(id, locale)
            )],
            precheck: None,
            pending: Some(PendingReview {
                id: id.to_string(),
                hive: None,
            }),
        },
        Err(error) => {
            let precheck = match &error {
                OptimizePlanError::PrecheckRefused { reason, .. } => Some(*reason),
                _ => None,
            };
            ReviewEffect {
                lines: vec![
                    domain_line(locale, optimize_code(&error), &error.to_string()),
                    host_last(id, locale),
                ],
                precheck,
                pending: Some(PendingReview {
                    id: id.to_string(),
                    hive: None,
                }),
            }
        }
    }
}

fn confirm(pending: &PendingReview, locale: Locale) -> ReviewEffect {
    if let Some(hive) = &pending.hive {
        return confirm_hive(hive, locale);
    }
    if pending.id == "uninstall" {
        return refuse_unconfirmed_software(locale);
    }
    let plan = plan_operation(&pending.id).ok();
    let gate = refuse_unconfirmed_optimize(plan.as_ref());
    let mut lines = vec![domain_line(locale, "confirmation_required", &gate)];
    if HOST_NOT_DISPOSABLE.contains(&pending.id.as_str()) {
        lines.push(host_last(&pending.id, locale));
    }
    ReviewEffect {
        lines,
        precheck: None,
        pending: Some(pending.clone()),
    }
}

fn confirm_hive(hive: &ArmedHive, locale: Locale) -> ReviewEffect {
    let live = preview_startup_hive(&hive.path, &hive.plan);
    let expected = if env_flag(STALE_ENV) {
        flipped_digest(&hive.digest)
    } else {
        match &live {
            Ok(preview) => preview.digest.clone(),
            Err(error) => {
                return ReviewEffect {
                    lines: vec![domain_line(locale, error.code(), &error.to_string())],
                    precheck: None,
                    pending: Some(PendingReview {
                        id: "startup".to_string(),
                        hive: Some(hive.clone()),
                    }),
                };
            }
        }
    };
    match execute_startup_hive(&hive.path, &hive.plan, &expected, true) {
        Ok(report) => {
            let outcome = serde_json::to_value(report.toggle.outcome)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "unknown".to_string());
            ReviewEffect {
                lines: vec![format!(
                    "startup hive outcome={outcome} | last={}",
                    copy::workbench_not_run(locale)
                )],
                precheck: None,
                pending: None,
            }
        }
        Err(error) => ReviewEffect {
            lines: vec![domain_line(locale, error.code(), &error.to_string())],
            precheck: None,
            pending: Some(PendingReview {
                id: "startup".to_string(),
                hive: Some(hive.clone()),
            }),
        },
    }
}

fn refuse_unconfirmed_optimize(plan: Option<&MaintenancePlanV1>) -> String {
    let owned = plan.cloned().unwrap_or_else(|| MaintenancePlanV1 {
        version: 1,
        catalogue_version: 1,
        operation_id: "dns.flush".to_string(),
        action_class: None,
    });
    let error = devsweep_core::optimize::MaintenanceExecutor::default().execute(
        devsweep_core::optimize::MaintenanceExecutionRequest {
            plan: &owned,
            expected_preview_digest: "",
            confirmed: false,
            cancel: None,
        },
    );
    match error {
        Err(error) => error.to_string(),
        Ok(_) => "Optimize run requires explicit confirmation".to_string(),
    }
}

fn host_last(id: &str, locale: Locale) -> String {
    if HOST_NOT_DISPOSABLE.contains(&id) {
        format!("last={}", copy::workbench_not_run(locale))
    } else {
        format!("last={}", copy::workbench_not_run(locale))
    }
}

fn domain_line(locale: Locale, code: &str, display: &str) -> String {
    match locale {
        Locale::En => format!("{code}: {display}"),
        Locale::ZhCn => format!("{code}：{display}"),
    }
}

fn optimize_code(error: &OptimizePlanError) -> &'static str {
    match error {
        OptimizePlanError::GuidanceNotExecutable(_) => "guidance_not_executable",
        OptimizePlanError::ObserveNotPlanable(_) => "observe_not_planable",
        OptimizePlanError::PrecheckRefused { reason, .. } => reason.code(),
        OptimizePlanError::DigestMismatch => "digest_mismatch",
        OptimizePlanError::UnknownOperation(_) => "unknown_operation",
        _ => "optimize_plan_error",
    }
}

fn software_segment(id: &str) -> Option<SoftwareSegment> {
    match id {
        "uninstall" => Some(SoftwareSegment::Uninstall),
        "updates" => Some(SoftwareSegment::Updates),
        "startup" => Some(SoftwareSegment::Startup),
        "leftovers" => Some(SoftwareSegment::Leftovers),
        _ => None,
    }
}

fn dummy_software_plan() -> SoftwareSelectionPlanV1 {
    SoftwareSelectionPlanV1 {
        version: SOFTWARE_PLAN_VERSION,
        inventory_fingerprint: format!("sha256:{}", "ab".repeat(32)),
        inventory_observed_at_unix_ms: 0,
        expires_at_unix_ms: 0,
        selected_ids: Vec::new(),
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    let value = std::env::var_os(key)?;
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn env_flag(key: &str) -> bool {
    std::env::var_os(key).is_some_and(|value| value == "1")
}

fn flipped_digest(digest: &str) -> String {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return format!("sha256:{}", "cd".repeat(32));
    };
    let mut chars: Vec<u8> = hex.as_bytes().to_vec();
    if let Some(last) = chars.last_mut() {
        *last = if *last == b'a' { b'b' } else { b'a' };
    }
    format!("sha256:{}", String::from_utf8_lossy(&chars))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{Mode, NavKey, Session, VISIBLE_ROW_CAPACITY};
    use devsweep_core::software::{
        StartupFixtureEntryV1, StartupFixtureHiveV1, execute_startup_hive,
    };
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn hive_entry(
        name: &str,
        location: SoftwareStartupLocation,
        class: StartupFixtureClass,
        approval: Option<Vec<u8>>,
    ) -> StartupFixtureEntryV1 {
        StartupFixtureEntryV1 {
            name: name.to_string(),
            location,
            class,
            run_value: b"C:\\fixture\\app.exe".to_vec(),
            approval,
        }
    }

    fn write_hive(path: &Path) {
        let hive = StartupFixtureHiveV1 {
            version: 1,
            entries: vec![
                hive_entry(
                    "ordinary",
                    SoftwareStartupLocation::CurrentUserRun,
                    StartupFixtureClass::Ordinary,
                    None,
                ),
                hive_entry(
                    "guard",
                    SoftwareStartupLocation::CurrentUserRun,
                    StartupFixtureClass::Security,
                    None,
                ),
                hive_entry(
                    "machine",
                    SoftwareStartupLocation::MachineRun,
                    StartupFixtureClass::Ordinary,
                    None,
                ),
                hive_entry(
                    "blank",
                    SoftwareStartupLocation::CurrentUserRun,
                    StartupFixtureClass::Ordinary,
                    Some(Vec::new()),
                ),
            ],
        };
        let bytes = serde_json::to_vec(&hive).expect("hive json");
        fs::write(path, bytes).expect("write hive");
    }

    #[test]
    fn isolated_hive_review_confirms_errors_and_does_not_touch_host_startup() {
        let _guard = env_lock().lock().expect("env lock");
        let root = std::env::temp_dir().join(format!(
            "devsweep-gpui-c12-hive-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("hive root");
        let hive_path = root.join("hive.json");
        write_hive(&hive_path);
        let before = fs::read(&hive_path).expect("hive bytes");
        unsafe {
            std::env::set_var(HIVE_ENV, &hive_path);
            std::env::remove_var(STALE_ENV);
            std::env::set_var(
                UPDATE_ENV,
                root.join("missing-update-provider.exe"),
            );
        }

        let mut review = WorkbenchReview::new();
        review.activate("leftovers", Locale::En);
        let leftovers = review.lines().join("\n");
        assert!(leftovers.contains("unavailable") || leftovers.contains("Leftovers"));
        assert!(!leftovers.to_ascii_lowercase().contains("succeed"));

        review.activate("updates", Locale::En);
        let updates = review.lines().join("\n");
        assert!(
            updates.contains("probe_failed") || updates.contains("update provider probe failed"),
            "{updates}"
        );
        assert!(!updates.to_ascii_lowercase().contains("succeed"));

        review.activate("startup", Locale::En);
        let startup = review.lines().join("\n");
        assert!(startup.contains("sha256:"), "{startup}");
        assert!(startup.contains("protected"), "{startup}");
        assert!(
            startup.contains("requires_administrator") || startup.contains("requires administrator"),
            "{startup}"
        );
        assert!(
            startup.contains("startup_identity_unavailable")
                || startup.contains("identity is unavailable"),
            "{startup}"
        );
        assert!(!startup.to_ascii_lowercase().contains("succeed"));
        assert_eq!(fs::read(&hive_path).expect("hive unchanged"), before);

        review.activate("startup", Locale::En);
        let confirmed = review.lines().join("\n");
        assert!(
            confirmed.contains("outcome="),
            "second Enter must report the domain hive outcome: {confirmed}"
        );
        assert!(confirmed.contains("last=not run"), "{confirmed}");
        assert_ne!(fs::read(&hive_path).expect("hive written"), before);

        unsafe {
            std::env::remove_var(HIVE_ENV);
            std::env::remove_var(UPDATE_ENV);
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_hive_digest_refuses_and_leaves_the_file() {
        let _guard = env_lock().lock().expect("env lock");
        let root = std::env::temp_dir().join(format!(
            "devsweep-gpui-c12-stale-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("stale root");
        let hive_path = root.join("hive.json");
        write_hive(&hive_path);
        let before = fs::read(&hive_path).expect("before");
        unsafe {
            std::env::set_var(HIVE_ENV, &hive_path);
            std::env::set_var(STALE_ENV, "1");
        }
        let mut review = WorkbenchReview::new();
        review.activate("startup", Locale::ZhCn);
        review.activate("startup", Locale::ZhCn);
        let lines = review.lines().join("\n");
        assert!(
            lines.contains("stale_confirmation") || lines.contains("digest is stale"),
            "{lines}"
        );
        assert!(!lines.contains("成功"));
        assert_eq!(fs::read(&hive_path).expect("unchanged"), before);
        unsafe {
            std::env::remove_var(HIVE_ENV);
            std::env::remove_var(STALE_ENV);
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn guidance_observe_and_host_rows_are_not_scheduled_as_success() {
        let guidance = plan_operation("guidance.drive_optimize").expect_err("guidance");
        assert!(guidance.to_string().contains("never dispatched"));
        let observe = plan_operation("maintenance.disk_health").expect_err("observe");
        assert!(observe.to_string().contains("cannot be planned"));

        let mut review = WorkbenchReview::new();
        review.activate("guidance.drive_optimize", Locale::En);
        let lines = review.lines().join("\n");
        assert!(lines.contains("never dispatched"), "{lines}");
        assert!(review.pending_id().is_none(), "guidance has no confirm arm");

        review.activate("maintenance.disk_health", Locale::ZhCn);
        let observe_lines = review.lines().join("\n");
        assert!(observe_lines.contains("cannot be planned"), "{observe_lines}");
        assert!(!observe_lines.contains("成功"));

        review.activate("maintenance.retrim", Locale::En);
        let host = review.lines().join("\n");
        assert!(host.contains("last=not run"), "{host}");
        assert!(!host.to_ascii_lowercase().contains("succeed"), "{host}");
        let notice = review.precheck();
        if let Some(reason) = notice {
            assert_eq!(reason, PrecheckReason::UacDeclined);
        }
        review.activate("maintenance.retrim", Locale::En);
        let second = review.lines().join("\n");
        assert!(
            second.contains("confirmation_required")
                || second.contains("explicit confirmation")
                || second.contains("uac_declined")
                || second.contains("last=not run"),
            "{second}"
        );
        assert!(!second.to_ascii_lowercase().contains("succeed"));
    }

    #[test]
    fn keyboard_enter_writes_the_domain_review_and_privilege_notice() {
        let mut session = Session::unbacked();
        session.show_mode(Mode::Optimize);
        session.select_mode_row(Mode::Optimize, "maintenance.retrim");
        session.apply_key(NavKey::Enter);
        let lines = crate::view::panel_lines(&session, None).join("\n");
        assert!(
            lines.contains("uac_declined")
                || lines.contains("Elevation was declined")
                || lines.contains("sha256:"),
            "{lines}"
        );
        assert!(lines.contains("last=not run") || lines.contains("未运行"), "{lines}");
        assert!(!lines.to_ascii_lowercase().contains("succeed"));
        if lines.contains("uac_declined") {
            assert_eq!(
                session.workbench_notice(),
                Some(&crate::shell::WorkbenchNotice::PrivilegeDecline)
            );
        }

        session.show_mode(Mode::Software);
        session.select_mode_row(Mode::Software, "leftovers");
        session.apply_key(NavKey::Enter);
        let leftovers = crate::view::panel_lines(&session, None).join("\n");
        assert!(leftovers.contains("Leftovers") || leftovers.contains("残留"));
        assert!(!leftovers.to_ascii_lowercase().contains("succeed"));
    }

    #[test]
    fn uninstall_confirm_is_the_domain_refusal_and_does_not_execute() {
        let mut review = WorkbenchReview::new();
        review.activate("uninstall", Locale::En);
        let lines = review.lines().join("\n");
        assert!(lines.contains("confirmation_required"), "{lines}");
        assert!(lines.contains("explicit confirmation"), "{lines}");
        assert!(!lines.to_ascii_lowercase().contains("removed"));
        assert!(!lines.to_ascii_lowercase().contains("succeed"));
        review.activate("uninstall", Locale::ZhCn);
        let zh = review.lines().join("\n");
        assert!(zh.contains("confirmation_required"), "{zh}");
        assert!(!zh.contains("成功"));
    }

    #[test]
    fn isolated_row_window_stays_bounded_and_keeps_offscreen_selection() {
        let root = std::env::temp_dir().join(format!(
            "devsweep-gpui-c12-rows-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("row root");
        for index in 0..1_000 {
            fs::write(root.join(format!("row-{index:04}")), b"isolated").expect("row file");
        }
        let ids = isolated_row_ids(&root);
        assert_eq!(ids.len(), 1_000);
        let far = ids[900].clone();
        let mut session = Session::unbacked();
        session.show_mode(Mode::Software);
        session.set_row_ids(Mode::Software, ids);
        session.set_mode_row_window(Mode::Software, 0);
        session.select_mode_row(Mode::Software, far.clone());
        let visible = session.shell().visible_row_ids(Mode::Software);
        assert!(visible.len() <= VISIBLE_ROW_CAPACITY);
        assert!(!visible.contains(&far));
        assert_eq!(session.shell().selection(Mode::Software), Some(far.as_str()));
        let lines = crate::view::panel_lines(&session, None);
        assert!(
            lines.iter().any(|line| line == &format!("> {far}")),
            "offscreen selection must stay visible once: {lines:?}"
        );
        assert!(
            lines.iter().filter(|line| line.contains("row-")).count() < 1_000,
            "panel must not paint 1000 isolated rows"
        );
        assert!(lines.len() < 1_000);
        session.show_mode(Mode::Optimize);
        session.apply_key(NavKey::Right);
        assert_eq!(session.shell().selection(Mode::Software), Some(far.as_str()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn direct_hive_execute_stale_matches_the_keyboard_refusal() {
        let root = std::env::temp_dir().join(format!(
            "devsweep-gpui-c12-direct-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("direct root");
        let hive_path = root.join("hive.json");
        write_hive(&hive_path);
        let id = startup_fixture_entry_id(SoftwareStartupLocation::CurrentUserRun, "ordinary");
        let plan = plan_startup_hive(&hive_path, &id, false).expect("plan");
        let preview = preview_startup_hive(&hive_path, &plan).expect("preview");
        let before = fs::read(&hive_path).expect("before");
        let error = execute_startup_hive(&hive_path, &plan, &flipped_digest(&preview.digest), true)
            .expect_err("stale");
        assert_eq!(error.code(), "stale_confirmation");
        assert_eq!(fs::read(&hive_path).expect("unchanged"), before);
        let _ = fs::remove_dir_all(&root);
    }
}
