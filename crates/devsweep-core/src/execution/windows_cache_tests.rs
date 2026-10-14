// Included inside execution::tests to reuse recording runners, never system trash.
#[cfg(windows)]
mod windows_cache_pipeline {
    use super::*;
    use crate::{
        model::{CleanupIntent, ScanReport},
        plan::validate_plan,
        rules::windows_caches::{OwnerState, with_owner_state_for_test},
        scan::{ScanOptions, Sweeper},
    };
    use std::{
        os::windows::fs::OpenOptionsExt,
        process::Command,
        time::{Duration, SystemTime},
    };

    const CHILD: &str = "DEVSWEEP_WINDOWS_CACHE_PIPELINE_FIXTURE";
    const TEST: &str = "execution::tests::windows_cache_pipeline::real_scan_saved_plan_preview_and_recorded_execution";

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn old(path: &Path) {
        fs::OpenOptions::new()
            .write(true)
            .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)
            .unwrap()
            .set_times(
                fs::FileTimes::new()
                    .set_modified(SystemTime::now() - Duration::from_secs(9 * 24 * 60 * 60)),
            )
            .unwrap();
    }

    fn make_fixture(root: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let local = root.join("Local");
        let roaming = root.join("Roaming");
        let mut approved = Vec::new();
        let mut retained = Vec::new();
        for vendor in ["Google/Chrome", "Microsoft/Edge"] {
            let data = local.join(vendor).join("User Data");
            write(
                &data.join("Local State"),
                r#"{"profile":{"info_cache":{"Default":{},"Profile 1":{}}}}"#,
            );
            for profile in ["Default", "Profile 1"] {
                let profile = data.join(profile);
                write(&profile.join("Preferences"), "{}");
                for cache in ["Cache", "Code Cache", "GPUCache"] {
                    let path = profile.join(cache);
                    write(&path.join("payload"), "cache fixture");
                    approved.push(path);
                }
                for name in [
                    "Cookies",
                    "Login Data",
                    "History",
                    "IndexedDB/data",
                    "Local Storage/data",
                    "Service Worker/data",
                ] {
                    let path = profile.join(name);
                    write(&path, "retained sentinel");
                    retained.push(path);
                }
            }
            let unregistered = data.join("Unregistered/Cache/payload");
            write(&unregistered, "retained sentinel");
            retained.push(unregistered);
        }
        write(
            &roaming.join("Mozilla/Firefox/profiles.ini"),
            "[Profile0]\nIsRelative=1\nPath=Profiles/a.default\n",
        );
        let history = roaming.join("Mozilla/Firefox/Profiles/a.default/places.sqlite");
        write(&history, "retained sentinel");
        retained.push(history);
        let firefox = local.join("Mozilla/Firefox/Profiles/a.default/cache2");
        write(&firefox.join("entries/payload"), "cache fixture");
        approved.push(firefox);
        for (name, install, exe, application) in [
            ("Code", "Microsoft VS Code", "Code.exe", "code"),
            ("Cursor", "cursor", "Cursor.exe", "cursor"),
        ] {
            let data = roaming.join(name);
            for cache in ["Cache", "Code Cache", "GPUCache", "CachedData"] {
                let path = data.join(cache);
                write(&path.join("payload"), "cache fixture");
                approved.push(path);
            }
            for name in [
                "User/globalStorage/state.vscdb",
                "User/workspaceStorage/agent-session",
                "Backups/document",
                "extensions/manifest",
            ] {
                let path = data.join(name);
                write(&path, "retained sentinel");
                retained.push(path);
            }
            let installation = local.join("Programs").join(install);
            write(
                &installation.join(exe),
                "installation marker; never executed",
            );
            write(
                &installation.join("resources/app/product.json"),
                &format!("{{\"applicationName\":\"{application}\"}}"),
            );
        }
        let temp = local.join("Temp");
        let stale = temp.join("stale.tmp");
        write(&stale, "cache fixture");
        old(&stale);
        approved.push(stale);
        for name in ["recent.tmp", "old-parent/recent-child.tmp"] {
            let path = temp.join(name);
            write(&path, "retained sentinel");
            retained.push(path);
        }
        old(&temp.join("old-parent"));
        let secret = temp.join("credentials/.NPMRC");
        write(&secret, "retained sentinel");
        old(&secret);
        old(&temp.join("credentials"));
        retained.push(secret);
        (approved, retained)
    }

    fn scan() -> ScanReport {
        let project =
            PathBuf::from(std::env::var_os("DEVSWEEP_WINDOWS_CACHE_FIXTURE_ROOT").unwrap())
                .join("project");
        let report = Sweeper::default()
            .full_scan_report(
                &ScanOptions {
                    include_projects: project.is_dir(),
                    include_global: true,
                    roots: project.is_dir().then_some(project).into_iter().collect(),
                },
                &mut |_| {},
            )
            .expect("real production global scan");
        serde_json::from_slice(&serde_json::to_vec(&report).unwrap())
            .expect("saved report roundtrip")
    }

    fn request(
        selected: Vec<TargetId>,
        execute: bool,
        digest: Option<ConfirmationDigest>,
        root: &Path,
    ) -> ExecutionRequest {
        ExecutionRequest {
            selected,
            execute,
            expected_digest: digest,
            audit_log: Some(root.join("audit.jsonl")),
            cancel: None,
        }
    }

    #[test]
    fn real_scan_saved_plan_preview_and_recorded_execution() {
        let Some(scenario) = std::env::var_os(CHILD) else {
            // Environment belongs only to each child. Parallel tests never see it.
            for scenario in [
                "positive",
                "busy",
                "unknown",
                "owner-drift",
                "profile-drift",
                "secret-drift",
                "tampered",
                "unsafe-root",
            ] {
                let fixture = TempDir::new().unwrap();
                let root = fixture.path();
                fs::create_dir_all(root.join("Local/Temp")).unwrap();
                fs::create_dir_all(root.join("home")).unwrap();
                let mut child = Command::new(std::env::current_exe().unwrap());
                child
                    .args(["--exact", TEST, "--nocapture"])
                    .env_clear()
                    .env(CHILD, scenario)
                    .env("DEVSWEEP_WINDOWS_CACHE_FIXTURE_ROOT", root)
                    .env("USERPROFILE", root.join("home"))
                    .env("HOME", root.join("home"))
                    .env("LOCALAPPDATA", root.join("Local"))
                    .env("APPDATA", root.join("Roaming"))
                    .env("TEMP", root.join("Local/Temp"))
                    .env("TMP", root.join("Local/Temp"))
                    .env("PATH", root.join("no-tools"))
                    .current_dir(root);
                for key in ["SystemRoot", "WINDIR", "SystemDrive"] {
                    if let Some(value) = std::env::var_os(key) {
                        child.env(key, value);
                    }
                }
                let output = child.output().expect("isolated native test child");
                assert!(
                    output.status.success(),
                    "{scenario}: {}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            return;
        };
        let scenario = scenario.to_str().unwrap();
        let root = PathBuf::from(std::env::var_os("DEVSWEEP_WINDOWS_CACHE_FIXTURE_ROOT").unwrap());
        let (mut approved, retained) = make_fixture(&root);
        if scenario == "unsafe-root" {
            let malformed = root.join("Local/Google/Chrome/User Data/Default/Cache");
            fs::remove_dir_all(&malformed).unwrap(); // Disposable fixture only.
            fs::write(&malformed, "retained malformed cache").unwrap();
            approved.retain(|path| path != &malformed);
            write(&root.join("project/tox.ini"), "[tox]");
            let healthy_project = root.join("project/.pytest_cache");
            write(&healthy_project.join("payload"), "cache fixture");
            approved.push(healthy_project);
        }
        let owner = match scenario {
            "busy" => OwnerState::Busy,
            "unknown" => OwnerState::Unknown,
            _ => OwnerState::Idle,
        };
        with_owner_state_for_test(owner, || {
            let report = scan();
            let all = validate_plan(&report.plan)
                .expect("every observed row survives real registry validation");
            assert_eq!(all.targets().len(), report.plan.targets.len());
            assert!(
                scenario == "unsafe-root"
                    || report
                        .plan
                        .targets
                        .iter()
                        .all(|target| !target.selected_by_default)
            );
            if scenario == "unsafe-root" {
                let malformed = root.join("Local/Google/Chrome/User Data/Default/Cache");
                assert!(
                    !report
                        .plan
                        .targets
                        .iter()
                        .any(|target| target.path.as_ref() == Some(&malformed))
                );
                let coverage = report
                    .health
                    .coverage
                    .iter()
                    .find(|unit| unit.id == "windows.chrome.cache")
                    .unwrap();
                assert_eq!(
                    coverage.status,
                    crate::model::ScanCoverageStatus::OwnershipUnknown
                );
                assert!(coverage.detail.contains("not a directory"));
                assert!(
                    report
                        .health
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.path == malformed
                            && diagnostic.detail.contains("not a directory"))
                );
                assert!(report.plan.targets.iter().any(|target| target.path.as_ref()
                    == Some(&root.join("Local/Google/Chrome/User Data/Profile 1/Cache"))));
                assert_eq!(
                    fs::read_to_string(&malformed).unwrap(),
                    "retained malformed cache"
                );
            }
            if matches!(scenario, "busy" | "unknown") {
                let app_targets = report
                    .plan
                    .targets
                    .iter()
                    .filter(|target| {
                        target.rule_id.starts_with("windows.")
                            && target.rule_id != "windows.user_temp.stale"
                    })
                    .collect::<Vec<_>>();
                assert_eq!(app_targets.len(), 21);
                assert!(
                    app_targets
                        .iter()
                        .all(|target| matches!(target.intent, CleanupIntent::InspectOnly { .. }))
                );
                let expected = if scenario == "busy" {
                    crate::model::ScanCoverageStatus::Busy
                } else {
                    crate::model::ScanCoverageStatus::OwnershipUnknown
                };
                assert!(
                    report
                        .health
                        .coverage
                        .iter()
                        .filter(|unit| unit.id.starts_with("windows.")
                            && unit.id != "windows.user_temp.stale")
                        .all(|unit| unit.status == expected)
                );
                let reviewed_id = &app_targets[0].id;
                let reviewed = Sweeper::default()
                    .full_scan_report_rescanning_target(
                        &ScanOptions {
                            include_projects: false,
                            include_global: true,
                            roots: vec![],
                        },
                        reviewed_id,
                        &mut |_| {},
                    )
                    .expect("reviewed Windows cache size uses the real report pipeline");
                validate_plan(&reviewed.plan).expect("reviewed inspect rows remain representable");
                let target = reviewed
                    .plan
                    .targets
                    .iter()
                    .find(|target| &target.id == reviewed_id)
                    .unwrap();
                assert!(matches!(target.intent, CleanupIntent::InspectOnly { .. }));
                assert!(!target.selected_by_default);
                assert!(target.size_complete);
                let reason = if scenario == "busy" {
                    "application is running"
                } else {
                    "activity could not be verified"
                };
                assert!(
                    target
                        .sizing_warnings
                        .iter()
                        .any(|warning| warning.detail.contains(reason)),
                    "reviewed sizing must preserve the current eligibility refusal: {:?}",
                    target.sizing_warnings
                );
            } else {
                let mut plan = report.plan.clone();
                plan.targets
                    .retain(|target| !matches!(target.intent, CleanupIntent::InspectOnly { .. }));
                assert_eq!(
                    plan.targets.len(),
                    approved.len(),
                    "all six families are executable in the positive fixture"
                );
                let plan = serde_json::from_slice(&serde_json::to_vec(&plan).unwrap())
                    .expect("saved v2 plan");
                let validated = validate_plan(&plan).expect("production trust boundary");
                let selected = validated
                    .targets()
                    .iter()
                    .map(|target| target.target().id.clone())
                    .collect::<Vec<_>>();
                let command = RecordingCommandRunner::default();
                let trash = RecordingTrashRunner::default();
                let executor = test_executor(command.clone(), trash.clone());
                let preview = executor
                    .run_plan(&validated, request(selected.clone(), false, None, &root))
                    .expect("live preview");
                assert!(trash.paths().is_empty());
                if scenario == "tampered" {
                    let mut altered: crate::model::UntrustedPlan = plan;
                    altered.targets[0].path = Some(root.join("Roaming/Code/User"));
                    assert!(
                        validate_plan(&altered).is_err(),
                        "profile/state root cannot inherit cache authority"
                    );
                } else {
                    if scenario == "profile-drift" {
                        write(
                            &root.join("Local/Google/Chrome/User Data/Local State"),
                            "{}",
                        );
                    } else if scenario == "secret-drift" {
                        write(
                            &root.join("Local/Google/Chrome/User Data/Default/Cache/.NETRC"),
                            "retained sentinel",
                        );
                    }
                    let run = || {
                        executor.run_plan(
                            &validated,
                            request(
                                selected,
                                true,
                                Some(preview.confirmation_digest.clone()),
                                &root,
                            ),
                        )
                    };
                    let outcome = if scenario == "owner-drift" {
                        with_owner_state_for_test(OwnerState::Busy, run)
                    } else {
                        run()
                    };
                    if matches!(scenario, "positive" | "unsafe-root") {
                        let outcome = outcome.expect("approved fake dispatch");
                        assert_eq!(outcome.succeeded, approved.len());
                        let mut actual = trash
                            .paths()
                            .iter()
                            .map(|path| crate::filesystem::normalize_absolute_path(path).unwrap())
                            .collect::<Vec<_>>();
                        let mut expected = approved
                            .iter()
                            .map(|path| crate::filesystem::normalize_absolute_path(path).unwrap())
                            .collect::<Vec<_>>();
                        actual.sort();
                        expected.sort();
                        assert_eq!(actual, expected);
                    } else {
                        assert!(outcome.is_err(), "live drift refuses before any action");
                        assert!(trash.paths().is_empty());
                    }
                }
                assert!(command.requests().is_empty());
            }
        });
        for path in retained {
            assert_eq!(fs::read_to_string(path).unwrap(), "retained sentinel");
        }
        for path in approved {
            assert!(path.exists(), "recording trash never moves fixture data");
        }
    }
}
