//! Clean preview identity for the GPUI shell.
//!
//! The shell calls shared scan and digest services. It does not hash locally
//! and it does not scan global caches from this helper. AI preview uses
//! `preview_ai` on a saved C15 plan and does not execute. AI scan and plan wrap
//! `scan_ai` and `plan_ai` behind an explicit home so neither can fall back to
//! a production default. Recovery listing and restore planning wrap
//! `recovery_list` and `restore_plan` and do not restore. Confirmed AI execute
//! wraps `execute_ai` with a caller-supplied cancel observer and trash runner.
//! This module does not construct a production Recycle Bin adapter except in
//! [`ai_execute_production`], which is reached only from a confirmed user
//! action.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use devsweep_core::{
    ai::{
        AiClass, AiError, AiExecuteReportV1, AiObservationV1, AiPlanV1, AiProvider,
        AiRecoveryListV1, AiScanRequest, execute_ai, plan_ai, preview_ai, recovery_list,
        restore_plan, scan_ai,
    },
    execution::{SystemTrashRunner, TrashRunner, confirmation_digest},
    model::{ScanCoverageScope, ScanReport, Scope, TargetId, UntrustedPlan},
    plan::{ValidatedPlan, validate_plan},
    process::CancelObserver,
    scan::{ScanOptions, Sweeper},
};

const PYTEST_CACHE_RULE: &str = "python.pytest_cache";

/// One project-only preview whose digest is the `python.pytest_cache` selection.
pub struct PytestCachePreview {
    /// Live `sha256:` confirmation digest for the pytest cache target only.
    pub digest: String,
    /// The validated plan produced by this scan.
    pub plan: ValidatedPlan,
    /// Identifier of the `python.pytest_cache` target.
    pub target_id: TargetId,
    /// The scan report used to validate the plan.
    pub report: ScanReport,
}

/// Scan one project root and return the confirmation digest for `.pytest_cache`.
///
/// Global caches are never requested. Missing `python.pytest_cache` is an error.
/// The digest uses the same saved-plan selection as `clean plan --select`.
pub fn pytest_cache_confirmation_digest(root: impl AsRef<Path>) -> Result<PytestCachePreview> {
    let root = root.as_ref().to_path_buf();
    let report = Sweeper::default().full_scan_report(
        &ScanOptions {
            include_projects: true,
            include_global: false,
            roots: vec![root],
        },
        &mut |_| {},
    )?;
    if report
        .plan
        .targets
        .iter()
        .any(|target| matches!(target.scope, Scope::Global))
        || report
            .health
            .coverage
            .iter()
            .any(|unit| unit.scope == ScanCoverageScope::Provider)
    {
        bail!("project-only scan included a global cache");
    }
    let mut target = report
        .plan
        .targets
        .iter()
        .find(|candidate| candidate.rule_id == PYTEST_CACHE_RULE)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing python.pytest_cache target"))?;
    target.selected_by_default = true;
    let target_id = target.id.clone();
    let plan = validate_plan(&UntrustedPlan {
        version: report.plan.version,
        targets: vec![target],
    })?;
    let digest = confirmation_digest(&plan, std::slice::from_ref(&target_id))?;
    Ok(PytestCachePreview {
        digest: format!("sha256:{}", digest.as_str()),
        plan,
        target_id,
        report,
    })
}

/// Plan target ids from one project-only scan of `root`.
///
/// Global caches are never requested. This does not select or execute targets.
pub fn project_target_ids(root: &Path) -> Result<Vec<String>> {
    let report = Sweeper::default().full_scan_report(
        &ScanOptions {
            include_projects: true,
            include_global: false,
            roots: vec![root.to_path_buf()],
        },
        &mut |_| {},
    )?;
    if report
        .plan
        .targets
        .iter()
        .any(|target| matches!(target.scope, Scope::Global))
        || report
            .health
            .coverage
            .iter()
            .any(|unit| unit.scope != ScanCoverageScope::Project)
    {
        bail!("project-only scan included a global cache");
    }
    Ok(report
        .plan
        .targets
        .iter()
        .map(|target| target.id.as_str().to_string())
        .collect())
}

/// Live `sha256:` digest for a saved C15 AI cleanup plan.
pub struct AiPreviewDigest {
    /// Live digest from [`preview_ai`]. Always starts with `sha256:`.
    pub digest: String,
}

/// One provider home from the shared core resolver.
///
/// This wraps `devsweep_core::ai::default_home_path`; it never invents a path
/// and it never reads the environment itself. `userprofile` stands in for
/// `USERPROFILE` (or `HOME`) and `appdata` for `APPDATA`, so an isolated test
/// environment resolves the same way the production environment does.
pub fn ai_provider_home(
    provider: AiProvider,
    userprofile: Option<&Path>,
    appdata: Option<&Path>,
) -> Option<PathBuf> {
    devsweep_core::ai::default_home_path(provider, userprofile, appdata)
}

/// Production path the window must not scan.
///
/// Reads `USERPROFILE` (falling back to `HOME`) and `APPDATA`, then delegates
/// to [`ai_provider_home`]. The Clean AI button does not call this. A test may
/// compare it to prove the window left the real profile unread.
#[allow(dead_code)]
pub fn ai_provider_home_from_env(provider: AiProvider) -> Option<PathBuf> {
    let userprofile = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
    ai_provider_home(provider, userprofile.as_deref(), appdata.as_deref())
}

