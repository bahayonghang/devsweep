//! C05 acceptance on temp fixtures. The command double is not installed winget.
//! These tests do not call `software updates check`, which launches that program.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use devsweep_core::software::{
    REGISTRY_RESIDUE_KEY_NAME, SOFTWARE_UPDATE_PLAN_VERSION, STARTUP_FIXTURE_HIVE_VERSION,
    SoftwareRegistryResiduePlanV1, SoftwareRegistryResidueReportV1,
    SoftwareRegistryResidueRestorePlanV1, SoftwareStartupLocation, SoftwareUpdateOutcome,
    SoftwareUpdatePlanV1, SoftwareUpdateReportV1, StartupFixtureClass, StartupFixtureEntryV1,
    StartupFixtureHiveV1, StartupHiveExecuteReportV1, StartupRestorePlanV1, StartupTogglePlanV1,
    execute_registry_residue, execute_registry_residue_restore, execute_software_update,
    execute_startup_hive, execute_startup_restore, plan_registry_residue,
    plan_registry_residue_restore, plan_software_update, plan_startup_hive, plan_startup_restore,
    preview_registry_residue, preview_registry_residue_restore, preview_software_update,
    preview_startup_hive, preview_startup_restore,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

fn provider_fingerprint(program: &Path) -> String {
    format!(
        "sha256:{}",
        sha256_hex(program.as_os_str().as_encoded_bytes())
    )
}

fn startup_entry_id(tag: &str, name: &str) -> String {
    format!(
        "startup:v1:{}",
        sha256_hex(format!("devsweep.startup.v1\0{tag}\0{name}").as_bytes())
    )
}

fn path_arg(path: &Path) -> String {
    path.to_str().expect("utf-8 fixture path").to_string()
}

fn run(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("devsweep process")
}

fn captured(output: &Output) -> String {
    format!(
        "status {:?}\nstdout {}\nstderr {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap_or_else(|error| {
        panic!("read {}: {error}", path.display());
    }))
    .unwrap_or_else(|error| panic!("json {error} from {}", path.display()))
}

fn assert_status(output: &Output, code: i32) {
    assert_eq!(output.status.code(), Some(code), "{}", captured(output));
}

struct Double {
    _dir: TempDir,
    root: PathBuf,
    program: PathBuf,
}

impl Double {
    fn new(fixture: Value) -> Self {
        let dir = TempDir::new().expect("update double directory");
        let root = dir.path().to_path_buf();
        let program = root.join(format!("update-double{}", std::env::consts::EXE_SUFFIX));
        fs::copy(env!("CARGO_BIN_EXE_process_fixture"), &program).expect("copy command double");
        assert!(
            !program
                .file_name()
                .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("winget.exe"))
        );
        fs::write(
            root.join("update-double.json"),
            serde_json::to_vec(&fixture).expect("double json"),
        )
        .expect("write update double");
        assert!(!root.join("provider.json").exists());
        Self {
            _dir: dir,
            root,
            program,
        }
    }

    fn calls(&self) -> Vec<Vec<String>> {
        let path = self.root.join("update-calls.jsonl");
        if !path.exists() {
            return Vec::new();
        }
        fs::read_to_string(&path)
            .expect("call log")
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line).expect("call json"))
            .collect()
    }
}

fn row(name: &str, id: &str, installed: &str, available: &str) -> Value {
    json!({
        "name": name,
        "id": id,
        "installed": installed,
        "available": available,
        "source": "winget"
    })
}

fn assert_read_only(calls: &[Vec<String>]) {
    assert!(!calls.is_empty(), "read-only probe did not run");
    for call in calls {
        assert_eq!(
            call,
            &vec!["upgrade".to_string(), "--disable-interactivity".to_string()]
        );
        assert!(call.iter().all(|arg| !arg.contains("accept")));
    }
}

fn assert_upgrade_argv(call: &[String], id: &str, version: &str, source: &str) {
    assert_eq!(
        call,
        &[
            "upgrade".to_string(),
            "--id".to_string(),
            id.to_string(),
            "--version".to_string(),
            version.to_string(),
            "--source".to_string(),
            source.to_string(),
            "--disable-interactivity".to_string(),
        ]
    );
    assert!(call.iter().all(|arg| !arg.contains("accept")));
    assert!(
        call.iter()
            .all(|arg| arg != "cmd.exe" && arg != "powershell")
    );
}

fn service_code(error: impl Coded) -> String {
    error.code().to_string()
}

