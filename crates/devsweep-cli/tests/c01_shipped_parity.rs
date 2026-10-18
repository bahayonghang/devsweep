//! Shipped `devsweep` process compared with the same core call.

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{Mutex, MutexGuard, PoisonError},
};

use devsweep_core::{
    analysis::{
        analyze_path, decode_analyze_trash_plan, plan_analyze_trash, preview_analyze_trash_plan,
    },
    software::{
        SoftwareAuditRecordV1, SoftwareEligibility, SoftwareEligibilityReason,
        SoftwareEligibilityState, SoftwareEntryV1, SoftwareIdentity, SoftwareInventoryV1,
        SoftwareLastUsedEvidence, SoftwareLastUsedReason, SoftwareLeftoverPlanV1, SoftwareScope,
        SoftwareSizeEvidence, SoftwareStartupLocation, discover_software_leftovers,
        list_startup_entries, plan_startup_toggle, preview_software_leftover_plan,
        preview_startup_toggle,
    },
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn lock_env() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

struct EnvGuard {
    saved: Vec<(String, Option<OsString>)>,
}

impl EnvGuard {
    fn set(vars: &[(&str, &Path)]) -> Self {
        let saved = vars
            .iter()
            .map(|(key, value)| {
                let previous = std::env::var_os(key);
                unsafe {
                    std::env::set_var(key, value);
                }
                ((*key).to_string(), previous)
            })
            .collect();
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in self.saved.drain(..) {
            unsafe {
                match previous {
                    Some(value) => std::env::set_var(&key, value),
                    None => std::env::remove_var(&key),
                }
            }
        }
    }
}

struct Captured {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_devsweep(args: &[String], env: &[(&str, &Path)], stdout: Stdio) -> Captured {
    let mut command = Command::new(env!("CARGO_BIN_EXE_devsweep"));
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let output = command.output().expect("devsweep process runs");
    captured(output)
}

fn captured(output: Output) -> Captured {
    Captured {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn path_arg(path: &Path) -> String {
    path.to_str().expect("fixture path is utf-8").to_string()
}

fn read_envelope(path: &Path, run: &Captured) -> serde_json::Value {
    let text = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "read {}: {error}\nstatus {:?}\nstderr {}\nstdout {}",
            path.display(),
            run.status,
            run.stderr,
            run.stdout
        )
    });
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("json {error}: {text}"))
}

fn success_data(path: &Path, run: &Captured) -> serde_json::Value {
    assert_eq!(
        run.status,
        Some(0),
        "stderr {}\nstdout {}",
        run.stderr,
        run.stdout
    );
    let document = read_envelope(path, run);
    assert_eq!(document["outcome"], "succeeded", "{document}");
    assert!(document["error"].is_null(), "{document}");
    document["data"].clone()
}

struct FileMark {
    exists: bool,
    len: u64,
}

fn mark(path: &Path) -> FileMark {
    match fs::metadata(path) {
        Ok(meta) => FileMark {
            exists: true,
            len: meta.len(),
        },
        Err(_) => FileMark {
            exists: false,
            len: 0,
        },
    }
}

fn assert_unchanged(path: &Path, before: &FileMark) {
    let after = mark(path);
    assert_eq!(after.exists, before.exists, "{}", path.display());
    assert_eq!(after.len, before.len, "{}", path.display());
}

fn real_clean_journal() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA"))
        .join("DevSweep")
        .join("audit")
        .join("v1")
        .join("clean.jsonl")
}

struct AnalyzeFixture {
    _temp: TempDir,
    file: PathBuf,
    bytes: Vec<u8>,
    plan_path: PathBuf,
    digest: String,
}