/// Why an explicit AI review home was refused. A refusal never falls back to
/// `USERPROFILE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplicitAiHomeRefusal {
    Empty,
    NotAbsolute,
    OutsideTemp,
    MissingPrefix,
    NotADirectory,
}

/// An explicit isolated AI home, or the reason none may be scanned.
///
/// `Isolated` is the only home the window may scan. `Unset` and `Refused` must
/// not be replaced by `USERPROFILE`, `APPDATA`, or a real provider profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplicitAiHome {
    /// No override was set. This is not permission to read a real profile.
    Unset,
    Isolated(PathBuf),
    Refused(ExplicitAiHomeRefusal),
}

const AI_REVIEW_HOME_ENV: &str = "DEVSWEEP_AI_REVIEW_HOME";
const AI_REVIEW_HOME_PREFIX: &str = "devsweep-gpui-c11-";

/// Reads `DEVSWEEP_AI_REVIEW_HOME`. Unset is not a real profile. A set value
/// is accepted only when it is an absolute directory under the temp root whose
/// path contains the isolated-fixture prefix.
pub fn explicit_ai_review_home() -> ExplicitAiHome {
    classify_explicit_ai_review_home(std::env::var_os(AI_REVIEW_HOME_ENV).as_deref(), &std::env::temp_dir())
}

/// Pure classifier for [`explicit_ai_review_home`]. Does not read the environment.
pub fn classify_explicit_ai_review_home(value: Option<&std::ffi::OsStr>, temp: &Path) -> ExplicitAiHome {
    let Some(value) = value else {
        return ExplicitAiHome::Unset;
    };
    if value.is_empty() {
        return ExplicitAiHome::Refused(ExplicitAiHomeRefusal::Empty);
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return ExplicitAiHome::Refused(ExplicitAiHomeRefusal::NotAbsolute);
    }
    if !path.starts_with(temp) {
        return ExplicitAiHome::Refused(ExplicitAiHomeRefusal::OutsideTemp);
    }
    let prefixed = path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|name| name.starts_with(AI_REVIEW_HOME_PREFIX))
    });
    if !prefixed {
        return ExplicitAiHome::Refused(ExplicitAiHomeRefusal::MissingPrefix);
    }
    if !path.is_dir() {
        return ExplicitAiHome::Refused(ExplicitAiHomeRefusal::NotADirectory);
    }
    ExplicitAiHome::Isolated(path)
}

/// Scan one explicit AI provider home. Never falls back to a production
/// default and never mutates the home.
pub fn ai_scan_home(
    provider: AiProvider,
    home: &Path,
    cancel: &dyn CancelObserver,
) -> std::result::Result<AiObservationV1, AiError> {
    scan_ai(
        &AiScanRequest {
            provider: Some(provider),
            home: Some(home.to_path_buf()),
        },
        cancel,
    )
}

/// Build one cleanup plan for one class from an accepted observation.
pub fn ai_plan(
    observation: &AiObservationV1,
    class: AiClass,
) -> std::result::Result<AiPlanV1, AiError> {
    plan_ai(observation, class)
}

/// Recompute the live C15 AI preview digest from a saved plan.
///
/// The shell does not hash locally. This does not mutate the home and does not
/// move files to the Recycle Bin.
pub fn ai_preview_digest(plan: &AiPlanV1) -> Result<AiPreviewDigest> {
    let preview = preview_ai(plan)?;
    if !preview.digest.starts_with("sha256:") {
        bail!("AI preview digest is missing the sha256: prefix");
    }
    Ok(AiPreviewDigest {
        digest: preview.digest,
    })
}

/// Recovery bundles under an isolated AI home. Does not restore files.
pub fn ai_recovery_list(home: impl AsRef<Path>) -> Result<AiRecoveryListV1> {
    Ok(recovery_list(home.as_ref())?)
}

/// Restore plan for one recovery bundle. Does not copy payload bytes.
pub fn ai_restore_plan(home: impl AsRef<Path>, bundle_id: &str) -> Result<AiPlanV1> {
    Ok(restore_plan(home.as_ref(), bundle_id)?)
}

/// Confirmed C15 AI execute with the caller-supplied cancel observer and trash
/// runner.
///
/// The shell does not construct a production Recycle Bin adapter. The injected
/// `TrashRunner` owns every move.
pub fn ai_execute(
    plan: &AiPlanV1,
    preview_digest: &str,
    confirmed: bool,
    cancel: &dyn CancelObserver,
    trash: &dyn TrashRunner,
) -> Result<AiExecuteReportV1> {
    Ok(execute_ai(plan, preview_digest, confirmed, cancel, trash)?)
}

/// Confirmed C15 AI execute with the shared production Recycle Bin runner.
///
/// This is the only entry that owns the real move, and only the shell's
/// confirmed user action reaches it. A stale digest or a missing confirmation
/// returns before the runner is touched.
pub fn ai_execute_production(
    plan: &AiPlanV1,
    preview_digest: &str,
    confirmed: bool,
    cancel: &dyn CancelObserver,
) -> Result<AiExecuteReportV1> {
    Ok(execute_ai(
        plan,
        preview_digest,
        confirmed,
        cancel,
        &SystemTrashRunner,
    )?)
}