fn plan_file(
    double: &Double,
    package_id: &str,
    source: &str,
    version: &str,
) -> SoftwareUpdatePlanV1 {
    SoftwareUpdatePlanV1 {
        version: SOFTWARE_UPDATE_PLAN_VERSION,
        package_id: package_id.to_string(),
        source: source.to_string(),
        installed_version: "1.0".to_string(),
        available_version: version.to_string(),
        provider_fingerprint: provider_fingerprint(&double.program),
    }
}

fn write_plan(path: &Path, plan: &SoftwareUpdatePlanV1) {
    fs::write(path, serde_json::to_vec_pretty(plan).expect("plan json")).expect("write plan");
}

fn cli_plan(
    double: &Double,
    package_id: &str,
    source: &str,
    version: &str,
    output: &Path,
) -> Output {
    run(&[
        "software".to_string(),
        "updates".to_string(),
        "plan".to_string(),
        "--provider".to_string(),
        path_arg(&double.program),
        "--package-id".to_string(),
        package_id.to_string(),
        "--source".to_string(),
        source.to_string(),
        "--available-version".to_string(),
        version.to_string(),
        "--output".to_string(),
        path_arg(output),
    ])
}

fn cli_preview(double: &Double, plan: &Path, output: &Path, human: bool) -> Output {
    let mut args = vec![
        "software".to_string(),
        "updates".to_string(),
        "preview".to_string(),
        "--provider".to_string(),
        path_arg(&double.program),
        "--plan".to_string(),
        path_arg(plan),
        "--output".to_string(),
        path_arg(output),
    ];
    if !human {
        args.extend(["--format".to_string(), "json".to_string()]);
    }
    run(&args)
}

fn assert_json_refusal(output: &Output, path: &Path, code: &str, exit: i32) {
    assert_status(output, exit);
    let document = read_json(path);
    assert_eq!(document["outcome"], "failed", "{document}");
    assert!(document["data"].is_null(), "{document}");
    assert_eq!(document["error"]["code"], code, "{document}");
}

fn assert_stderr_code(output: &Output, code: &str, exit: i32) {
    assert_status(output, exit);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("{code}:")),
        "missing {code} in {stderr}"
    );
}

trait Coded {
    fn code(&self) -> &str;
}

impl Coded for devsweep_core::software::SoftwareUpdateError {
    fn code(&self) -> &str {
        devsweep_core::software::SoftwareUpdateError::code(self)
    }
}

impl Coded for devsweep_core::software::SoftwareStartupError {
    fn code(&self) -> &str {
        devsweep_core::software::SoftwareStartupError::code(self)
    }
}

impl Coded for devsweep_core::software::SoftwareLeftoverError {
    fn code(&self) -> &str {
        devsweep_core::software::SoftwareLeftoverError::code(self)
    }
}

