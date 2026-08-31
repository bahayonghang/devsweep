use std::{
    fs,
    io::Read,
    path::Path,
    process::{Command, Output, Stdio},
};

use tempfile::TempDir;

fn devsweep(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("devsweep process runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn envelope(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("JSON envelope")
}

fn digest_placeholder() -> String {
    format!("sha256:{}", "a".repeat(64))
}

#[test]
fn shipped_modes_are_reachable_and_removed_roots_stay_unknown() {
    for (args, command) in [
        (
            ["status", "snapshot", "--format", "json"].as_slice(),
            "status.snapshot",
        ),
        (
            ["optimize", "list", "--format", "json"].as_slice(),
            "optimize.list",
        ),
        (
            ["history", "list", "--format", "json"].as_slice(),
            "history.list",
        ),
    ] {
        let output = devsweep(args);
        let code = output.status.code();
        assert!(
            code == Some(0) || code == Some(5),
            "{command} exit {code:?} stderr={}",
            stderr(&output)
        );
        let document = envelope(&output);
        assert_eq!(document["schema_version"], 1);
        assert_eq!(document["command"], command);
        assert!(document["error"].is_null());
    }

    let directory = TempDir::new().unwrap();
    let root = directory.path().join("analyze-root");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.bin"), [1, 2, 3, 4]).unwrap();
    let analyze = devsweep(&[
        "analyze",
        "scan",
        "--root",
        root.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(
        analyze.status.code() == Some(0) || analyze.status.code() == Some(5),
        "analyze.scan {}",
        stderr(&analyze)
    );
    assert_eq!(envelope(&analyze)["command"], "analyze.scan");
    assert!(envelope(&analyze)["data"].get("action").is_none());
    assert!(envelope(&analyze)["data"].get("intent").is_none());

    for root in ["tui", "scan", "inventory", "protect", "rules"] {
        assert_eq!(devsweep(&[root]).status.code(), Some(2), "removed {root}");
    }
}

#[test]
fn locale_switch_localizes_human_copy_and_leaves_machine_documents_invariant() {
    let english = devsweep(&["optimize", "list"]);
    let chinese = devsweep(&["--language", "zh-CN", "optimize", "list"]);
    assert_eq!(english.status.code(), Some(0));
    assert_eq!(chinese.status.code(), Some(0));
    let english_text = stdout(&english);
    let chinese_text = stdout(&chinese);
    assert_ne!(english_text, chinese_text);
    assert!(english_text.contains("dns.flush"));
    assert!(chinese_text.contains("dns.flush"));
    assert!(english_text.contains("Runs here") || english_text.contains("dns.flush"));
    assert!(chinese_text.contains("在此运行") || chinese_text.contains("dns.flush"));

    let machine = devsweep(&["optimize", "list", "--format", "json"]);
    let rejected = devsweep(&[
        "--language",
        "zh-CN",
        "optimize",
        "list",
        "--format",
        "json",
    ]);
    assert_eq!(machine.status.code(), Some(0));
    assert_eq!(rejected.status.code(), Some(2));
    let document = envelope(&machine);
    assert_eq!(document["command"], "optimize.list");
    assert_eq!(document["data"]["entries"].as_array().unwrap().len(), 8);
    let rejected_document = envelope(&rejected);
    assert_eq!(rejected_document["error"]["code"], "invalid_cli");
}

#[test]
fn preview_digest_mismatch_is_stale_authority_before_side_effects() {
    let directory = TempDir::new().unwrap();
    let plan = directory.path().join("empty-clean.json");
    fs::write(&plan, br#"{"version":2,"targets":[]}"#).unwrap();
    let preview = devsweep(&[
        "clean",
        "preview",
        "--plan",
        plan.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(preview.status.code(), Some(0), "{}", stderr(&preview));
    let live_digest = envelope(&preview)["data"]["digest"]
        .as_str()
        .expect("clean preview digest")
        .to_string();
    assert!(live_digest.starts_with("sha256:"));
    let stale = devsweep(&[
        "clean",
        "execute",
        "--plan",
        plan.to_str().unwrap(),
        "--preview-digest",
        &digest_placeholder(),
        "--confirm",
        "--format",
        "json",
    ]);
    assert_eq!(stale.status.code(), Some(3), "{}", stderr(&stale));
    let stale_document = envelope(&stale);
    assert_eq!(stale_document["error"]["code"], "stale_confirmation");
    assert_eq!(fs::read(&plan).unwrap(), br#"{"version":2,"targets":[]}"#);
    assert_ne!(live_digest, digest_placeholder());

    let optimize_plan = directory.path().join("dns.json");
    let planned = devsweep(&[
        "optimize",
        "plan",
        "--operation",
        "dns.flush",
        "--output",
        optimize_plan.to_str().unwrap(),
    ]);
    assert_eq!(planned.status.code(), Some(0), "{}", stderr(&planned));
    let optimize_preview = devsweep(&[
        "optimize",
        "preview",
        "--plan",
        optimize_plan.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(
        optimize_preview.status.code(),
        Some(0),
        "{}",
        stderr(&optimize_preview)
    );
    let optimize_preview_json = envelope(&optimize_preview);
    let optimize_digest = optimize_preview_json["data"]["digest"]
        .as_str()
        .expect("optimize digest")
        .to_string();
    let optimize_stale = devsweep(&[
        "optimize",
        "run",
        "--plan",
        optimize_plan.to_str().unwrap(),
        "--preview-digest",
        &digest_placeholder(),
        "--confirm",
        "--format",
        "json",
    ]);
    assert_eq!(optimize_stale.status.code(), Some(3));
    let optimize_error = envelope(&optimize_stale);
    assert_eq!(
        optimize_error["error"]["code"],
        "invalid_optimize_authority"
    );
    assert_ne!(optimize_digest, digest_placeholder());

    let software_stale = devsweep(&[
        "software",
        "uninstall",
        "--plan",
        plan.to_str().unwrap(),
        "--preview-digest",
        &digest_placeholder(),
        "--confirm",
        "--format",
        "json",
    ]);
    let software_code = software_stale.status.code();
    assert!(
        software_code == Some(3) || software_code == Some(6),
        "software uninstall hostile plan {software_code:?}"
    );
    if software_code == Some(3) {
        assert_eq!(
            envelope(&software_stale)["error"]["code"],
            "invalid_software_authority"
        );
    }
}

#[test]
fn cancellation_joins_status_live_and_leaves_no_producer_after_broken_pipe() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(["status", "live", "--format", "ndjson", "--interval", "1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("status live starts");
    let mut stdout = child.stdout.take().expect("live stdout");
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 8192];
    while buffer.len() < 64 {
        let read = stdout.read(&mut chunk).expect("read live");
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    drop(stdout);
    let status = child.wait().expect("live joins after pipe close");
    assert_eq!(status.code(), Some(0));
    let text = String::from_utf8(buffer).expect("ndjson utf-8");
    let mut sequences = Vec::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let event: serde_json::Value = serde_json::from_str(line).expect("status event");
        assert_eq!(event["schema_version"], 1);
        sequences.push(event["sequence"].as_u64().expect("sequence"));
        let name = event["event"].as_str().expect("event name");
        assert!(
            matches!(
                name,
                "status_started" | "status_snapshot" | "tick_skipped" | "status_terminal"
            ),
            "unexpected event {name}"
        );
    }
    assert!(
        sequences.windows(2).all(|pair| pair[1] > pair[0]),
        "stale or non-monotonic sequences {sequences:?}"
    );
}

#[test]
fn partial_and_unavailable_states_stay_truthful() {
    let snapshot = devsweep(&["status", "snapshot", "--format", "json"]);
    let code = snapshot.status.code();
    assert!(code == Some(0) || code == Some(5), "snapshot {code:?}");
    let document = envelope(&snapshot);
    let outcome = document["outcome"].as_str().expect("outcome");
    if code == Some(5) {
        assert_eq!(outcome, "partial");
    } else {
        assert_eq!(outcome, "succeeded");
    }
    let unsupported = document["data"]["unsupported_capabilities"]
        .as_array()
        .expect("unsupported capabilities");
    assert_eq!(unsupported.len(), 6);

    let hostile = devsweep(&[
        "optimize",
        "plan",
        "--operation",
        "firewall.reset",
        "--output",
        "unused-hostile.json",
    ]);
    assert_eq!(hostile.status.code(), Some(6));
    assert!(!Path::new("unused-hostile.json").exists());

    let guidance = TempDir::new().unwrap();
    let guidance_out = guidance.path().join("guidance.json");
    let guidance_plan = devsweep(&[
        "optimize",
        "plan",
        "--operation",
        "guidance.drive_optimize",
        "--output",
        guidance_out.to_str().unwrap(),
    ]);
    assert_eq!(guidance_plan.status.code(), Some(6));
    assert!(!guidance_out.exists());
}

#[test]
fn mixed_audit_versions_are_preserved_and_never_read_as_v1() {
    let isolated = TempDir::new().unwrap();
    let journal_dir = isolated.path().join("DevSweep/audit/v1");
    fs::create_dir_all(&journal_dir).unwrap();
    let clean = journal_dir.join("clean.jsonl");
    let v1 = include_str!(
        "../../devsweep-core/tests/fixtures/history/clean-v1/protection-mutation.jsonl"
    );
    let unknown =
        include_str!("../../devsweep-core/tests/fixtures/history/clean-v1/unknown-version.jsonl");
    let mixed = format!("{v1}{unknown}");
    fs::write(&clean, &mixed).unwrap();
    let presentation = isolated
        .path()
        .join("DevSweep/settings/presentation-v1.json");
    fs::create_dir_all(presentation.parent().unwrap()).unwrap();
    let newer = br#"{"schema_version":2,"language":"en"}"#;
    fs::write(&presentation, newer).unwrap();

    let listed = Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(["history", "list", "--format", "json"])
        .env("LOCALAPPDATA", isolated.path())
        .env("APPDATA", isolated.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("history list");
    assert_eq!(listed.status.code(), Some(0), "{}", stderr(&listed));
    let document = envelope(&listed);
    assert_eq!(document["command"], "history.list");
    let encoded = document.to_string();
    assert!(encoded.contains("op-protect-v1-fixture"));
    assert!(encoded.contains("unknown_schema") || encoded.contains("unsupported"));
    assert!(!encoded.contains("preserved-unknown-version"));
    assert_eq!(fs::read(&clean).unwrap(), mixed.as_bytes());
    assert_eq!(fs::read(&presentation).unwrap(), newer);

    let restarted = Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(["history", "list", "--format", "json"])
        .env("LOCALAPPDATA", isolated.path())
        .env("APPDATA", isolated.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("history restart");
    assert_eq!(restarted.status.code(), Some(0));
    assert_eq!(envelope(&restarted)["data"], document["data"]);
    assert_eq!(fs::read(&presentation).unwrap(), newer);
}

#[test]
fn coordinator_contract_keeps_one_mutating_command_and_separate_digests() {
    let missing_confirm = devsweep(&[
        "optimize",
        "run",
        "--plan",
        "missing.json",
        "--preview-digest",
        &digest_placeholder(),
    ]);
    assert_eq!(missing_confirm.status.code(), Some(2));

    let missing_software = devsweep(&[
        "software",
        "uninstall",
        "--plan",
        "missing.json",
        "--preview-digest",
        &digest_placeholder(),
    ]);
    assert_eq!(missing_software.status.code(), Some(2));

    let missing_clean = devsweep(&[
        "clean",
        "execute",
        "--plan",
        "missing.json",
        "--preview-digest",
        &digest_placeholder(),
    ]);
    assert_eq!(missing_clean.status.code(), Some(2));

    let directory = TempDir::new().unwrap();
    let clean_plan = directory.path().join("empty-clean.json");
    fs::write(&clean_plan, br#"{"version":2,"targets":[]}"#).unwrap();
    let clean_preview = envelope(&devsweep(&[
        "clean",
        "preview",
        "--plan",
        clean_plan.to_str().unwrap(),
        "--format",
        "json",
    ]));
    let optimize_plan = directory.path().join("dns.json");
    assert_eq!(
        devsweep(&[
            "optimize",
            "plan",
            "--operation",
            "dns.flush",
            "--output",
            optimize_plan.to_str().unwrap(),
        ])
        .status
        .code(),
        Some(0)
    );
    let optimize_preview = envelope(&devsweep(&[
        "optimize",
        "preview",
        "--plan",
        optimize_plan.to_str().unwrap(),
        "--format",
        "json",
    ]));
    assert_eq!(clean_preview["command"], "clean.preview");
    assert_eq!(optimize_preview["command"], "optimize.preview");
    assert_ne!(
        clean_preview["data"]["digest"],
        optimize_preview["data"]["digest"]
    );
}