fn analyze_fixture() -> AnalyzeFixture {
    let temp = TempDir::new().expect("analyze fixture");
    let root = temp.path().join("root");
    fs::create_dir_all(&root).expect("root");
    let file = root.join("keep.bin");
    let bytes = b"isolated-keep".to_vec();
    fs::write(&file, &bytes).expect("keep.bin");
    let outcome = analyze_path(&root, None, None).expect("analyze");
    let snapshot = outcome.snapshot();
    let keep = snapshot
        .nodes
        .iter()
        .find(|node| node.name == "keep.bin")
        .expect("keep.bin node");
    let plan = plan_analyze_trash(snapshot, &[keep.id]).unwrap_or_else(|error| {
        panic!("plan {} {error}", error.code());
    });
    let plan_path = temp.path().join("plan.json");
    let encoded = serde_json::to_vec(&plan).expect("plan json");
    fs::write(&plan_path, &encoded).expect("write plan");
    let decoded = decode_analyze_trash_plan(&encoded).expect("decode plan");
    let preview = preview_analyze_trash_plan(&decoded).unwrap_or_else(|error| {
        panic!("preview {} {error}", error.code());
    });
    assert!(
        preview.items.iter().any(|item| item.node_id == keep.id),
        "keep.bin refused {:?}",
        preview.refused
    );
    AnalyzeFixture {
        _temp: temp,
        file,
        bytes,
        plan_path,
        digest: preview.digest,
    }
}