#[test]
fn ac1_update_refusals_and_restart_are_explicit() {
    let ambiguous = Double::new(json!({
        "rows": [
            row("Dup One", "Dup.Pkg", "1.0", "2.0"),
            row("Dup Two", "Dup.Pkg", "1.0", "2.0")
        ]
    }));
    let ambiguous_error =
        plan_software_update(ambiguous.program.as_os_str(), "Dup.Pkg", "winget", "2.0")
            .expect_err("ambiguous id");
    assert_eq!(service_code(ambiguous_error), "ambiguous_package");
    assert_read_only(&ambiguous.calls());
    let ambiguous_plan = ambiguous.root.join("plan.json");
    write_plan(
        &ambiguous_plan,
        &plan_file(&ambiguous, "Dup.Pkg", "winget", "2.0"),
    );
    let ambiguous_preview = ambiguous.root.join("preview.json");
    let preview = cli_preview(&ambiguous, &ambiguous_plan, &ambiguous_preview, false);
    assert_json_refusal(&preview, &ambiguous_preview, "ambiguous_package", 3);
    assert_read_only(&ambiguous.calls());
    let cli_ambiguous = ambiguous.root.join("cli-plan.json");
    let planned = cli_plan(&ambiguous, "Dup.Pkg", "winget", "2.0", &cli_ambiguous);
    assert_stderr_code(&planned, "ambiguous_package", 3);
    assert!(!cli_ambiguous.exists());
    assert_read_only(&ambiguous.calls());

    let unsupported = Double::new(json!({ "rows": [row("Store App", "Store.App", "1.0", "2.0")] }));
    let unsupported_error = plan_software_update(
        unsupported.program.as_os_str(),
        "Store.App",
        "msstore",
        "2.0",
    )
    .expect_err("unsupported source");
    assert_eq!(service_code(unsupported_error), "unsupported_source");
    assert!(unsupported.calls().is_empty());
    assert!(!unsupported.root.join("update-calls.jsonl").exists());
    let unsupported_out = unsupported.root.join("plan.json");
    let planned = cli_plan(
        &unsupported,
        "Store.App",
        "msstore",
        "2.0",
        &unsupported_out,
    );
    assert_stderr_code(&planned, "unsupported_source", 4);
    assert!(unsupported.calls().is_empty());
    assert!(!unsupported.root.join("update-calls.jsonl").exists());

    let network = Double::new(json!({
        "network": true,
        "rows": [row("Contoso", "Contoso.App", "1.0", "2.0")]
    }));
    let network_error =
        plan_software_update(network.program.as_os_str(), "Contoso.App", "winget", "2.0")
            .expect_err("network");
    assert_eq!(service_code(network_error), "network_unavailable");
    assert_read_only(&network.calls());
    let network_plan = network.root.join("plan.json");
    write_plan(
        &network_plan,
        &plan_file(&network, "Contoso.App", "winget", "2.0"),
    );
    let network_preview = network.root.join("preview.json");
    let preview = cli_preview(&network, &network_plan, &network_preview, false);
    assert_json_refusal(&preview, &network_preview, "network_unavailable", 4);
    assert_read_only(&network.calls());

    let changed = Double::new(json!({
        "rows": [row("Contoso", "Contoso.App", "1.0", "2.0")]
    }));
    let changed_error =
        plan_software_update(changed.program.as_os_str(), "Contoso.App", "winget", "9.0")
            .expect_err("changed version");
    assert_eq!(service_code(changed_error), "version_changed");
    assert_read_only(&changed.calls());
    let changed_plan = changed.root.join("plan.json");
    write_plan(
        &changed_plan,
        &plan_file(&changed, "Contoso.App", "winget", "9.0"),
    );
    let changed_preview = changed.root.join("preview.json");
    let preview = cli_preview(&changed, &changed_plan, &changed_preview, false);
    assert_json_refusal(&preview, &changed_preview, "version_changed", 3);
    assert_read_only(&changed.calls());

    let restart = Double::new(json!({
        "upgrade_result": "restart_required",
        "rows": [row("Contoso", "Contoso.App", "1.0", "2.0")]
    }));
    let plan = plan_software_update(restart.program.as_os_str(), "Contoso.App", "winget", "2.0")
        .expect("restart plan");
    let preview =
        preview_software_update(restart.program.as_os_str(), &plan).expect("restart preview");
    let report = execute_software_update(restart.program.as_os_str(), &plan, &preview.digest, true)
        .expect("restart execute");
    assert_eq!(report.outcome, SoftwareUpdateOutcome::RestartRequired);
    assert!(!report.updated);
    let upgrade_calls = restart
        .calls()
        .into_iter()
        .filter(|call| call.iter().any(|arg| arg == "--id"))
        .collect::<Vec<_>>();
    assert_eq!(upgrade_calls.len(), 1);
    assert_upgrade_argv(&upgrade_calls[0], "Contoso.App", "2.0", "winget");

    let cli_plan_path = restart.root.join("cli-plan.json");
    let planned = cli_plan(&restart, "Contoso.App", "winget", "2.0", &cli_plan_path);
    assert_status(&planned, 0);
    let cli_plan_value = read_json(&cli_plan_path);
    assert_eq!(
        cli_plan_value,
        serde_json::to_value(&plan).expect("plan value")
    );
    let human = restart.root.join("human.txt");
    let human_run = cli_preview(&restart, &cli_plan_path, &human, true);
    assert_status(&human_run, 0);
    let human_text = fs::read_to_string(&human).expect("human preview");
    assert!(human_text.contains("Contoso.App"), "{human_text}");
    assert!(human_text.contains(&preview.digest), "{human_text}");
    let json_preview = restart.root.join("json-preview.json");
    let json_run = cli_preview(&restart, &cli_plan_path, &json_preview, false);
    assert_status(&json_run, 0);
    let data = read_json(&json_preview);
    assert_eq!(data["outcome"], "succeeded", "{data}");
    assert_eq!(
        data["data"],
        serde_json::to_value(&preview).expect("preview value")
    );
    let json_execute = restart.root.join("execute.json");
    let executed = run(&[
        "software".to_string(),
        "updates".to_string(),
        "execute".to_string(),
        "--provider".to_string(),
        path_arg(&restart.program),
        "--plan".to_string(),
        path_arg(&cli_plan_path),
        "--preview-digest".to_string(),
        preview.digest.clone(),
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&json_execute),
    ]);
    assert_status(&executed, 0);
    let envelope = read_json(&json_execute);
    assert_eq!(envelope["outcome"], "restart_required", "{envelope}");
    assert_eq!(envelope["data"]["updated"], false, "{envelope}");
    assert_eq!(
        envelope["data"]["outcome"], "restart_required",
        "{envelope}"
    );
    let cli_report: SoftwareUpdateReportV1 =
        serde_json::from_value(envelope["data"].clone()).expect("cli report");
    assert_eq!(cli_report, report);
    let human_execute = restart.root.join("execute.txt");
    let human_execute_run = run(&[
        "software".to_string(),
        "updates".to_string(),
        "execute".to_string(),
        "--provider".to_string(),
        path_arg(&restart.program),
        "--plan".to_string(),
        path_arg(&cli_plan_path),
        "--preview-digest".to_string(),
        preview.digest,
        "--confirm".to_string(),
        "--output".to_string(),
        path_arg(&human_execute),
    ]);
    assert_status(&human_execute_run, 0);
    let human_execute_text = fs::read_to_string(&human_execute).expect("human execute");
    assert!(
        human_execute_text.contains("Contoso.App"),
        "{human_execute_text}"
    );
    assert!(
        human_execute_text.contains("restart_required"),
        "{human_execute_text}"
    );
    assert!(human_execute_text.contains("false"), "{human_execute_text}");
    for call in restart.calls() {
        if call.iter().any(|arg| arg == "--id") {
            assert_upgrade_argv(&call, "Contoso.App", "2.0", "winget");
        } else {
            assert_eq!(
                call,
                vec!["upgrade".to_string(), "--disable-interactivity".to_string()]
            );
        }
    }

    let bulk = Double::new(json!({
        "rows": [row("Contoso", "Contoso.App", "1.0", "2.0")]
    }));
    let bulk_out = bulk.root.join("bulk.json");
    let selected = run(&[
        "software".to_string(),
        "updates".to_string(),
        "plan".to_string(),
        "--provider".to_string(),
        path_arg(&bulk.program),
        "--package-id".to_string(),
        "Contoso.App".to_string(),
        "--source".to_string(),
        "winget".to_string(),
        "--available-version".to_string(),
        "2.0".to_string(),
        "--output".to_string(),
        path_arg(&bulk_out),
        "--select".to_string(),
        "Contoso.App".to_string(),
    ]);
    assert_status(&selected, 2);
    assert!(bulk.calls().is_empty());
    let repeated = run(&[
        "software".to_string(),
        "updates".to_string(),
        "plan".to_string(),
        "--provider".to_string(),
        path_arg(&bulk.program),
        "--package-id".to_string(),
        "Contoso.App".to_string(),
        "--package-id".to_string(),
        "Other.App".to_string(),
        "--source".to_string(),
        "winget".to_string(),
        "--available-version".to_string(),
        "2.0".to_string(),
        "--output".to_string(),
        path_arg(&bulk.root.join("repeated.json")),
    ]);
    assert_status(&repeated, 2);
    assert!(bulk.calls().is_empty());
    assert!(!bulk.root.join("update-calls.jsonl").exists());
}

