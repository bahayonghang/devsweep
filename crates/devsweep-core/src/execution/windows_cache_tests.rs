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
        collections::BTreeMap,
        os::windows::fs::{MetadataExt, OpenOptionsExt},
        process::Command,
        time::{Duration, SystemTime},
    };
    use sha2::{Digest, Sha256};

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

    fn sha256_hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn is_reparse(metadata: &fs::Metadata) -> bool {
        metadata.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
            != 0
    }

    fn file_hashes(roots: &[PathBuf]) -> BTreeMap<PathBuf, String> {
        let mut hashes = BTreeMap::new();
        let mut pending = roots.to_vec();
        while let Some(path) = pending.pop() {
            let metadata = fs::symlink_metadata(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            if is_reparse(&metadata) {
                hashes.insert(path, "reparse".to_string());
                continue;
            }
            if metadata.is_dir() {
                for entry in fs::read_dir(&path).unwrap() {
                    pending.push(entry.unwrap().path());
                }
                continue;
            }
            if metadata.is_file() {
                hashes.insert(path.clone(), sha256_hex(&fs::read(&path).unwrap()));
            }
        }
        hashes
    }

    fn assert_same_hashes(before: &BTreeMap<PathBuf, String>, roots: &[PathBuf]) {
        let after = file_hashes(roots);
        for (path, hash) in before {
            assert_eq!(
                after.get(path),
                Some(hash),
                "fixture bytes changed: {}",
                path.display()
            );
        }
    }

    fn create_junction(link: &Path, target: &Path) -> std::io::Result<()> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::{
            Foundation::{CloseHandle, GENERIC_WRITE, INVALID_HANDLE_VALUE},
            Storage::FileSystem::{
                CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
                FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
            },
            System::{IO::DeviceIoControl, Ioctl::FSCTL_SET_REPARSE_POINT},
        };
        const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
        fs::create_dir(link)?;
        let substitute = format!(r"\??\{}", target.display());
        let print_name = target.display().to_string();
        let mut substitute: Vec<u16> = std::ffi::OsStr::new(&substitute).encode_wide().collect();
        let mut print: Vec<u16> = std::ffi::OsStr::new(&print_name).encode_wide().collect();
        substitute.push(0);
        print.push(0);
        let substitute_bytes = u16::try_from((substitute.len() - 1) * 2).unwrap();
        let print_bytes = u16::try_from((print.len() - 1) * 2).unwrap();
        let path_units = substitute.len() + print.len();
        let reparse_data_length = u16::try_from(8 + path_units * 2).unwrap();
        let mut buffer = Vec::with_capacity(8 + usize::from(reparse_data_length));
        buffer.extend_from_slice(&IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes());
        buffer.extend_from_slice(&reparse_data_length.to_le_bytes());
        buffer.extend_from_slice(&0u16.to_le_bytes());
        buffer.extend_from_slice(&0u16.to_le_bytes());
        buffer.extend_from_slice(&substitute_bytes.to_le_bytes());
        buffer.extend_from_slice(&u16::try_from(substitute.len() * 2).unwrap().to_le_bytes());
        buffer.extend_from_slice(&print_bytes.to_le_bytes());
        for unit in substitute.into_iter().chain(print) {
            buffer.extend_from_slice(&unit.to_le_bytes());
        }
        let wide: Vec<u16> = link
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: wide is null-terminated for this call. The directory was created above.
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error());
        }
        let mut returned = 0u32;
        // SAFETY: handle is open, buffer lives for the call, and no output buffer is requested.
        let ok = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_SET_REPARSE_POINT,
                buffer.as_ptr().cast(),
                u32::try_from(buffer.len()).unwrap(),
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        let error = if ok == 0 {
            Some(std::io::Error::last_os_error())
        } else {
            None
        };
        // SAFETY: handle was returned by CreateFileW above and is closed once.
        unsafe {
            CloseHandle(handle);
        }
        if let Some(error) = error {
            return Err(error);
        }
        Ok(())
    }

    fn point_junction(link: &Path, target: &Path) {
        if link.symlink_metadata().is_ok() {
            fs::remove_dir_all(link).unwrap();
        }
        fs::create_dir_all(target).unwrap();
        let secret = target.join("secret");
        if !secret.exists() {
            write(&secret, "personal reparse sentinel");
        }
        create_junction(link, target).unwrap_or_else(|error| {
            panic!("junction {} -> {}: {error}", link.display(), target.display())
        });
        assert!(
            is_reparse(&fs::symlink_metadata(link).unwrap()),
            "junction was not a reparse point: {}",
            link.display()
        );
    }

    fn global_scan() -> ScanReport {
        let report = Sweeper::default()
            .full_scan_report(
                &ScanOptions {
                    include_projects: false,
                    include_global: true,
                    roots: Vec::new(),
                },
                &mut |_| {},
            )
            .expect("real production global scan");
        serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).expect("saved report roundtrip")
    }

    fn trash_targets<'a>(
        report: &'a ScanReport,
        rule_id: &str,
    ) -> Vec<&'a crate::model::UntrustedTarget> {
        report
            .plan
            .targets
            .iter()
            .filter(|target| {
                target.rule_id == rule_id
                    && matches!(
                        target.intent,
                        crate::model::CleanupIntent::TrashProjectArtifact { .. }
                    )
            })
            .collect()
    }

    fn refuse_preview(report: &ScanReport, selected: Vec<TargetId>, root: &Path) {
        let validated = validate_plan(&report.plan).expect("production trust boundary");
        let command = RecordingCommandRunner::default();
        let trash = RecordingTrashRunner::default();
        let executor = test_executor(command.clone(), trash.clone());
        let preview = executor
            .run_plan(
                &validated,
                request(selected.clone(), false, None, root),
            )
            .expect("live preview");
        assert!(trash.paths().is_empty());
        let stale = executor
            .run_plan(
                &validated,
                request(
                    selected,
                    true,
                    Some(ConfirmationDigest::new("stale-preview-digest")),
                    root,
                ),
            )
            .expect_err("stale preview digest refuses before action");
        assert!(
            stale.to_string().contains("stale"),
            "refused preview error: {stale}"
        );
        assert!(trash.paths().is_empty());
        assert!(command.requests().is_empty());
        let _ = preview;
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
            let tracked = approved
                .iter()
                .chain(retained.iter())
                .cloned()
                .collect::<Vec<_>>();
            let before = file_hashes(&tracked);
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
                assert!(
                    !report.health.is_complete(),
                    "unreadable cache must not become an empty complete scan"
                );
                assert!(
                    !report.plan.targets.is_empty(),
                    "partial provider failure must keep the healthy sibling target"
                );
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
                        .filter(|unit| {
                            matches!(
                                unit.id.as_str(),
                                "windows.chrome.cache"
                                    | "windows.edge.cache"
                                    | "windows.firefox.cache"
                                    | "windows.vscode.cache"
                                    | "windows.cursor.cache"
                            )
                        })
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
            assert_same_hashes(&before, &tracked);
        });
        for path in retained {
            assert_eq!(fs::read_to_string(path).unwrap(), "retained sentinel");
        }
        for path in approved {
            assert!(path.exists(), "recording trash never moves fixture data");
        }
    }

    const CASES: &str = "execution::tests::windows_cache_pipeline::catalogue_omissions_links_and_distinct_totals";
    const CASE_ENV: &str = "DEVSWEEP_WINDOWS_CACHE_CASE";

    fn spawn_case(scenario: &str) {
        spawn_isolated(CASES, CASE_ENV, scenario);
    }

    fn spawn_isolated(test: &str, env_key: &str, scenario: &str) {
        let fixture = TempDir::new().unwrap();
        let root = fixture.path();
        fs::create_dir_all(root.join("Local/Temp")).unwrap();
        fs::create_dir_all(root.join("home")).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args(["--exact", test, "--nocapture"])
            .env_clear()
            .env(env_key, scenario)
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

    fn plant_omission_trees(root: &Path) -> Vec<PathBuf> {
        let files = [
            root.join("Local/D3DSCache/shader.bin"),
            root.join("Local/Microsoft/Windows/WER/ReportArchive/report.wer"),
            root.join("Windows/SoftwareDistribution/Download/update.bin"),
            root.join("Local/Microsoft/Windows/DeliveryOptimization/file.bin"),
            root.join("home/Downloads/tax.pdf"),
            root.join("home/.claude/projects/session.json"),
            root.join("home/.codex/sessions/session.json"),
            root.join("Roaming/Cursor/User/workspaceStorage/agent-session"),
        ];
        for path in &files {
            write(path, "omission sentinel");
        }
        files.to_vec()
    }

    fn install_cursor_without_caches(root: &Path) {
        let install = root.join("Local/Programs/cursor");
        write(
            &install.join("Cursor.exe"),
            "installation marker; never executed",
        );
        write(
            &install.join("resources/app/product.json"),
            r#"{"applicationName":"cursor"}"#,
        );
        fs::create_dir_all(root.join("Roaming/Cursor/User")).unwrap();
    }

    fn omissions_case(root: &Path) {
        let planted = plant_omission_trees(root);
        install_cursor_without_caches(root);
        let keep = root.join("Local/Temp/keep.tmp");
        write(&keep, "temp sentinel");
        old(&keep);
        let tracked = planted
            .iter()
            .cloned()
            .chain(std::iter::once(keep.clone()))
            .collect::<Vec<_>>();
        let before = file_hashes(&tracked);
        let report = global_scan();
        assert_same_hashes(&before, &tracked);
        assert!(
            report.health.is_complete(),
            "omission rows must not mark scan health partial: coverage={:?} diagnostics={:?}",
            report
                .health
                .coverage
                .iter()
                .map(|unit| (unit.id.clone(), unit.status, unit.detail.clone()))
                .collect::<Vec<_>>(),
            report.health.diagnostics
        );
        let catalogue = crate::rules::rule_catalogue();
        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.contains("delete_permanently"));
        for (id, detail) in crate::rules::WINDOWS_CACHE_COVERAGE_OMISSIONS {
            let rows: Vec<_> = report
                .health
                .coverage
                .iter()
                .filter(|unit| unit.id == *id)
                .collect();
            assert_eq!(rows.len(), 1, "{id}");
            assert_eq!(rows[0].scope, crate::model::ScanCoverageScope::Provider);
            assert!(rows[0].path.is_none(), "{id}");
            assert_eq!(rows[0].status, crate::model::ScanCoverageStatus::Unsupported);
            assert_eq!(rows[0].target_count, 0);
            assert_eq!(rows[0].detail, *detail);
            assert!(json.contains(detail), "{id}");
            assert!(catalogue.iter().all(|doc| doc.id != *id));
            assert!(crate::rules::windows_cache_family(id).is_none());
            assert!(report.plan.targets.iter().all(|target| target.rule_id != *id));
        }
        for path in &planted {
            assert!(
                report
                    .plan
                    .targets
                    .iter()
                    .all(|target| target.path.as_ref() != Some(path))
            );
            assert!(
                report
                    .health
                    .coverage
                    .iter()
                    .all(|unit| unit.path.as_ref() != Some(path))
            );
        }
        assert!(report.plan.targets.iter().all(|target| {
            target.rule_id != "docker" && target.rule_id != "cargo.home.inspect"
        }));
        let keep_id = {
            let temp = trash_targets(&report, "windows.user_temp.stale");
            assert_eq!(temp.len(), 1);
            temp[0].id.clone()
        };
        refuse_preview(&report, vec![keep_id], root);
        assert_same_hashes(&before, &tracked);

        let project = root.join("project");
        write(&project.join("tox.ini"), "[tox]\n");
        write(&project.join(".pytest_cache/payload"), "cache fixture");
        write(&project.join("target/fresh.bin"), "newer than sources");
        write(&project.join("dist/app.js"), "newer than sources");
        let project_report = Sweeper::default()
            .full_scan_report(
                &ScanOptions {
                    include_projects: true,
                    include_global: false,
                    roots: vec![project],
                },
                &mut |_| {},
            )
            .expect("project-only scan");
        let project_report: ScanReport =
            serde_json::from_slice(&serde_json::to_vec(&project_report).unwrap()).unwrap();
        for (id, _) in crate::rules::WINDOWS_CACHE_COVERAGE_OMISSIONS {
            assert!(
                project_report
                    .health
                    .coverage
                    .iter()
                    .all(|unit| unit.id != *id),
                "{id} leaked into a project-only scan"
            );
        }
        assert!(
            project_report
                .plan
                .targets
                .iter()
                .any(|target| target.rule_id == "python.pytest_cache")
        );
        assert!(project_report.plan.targets.iter().all(|target| {
            target.path.as_ref().is_none_or(|path| {
                !path.ends_with("target")
                    && !path.ends_with("dist")
                    && !path.ends_with("fresh.bin")
                    && !path.ends_with("app.js")
            })
        }));
        assert_same_hashes(&before, &tracked);
    }

    fn reparse_case(root: &Path) {
        let _fixture = make_fixture(root);
        let outside = root.join("outside-personal");
        let links = [
            root.join("Local/Google/Chrome/User Data/Default/Cache"),
            root.join("Local/Microsoft/Edge/User Data/Default/Cache"),
            root.join("Local/Mozilla/Firefox/Profiles/a.default/cache2"),
            root.join("Roaming/Code/Cache"),
            root.join("Roaming/Cursor/Cache"),
            root.join("Local/Temp/linked-dir"),
        ];
        for link in &links {
            point_junction(link, &outside);
        }
        write(
            &root.join("Roaming/Mozilla/Firefox/profiles.ini"),
            "[Profile0]\nIsRelative=1\nPath=Profiles/a.default\n[Profile1]\nIsRelative=1\nPath=Profiles/b.default\n",
        );
        fs::create_dir_all(root.join("Roaming/Mozilla/Firefox/Profiles/b.default")).unwrap();
        write(
            &root
                .join("Local/Mozilla/Firefox/Profiles/b.default/cache2/entries/payload"),
            "cache fixture",
        );
        let secret = outside.join("secret");
        let sibling = root.join("Local/Google/Chrome/User Data/Profile 1/Cache/payload");
        let tracked = vec![secret.clone(), sibling, root.join("Local/Temp/stale.tmp")];
        let before = file_hashes(&tracked);
        let report = global_scan();
        assert_same_hashes(&before, &tracked);
        for link in &links {
            assert!(
                report
                    .plan
                    .targets
                    .iter()
                    .all(|target| target.path.as_ref() != Some(link)),
                "reparse path became a target: {}",
                link.display()
            );
            assert!(
                report.health.diagnostics.iter().any(|diagnostic| {
                    &diagnostic.path == link && diagnostic.detail.contains("reparse")
                }),
                "missing reparse diagnostic for {}: {:?}",
                link.display(),
                report.health.diagnostics
            );
        }
        for rule in [
            "windows.chrome.cache",
            "windows.edge.cache",
            "windows.firefox.cache",
            "windows.vscode.cache",
            "windows.cursor.cache",
            "windows.user_temp.stale",
        ] {
            assert!(
                !trash_targets(&report, rule).is_empty(),
                "{rule} lost its healthy sibling"
            );
        }
        assert!(
            report
                .plan
                .targets
                .iter()
                .all(|target| target.path.as_ref() != Some(&secret))
        );
        assert!(!report.health.is_complete());
        assert!(!report.plan.targets.is_empty());
        let sibling_id = report
            .plan
            .targets
            .iter()
            .find(|target| {
                target.path.as_ref()
                    == Some(&root.join("Local/Google/Chrome/User Data/Profile 1/Cache"))
            })
            .unwrap()
            .id
            .clone();
        refuse_preview(&report, vec![sibling_id], root);
        assert_same_hashes(&before, &tracked);
        assert_eq!(
            fs::read_to_string(&secret).unwrap(),
            "personal reparse sentinel"
        );
    }

    fn hardlink_case(root: &Path) {
        let _fixture = make_fixture(root);
        let personal = root.join("home/personal.bin");
        write(&personal, "personal hardlink sentinel");
        let links = [
            root.join("Local/Google/Chrome/User Data/Default/Cache/payload"),
            root.join("Local/Microsoft/Edge/User Data/Default/Cache/payload"),
            root.join("Local/Mozilla/Firefox/Profiles/a.default/cache2/entries/payload"),
            root.join("Roaming/Code/Cache/payload"),
            root.join("Roaming/Cursor/Cache/payload"),
            root.join("Local/Temp/stale.tmp"),
        ];
        for link in &links {
            fs::remove_file(link).unwrap();
            fs::hard_link(&personal, link).unwrap_or_else(|error| {
                panic!("hard link {} -> {}: {error}", personal.display(), link.display())
            });
        }
        old(&links[5]);
        let tracked = [personal.clone()];
        let before = file_hashes(&tracked);
        let report = global_scan();
        assert_same_hashes(&before, &tracked);
        assert!(
            report
                .plan
                .targets
                .iter()
                .all(|target| target.path.as_ref() != Some(&personal))
        );
        for rule in [
            "windows.chrome.cache",
            "windows.edge.cache",
            "windows.firefox.cache",
            "windows.vscode.cache",
            "windows.cursor.cache",
            "windows.user_temp.stale",
        ] {
            assert!(!trash_targets(&report, rule).is_empty(), "{rule}");
        }
        let selected = {
            let targets = trash_targets(&report, "windows.chrome.cache");
            targets[0].id.clone()
        };
        refuse_preview(&report, vec![selected], root);
        assert_same_hashes(&before, &tracked);
        assert_eq!(
            fs::read_to_string(&personal).unwrap(),
            "personal hardlink sentinel"
        );
    }

    fn temp_busy_case(root: &Path) {
        let _fixture = make_fixture(root);
        let locked = root.join("Local/Temp/locked.bin");
        write(&locked, "locked sentinel");
        old(&locked);
        let sibling = root.join("Local/Temp/stale.tmp");
        let guard = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&locked)
            .unwrap();
        let tracked = [sibling.clone()];
        let before = file_hashes(&tracked);
        let report = global_scan();
        drop(guard);
        assert_same_hashes(&before, &tracked);
        assert_eq!(fs::read_to_string(&locked).unwrap(), "locked sentinel");
        let locked_target = report
            .plan
            .targets
            .iter()
            .find(|target| target.path.as_ref() == Some(&locked))
            .expect("locked temp entry stays visible");
        assert!(matches!(
            locked_target.intent,
            CleanupIntent::InspectOnly { .. }
        ));
        assert!(
            locked_target
                .sizing_warnings
                .iter()
                .any(|warning| warning.detail.contains("locked"))
        );
        let sibling_id = {
            let targets = trash_targets(&report, "windows.user_temp.stale");
            targets
                .iter()
                .find(|target| target.path.as_ref() == Some(&sibling))
                .expect("unlocked stale temp stays selectable")
                .id
                .clone()
        };
        refuse_preview(&report, vec![sibling_id], root);
        assert_same_hashes(&before, &tracked);
        assert_eq!(fs::read_to_string(&locked).unwrap(), "locked sentinel");
    }

    fn cache_dir(target: &crate::model::UntrustedTarget, profile: &str) -> bool {
        target.rule_id == "windows.chrome.cache"
            && target.path.as_ref().is_some_and(|path| {
                path.file_name().is_some_and(|name| name == "Cache")
                    && path.components().any(|component| component.as_os_str() == profile)
            })
    }

    fn distinct_case(root: &Path) {
        let data = root.join("Local/Google/Chrome/User Data");
        write(
            &data.join("Local State"),
            r#"{"profile":{"info_cache":{"Default":{},"Profile 1":{}}}}"#,
        );
        for profile in ["Default", "Profile 1"] {
            write(&data.join(profile).join("Preferences"), "{}");
        }
        let partial_path = data.join("Default/Cache/payload");
        fs::create_dir_all(partial_path.parent().unwrap()).unwrap();
        fs::write(&partial_path, vec![b'p'; 100]).unwrap();
        let mut deep = data.join("Default/Cache");
        for _ in 0..64 {
            deep.push("n");
        }
        fs::create_dir_all(&deep).unwrap();
        let sibling_path = data.join("Profile 1/Cache/payload");
        fs::create_dir_all(sibling_path.parent().unwrap()).unwrap();
        fs::write(&sibling_path, vec![b'v'; 40]).unwrap();
        let moved_path = root.join("Local/Temp/moved.bin");
        fs::create_dir_all(moved_path.parent().unwrap()).unwrap();
        fs::write(&moved_path, b"moved!!").unwrap();
        old(&moved_path);
        let cookies = data.join("Default/Cookies");
        write(&cookies, "retained sentinel");
        let tracked = vec![
            partial_path,
            sibling_path,
            moved_path.clone(),
            cookies.clone(),
        ];
        let before = file_hashes(&tracked);
        let report = global_scan();
        assert_same_hashes(&before, &tracked);
        let (estimated, moved_estimate, moved_id, verified) = {
            let partial = report
                .plan
                .targets
                .iter()
                .find(|target| cache_dir(target, "Default"))
                .expect("partial chrome cache");
            let sibling = report
                .plan
                .targets
                .iter()
                .find(|target| cache_dir(target, "Profile 1"))
                .expect("complete chrome cache");
            let moved = report
                .plan
                .targets
                .iter()
                .find(|target| target.path.as_ref() == Some(&moved_path))
                .expect("stale temp file");
            assert_eq!(partial.estimated_bytes, 100, "{partial:?}");
            assert!(!partial.size_complete, "{partial:?}");
            assert_eq!(sibling.estimated_bytes, 40, "{sibling:?}");
            assert!(sibling.size_complete, "{sibling:?}");
            assert_eq!(moved.estimated_bytes, 7, "{moved:?}");
            assert!(moved.size_complete);
            assert!(
                report
                    .plan
                    .targets
                    .iter()
                    .all(|target| target.path.as_ref() != Some(&cookies))
            );
            let estimated = partial.estimated_bytes;
            let sibling_bytes = sibling.estimated_bytes;
            let moved_estimate = moved.estimated_bytes;
            let moved_id = moved.id.clone();
            let verified = report.health.totals.verified_bytes;
            assert_eq!(verified, sibling_bytes + moved_estimate);
            assert_eq!(report.health.totals.partial_lower_bound_bytes, estimated);
            (estimated, moved_estimate, moved_id, verified)
        };
        assert_ne!(estimated, verified);
        refuse_preview(&report, vec![moved_id.clone()], root);
        assert_same_hashes(&before, &tracked);
        assert_eq!(
            crate::history::clean_moved_totals()
                .expect("history reader")
                .known_bytes,
            0
        );

        let validated = validate_plan(&report.plan).expect("production trust boundary");
        let command = RecordingCommandRunner::default();
        let trash = RecordingTrashRunner::default();
        let executor = test_executor(command.clone(), trash.clone());
        let audit = clean_audit_v1_path().expect("fixture history store");
        let preview = executor
            .run_plan(
                &validated,
                ExecutionRequest {
                    selected: vec![moved_id.clone()],
                    execute: false,
                    expected_digest: None,
                    audit_log: Some(audit.clone()),
                    cancel: None,
                },
            )
            .expect("live preview");
        let outcome = executor
            .run_plan(
                &validated,
                ExecutionRequest {
                    selected: vec![moved_id],
                    execute: true,
                    expected_digest: Some(preview.confirmation_digest),
                    audit_log: Some(audit),
                    cancel: None,
                },
            )
            .expect("recorded trash");
        assert_eq!(outcome.succeeded, 1);
        assert!(trash.paths().len() == 1);
        assert!(command.requests().is_empty());
        let history = crate::history::clean_moved_totals().expect("history reader");
        assert_ne!(verified, history.known_bytes);
        assert_ne!(estimated, history.known_bytes);
        assert_eq!(history.known_bytes, moved_estimate);
        let history_json = serde_json::to_string(&history).unwrap();
        let totals_json = serde_json::to_string(&report.health.totals).unwrap();
        assert!(history_json.contains("known_bytes"));
        assert!(!history_json.contains("verified_bytes"));
        assert!(totals_json.contains("verified_bytes"));
        assert!(!totals_json.contains("known_bytes"));
        assert_same_hashes(&before, &tracked);
        assert_eq!(fs::read(&moved_path).unwrap(), b"moved!!");
    }

    #[test]
    fn catalogue_omissions_links_and_distinct_totals() {
        let Some(scenario) = std::env::var_os(CASE_ENV) else {
            for scenario in ["omissions", "reparse", "hardlink", "temp-busy", "distinct"] {
                spawn_case(scenario);
            }
            return;
        };
        let scenario = scenario.to_str().unwrap();
        let root = PathBuf::from(std::env::var_os("DEVSWEEP_WINDOWS_CACHE_FIXTURE_ROOT").unwrap());
        with_owner_state_for_test(OwnerState::Idle, || match scenario {
            "omissions" => omissions_case(&root),
            "reparse" => reparse_case(&root),
            "hardlink" => hardlink_case(&root),
            "temp-busy" => temp_busy_case(&root),
            "distinct" => distinct_case(&root),
            other => panic!("unknown catalogue case {other}"),
        });
    }

    const FREED: &str =
        "execution::tests::windows_cache_pipeline::verified_freed_bytes_after_isolated_rename";
    const FREED_ENV: &str = "DEVSWEEP_WINDOWS_CACHE_FREED";

    #[derive(Clone, Default)]
    struct IsolatedRenameTrashRunner {
        destinations: Rc<RefCell<Vec<PathBuf>>>,
    }

    impl IsolatedRenameTrashRunner {
        fn destinations(&self) -> Vec<PathBuf> {
            self.destinations.borrow().clone()
        }
    }

    impl TrashRunner for IsolatedRenameTrashRunner {
        fn move_to_trash(&self, path: &Path) -> Result<()> {
            let parent = path
                .parent()
                .ok_or_else(|| anyhow!("validated path has no parent"))?;
            let dest_dir = parent.join("isolated-trash");
            fs::create_dir_all(&dest_dir)?;
            let name = path
                .file_name()
                .ok_or_else(|| anyhow!("validated path has no file name"))?;
            let dest = dest_dir.join(name);
            fs::rename(path, &dest)?;
            self.destinations.borrow_mut().push(dest);
            Ok(())
        }
    }

    #[test]
    fn verified_freed_bytes_after_isolated_rename() {
        if std::env::var_os(FREED_ENV).is_none() {
            spawn_isolated(FREED, FREED_ENV, "freed");
            return;
        }
        let root = PathBuf::from(std::env::var_os("DEVSWEEP_WINDOWS_CACHE_FIXTURE_ROOT").unwrap());
        with_owner_state_for_test(OwnerState::Idle, || {
            let data = root.join("Local/Google/Chrome/User Data");
            write(
                &data.join("Local State"),
                r#"{"profile":{"info_cache":{"Default":{},"Profile 1":{}}}}"#,
            );
            for profile in ["Default", "Profile 1"] {
                write(&data.join(profile).join("Preferences"), "{}");
            }
            let partial_path = data.join("Default/Cache/payload");
            fs::create_dir_all(partial_path.parent().unwrap()).unwrap();
            fs::write(&partial_path, vec![b'p'; 100]).unwrap();
            let mut deep = data.join("Default/Cache");
            for _ in 0..64 {
                deep.push("n");
            }
            fs::create_dir_all(&deep).unwrap();
            let sibling_path = data.join("Profile 1/Cache/payload");
            fs::create_dir_all(sibling_path.parent().unwrap()).unwrap();
            fs::write(&sibling_path, vec![b'v'; 40]).unwrap();
            let source = root.join("Local/Temp/moved.bin");
            let payload = vec![b'f'; 17];
            fs::create_dir_all(source.parent().unwrap()).unwrap();
            fs::write(&source, &payload).unwrap();
            old(&source);
            let cookies = data.join("Default/Cookies");
            write(&cookies, "retained sentinel");

            let report = global_scan();
            let (estimated, verified, moved_estimate, moved_id) = {
                let partial = report
                    .plan
                    .targets
                    .iter()
                    .find(|target| cache_dir(target, "Default"))
                    .expect("partial chrome cache");
                let sibling = report
                    .plan
                    .targets
                    .iter()
                    .find(|target| cache_dir(target, "Profile 1"))
                    .expect("complete chrome cache");
                let moved = report
                    .plan
                    .targets
                    .iter()
                    .find(|target| target.path.as_ref() == Some(&source))
                    .expect("stale temp file");
                assert_eq!(partial.estimated_bytes, 100, "{partial:?}");
                assert!(!partial.size_complete, "{partial:?}");
                assert_eq!(sibling.estimated_bytes, 40, "{sibling:?}");
                assert!(sibling.size_complete, "{sibling:?}");
                assert_eq!(moved.estimated_bytes, 17, "{moved:?}");
                assert!(moved.size_complete);
                assert!(matches!(
                    moved.intent,
                    crate::model::CleanupIntent::TrashProjectArtifact { .. }
                ));
                let estimated = partial.estimated_bytes;
                let verified = report.health.totals.verified_bytes;
                assert_eq!(verified, sibling.estimated_bytes + moved.estimated_bytes);
                assert_eq!(report.health.totals.partial_lower_bound_bytes, estimated);
                (
                    estimated,
                    verified,
                    moved.estimated_bytes,
                    moved.id.clone(),
                )
            };
            assert_ne!(estimated, verified);
            assert_eq!(
                crate::history::clean_moved_totals()
                    .expect("history reader")
                    .known_bytes,
                0
            );

            let validated = validate_plan(&report.plan).expect("production trust boundary");
            let command = RecordingCommandRunner::default();
            let trash = IsolatedRenameTrashRunner::default();
            let executor = test_executor(command.clone(), trash.clone());
            let audit = clean_audit_v1_path().expect("fixture history store");
            let preview = executor
                .run_plan(
                    &validated,
                    ExecutionRequest {
                        selected: vec![moved_id.clone()],
                        execute: false,
                        expected_digest: None,
                        audit_log: Some(audit.clone()),
                        cancel: None,
                    },
                )
                .expect("live preview");
            assert!(trash.destinations().is_empty());
            assert!(source.exists(), "preview must not move the fixture");
            let outcome = executor
                .run_plan(
                    &validated,
                    ExecutionRequest {
                        selected: vec![moved_id],
                        execute: true,
                        expected_digest: Some(preview.confirmation_digest),
                        audit_log: Some(audit),
                        cancel: None,
                    },
                )
                .expect("isolated rename trash");
            assert_eq!(outcome.succeeded, 1);
            assert!(!outcome.dry_run);
            assert!(command.requests().is_empty());
            assert_eq!(outcome.outcomes[0].action, ActionKind::MoveToTrash);
            assert_eq!(
                outcome.outcomes[0].estimated_recoverable,
                CapacityEstimate::Verified {
                    bytes: moved_estimate
                }
            );

            assert!(!source.exists(), "validated source must be gone after rename");
            let destinations = trash.destinations();
            assert_eq!(destinations.len(), 1);
            let dest = &destinations[0];
            assert!(
                dest.starts_with(&root),
                "renamed file left the isolated root: {}",
                dest.display()
            );
            assert_eq!(
                dest.parent().and_then(|parent| parent.file_name()),
                Some(std::ffi::OsStr::new("isolated-trash"))
            );
            let freed = fs::metadata(dest).expect("renamed file").len();
            assert_eq!(freed, u64::try_from(payload.len()).unwrap());
            assert_eq!(fs::read(dest).unwrap(), payload);

            let history = crate::history::clean_moved_totals().expect("history reader");
            assert_eq!(history.known_bytes, moved_estimate);
            assert_ne!(estimated, history.known_bytes);
            assert_ne!(verified, history.known_bytes);
            assert_ne!(estimated, freed);
            assert_ne!(verified, freed);

            let report_json = serde_json::to_string(&outcome).unwrap();
            let totals_json = serde_json::to_string(&report.health.totals).unwrap();
            let history_json = serde_json::to_string(&history).unwrap();
            assert!(report_json.contains("estimated_recoverable"));
            assert!(totals_json.contains("verified_bytes"));
            assert!(history_json.contains("known_bytes"));
            assert!(!history_json.contains("verified_bytes"));
            assert!(!totals_json.contains("known_bytes"));
            assert!(!report_json.contains("verified_freed_bytes"));
            assert!(!report_json.contains("delete_permanently"));
            assert!(report_json.contains("move_to_trash"));
            assert_eq!(
                serde_json::to_value(&outcome).unwrap()["outcomes"][0]["action"]["type"],
                "move_to_trash"
            );
            assert_eq!(fs::read_to_string(&cookies).unwrap(), "retained sentinel");
            assert_eq!(fs::read(&sibling_path).unwrap(), vec![b'v'; 40]);
        });
    }
}