#[cfg(test)]
pub(crate) use tests::{IsolatedRenameTrash, cleanup, isolated_pytest_project};

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        process::{Output, Stdio},
        sync::atomic::{AtomicU64, Ordering},
    };

    use devsweep_core::{
        ai::{
            AiClass, AiError, AiExit, AiPlanKind, AiProvider, AiScanRequest, plan_ai, preview_ai,
            scan_ai,
        },
        execution::{TrashRunner, confirmation_digest},
        model::{ScanCoverageScope, Scope},
        process::FlagCancelObserver,
    };
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::copy::{self, Locale};
    use crate::shell::{CleanOutcomeNotice, Mode, Session};
    use crate::view::panel_lines;

    pub(crate) struct IsolatedRenameTrash {
        home: PathBuf,
        calls: AtomicU64,
    }

    impl IsolatedRenameTrash {
        pub(crate) fn new(home: impl Into<PathBuf>) -> Self {
            let home = home.into();
            assert!(
                home.file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
                "refusing a trash runner outside the temp prefix"
            );
            Self {
                home,
                calls: AtomicU64::new(0),
            }
        }

        pub(crate) fn call_count(&self) -> u64 {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl TrashRunner for IsolatedRenameTrash {
        fn move_to_trash(&self, path: &Path) -> anyhow::Result<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if !path_is_inside_temp_home(path, &self.home) {
                anyhow::bail!("refusing a trash path outside the temp home");
            }
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| anyhow::anyhow!("trash path is missing a file name"))?;
            let dest = path.with_file_name(format!("{name}.trashed-by-test"));
            if !path_is_inside_temp_home(&dest, &self.home) {
                anyhow::bail!("refusing a trash destination outside the temp home");
            }
            fs::rename(path, dest)?;
            Ok(())
        }
    }

    fn path_is_inside_temp_home(path: &Path, home: &Path) -> bool {
        let isolated = home
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.starts_with("devsweep-gpui-c11-"));
        if !isolated {
            return false;
        }
        if path.starts_with(home) {
            return true;
        }
        // The trash destination does not exist yet, so canonicalizing the
        // whole path fails. Resolve the canonical parent and keep the final
        // name, which is what `fs::rename` will address.
        let canonical = |value: &Path| -> Option<PathBuf> {
            if let Ok(resolved) = fs::canonicalize(value) {
                return Some(resolved);
            }
            let parent = fs::canonicalize(value.parent()?).ok()?;
            Some(parent.join(value.file_name()?))
        };
        match (canonical(path), canonical(home)) {
            (Some(path), Some(home)) => path.starts_with(home),
            _ => false,
        }
    }

    const RECOVERY_BUNDLE_ID: &str = "0123456789abcdef0123456789abcdef";
    const RECOVERY_RELATIVE: &str = "cache/regen-a";
    const RECOVERY_PAYLOAD: &[u8] = b"c11-regen-cache-bytes";

    pub(crate) fn isolated_pytest_project() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-{}-pytest",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing a project fixture outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".pytest_cache")).unwrap();
        fs::write(root.join("tox.ini"), "[tox]\n").unwrap();
        fs::write(root.join(".pytest_cache").join("payload"), "cache fixture").unwrap();
        root
    }

    pub(crate) fn cleanup(root: &Path) {
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-"))
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn explicit_ai_review_home_refuses_a_real_profile_and_accepts_only_an_isolated_dir() {
        let temp = env::temp_dir();
        assert_eq!(
            classify_explicit_ai_review_home(None, &temp),
            ExplicitAiHome::Unset
        );
        let real = PathBuf::from(r"C:\Users\lyh\.codex");
        assert_eq!(
            classify_explicit_ai_review_home(Some(real.as_os_str()), &temp),
            ExplicitAiHome::Refused(ExplicitAiHomeRefusal::OutsideTemp)
        );
        let missing_prefix = temp.join("not-the-fixture");
        assert_eq!(
            classify_explicit_ai_review_home(Some(missing_prefix.as_os_str()), &temp),
            ExplicitAiHome::Refused(ExplicitAiHomeRefusal::MissingPrefix)
        );
        let isolated = temp.join(format!(
            "devsweep-gpui-c11-explicit-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&isolated);
        fs::create_dir_all(&isolated).unwrap();
        assert_eq!(
            classify_explicit_ai_review_home(Some(isolated.as_os_str()), &temp),
            ExplicitAiHome::Isolated(isolated.clone())
        );
        let _ = fs::remove_dir_all(&isolated);
        assert!(matches!(
            classify_explicit_ai_review_home(Some(isolated.as_os_str()), &temp),
            ExplicitAiHome::Refused(ExplicitAiHomeRefusal::NotADirectory)
        ));
    }

    #[test]
    fn pytest_cache_confirmation_digest_matches_core() {
        let root = isolated_pytest_project();
        let preview = pytest_cache_confirmation_digest(&root).expect("project-only pytest preview");
        assert!(
            preview
                .report
                .plan
                .targets
                .iter()
                .all(|target| !matches!(target.scope, Scope::Global)),
            "scan included a global cache target"
        );
        assert!(
            preview
                .report
                .health
                .coverage
                .iter()
                .all(|unit| unit.scope == ScanCoverageScope::Project),
            "scan included a global provider coverage unit"
        );
        let core = confirmation_digest(&preview.plan, std::slice::from_ref(&preview.target_id))
            .expect("core confirmation digest");
        let core_digest = format!("sha256:{}", core.as_str());
        assert_eq!(preview.digest, core_digest);
        assert!(preview.digest.starts_with("sha256:"));
        assert!(core_digest.starts_with("sha256:"));
        cleanup(&root);
    }

    #[test]
    fn project_target_ids_use_one_isolated_pytest_root() {
        let root = isolated_pytest_project();
        let name = root
            .file_name()
            .and_then(|value| value.to_str())
            .expect("utf-8 fixture name");
        assert!(
            name.starts_with("devsweep-gpui-c11-"),
            "fixture directory must start with devsweep-gpui-c11-: {name}"
        );
        let ids = project_target_ids(&root).expect("project-only target ids");
        let preview = pytest_cache_confirmation_digest(&root).expect("pytest target identity");
        assert!(
            ids.iter().any(|id| id == preview.target_id.as_str()),
            "returned ids missing pytest target {}: {ids:?}",
            preview.target_id.as_str()
        );
        assert!(
            preview
                .report
                .health
                .coverage
                .iter()
                .all(|unit| unit.scope == ScanCoverageScope::Project),
            "coverage included a non-project unit"
        );
        cleanup(&root);
    }

    #[test]
    fn pytest_cache_confirmation_digest_matches_cli() {
        let root = isolated_pytest_project();
        let work = isolated_cli_work();
        let bin = ensure_devsweep_cli();
        let observation = work.join("observation.json");
        let plan = work.join("plan.json");

        let scan_args = [
            "clean",
            "scan",
            "--root",
            root.to_str().expect("utf-8 project root"),
            "--scope",
            "projects",
            "--format",
            "json",
        ];
        assert_avoids_global_scope(&scan_args);
        let scan = run_devsweep(&bin, &scan_args, &work);
        assert_eq!(scan.status.code(), Some(0), "{}", stderr(&scan));
        let scan_document = envelope(&scan);
        assert_eq!(scan_document["command"], "clean.scan");
        assert_observation_is_project_only(&scan_document);
        fs::write(&observation, &scan.stdout).unwrap();

        let target_id = scan_document["data"]["plan"]["targets"]
            .as_array()
            .expect("scan targets")
            .iter()
            .find(|target| target["rule_id"] == PYTEST_CACHE_RULE)
            .and_then(|target| target["id"].as_str())
            .expect("python.pytest_cache target id")
            .to_string();

        let plan_args = [
            "clean",
            "plan",
            "--observation",
            observation.to_str().expect("utf-8 observation"),
            "--select",
            &target_id,
            "--output",
            plan.to_str().expect("utf-8 plan"),
        ];
        assert_avoids_global_scope(&plan_args);
        let planned = run_devsweep(&bin, &plan_args, &work);
        assert_eq!(planned.status.code(), Some(0), "{}", stderr(&planned));
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&plan).unwrap()).expect("saved plan JSON");
        assert_eq!(saved["targets"].as_array().unwrap().len(), 1);
        assert_eq!(saved["targets"][0]["id"], target_id);
        assert_eq!(saved["targets"][0]["rule_id"], PYTEST_CACHE_RULE);
        assert_eq!(saved["targets"][0]["scope"]["type"], "project");

        let preview_args = [
            "clean",
            "preview",
            "--plan",
            plan.to_str().expect("utf-8 plan"),
            "--format",
            "json",
        ];
        assert_avoids_global_scope(&preview_args);
        let preview = run_devsweep(&bin, &preview_args, &work);
        assert_eq!(preview.status.code(), Some(0), "{}", stderr(&preview));
        let preview_document = envelope(&preview);
        assert_eq!(preview_document["command"], "clean.preview");
        let cli_digest = preview_document["data"]["digest"]
            .as_str()
            .expect("clean preview digest")
            .to_string();

        let gpui = pytest_cache_confirmation_digest(&root).expect("project-only pytest preview");
        assert_eq!(cli_digest, gpui.digest);
        assert!(cli_digest.starts_with("sha256:"));
        assert_eq!(gpui.target_id.as_str(), target_id);
        assert!(
            root.join(".pytest_cache").join("payload").is_file(),
            "preview never cleans the fixture"
        );

        cleanup(&root);
        cleanup(&work);
    }

    #[test]
    fn ai_preview_digest_matches_cli() {
        let (home, planted) = isolated_ai_home();
        let work = isolated_cli_work();
        let bin = ensure_devsweep_cli();
        let observation = work.join("ai-observation.json");
        let plan = work.join("ai-plan.json");
        let home_arg = home.to_str().expect("utf-8 AI home");

        let scan_args = [
            "clean",
            "ai",
            "scan",
            "--provider",
            "codex",
            "--home",
            home_arg,
            "--format",
            "json",
        ];
        let scan = run_devsweep_ai(&bin, &scan_args, &work);
        assert_eq!(scan.status.code(), Some(0), "{}", stderr(&scan));
        let scan_document = envelope(&scan);
        assert_eq!(scan_document["command"], "clean.ai.scan");
        fs::write(&observation, &scan.stdout).unwrap();

        let plan_args = [
            "clean",
            "ai",
            "plan",
            "--scan",
            observation.to_str().expect("utf-8 observation"),
            "--class",
            "regenerable_cache",
            "--output",
            plan.to_str().expect("utf-8 plan"),
        ];
        let planned = run_devsweep_ai(&bin, &plan_args, &work);
        assert_eq!(planned.status.code(), Some(0), "{}", stderr(&planned));
        let saved: AiPlanV1 =
            serde_json::from_slice(&fs::read(&plan).unwrap()).expect("saved AI plan JSON");
        assert_eq!(saved.targets.len(), 1);
        assert_eq!(saved.targets[0].relative_path, "cache/regen-a");
        for target in &saved.targets {
            for denied in [
                "credentials",
                "memory",
                "settings",
                "skills",
                "mcp",
                "models",
                "projects",
            ] {
                assert!(
                    !target.relative_path.contains(denied),
                    "selected set included protected content {denied}: {}",
                    target.relative_path
                );
            }
        }

        let preview_args = [
            "clean",
            "ai",
            "preview",
            "--plan",
            plan.to_str().expect("utf-8 plan"),
            "--format",
            "json",
        ];
        let preview = run_devsweep_ai(&bin, &preview_args, &work);
        assert_eq!(preview.status.code(), Some(0), "{}", stderr(&preview));
        let preview_document = envelope(&preview);
        assert_eq!(preview_document["command"], "clean.ai.preview");
        let cli_digest = preview_document["data"]["digest"]
            .as_str()
            .expect("clean ai preview digest")
            .to_string();

        let gpui = ai_preview_digest(&saved).expect("GPUI AI preview");
        assert_eq!(cli_digest, gpui.digest);
        assert!(cli_digest.starts_with("sha256:"));
        assert!(gpui.digest.starts_with("sha256:"));

        for (path, bytes) in &planted {
            assert_eq!(
                fs::read(path).expect("protected file still readable"),
                *bytes,
                "protected file changed: {}",
                path.display()
            );
        }
        assert!(
            home.join("cache")
                .join("regen-a")
                .join("data.bin")
                .is_file(),
            "preview never cleans the regen cache fixture"
        );

        cleanup(&home);
        cleanup(&work);
    }

    #[test]
    fn ai_recovery_identity_matches_cli() {
        let (home, payload_path, payload_bytes, content_sha256) = isolated_recovery_bundle();
        let work = isolated_cli_work();
        let bin = ensure_devsweep_cli();
        let restore_plan_path = work.join("restore-plan.json");
        let home_arg = home.to_str().expect("utf-8 AI home");

        for denied in [
            "credentials",
            "memory",
            "settings",
            "skills",
            "mcp",
            "models",
            "projects",
        ] {
            assert!(
                !RECOVERY_RELATIVE.contains(denied),
                "recovery relative_path included protected content {denied}: {RECOVERY_RELATIVE}"
            );
        }

        let list_args = [
            "clean",
            "ai",
            "recovery-list",
            "--home",
            home_arg,
            "--format",
            "json",
        ];
        let listed = run_devsweep_ai(&bin, &list_args, &work);
        assert_eq!(listed.status.code(), Some(0), "{}", stderr(&listed));
        let list_document = envelope(&listed);
        assert_eq!(list_document["command"], "clean.ai.recovery-list");
        let cli_bundles = list_document["data"]["bundles"]
            .as_array()
            .expect("recovery-list bundles");
        assert_eq!(cli_bundles.len(), 1);
        let cli_id = cli_bundles[0]["id"].as_str().expect("CLI bundle id");
        let cli_relative = cli_bundles[0]["relative_path"]
            .as_str()
            .expect("CLI relative_path");
        let cli_hash = cli_bundles[0]["content_sha256"]
            .as_str()
            .expect("CLI content_sha256");

        let gpui_list = ai_recovery_list(&home).expect("GPUI recovery list");
        assert_eq!(gpui_list.bundles.len(), 1);
        assert_eq!(cli_id, gpui_list.bundles[0].id);
        assert_eq!(cli_relative, gpui_list.bundles[0].relative_path);
        assert_eq!(cli_hash, gpui_list.bundles[0].content_sha256);
        assert_eq!(cli_id, RECOVERY_BUNDLE_ID);
        assert_eq!(cli_relative, RECOVERY_RELATIVE);
        assert_eq!(cli_hash, content_sha256);

        let restore_args = [
            "clean",
            "ai",
            "restore-plan",
            "--home",
            home_arg,
            "--bundle",
            RECOVERY_BUNDLE_ID,
            "--output",
            restore_plan_path.to_str().expect("utf-8 restore plan"),
        ];
        let restored = run_devsweep_ai(&bin, &restore_args, &work);
        assert_eq!(restored.status.code(), Some(0), "{}", stderr(&restored));
        let cli_plan: AiPlanV1 =
            serde_json::from_slice(&fs::read(&restore_plan_path).expect("CLI restore plan file"))
                .expect("CLI restore plan JSON");
        let gpui_plan = ai_restore_plan(&home, RECOVERY_BUNDLE_ID).expect("GPUI restore plan");
        assert_eq!(cli_plan, gpui_plan);
        assert_eq!(gpui_plan.kind, AiPlanKind::Restore);
        assert_eq!(gpui_plan.targets.len(), 1);
        assert_eq!(gpui_plan.targets[0].action, "restore_bytes");
        assert_eq!(gpui_plan.targets[0].content_sha256, content_sha256);
        assert_eq!(gpui_plan.bundle_id.as_deref(), Some(RECOVERY_BUNDLE_ID));

        assert_eq!(
            fs::read(&payload_path).expect("payload still readable"),
            payload_bytes,
            "recovery-list and restore-plan must not change payload bytes"
        );

        cleanup(&home);
        cleanup(&work);
    }

    #[test]
    fn ai_execute_gate_stays_inside_temp() {
        let (home, planted) = isolated_ai_home();
        let credentials = planted
            .iter()
            .find(|(path, _)| {
                path.file_name().and_then(|value| value.to_str()) == Some("credentials")
            })
            .expect("credentials file planted");
        let credentials_bytes = credentials.1.clone();
        let regen = home.join("cache").join("regen-a");
        let regen_file = regen.join("data.bin");
        let original = fs::read(&regen_file).expect("regen cache bytes");
        let cancel = FlagCancelObserver::new();
        let observation = scan_ai(
            &AiScanRequest {
                provider: Some(AiProvider::Codex),
                home: Some(home.clone()),
            },
            &cancel,
        )
        .expect("scan_ai");
        let mut plan = plan_ai(&observation, AiClass::RegenerableCache).expect("plan_ai");
        plan.targets
            .retain(|target| target.relative_path == "cache/regen-a");
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.targets[0].relative_path, "cache/regen-a");
        for target in &plan.targets {
            assert!(
                !target.relative_path.contains("credentials"),
                "selected set included credentials: {}",
                target.relative_path
            );
        }
        let preview = preview_ai(&plan).expect("preview_ai");
        assert!(preview.digest.starts_with("sha256:"));
        let gpui_preview = ai_preview_digest(&plan).expect("GPUI AI preview");
        assert_eq!(gpui_preview.digest, preview.digest);

        let trash = IsolatedRenameTrash::new(PathBuf::from(&plan.home));
        let wrong_digest = format!("sha256:{}", "0".repeat(64));
        assert_ne!(wrong_digest, preview.digest);
        ai_execute(&plan, &wrong_digest, true, &cancel, &trash)
            .expect_err("wrong digest must not execute");
        assert_eq!(trash.call_count(), 0);
        assert!(
            regen_file.is_file(),
            "wrong digest must leave the regen cache in place"
        );

        ai_execute(&plan, &preview.digest, false, &cancel, &trash)
            .expect_err("unconfirmed execute must not run");
        assert_eq!(trash.call_count(), 0);
        assert!(
            regen_file.is_file(),
            "unconfirmed execute must leave the regen cache in place"
        );

        let report = ai_execute(&plan, &preview.digest, true, &cancel, &trash)
            .expect("confirmed matching digest");
        assert_eq!(report.outcome, "succeeded");
        assert_eq!(trash.call_count(), 1);
        assert!(
            !regen.exists(),
            "confirmed execute must move the original regen path"
        );
        assert_eq!(report.bundle_ids.len(), 1);
        let payload = home
            .join(".devsweep-recovery")
            .join(&report.bundle_ids[0])
            .join("payload");
        let recovered = if payload.is_file() {
            fs::read(&payload).expect("recovery payload file")
        } else {
            fs::read(payload.join("data.bin")).expect("recovery payload data.bin")
        };
        assert_eq!(recovered, original);
        assert_eq!(
            fs::read(&credentials.0).expect("credentials still readable"),
            credentials_bytes,
            "credentials must remain unchanged"
        );

        cleanup(&home);
    }

    #[test]
    fn ai_cli_stale_execute_matches_notice() {
        let (home, planted) = isolated_ai_home();
        let credentials = planted
            .iter()
            .find(|(path, _)| {
                path.file_name().and_then(|value| value.to_str()) == Some("credentials")
            })
            .expect("credentials file planted");
        let credentials_bytes = credentials.1.clone();
        let regen_file = home.join("cache").join("regen-a").join("data.bin");
        let regen_bytes = fs::read(&regen_file).expect("regen cache bytes");
        let work = isolated_cli_work();
        let bin = ensure_devsweep_cli();
        let plan_path = work.join("ai-plan.json");
        let cancel = FlagCancelObserver::new();

        let observation = scan_ai(
            &AiScanRequest {
                provider: Some(AiProvider::Codex),
                home: Some(home.clone()),
            },
            &cancel,
        )
        .expect("scan_ai");
        let mut plan = plan_ai(&observation, AiClass::RegenerableCache).expect("plan_ai");
        plan.targets
            .retain(|target| target.relative_path == "cache/regen-a");
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.targets[0].relative_path, "cache/regen-a");
        for target in &plan.targets {
            assert!(
                !target.relative_path.contains("credentials"),
                "selected set included credentials: {}",
                target.relative_path
            );
        }
        fs::write(
            &plan_path,
            serde_json::to_vec_pretty(&plan).expect("serialize AI plan"),
        )
        .expect("write AI plan");

        let preview = preview_ai(&plan).expect("preview_ai");
        let live_digest = preview.digest.clone();
        assert!(live_digest.starts_with("sha256:"));
        let mut wrong_digest = format!("sha256:{}", "0".repeat(64));
        if wrong_digest == live_digest {
            wrong_digest = format!("sha256:{}", "1".repeat(64));
        }
        assert!(
            well_formed_preview_digest(&wrong_digest),
            "wrong digest must match parse_preview_digest: {wrong_digest}"
        );
        assert_ne!(wrong_digest, live_digest);

        let execute_args = [
            "clean",
            "ai",
            "execute",
            "--plan",
            plan_path.to_str().expect("utf-8 plan"),
            "--preview-digest",
            &wrong_digest,
            "--confirm",
            "--format",
            "json",
        ];
        let executed = run_devsweep_ai_stale_execute(&bin, &execute_args, &work, &live_digest);
        if !regen_file.is_file() {
            panic!(
                "regen cache disappeared after stale execute; do not retry with the live digest. stdout={} stderr={}",
                String::from_utf8_lossy(&executed.stdout),
                stderr(&executed)
            );
        }
        assert!(
            !executed.status.success(),
            "stale execute must be non-zero: stdout={} stderr={}",
            String::from_utf8_lossy(&executed.stdout),
            stderr(&executed)
        );
        let document = envelope(&executed);
        assert_eq!(document["error"]["code"], "stale_confirmation");
        assert_eq!(
            fs::read(&regen_file).expect("regen cache still readable"),
            regen_bytes,
            "stale execute must leave the regen cache unchanged"
        );
        assert_eq!(
            fs::read(&credentials.0).expect("credentials still readable"),
            credentials_bytes,
            "credentials must remain unchanged"
        );

        let code = document["error"]["code"].as_str().expect("JSON error.code");
        assert_eq!(code, "stale_confirmation");
        let error = AiError {
            exit: AiExit::Stale,
            code: "stale_confirmation",
            message: "the live AI identity does not match the preview digest".to_string(),
        };
        let notice = CleanOutcomeNotice::from_ai_error(&error).expect("stale_confirmation maps");
        assert_eq!(notice, CleanOutcomeNotice::StalePreview);

        let mut session = Session::unbacked();
        session.show_mode(Mode::Clean);
        session.set_clean_outcome(Some(notice));
        let expected = copy::clean_outcome_stale_preview(Locale::En);
        assert_eq!(expected, "The preview digest is stale.");
        let lines = panel_lines(&session, None);
        assert_eq!(
            lines.last(),
            Some(&expected.to_string()),
            "Clean panel_lines must end with the stale-preview sentence: {lines:?}"
        );

        cleanup(&home);
        cleanup(&work);
    }

    fn isolated_cli_work() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-{}-cli",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing a CLI work directory outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    pub(crate) fn isolated_ai_home() -> (PathBuf, Vec<(PathBuf, Vec<u8>)>) {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-{}-ai-home",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing an AI home outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("cache").join("regen-a")).unwrap();
        fs::write(
            root.join("cache").join("regen-a").join("data.bin"),
            b"cache-bytes",
        )
        .unwrap();

        let protected = [
            ("settings.json", b"{}\n".as_slice()),
            ("credentials", b"secret".as_slice()),
            ("memory.md", b"memory".as_slice()),
            ("skills/skill.md", b"skill".as_slice()),
            ("mcp/server.json", b"{}".as_slice()),
            ("models/weights.bin", b"model".as_slice()),
            ("projects/code.txt", b"project".as_slice()),
        ];
        let mut planted = Vec::new();
        for (relative, bytes) in protected {
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, bytes).unwrap();
            planted.push((path, bytes.to_vec()));
        }
        (root, planted)
    }

    fn isolated_recovery_bundle() -> (PathBuf, PathBuf, Vec<u8>, String) {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "devsweep-gpui-c11-{}-{}-ai-recovery",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(
            root.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with("devsweep-gpui-c11-")),
            "refusing a recovery home outside the temp prefix"
        );
        let _ = fs::remove_dir_all(&root);
        let bundle = root.join(".devsweep-recovery").join(RECOVERY_BUNDLE_ID);
        fs::create_dir_all(&bundle).unwrap();
        let payload_path = bundle.join("payload");
        fs::write(&payload_path, RECOVERY_PAYLOAD).unwrap();
        let payload_bytes = fs::read(&payload_path).unwrap();
        let content_sha256 = sha256_file_hex(&payload_bytes);
        let manifest = serde_json::json!({
            "schema": "devsweep.ai.recovery.v1",
            "version": 1,
            "id": RECOVERY_BUNDLE_ID,
            "provider": "codex",
            "class": "regenerable_cache",
            "relative_path": RECOVERY_RELATIVE,
            "content_sha256": content_sha256,
            "kind": "file",
        });
        fs::write(
            bundle.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        (root, payload_path, payload_bytes, content_sha256)
    }

    fn sha256_file_hex(bytes: &[u8]) -> String {
        let digest = Sha256::digest(bytes);
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("desktop-gpui lives in the repository")
            .to_path_buf()
    }

    fn ensure_devsweep_cli() -> PathBuf {
        let repo = repo_root();
        let output = crate::process_spawn::command_without_window("cargo")
            .args([
                "build",
                "--locked",
                "-p",
                "devsweep-cli",
                "--bin",
                "devsweep",
                "--target-dir",
                "dist/ci-target",
            ])
            .current_dir(&repo)
            .output()
            .expect("cargo build starts");
        assert!(
            output.status.success(),
            "cargo build --locked -p devsweep-cli failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bin = repo.join(format!(
            "dist/ci-target/debug/devsweep{}",
            std::env::consts::EXE_SUFFIX
        ));
        assert!(bin.is_file(), "missing built CLI {}", bin.display());
        bin
    }

    fn run_devsweep(bin: &Path, args: &[&str], config_dir: &Path) -> Output {
        assert_avoids_global_scope(args);
        crate::process_spawn::command_without_window(bin)
            .args(args)
            .env("LOCALAPPDATA", config_dir)
            .env("APPDATA", config_dir)
            .env("XDG_CONFIG_HOME", config_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("devsweep process runs")
    }

    fn run_devsweep_ai(bin: &Path, args: &[&str], config_dir: &Path) -> Output {
        assert_ai_non_mutating(args);
        crate::process_spawn::command_without_window(bin)
            .args(args)
            .env("LOCALAPPDATA", config_dir)
            .env("APPDATA", config_dir)
            .env("XDG_CONFIG_HOME", config_dir)
            .env("USERPROFILE", config_dir)
            .env("HOME", config_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("devsweep process runs")
    }

    fn well_formed_preview_digest(value: &str) -> bool {
        let Some(digest) = value.strip_prefix("sha256:") else {
            return false;
        };
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }

    fn run_devsweep_ai_stale_execute(
        bin: &Path,
        args: &[&str],
        config_dir: &Path,
        live_digest: &str,
    ) -> Output {
        assert!(
            args.windows(2).any(|pair| pair == ["clean", "ai"]),
            "expected clean ai argv: {args:?}"
        );
        assert!(
            args.contains(&"execute"),
            "stale execute helper requires execute: {args:?}"
        );
        assert!(
            args.contains(&"--confirm"),
            "stale execute requires --confirm: {args:?}"
        );
        let digest = args
            .windows(2)
            .find(|pair| pair[0] == "--preview-digest")
            .map(|pair| pair[1])
            .expect("stale execute requires --preview-digest");
        assert!(
            well_formed_preview_digest(digest),
            "preview digest must be sha256: plus 64 lowercase hex: {digest}"
        );
        assert_ne!(
            digest, live_digest,
            "stale execute must not pass the live digest"
        );
        crate::process_spawn::command_without_window(bin)
            .args(args)
            .env("LOCALAPPDATA", config_dir)
            .env("APPDATA", config_dir)
            .env("XDG_CONFIG_HOME", config_dir)
            .env("USERPROFILE", config_dir)
            .env("HOME", config_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("devsweep process runs")
    }

    fn assert_ai_non_mutating(args: &[&str]) {
        assert!(
            args.windows(2).any(|pair| pair == ["clean", "ai"]),
            "expected clean ai argv: {args:?}"
        );
        assert!(
            !args.contains(&"execute"),
            "AI execute is forbidden in this test: {args:?}"
        );
        if args.contains(&"scan")
            || args.contains(&"recovery-list")
            || args.contains(&"restore-plan")
        {
            assert!(
                args.windows(2).any(|pair| pair[0] == "--home"
                    && Path::new(pair[1]).is_absolute()
                    && Path::new(pair[1])
                        .file_name()
                        .and_then(|value| value.to_str())
                        .is_some_and(|value| value.starts_with("devsweep-gpui-c11-"))),
                "clean ai must pass an isolated absolute --home: {args:?}"
            );
        }
        if args.contains(&"scan") {
            assert!(
                args.windows(2).any(|pair| pair[0] == "--provider"
                    && (pair[1] == "codex" || pair[1] == "claude-code")),
                "clean ai scan must pass --provider codex or claude-code: {args:?}"
            );
        }
    }

    fn assert_avoids_global_scope(args: &[&str]) {
        assert!(
            !args
                .windows(2)
                .any(|pair| pair == ["--scope", "global"] || pair == ["--scope", "all"]),
            "command requested global or all scope: {args:?}"
        );
        if args.contains(&"scan") {
            assert!(
                args.windows(2).any(|pair| pair == ["--scope", "projects"]),
                "clean scan must use --scope projects: {args:?}"
            );
        }
    }

    fn assert_observation_is_project_only(document: &serde_json::Value) {
        let targets = document["data"]["plan"]["targets"]
            .as_array()
            .expect("scan targets");
        assert!(
            targets
                .iter()
                .all(|target| target["scope"]["type"] == "project"),
            "scan included a global cache target"
        );
        if let Some(coverage) = document["data"]["health"]["coverage"].as_array() {
            assert!(
                coverage.iter().all(|unit| unit["scope"] == "project"),
                "scan included a global provider coverage unit"
            );
        }
    }

    fn envelope(output: &Output) -> serde_json::Value {
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "JSON envelope ({error}): stdout={} stderr={}",
                String::from_utf8_lossy(&output.stdout),
                stderr(output)
            )
        })
    }

    fn stderr(output: &Output) -> String {
        String::from_utf8_lossy(&output.stderr).into_owned()
    }
}