fn software_id(identity: &SoftwareIdentity) -> String {
    let canonical = serde_json::to_vec(&("devsweep.software.identity.v1", identity))
        .expect("identity canonical bytes");
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(canonical) {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("software:v1:msix:{hex}")
}

fn succeeded_uninstall_journal(identity: &SoftwareIdentity, operation_id: &str) -> String {
    let identity = serde_json::to_value(identity).expect("identity json");
    let common = serde_json::json!({
        "schema_version": 1,
        "domain": "software",
        "operation_id": operation_id,
        "timestamp_unix_ms": 1,
        "identity": identity,
        "inventory_fingerprint": format!("sha256:{}", "1".repeat(64)),
        "preview_digest": format!("sha256:{}", "2".repeat(64)),
        "irreversible": true,
        "reboot_evidence": "none",
    });
    let mut lines = String::new();
    for extra in [
        serde_json::json!({"transition":{"kind":"validated"},"status_code":"validated"}),
        serde_json::json!({"transition":{"kind":"dispatch_started"},"status_code":"dispatch_started"}),
        serde_json::json!({"transition":{"kind":"adapter_completed"},"status_code":"adapter_succeeded","adapter_outcome":"success"}),
        serde_json::json!({"transition":{"kind":"requery_observed"},"status_code":"requery_absent","installed_state":"absent","requery_result":"absent"}),
        serde_json::json!({"transition":{"kind":"terminal","outcome":"removed"},"status_code":"removed","adapter_outcome":"success","installed_state":"absent","requery_result":"absent"}),
    ] {
        let mut value = common.clone();
        let object = value.as_object_mut().expect("journal object");
        for (key, item) in extra.as_object().expect("transition object") {
            object.insert(key.clone(), item.clone());
        }
        let record: SoftwareAuditRecordV1 = serde_json::from_value(value.clone())
            .unwrap_or_else(|error| panic!("audit record {error}: {value}"));
        lines.push_str(&serde_json::to_string(&record).expect("audit line"));
        lines.push('\n');
    }
    lines
}

fn authority_exit(code: &str) -> i32 {
    match code {
        "startup_entry_changed" | "stale_candidate" | "changed_since_snapshot" => 3,
        other => panic!("unmapped refusal {other}"),
    }
}

fn assert_cli_refusal(run: &Captured, output: &Path, core_code: &str) {
    assert_eq!(
        run.status,
        Some(authority_exit(core_code)),
        "stderr {}\nstdout {}",
        run.stderr,
        run.stdout
    );
    assert_ne!(run.status, Some(0));
    let document = read_envelope(output, run);
    assert_eq!(document["outcome"], "failed", "{document}");
    assert!(document["data"].is_null(), "{document}");
    assert_eq!(document["error"]["code"], core_code, "{document}");
}

fn flip_hex_tail(value: &str) -> String {
    let mut chars: Vec<char> = value.chars().collect();
    let last = chars.last_mut().expect("identity hash");
    *last = if *last == 'a' { 'b' } else { 'a' };
    chars.into_iter().collect()
}

fn item_identity(item: &serde_json::Value) -> (String, String, String, String) {
    (
        item["id"].as_str().unwrap_or("").to_string(),
        item["path"].as_str().unwrap_or("").to_string(),
        item["origin"].as_str().unwrap_or("").to_string(),
        item["certainty"].as_str().unwrap_or("").to_string(),
    )
}

#[test]
fn startup_preview_matches_the_core_service() {
    let _lock = lock_env();
    let temp = TempDir::new().expect("startup appdata");
    let _env = EnvGuard::set(&[("APPDATA", temp.path())]);
    let startup = temp
        .path()
        .join(r"Microsoft\Windows\Start Menu\Programs\Startup");
    fs::create_dir_all(&startup).expect("startup folder");
    fs::write(startup.join("DevSweepC01Isolated.cmd"), b"rem isolated\r\n").expect("startup file");

    let list = list_startup_entries();
    let entry = list
        .entries
        .iter()
        .find(|entry| {
            entry.name == "DevSweepC01Isolated.cmd"
                && entry.location == SoftwareStartupLocation::CurrentUserStartupFolder
        })
        .unwrap_or_else(|| panic!("isolated startup entry was not observed"));
    let plan = plan_startup_toggle(&entry.id, false).unwrap_or_else(|error| {
        panic!("startup plan {} {error}", error.code());
    });
    let plan_path = temp.path().join("startup-plan.json");
    let encoded = serde_json::to_vec(&plan).expect("startup plan json");
    fs::write(&plan_path, &encoded).expect("write startup plan");
    let decoded: devsweep_core::software::StartupTogglePlanV1 =
        serde_json::from_slice(&encoded).expect("decode startup plan");
    let core = preview_startup_toggle(&decoded).unwrap_or_else(|error| {
        panic!("startup preview {} {error}", error.code());
    });
    let core_value = serde_json::to_value(&core).expect("startup preview json");

    let output = temp.path().join("startup-preview.json");
    let run = run_devsweep(
        &[
            "software".to_string(),
            "startup".to_string(),
            "preview".to_string(),
            "--plan".to_string(),
            path_arg(&plan_path),
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_arg(&output),
        ],
        &[("APPDATA", temp.path())],
        Stdio::piped(),
    );
    let data = success_data(&output, &run);
    assert_eq!(data["entry"]["id"], core_value["entry"]["id"]);
    assert_eq!(data["digest"], core_value["digest"]);
    assert!(
        data["digest"]
            .as_str()
            .is_some_and(|digest| digest.starts_with("sha256:") && digest.len() == 71)
    );
    assert!(data["error"].is_null() && core_value.get("error").is_none());
    assert_eq!(data, core_value);
}

#[test]
fn leftover_preview_matches_the_core_service() {
    let _lock = lock_env();
    let temp = TempDir::new().expect("leftover roots");
    let _env = EnvGuard::set(&[
        ("APPDATA", temp.path()),
        ("LOCALAPPDATA", temp.path()),
        ("ProgramData", temp.path()),
        ("USERPROFILE", temp.path()),
    ]);
    let candidate = temp.path().join("ContosoTools");
    fs::create_dir_all(&candidate).expect("candidate");
    fs::write(candidate.join("note.txt"), b"isolated").expect("candidate file");
    let identity = SoftwareIdentity::Msix {
        package_full_name: "Contoso.Tools_1.0.0.0_x64__8wekyb3d8bbwe".to_string(),
    };
    let id = software_id(&identity);
    let inventory = SoftwareInventoryV1 {
        version: 1,
        observed_at_unix_ms: 1,
        sources: Vec::new(),
        entries: vec![SoftwareEntryV1 {
            id: id.clone(),
            identity: identity.clone(),
            scope: SoftwareScope::CurrentUser,
            display_name: Some("ContosoTools".to_string()),
            publisher: None,
            version: None,
            provenance: Vec::new(),
            eligibility: SoftwareEligibility {
                state: SoftwareEligibilityState::Selectable,
                reason: SoftwareEligibilityReason::EligibleCurrentUserMsix,
            },
            size: SoftwareSizeEvidence::Unknown {
                reason_code: "measurement_unavailable".to_string(),
            },
            last_used: SoftwareLastUsedEvidence::Unknown {
                reason_code: SoftwareLastUsedReason::NoSupportedExactSource,
            },
            launch: None,
        }],
        fingerprint: format!("sha256:{}", "0".repeat(64)),
    };
    let journal = temp
        .path()
        .join("DevSweep")
        .join("audit")
        .join("v1")
        .join("software.jsonl");
    fs::create_dir_all(journal.parent().expect("journal parent")).expect("journal dir");
    fs::write(
        &journal,
        succeeded_uninstall_journal(&identity, "software-op-isolated"),
    )
    .expect("journal");
    let discovered = discover_software_leftovers(&inventory, std::slice::from_ref(&id), None)
        .unwrap_or_else(|error| panic!("discover {} {error}", error.code()));
    let candidates = &discovered
        .apps
        .iter()
        .find(|app| app.software_id == id)
        .expect("leftover app")
        .candidates;
    assert_eq!(candidates.len(), 1, "{candidates:?}");
    let plan = SoftwareLeftoverPlanV1 {
        version: 1,
        software_id: id,
        identity,
        display_name: Some("ContosoTools".to_string()),
        publisher: None,
        uninstall_operation_id: "software-op-isolated".to_string(),
        selected_candidate_ids: vec![candidates[0].id.clone()],
    };
    let plan_path = temp.path().join("leftover-plan.json");
    let inventory_path = temp.path().join("inventory.json");
    fs::write(&plan_path, serde_json::to_vec(&plan).expect("plan json")).expect("write plan");
    fs::write(
        &inventory_path,
        serde_json::to_vec(&inventory).expect("inventory json"),
    )
    .expect("write inventory");
    let core = preview_software_leftover_plan(&inventory, &plan, None)
        .unwrap_or_else(|error| panic!("leftover preview {} {error}", error.code()));
    let core_value = serde_json::to_value(&core).expect("leftover preview json");

    let output = temp.path().join("leftover-preview.json");
    let run = run_devsweep(
        &[
            "software".to_string(),
            "leftovers".to_string(),
            "preview".to_string(),
            "--inventory".to_string(),
            path_arg(&inventory_path),
            "--plan".to_string(),
            path_arg(&plan_path),
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_arg(&output),
        ],
        &[
            ("APPDATA", temp.path()),
            ("LOCALAPPDATA", temp.path()),
            ("ProgramData", temp.path()),
        ],
        Stdio::piped(),
    );
    let data = success_data(&output, &run);
    let core_items = core_value["items"].as_array().expect("core items");
    let cli_items = data["items"].as_array().expect("cli items");
    assert_eq!(
        cli_items.iter().map(item_identity).collect::<Vec<_>>(),
        core_items.iter().map(item_identity).collect::<Vec<_>>()
    );
    assert_eq!(data["digest"], core_value["digest"]);
    assert_eq!(data["v1_digest"], core_value["v1_digest"]);
    assert!(
        data["digest"]
            .as_str()
            .is_some_and(|digest| digest.starts_with("sha256:") && digest.len() == 71)
    );
    assert!(data["refused"].is_null() && core_value["refused"].is_null());
    assert_eq!(data["label_trust"], core_value["label_trust"]);
    assert_eq!(data["error"].as_str(), core_value["error"].as_str());
}

#[test]
fn analyze_trash_preview_matches_the_core_service() {
    let _lock = lock_env();
    let fixture = analyze_fixture();
    let encoded = fs::read(&fixture.plan_path).expect("plan bytes");
    let plan = decode_analyze_trash_plan(&encoded).expect("decode plan");
    let core = preview_analyze_trash_plan(&plan).unwrap_or_else(|error| {
        panic!("core preview {} {error}", error.code());
    });
    let core_value = serde_json::to_value(&core).expect("preview json");
    let output = fixture._temp.path().join("trash-preview.json");
    let run = run_devsweep(
        &[
            "analyze".to_string(),
            "trash-preview".to_string(),
            "--plan".to_string(),
            path_arg(&fixture.plan_path),
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_arg(&output),
        ],
        &[],
        Stdio::piped(),
    );
    let data = success_data(&output, &run);
    assert_eq!(data["digest"], core_value["digest"]);
    assert_eq!(data["operation_id"], core_value["operation_id"]);
    assert_eq!(data["items"], core_value["items"]);
    assert_eq!(data["refused"], core_value["refused"]);
    assert!(
        data["digest"]
            .as_str()
            .is_some_and(|digest| digest.starts_with("sha256:") && digest.len() == 71)
    );
    assert_eq!(data, core_value);
    assert_eq!(fs::read(&fixture.file).expect("fixture"), fixture.bytes);
}

#[test]
fn analyze_trash_execute_without_confirm_leaves_the_fixture() {
    let _lock = lock_env();
    let fixture = analyze_fixture();
    let journal = real_clean_journal();
    let before_journal = mark(&journal);
    let before_file = fs::read(&fixture.file).expect("fixture bytes");
    let run = run_devsweep(
        &[
            "analyze".to_string(),
            "trash-execute".to_string(),
            "--plan".to_string(),
            path_arg(&fixture.plan_path),
            "--preview-digest".to_string(),
            fixture.digest.clone(),
        ],
        &[],
        Stdio::piped(),
    );
    assert_eq!(run.status, Some(2), "stderr {}", run.stderr);
    assert!(
        run.stderr.contains("--confirm") || run.stderr.contains("confirm"),
        "stderr {}",
        run.stderr
    );
    assert_eq!(
        fs::read(&fixture.file).expect("fixture remains"),
        before_file
    );
    assert_unchanged(&journal, &before_journal);
}

#[test]
fn analyze_trash_execute_closed_stdout_does_not_move_the_fixture() {
    let _lock = lock_env();
    let fixture = analyze_fixture();
    let isolated = TempDir::new().expect("isolated local app data");
    let journal = real_clean_journal();
    let before_journal = mark(&journal);
    let isolated_journal = isolated
        .path()
        .join("DevSweep")
        .join("audit")
        .join("v1")
        .join("clean.jsonl");
    let (reader, writer) = std::io::pipe().expect("stdout pipe");
    drop(reader);
    let run = run_devsweep(
        &[
            "analyze".to_string(),
            "trash-execute".to_string(),
            "--plan".to_string(),
            path_arg(&fixture.plan_path),
            "--preview-digest".to_string(),
            fixture.digest,
            "--confirm".to_string(),
        ],
        &[("LOCALAPPDATA", isolated.path())],
        Stdio::from(writer),
    );
    assert_eq!(
        run.status,
        Some(0),
        "stderr {}\nstdout {}",
        run.stderr,
        run.stdout
    );
    assert!(run.stderr.contains("broken_pipe"), "stderr {}", run.stderr);
    assert_eq!(
        fs::read(&fixture.file).expect("fixture remains"),
        fixture.bytes
    );
    assert!(
        !isolated_journal.exists(),
        "isolated clean journal was created"
    );
    assert_unchanged(&journal, &before_journal);
}

#[test]
fn startup_preview_refusal_matches_the_core_service() {
    let _lock = lock_env();
    let temp = TempDir::new().expect("startup appdata");
    let _env = EnvGuard::set(&[("APPDATA", temp.path())]);
    let startup = temp
        .path()
        .join(r"Microsoft\Windows\Start Menu\Programs\Startup");
    fs::create_dir_all(&startup).expect("startup folder");
    let entry_path = startup.join("DevSweepC01Stale.cmd");
    let entry_bytes = b"rem stale\r\n".to_vec();
    fs::write(&entry_path, &entry_bytes).expect("startup file");

    let list = list_startup_entries();
    let entry = list
        .entries
        .iter()
        .find(|entry| {
            entry.name == "DevSweepC01Stale.cmd"
                && entry.location == SoftwareStartupLocation::CurrentUserStartupFolder
        })
        .expect("stale startup entry was not observed");
    let mut plan = plan_startup_toggle(&entry.id, false).unwrap_or_else(|error| {
        panic!("startup plan {} {error}", error.code());
    });
    plan.expected_entry_identity_sha256 = flip_hex_tail(&plan.expected_entry_identity_sha256);
    let plan_path = temp.path().join("startup-plan.json");
    let encoded = serde_json::to_vec(&plan).expect("startup plan json");
    fs::write(&plan_path, &encoded).expect("write startup plan");
    let decoded: devsweep_core::software::StartupTogglePlanV1 =
        serde_json::from_slice(&encoded).expect("decode startup plan");
    let core = preview_startup_toggle(&decoded).expect_err("stale startup preview");
    assert_eq!(core.code(), "startup_entry_changed");

    let output = temp.path().join("startup-preview.json");
    let run = run_devsweep(
        &[
            "software".to_string(),
            "startup".to_string(),
            "preview".to_string(),
            "--plan".to_string(),
            path_arg(&plan_path),
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_arg(&output),
        ],
        &[("APPDATA", temp.path())],
        Stdio::piped(),
    );
    assert_cli_refusal(&run, &output, core.code());
    assert_eq!(
        fs::read(&entry_path).expect("startup file remains"),
        entry_bytes
    );
    assert_eq!(fs::read(&plan_path).expect("plan remains"), encoded);
}

#[test]
fn leftover_preview_refusal_matches_the_core_service() {
    let _lock = lock_env();
    let temp = TempDir::new().expect("leftover roots");
    let _env = EnvGuard::set(&[
        ("APPDATA", temp.path()),
        ("LOCALAPPDATA", temp.path()),
        ("ProgramData", temp.path()),
        ("USERPROFILE", temp.path()),
    ]);
    let candidate = temp.path().join("ContosoTools");
    fs::create_dir_all(&candidate).expect("candidate");
    let note = candidate.join("note.txt");
    let note_bytes = b"isolated".to_vec();
    fs::write(&note, &note_bytes).expect("candidate file");
    let identity = SoftwareIdentity::Msix {
        package_full_name: "Contoso.Tools_1.0.0.0_x64__8wekyb3d8bbwe".to_string(),
    };
    let id = software_id(&identity);
    let inventory = SoftwareInventoryV1 {
        version: 1,
        observed_at_unix_ms: 1,
        sources: Vec::new(),
        entries: vec![SoftwareEntryV1 {
            id: id.clone(),
            identity: identity.clone(),
            scope: SoftwareScope::CurrentUser,
            display_name: Some("ContosoTools".to_string()),
            publisher: None,
            version: None,
            provenance: Vec::new(),
            eligibility: SoftwareEligibility {
                state: SoftwareEligibilityState::Selectable,
                reason: SoftwareEligibilityReason::EligibleCurrentUserMsix,
            },
            size: SoftwareSizeEvidence::Unknown {
                reason_code: "measurement_unavailable".to_string(),
            },
            last_used: SoftwareLastUsedEvidence::Unknown {
                reason_code: SoftwareLastUsedReason::NoSupportedExactSource,
            },
            launch: None,
        }],
        fingerprint: format!("sha256:{}", "0".repeat(64)),
    };
    let journal = temp
        .path()
        .join("DevSweep")
        .join("audit")
        .join("v1")
        .join("software.jsonl");
    fs::create_dir_all(journal.parent().expect("journal parent")).expect("journal dir");
    let journal_bytes = succeeded_uninstall_journal(&identity, "software-op-isolated");
    fs::write(&journal, &journal_bytes).expect("journal");
    let discovered = discover_software_leftovers(&inventory, std::slice::from_ref(&id), None)
        .unwrap_or_else(|error| panic!("discover {} {error}", error.code()));
    let live_id = discovered
        .apps
        .iter()
        .find(|app| app.software_id == id)
        .expect("leftover app")
        .candidates
        .first()
        .expect("candidate")
        .id
        .clone();
    let plan = SoftwareLeftoverPlanV1 {
        version: 1,
        software_id: id,
        identity,
        display_name: Some("ContosoTools".to_string()),
        publisher: None,
        uninstall_operation_id: "software-op-isolated".to_string(),
        selected_candidate_ids: vec![format!("{live_id}-stale")],
    };
    let plan_path = temp.path().join("leftover-plan.json");
    let inventory_path = temp.path().join("inventory.json");
    let plan_bytes = serde_json::to_vec(&plan).expect("plan json");
    let inventory_bytes = serde_json::to_vec(&inventory).expect("inventory json");
    fs::write(&plan_path, &plan_bytes).expect("write plan");
    fs::write(&inventory_path, &inventory_bytes).expect("write inventory");
    let decoded_inventory: SoftwareInventoryV1 =
        serde_json::from_slice(&inventory_bytes).expect("decode inventory");
    let decoded_plan: SoftwareLeftoverPlanV1 =
        serde_json::from_slice(&plan_bytes).expect("decode plan");
    let core = preview_software_leftover_plan(&decoded_inventory, &decoded_plan, None)
        .expect_err("stale leftover preview");
    assert_eq!(core.code(), "stale_candidate");

    let output = temp.path().join("leftover-preview.json");
    let run = run_devsweep(
        &[
            "software".to_string(),
            "leftovers".to_string(),
            "preview".to_string(),
            "--inventory".to_string(),
            path_arg(&inventory_path),
            "--plan".to_string(),
            path_arg(&plan_path),
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_arg(&output),
        ],
        &[
            ("APPDATA", temp.path()),
            ("LOCALAPPDATA", temp.path()),
            ("ProgramData", temp.path()),
        ],
        Stdio::piped(),
    );
    assert_cli_refusal(&run, &output, core.code());
    assert_eq!(fs::read(&note).expect("note remains"), note_bytes);
    assert_eq!(
        fs::read(&journal).expect("journal remains"),
        journal_bytes.as_bytes()
    );
    assert_eq!(fs::read(&plan_path).expect("plan remains"), plan_bytes);
    assert_eq!(
        fs::read(&inventory_path).expect("inventory remains"),
        inventory_bytes
    );
}

#[test]
fn analyze_trash_preview_refusal_matches_the_core_service() {
    let _lock = lock_env();
    let fixture = analyze_fixture();
    let stale_bytes = b"stale-keep".to_vec();
    fs::write(&fixture.file, &stale_bytes).expect("replace keep.bin");
    let plan_bytes = fs::read(&fixture.plan_path).expect("plan bytes");
    let plan = decode_analyze_trash_plan(&plan_bytes).expect("decode plan");
    let core = preview_analyze_trash_plan(&plan).expect_err("stale analyze preview");
    assert_eq!(core.code(), "changed_since_snapshot");

    let output = fixture._temp.path().join("trash-preview.json");
    let run = run_devsweep(
        &[
            "analyze".to_string(),
            "trash-preview".to_string(),
            "--plan".to_string(),
            path_arg(&fixture.plan_path),
            "--format".to_string(),
            "json".to_string(),
            "--output".to_string(),
            path_arg(&output),
        ],
        &[],
        Stdio::piped(),
    );
    assert_cli_refusal(&run, &output, core.code());
    assert_eq!(
        fs::read(&fixture.file).expect("fixture remains"),
        stale_bytes
    );
    assert_eq!(
        fs::read(&fixture.plan_path).expect("plan remains"),
        plan_bytes
    );
}