fn ordinary_entry(
    class: StartupFixtureClass,
    location: SoftwareStartupLocation,
) -> StartupFixtureEntryV1 {
    StartupFixtureEntryV1 {
        name: "Contoso".to_string(),
        location,
        class,
        run_value: b"Contoso".to_vec(),
        approval: None,
    }
}

fn write_hive(path: &Path, entry: StartupFixtureEntryV1) {
    let hive = StartupFixtureHiveV1 {
        version: STARTUP_FIXTURE_HIVE_VERSION,
        entries: vec![entry],
    };
    let mut bytes = serde_json::to_vec_pretty(&hive).expect("hive json");
    bytes.push(b'\n');
    fs::write(path, bytes).expect("write hive");
}

fn residue_key(dir: &Path) -> PathBuf {
    fs::create_dir_all(dir).expect("residue directory");
    let key = dir.join(REGISTRY_RESIDUE_KEY_NAME);
    fs::write(&key, b"before\n").expect("residue key");
    key
}

#[test]
fn ac4_cli_json_matches_the_shared_service() {
    let temp = TempDir::new().expect("c05 fixtures");
    let hive_path = temp.path().join("hive.json");
    write_hive(
        &hive_path,
        ordinary_entry(
            StartupFixtureClass::Ordinary,
            SoftwareStartupLocation::CurrentUserRun,
        ),
    );
    let entry_id = startup_entry_id("current_user_run", "Contoso");
    let before = fs::read(&hive_path).expect("hive bytes");
    let service_plan = plan_startup_hive(&hive_path, &entry_id, true).expect("hive plan");
    let cli_plan_path = temp.path().join("hive-plan.json");
    let planned = run(&[
        "software".to_string(),
        "startup".to_string(),
        "hive-plan".to_string(),
        "--hive".to_string(),
        path_arg(&hive_path),
        "--entry".to_string(),
        entry_id.clone(),
        "--enabled".to_string(),
        "true".to_string(),
        "--output".to_string(),
        path_arg(&cli_plan_path),
    ]);
    assert_status(&planned, 0);
    let cli_plan: StartupTogglePlanV1 =
        serde_json::from_slice(&fs::read(&cli_plan_path).expect("cli hive plan"))
            .expect("cli plan");
    assert_eq!(cli_plan, service_plan);
    let service_preview = preview_startup_hive(&hive_path, &service_plan).expect("hive preview");
    let preview_path = temp.path().join("hive-preview.json");
    let previewed = run(&[
        "software".to_string(),
        "startup".to_string(),
        "hive-preview".to_string(),
        "--hive".to_string(),
        path_arg(&hive_path),
        "--plan".to_string(),
        path_arg(&cli_plan_path),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&preview_path),
    ]);
    assert_status(&previewed, 0);
    let preview_envelope = read_json(&preview_path);
    assert_eq!(
        preview_envelope["data"],
        serde_json::to_value(&service_preview).expect("preview value")
    );
    let execute_path = temp.path().join("hive-execute.json");
    let executed = run(&[
        "software".to_string(),
        "startup".to_string(),
        "hive-execute".to_string(),
        "--hive".to_string(),
        path_arg(&hive_path),
        "--plan".to_string(),
        path_arg(&cli_plan_path),
        "--preview-digest".to_string(),
        service_preview.digest.clone(),
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&execute_path),
    ]);
    assert_status(&executed, 0);
    let service_report =
        execute_startup_hive(&hive_path, &service_plan, &service_preview.digest, true)
            .expect("service already in state");
    assert_eq!(
        service_report.toggle.outcome,
        devsweep_core::software::StartupStrictOutcome::AlreadyInState
    );
    assert!(service_report.backup.is_none());
    let execute_envelope = read_json(&execute_path);
    assert_eq!(
        execute_envelope["outcome"], "succeeded",
        "{execute_envelope}"
    );
    let cli_report: StartupHiveExecuteReportV1 =
        serde_json::from_value(execute_envelope["data"].clone()).expect("cli hive report");
    assert_eq!(cli_report, service_report);
    assert_eq!(fs::read(&hive_path).expect("hive after already"), before);

    let disable_hive = temp.path().join("disable-hive.json");
    write_hive(
        &disable_hive,
        ordinary_entry(
            StartupFixtureClass::Ordinary,
            SoftwareStartupLocation::CurrentUserRun,
        ),
    );
    let disable_plan = plan_startup_hive(&disable_hive, &entry_id, false).expect("disable plan");
    let disable_plan_path = temp.path().join("disable-plan.json");
    let planned = run(&[
        "software".to_string(),
        "startup".to_string(),
        "hive-plan".to_string(),
        "--hive".to_string(),
        path_arg(&disable_hive),
        "--entry".to_string(),
        entry_id.clone(),
        "--enabled".to_string(),
        "false".to_string(),
        "--output".to_string(),
        path_arg(&disable_plan_path),
    ]);
    assert_status(&planned, 0);
    let cli_disable: StartupTogglePlanV1 =
        serde_json::from_slice(&fs::read(&disable_plan_path).expect("disable plan")).expect("plan");
    assert_eq!(cli_disable, disable_plan);
    let disable_preview =
        preview_startup_hive(&disable_hive, &disable_plan).expect("disable preview");
    let disable_preview_path = temp.path().join("disable-preview.json");
    let previewed = run(&[
        "software".to_string(),
        "startup".to_string(),
        "hive-preview".to_string(),
        "--hive".to_string(),
        path_arg(&disable_hive),
        "--plan".to_string(),
        path_arg(&disable_plan_path),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&disable_preview_path),
    ]);
    assert_status(&previewed, 0);
    assert_eq!(
        read_json(&disable_preview_path)["data"],
        serde_json::to_value(&disable_preview).expect("disable preview value")
    );
    let disable_execute_path = temp.path().join("disable-execute.json");
    let executed = run(&[
        "software".to_string(),
        "startup".to_string(),
        "hive-execute".to_string(),
        "--hive".to_string(),
        path_arg(&disable_hive),
        "--plan".to_string(),
        path_arg(&disable_plan_path),
        "--preview-digest".to_string(),
        disable_preview.digest,
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&disable_execute_path),
    ]);
    assert_status(&executed, 0);
    let disable_envelope = read_json(&disable_execute_path);
    let cli_disable_report: StartupHiveExecuteReportV1 =
        serde_json::from_value(disable_envelope["data"].clone()).unwrap_or_else(|error| {
            panic!("disable report {error}: {}", disable_envelope["data"]);
        });
    let backup = cli_disable_report.backup.clone().expect("backup");
    let service_restore = plan_startup_restore(&backup).expect("service restore plan");
    let restore_plan_path = temp.path().join("restore-plan.json");
    let planned = run(&[
        "software".to_string(),
        "startup".to_string(),
        "restore-plan".to_string(),
        "--report".to_string(),
        path_arg(&disable_execute_path),
        "--output".to_string(),
        path_arg(&restore_plan_path),
    ]);
    assert_status(&planned, 0);
    let cli_restore: StartupRestorePlanV1 =
        serde_json::from_slice(&fs::read(&restore_plan_path).expect("restore plan"))
            .expect("restore");
    assert_eq!(cli_restore, service_restore);
    let service_restore_preview =
        preview_startup_restore(&disable_hive, &service_restore).expect("restore preview");
    let restore_preview_path = temp.path().join("restore-preview.json");
    let previewed = run(&[
        "software".to_string(),
        "startup".to_string(),
        "restore-preview".to_string(),
        "--hive".to_string(),
        path_arg(&disable_hive),
        "--plan".to_string(),
        path_arg(&restore_plan_path),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&restore_preview_path),
    ]);
    assert_status(&previewed, 0);
    assert_eq!(
        read_json(&restore_preview_path)["data"],
        serde_json::to_value(&service_restore_preview).expect("restore preview value")
    );
    let mut changed: StartupFixtureHiveV1 =
        serde_json::from_slice(&fs::read(&disable_hive).expect("hive")).expect("hive");
    changed.entries[0].approval = Some(vec![9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let mut changed_bytes = serde_json::to_vec_pretty(&changed).expect("changed hive");
    changed_bytes.push(b'\n');
    fs::write(&disable_hive, &changed_bytes).expect("external hive");
    let service_error = execute_startup_restore(
        &disable_hive,
        &service_restore,
        &service_restore_preview.digest,
        true,
    )
    .expect_err("service external change");
    assert_eq!(service_code(service_error), "startup_state_changed");
    assert_eq!(fs::read(&disable_hive).expect("hive"), changed_bytes);
    let restore_execute_path = temp.path().join("restore-execute.json");
    let refused = run(&[
        "software".to_string(),
        "startup".to_string(),
        "restore-execute".to_string(),
        "--hive".to_string(),
        path_arg(&disable_hive),
        "--plan".to_string(),
        path_arg(&restore_plan_path),
        "--preview-digest".to_string(),
        service_restore_preview.digest,
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&restore_execute_path),
    ]);
    assert_json_refusal(&refused, &restore_execute_path, "startup_state_changed", 3);
    assert_eq!(
        fs::read(&disable_hive).expect("hive unchanged"),
        changed_bytes
    );

    let protected_hive = temp.path().join("protected.json");
    write_hive(
        &protected_hive,
        ordinary_entry(
            StartupFixtureClass::Security,
            SoftwareStartupLocation::CurrentUserRun,
        ),
    );
    let protected_bytes = fs::read(&protected_hive).expect("protected hive");
    let protected_error =
        plan_startup_hive(&protected_hive, &entry_id, false).expect_err("protected");
    assert_eq!(service_code(protected_error), "protected");
    let protected_plan = temp.path().join("protected-plan.json");
    let protected_cli = run(&[
        "software".to_string(),
        "startup".to_string(),
        "hive-plan".to_string(),
        "--hive".to_string(),
        path_arg(&protected_hive),
        "--entry".to_string(),
        entry_id,
        "--enabled".to_string(),
        "false".to_string(),
        "--output".to_string(),
        path_arg(&protected_plan),
    ]);
    assert_stderr_code(&protected_cli, "protected", 4);
    assert_eq!(
        fs::read(&protected_hive).expect("protected unchanged"),
        protected_bytes
    );
    assert!(!protected_plan.exists());

    let service_dir = temp.path().join("residue-service");
    let cli_dir = temp.path().join("residue-cli");
    let service_key = residue_key(&service_dir);
    let cli_key = residue_key(&cli_dir);
    let original = fs::read(&service_key).expect("original");
    assert_eq!(fs::read(&cli_key).expect("cli original"), original);
    let service_residue = plan_registry_residue(&service_key).expect("residue plan");
    let residue_plan_path = temp.path().join("residue-plan.json");
    let planned = run(&[
        "software".to_string(),
        "leftovers".to_string(),
        "registry-plan".to_string(),
        "--key".to_string(),
        path_arg(&cli_key),
        "--output".to_string(),
        path_arg(&residue_plan_path),
    ]);
    assert_status(&planned, 0);
    let cli_residue: SoftwareRegistryResiduePlanV1 =
        serde_json::from_slice(&fs::read(&residue_plan_path).expect("residue plan")).expect("plan");
    assert_eq!(cli_residue, service_residue);
    let service_residue_preview =
        preview_registry_residue(&service_key, &service_residue).expect("residue preview");
    let residue_preview_path = temp.path().join("residue-preview.json");
    let previewed = run(&[
        "software".to_string(),
        "leftovers".to_string(),
        "registry-preview".to_string(),
        "--key".to_string(),
        path_arg(&cli_key),
        "--plan".to_string(),
        path_arg(&residue_plan_path),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&residue_preview_path),
    ]);
    assert_status(&previewed, 0);
    assert_eq!(
        read_json(&residue_preview_path)["data"],
        serde_json::to_value(&service_residue_preview).expect("residue preview value")
    );
    let human_residue = temp.path().join("residue-human.txt");
    let human = run(&[
        "software".to_string(),
        "leftovers".to_string(),
        "registry-preview".to_string(),
        "--key".to_string(),
        path_arg(&cli_key),
        "--plan".to_string(),
        path_arg(&residue_plan_path),
        "--output".to_string(),
        path_arg(&human_residue),
    ]);
    assert_status(&human, 0);
    let human_residue_text = fs::read_to_string(&human_residue).expect("residue human");
    assert!(
        human_residue_text.contains(REGISTRY_RESIDUE_KEY_NAME),
        "{human_residue_text}"
    );
    assert!(
        human_residue_text.contains(&service_residue_preview.digest),
        "{human_residue_text}"
    );
    let service_backup = service_dir.join("backup.bin");
    let service_report = execute_registry_residue(
        &service_key,
        &service_backup,
        &service_residue,
        &service_residue_preview.digest,
        true,
    )
    .expect("service residue execute");
    let cli_backup = cli_dir.join("backup.bin");
    let residue_execute_path = temp.path().join("residue-execute.json");
    let executed = run(&[
        "software".to_string(),
        "leftovers".to_string(),
        "registry-execute".to_string(),
        "--key".to_string(),
        path_arg(&cli_key),
        "--backup".to_string(),
        path_arg(&cli_backup),
        "--plan".to_string(),
        path_arg(&residue_plan_path),
        "--preview-digest".to_string(),
        service_residue_preview.digest.clone(),
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&residue_execute_path),
    ]);
    assert_status(&executed, 0);
    let residue_envelope = read_json(&residue_execute_path);
    let cli_report: SoftwareRegistryResidueReportV1 =
        serde_json::from_value(residue_envelope["data"].clone()).expect("residue report");
    assert_eq!(cli_report, service_report);
    assert_eq!(fs::read(&service_backup).expect("service backup"), original);
    assert_eq!(fs::read(&cli_backup).expect("cli backup"), original);
    assert_eq!(fs::read(&service_key).expect("service key"), b"removed\n");
    assert_eq!(fs::read(&cli_key).expect("cli key"), b"removed\n");
    let service_restore_plan =
        plan_registry_residue_restore(&service_report).expect("residue restore plan");
    let residue_restore_plan = temp.path().join("residue-restore-plan.json");
    let planned = run(&[
        "software".to_string(),
        "leftovers".to_string(),
        "registry-restore-plan".to_string(),
        "--report".to_string(),
        path_arg(&residue_execute_path),
        "--output".to_string(),
        path_arg(&residue_restore_plan),
    ]);
    assert_status(&planned, 0);
    let cli_restore_plan: SoftwareRegistryResidueRestorePlanV1 =
        serde_json::from_slice(&fs::read(&residue_restore_plan).expect("restore plan"))
            .expect("restore plan");
    assert_eq!(cli_restore_plan, service_restore_plan);
    let service_restore_preview =
        preview_registry_residue_restore(&service_key, &service_backup, &service_restore_plan)
            .expect("residue restore preview");
    let residue_restore_preview = temp.path().join("residue-restore-preview.json");
    let previewed = run(&[
        "software".to_string(),
        "leftovers".to_string(),
        "registry-restore-preview".to_string(),
        "--key".to_string(),
        path_arg(&cli_key),
        "--backup".to_string(),
        path_arg(&cli_backup),
        "--plan".to_string(),
        path_arg(&residue_restore_plan),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&residue_restore_preview),
    ]);
    assert_status(&previewed, 0);
    assert_eq!(
        read_json(&residue_restore_preview)["data"],
        serde_json::to_value(&service_restore_preview).expect("restore preview value")
    );
    execute_registry_residue_restore(
        &service_key,
        &service_backup,
        &service_restore_plan,
        &service_restore_preview.digest,
        true,
    )
    .expect("service restore");
    let restore_execute = temp.path().join("residue-restore-execute.json");
    let restored = run(&[
        "software".to_string(),
        "leftovers".to_string(),
        "registry-restore-execute".to_string(),
        "--key".to_string(),
        path_arg(&cli_key),
        "--backup".to_string(),
        path_arg(&cli_backup),
        "--plan".to_string(),
        path_arg(&residue_restore_plan),
        "--preview-digest".to_string(),
        service_restore_preview.digest.clone(),
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&restore_execute),
    ]);
    assert_status(&restored, 0);
    assert_eq!(read_json(&restore_execute)["data"], Value::Null);
    assert_eq!(fs::read(&service_key).expect("service restored"), original);
    assert_eq!(fs::read(&cli_key).expect("cli restored"), original);
    fs::write(&service_key, b"external\n").expect("external service key");
    fs::write(&cli_key, b"external\n").expect("external cli key");
    let service_changed = execute_registry_residue_restore(
        &service_key,
        &service_backup,
        &service_restore_plan,
        &service_restore_preview.digest,
        true,
    )
    .expect_err("service residue changed");
    assert_eq!(service_code(service_changed), "residue_changed");
    assert_eq!(fs::read(&service_key).expect("service key"), b"external\n");
    assert_eq!(fs::read(&service_backup).expect("service backup"), original);
    let refused_path = temp.path().join("residue-refused.json");
    let refused = run(&[
        "software".to_string(),
        "leftovers".to_string(),
        "registry-restore-execute".to_string(),
        "--key".to_string(),
        path_arg(&cli_key),
        "--backup".to_string(),
        path_arg(&cli_backup),
        "--plan".to_string(),
        path_arg(&residue_restore_plan),
        "--preview-digest".to_string(),
        service_restore_preview.digest,
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        path_arg(&refused_path),
    ]);
    assert_json_refusal(&refused, &refused_path, "residue_changed", 3);
    assert_eq!(fs::read(&cli_key).expect("cli key"), b"external\n");
    assert_eq!(fs::read(&cli_backup).expect("cli backup"), original);

    let vendor = temp.path().join("VendorPrefix");
    fs::write(&vendor, b"vendor").expect("vendor key");
    let vendor_error = plan_registry_residue(&vendor).expect_err("vendor prefix");
    assert_eq!(service_code(vendor_error), "invalid_identity");
    let vendor_plan = temp.path().join("vendor-plan.json");
    let vendor_cli = run(&[
        "software".to_string(),
        "leftovers".to_string(),
        "registry-plan".to_string(),
        "--key".to_string(),
        path_arg(&vendor),
        "--output".to_string(),
        path_arg(&vendor_plan),
    ]);
    assert_stderr_code(&vendor_cli, "invalid_identity", 3);
    assert_eq!(fs::read(&vendor).expect("vendor bytes"), b"vendor");
    assert!(!vendor_plan.exists());
    let bulk = run(&[
        "software".to_string(),
        "leftovers".to_string(),
        "registry-plan".to_string(),
        "--key".to_string(),
        path_arg(&cli_key),
        "--output".to_string(),
        path_arg(&temp.path().join("bulk-residue.json")),
        "--select".to_string(),
        "DevSweepResidue".to_string(),
    ]);
    assert_status(&bulk, 2);
}
