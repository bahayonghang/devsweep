// Included inside execution::tests; all side effects use recording runners.
mod existing_rule_pipeline {
    use super::*;
    use crate::{
        filesystem::{path_is_within, paths_equal},
        model::CleanupIntent,
        rules::{KnownCacheAction, ProjectMarker},
        scan::{ScanOptions, Sweeper},
    };

    fn file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("fixture directory");
        fs::write(path, content).expect("fixture file");
    }

    fn verify(
        plan: &crate::model::UntrustedPlan,
        selected: Vec<TargetId>,
        home: Option<PathBuf>,
        audit: &Path,
    ) -> ExecutionReport {
        let persisted =
            serde_json::from_slice(&serde_json::to_vec(plan).expect("saved v2")).expect("load v2");
        let validated = crate::plan::validate_plan(&persisted).expect("production registry");
        let command = RecordingCommandRunner::default();
        let trash = RecordingTrashRunner::default();
        let executor = Executor::new(command, trash).with_safety_policy(
            SafetyPolicy::from_user_list(UserProtectionList::empty_in_memory_for_tests_only())
                .with_home(home),
        );
        let preview = executor
            .run_plan(
                &validated,
                ExecutionRequest {
                    selected: selected.clone(),
                    execute: false,
                    audit_log: None,
                    expected_digest: None,
                    cancel: None,
                },
            )
            .expect("live preview");
        executor
            .run_plan(
                &validated,
                ExecutionRequest {
                    selected,
                    execute: true,
                    audit_log: Some(audit.to_path_buf()),
                    expected_digest: Some(preview.confirmation_digest),
                    cancel: None,
                },
            )
            .expect("recorded execution")
    }

    #[test]
    fn existing_project_directories_and_nested_python_roundtrip() {
        for rule in crate::rules::PROJECT_DIR_RULES {
            let fixture = TempDir::new().expect("fixture");
            let root = fixture.path();
            let marker = root.join(if rule.marker == ProjectMarker::Python {
                "pyproject.toml"
            } else {
                "package.json"
            });
            file(&marker, "{}");
            let directory = if rule.marker == ProjectMarker::Python {
                root.join("nested/src")
            } else {
                root.to_path_buf()
            };
            let sentinel = directory.join(rule.relative).join("generated.bin");
            file(&sentinel, "retained by recording runner");
            let scanned = crate::scan::ProjectScanner::new()
                .scan_roots(&[root.to_path_buf()])
                .expect("real scanner");
            let plan = crate::plan::untrusted_plan_from_scan(&scanned).expect("real conversion");
            let target = plan
                .targets
                .iter()
                .find(|target| target.rule_id == rule.id)
                .expect("observed rule");
            let report = verify(
                &plan,
                vec![target.id.clone()],
                None,
                &root.join("audit.jsonl"),
            );
            assert_eq!(report.succeeded, 1, "{}: {:?}", rule.id, report.failures);
            assert_eq!(
                fs::read_to_string(&sentinel).unwrap(),
                "retained by recording runner"
            );
            assert!(marker.exists());
            let mut tampered = plan;
            tampered.targets[0].path = Some(root.join("source"));
            assert!(crate::plan::validate_plan(&tampered).is_err());
        }
        let fixture = TempDir::new().unwrap();
        file(&fixture.path().join("pyproject.toml"), "[project]");
        let sentinel = fixture.path().join("src/nested/__pycache__/module.pyc");
        file(&sentinel, "bytecode");
        let scanned = crate::scan::ProjectScanner::new()
            .scan_roots(&[fixture.path().to_path_buf()])
            .unwrap();
        let plan = crate::plan::untrusted_plan_from_scan(&scanned).unwrap();
        let target = plan
            .targets
            .iter()
            .find(|target| target.rule_id == "python.__pycache__")
            .unwrap();
        assert_eq!(
            verify(
                &plan,
                vec![target.id.clone()],
                None,
                &fixture.path().join("audit.jsonl")
            )
            .succeeded,
            1
        );
        assert!(sentinel.exists());
    }

    #[test]
    fn tox_only_and_nearer_tox_projects_roundtrip_with_live_boundary_checks() {
        let fixture = TempDir::new().unwrap();
        let root = fixture.path();
        file(&root.join("tox.ini"), "[tox]");
        file(&root.join(".tox/payload"), "retained");
        file(&root.join("nested/src/.pytest_cache/payload"), "retained");
        let scan = || {
            let scanned = crate::scan::ProjectScanner::new()
                .scan_roots(&[root.to_path_buf()])
                .unwrap();
            crate::plan::untrusted_plan_from_scan(&scanned).unwrap()
        };
        let plan = scan();
        assert_eq!(plan.targets.len(), 2);
        assert_eq!(
            verify(
                &plan,
                plan.targets
                    .iter()
                    .map(|target| target.id.clone())
                    .collect(),
                None,
                &root.join("audit.jsonl")
            )
            .succeeded,
            2
        );
        let validated = crate::plan::validate_plan(&plan).unwrap();
        let command = RecordingCommandRunner::default();
        let trash = RecordingTrashRunner::default();
        let executor = test_executor(command.clone(), trash.clone());
        let nested = plan
            .targets
            .iter()
            .find(|target| target.path.as_ref().unwrap().ends_with(".pytest_cache"))
            .unwrap();
        let preview = executor
            .run_plan(
                &validated,
                ExecutionRequest {
                    selected: vec![nested.id.clone()],
                    execute: false,
                    audit_log: None,
                    expected_digest: None,
                    cancel: None,
                },
            )
            .unwrap();
        file(&root.join("nested/tox.ini"), "[tox]");
        let drifted = executor
            .run_plan(
                &validated,
                ExecutionRequest {
                    selected: vec![nested.id.clone()],
                    execute: true,
                    audit_log: Some(root.join("drift-audit.jsonl")),
                    expected_digest: Some(preview.confirmation_digest),
                    cancel: None,
                },
            )
            .unwrap();
        assert_eq!(drifted.failed, 1);
        assert!(drifted.failures[0].message.contains("nearer project"));
        assert!(command.requests().is_empty());
        assert!(trash.paths().is_empty());
        let fresh = scan();
        assert_eq!(
            verify(
                &fresh,
                fresh
                    .targets
                    .iter()
                    .map(|target| target.id.clone())
                    .collect(),
                None,
                &root.join("fresh-audit.jsonl")
            )
            .succeeded,
            2
        );
        assert!(root.join(".tox/payload").exists());
        assert!(root.join("nested/src/.pytest_cache/payload").exists());
    }

    #[test]
    fn rust_metadata_and_local_fallback_roundtrip_without_running_cleanup() {
        const TEST: &str = "execution::tests::existing_rule_pipeline::rust_metadata_and_local_fallback_roundtrip_without_running_cleanup";
        if std::env::var_os("CARGO_TARGET_DIR").is_some() {
            assert!(std::env::var_os("DEVSWEEP_CARGO_PIPELINE_CHILD").is_none());
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", TEST, "--nocapture"])
                .env_remove("CARGO_TARGET_DIR")
                .env("DEVSWEEP_CARGO_PIPELINE_CHILD", "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        for valid in [true, false] {
            let fixture = TempDir::new().unwrap();
            let root = fixture.path();
            file(
                &root.join("Cargo.toml"),
                if valid {
                    "[package]\nname='pipeline-fixture'\nversion='0.1.0'\nedition='2021'\n"
                } else {
                    "invalid Cargo manifest"
                },
            );
            file(&root.join("src/lib.rs"), "// retained source");
            file(&root.join("target/debug/fixture.bin"), "output");
            let scanned = crate::scan::ProjectScanner::new()
                .scan_roots(&[root.to_path_buf()])
                .unwrap();
            let plan = crate::plan::untrusted_plan_from_scan(&scanned).unwrap();
            let target = plan
                .targets
                .iter()
                .find(|target| target.rule_id == "rust.target")
                .unwrap();
            assert_eq!(
                matches!(target.intent, CleanupIntent::RunBuiltInAction { .. }),
                valid
            );
            let result = verify(
                &plan,
                vec![target.id.clone()],
                None,
                &root.join("audit.jsonl"),
            );
            assert_eq!(result.succeeded, 1, "valid={valid}: {:?}", result.failures);
            assert_eq!(
                fs::read_to_string(root.join("src/lib.rs")).unwrap(),
                "// retained source"
            );
            assert!(root.join("target/debug/fixture.bin").exists());
        }
    }

    #[test]
    fn known_global_roots_and_overrides_roundtrip_without_enabling_retained_roots() {
        const CHILD: &str = "DEVSWEEP_KNOWN_ROOT_FIXTURE";
        const TEST: &str = "execution::tests::existing_rule_pipeline::known_global_roots_and_overrides_roundtrip_without_enabling_retained_roots";
        let Some(scenario) = std::env::var_os(CHILD) else {
            for scenario in ["default", "overrides"] {
                let fixture = crate::test_support::tempdir().unwrap();
                let root = fixture.path();
                fs::create_dir_all(root.join("home")).unwrap();
                let mut child = std::process::Command::new(std::env::current_exe().unwrap());
                child
                    .args(["--exact", TEST, "--nocapture"])
                    .env_clear()
                    .env(CHILD, scenario)
                    .env("HOME", root.join("home"))
                    .env("USERPROFILE", root.join("home"))
                    .env("APPDATA", root.join("home/AppData/Roaming"))
                    .env("LOCALAPPDATA", root.join("home/AppData/Local"))
                    .env("PATH", root.join("no-tools"))
                    .env("TEMP", root)
                    .env("TMP", root)
                    .current_dir(root);
                for key in ["SystemRoot", "WINDIR", "SystemDrive"] {
                    if let Some(value) = std::env::var_os(key) {
                        child.env(key, value);
                    }
                }
                if scenario == "overrides" {
                    child
                        .env("GRADLE_USER_HOME", root.join("custom Gradle 空格"))
                        .env("NUGET_PACKAGES", root.join("custom NuGet 空格"))
                        .env("GOMODCACHE", root.join("custom Go 空格"));
                }
                let output = child.output().unwrap();
                assert!(
                    output.status.success(),
                    "{scenario}: {} {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            return;
        };
        let home = PathBuf::from(std::env::var_os("HOME").unwrap());
        let mut executable = 0;
        let mut sentinels = Vec::new();
        for rule in crate::rules::global_cache_rules() {
            let root = crate::rules::configured_cache_path(rule, &home, |key| {
                std::env::var_os(key).map(PathBuf::from)
            });
            let sentinel = root.join("retained.bin");
            file(&sentinel, "retained");
            sentinels.push(sentinel);
            executable += usize::from(rule.action == KnownCacheAction::Trash);
        }
        file(&home.join(".cargo/credentials.toml"), "retained secret");
        let report = Sweeper::default()
            .full_scan_report(
                &ScanOptions {
                    roots: vec![],
                    include_projects: false,
                    include_global: true,
                },
                &mut |_| {},
            )
            .unwrap();
        let plan = report.plan;
        for rule in crate::rules::global_cache_rules() {
            let target = plan
                .targets
                .iter()
                .find(|target| target.rule_id == rule.id)
                .unwrap_or_else(|| panic!("{} in {scenario:?}", rule.id));
            assert_eq!(
                matches!(target.intent, CleanupIntent::InspectOnly { .. }),
                rule.action == KnownCacheAction::InspectOnly
            );
        }
        let validated = crate::plan::validate_plan(&plan)
            .expect("all inspect and executable observations validate");
        let executor = test_executor(
            RecordingCommandRunner::default(),
            RecordingTrashRunner::default(),
        );
        for target in plan
            .targets
            .iter()
            .filter(|target| matches!(target.intent, CleanupIntent::InspectOnly { .. }))
        {
            assert!(
                executor
                    .run_plan(
                        &validated,
                        ExecutionRequest {
                            selected: vec![target.id.clone()],
                            execute: false,
                            audit_log: None,
                            expected_digest: None,
                            cancel: None
                        }
                    )
                    .is_err(),
                "inspect-only selection cannot become authority"
            );
        }
        let selected = plan
            .targets
            .iter()
            .filter(|target| !matches!(target.intent, CleanupIntent::InspectOnly { .. }))
            .map(|target| target.id.clone())
            .collect();
        let result = verify(
            &plan,
            selected,
            Some(home.clone()),
            &home.join("audit.jsonl"),
        );
        assert_eq!(result.succeeded, executable, "{:?}", result.failures);
        assert_eq!(result.failed, 0);
        assert_eq!(result.skipped, 0);
        for sentinel in sentinels {
            assert_eq!(fs::read_to_string(sentinel).unwrap(), "retained");
        }
        assert_eq!(
            fs::read_to_string(home.join(".cargo/credentials.toml")).unwrap(),
            "retained secret"
        );
    }

    fn plant_protected_ai_home(home: &Path) -> Vec<PathBuf> {
        let current_version = home.join("versions/2.0/payload.bin");
        let current_marker = home.join("versions/current.txt");
        let active_session = home.join("sessions/s1.json");
        let active_marker = home.join("sessions/active");
        let credentials = home.join("credentials");
        let memory = home.join("memory.md");
        let settings = home.join("settings.json");
        let skill = home.join("skills/skill.md");
        let mcp = home.join("mcp/server.json");
        let models = home.join("models/weights.bin");
        let project = home.join("projects/code.txt");
        file(&current_version, "current-version");
        file(&current_marker, "2.0\n");
        file(
            &active_session,
            r#"{"version":1,"id":"s1","active":true,"body":"live"}"#,
        );
        file(&active_marker, "s1\n");
        file(&credentials, "secret");
        file(&memory, "memory");
        file(&settings, "{}");
        file(&skill, "skill");
        file(&mcp, "{}");
        file(&models, "model");
        file(&project, "project");
        vec![
            current_version,
            current_marker,
            home.join("versions/2.0"),
            active_session,
            active_marker,
            credentials,
            memory,
            settings,
            skill,
            home.join("skills"),
            mcp,
            home.join("mcp"),
            models,
            home.join("models"),
            project,
            home.join("projects"),
        ]
    }

    #[test]
    fn generic_clean_scan_does_not_select_protected_ai_paths() {
        const CHILD: &str = "DEVSWEEP_C15_AC2_GENERIC_SCAN";
        const TEST: &str = "execution::tests::existing_rule_pipeline::generic_clean_scan_does_not_select_protected_ai_paths";
        if std::env::var_os(CHILD).is_none() {
            let fixture = TempDir::new().unwrap();
            let root = fixture.path();
            fs::create_dir_all(root.join("home")).unwrap();
            let mut child = std::process::Command::new(std::env::current_exe().unwrap());
            child
                .args(["--exact", TEST, "--nocapture"])
                .env_clear()
                .env(CHILD, "1")
                .env("HOME", root.join("home"))
                .env("USERPROFILE", root.join("home"))
                .env("APPDATA", root.join("home/AppData/Roaming"))
                .env("LOCALAPPDATA", root.join("home/AppData/Local"))
                .env("PATH", root.join("no-tools"))
                .env("TEMP", root)
                .env("TMP", root)
                .current_dir(root);
            for key in ["SystemRoot", "WINDIR", "SystemDrive"] {
                if let Some(value) = std::env::var_os(key) {
                    child.env(key, value);
                }
            }
            let output = child.output().unwrap();
            assert!(
                output.status.success(),
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }

        let home = PathBuf::from(std::env::var_os("HOME").unwrap());
        let cache_dir = home.join(".gradle/caches");
        let sentinel = cache_dir.join("generic-cache.bin");
        file(&sentinel, "selectable generic cache");
        let mut protected = plant_protected_ai_home(&home.join(".claude"));
        protected.extend(plant_protected_ai_home(&home.join(".codex")));
        if let Some(appdata) = std::env::var_os("APPDATA") {
            protected.extend(plant_protected_ai_home(
                &PathBuf::from(appdata).join("Cursor"),
            ));
        }

        let report = Sweeper::default()
            .full_scan_report(
                &ScanOptions {
                    roots: vec![],
                    include_projects: false,
                    include_global: true,
                },
                &mut |_| {},
            )
            .expect("production generic global scan");
        let target_paths: Vec<_> = report
            .plan
            .targets
            .iter()
            .filter_map(|target| target.path.as_ref())
            .collect();
        assert!(
            report.plan.targets.iter().any(|target| {
                target.rule_id == "gradle.caches"
                    && target
                        .path
                        .as_ref()
                        .is_some_and(|path| paths_equal(path, &cache_dir))
            }),
            "generic cache sentinel must appear so exclusion is not vacuous; targets={target_paths:?}"
        );
        for protected in &protected {
            for path in &target_paths {
                assert!(
                    !paths_equal(path, protected) && !path_is_within(path, protected),
                    "protected {} selected as {}",
                    protected.display(),
                    path.display()
                );
            }
            assert!(
                fs::symlink_metadata(protected).is_ok(),
                "scan must not delete {}",
                protected.display()
            );
        }
        assert_eq!(
            fs::read_to_string(&sentinel).unwrap(),
            "selectable generic cache"
        );
    }
}
