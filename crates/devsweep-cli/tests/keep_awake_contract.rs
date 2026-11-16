//! Cross-process CLI queries against a fake, in-process core owner. No power API.

use std::process::{Command, Output};
use std::sync::Arc;
use std::time::Duration;

use devsweep_core::status::{FakeClock, KeepAwakeService, KeepAwakeSession, RecordingBackend};

fn cli(root: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(args)
        .env("LOCALAPPDATA", root)
        .env("APPDATA", root)
        .output()
        .expect("isolated read-only CLI")
}

#[test]
fn cli_queries_and_stops_a_shared_gui_tuple_and_reports_closed_endpoint_unavailable() {
    let isolated = tempfile::TempDir::new().expect("isolated CLI home");
    let (backend, _) = RecordingBackend::pair();
    let service = KeepAwakeService::new(Arc::new(FakeClock::new()), Box::new(backend), true);
    let mut session = KeepAwakeSession::open(service, "fake-gui", Duration::from_secs(30), true)
        .expect("fake owner");
    let endpoint = session.endpoint().to_string();
    let token = session.token().to_string();
    let query = |verb| {
        cli(
            isolated.path(),
            &[
                "status",
                "keep-awake",
                verb,
                "--endpoint",
                &endpoint,
                "--token",
                &token,
                "--owner",
                "fake-gui",
                "--format",
                "json",
            ],
        )
    };
    let active = query("status");
    assert!(
        active.status.success(),
        "{}",
        String::from_utf8_lossy(&active.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&active.stdout).expect("active JSON");
    assert_eq!(document["data"]["lease"]["state"], "active");
    assert_eq!(document["data"]["endpoint"], endpoint);
    let stopped = query("stop");
    assert!(
        stopped.status.success(),
        "{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&stopped.stdout).expect("stop JSON");
    assert_eq!(document["data"]["lease"]["state"], "stopped");
    let retained = query("status");
    assert!(retained.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&retained.stdout).expect("retained JSON")["data"]
            ["lease"]["state"],
        "stopped"
    );
    session.shutdown().expect("clear and join");
    let closed = query("status");
    assert_eq!(closed.status.code(), Some(4));
    let document: serde_json::Value = serde_json::from_slice(&closed.stdout).expect("unknown JSON");
    assert_eq!(document["error"]["code"], "network_unavailable");
    assert!(!String::from_utf8_lossy(&closed.stdout).contains("\"stopped\""));
    for (locale, expected) in [("en", "its state is unknown"), ("zh-CN", "会话状态未知")] {
        let unavailable = cli(
            isolated.path(),
            &[
                "--language",
                locale,
                "status",
                "keep-awake",
                "status",
                "--endpoint",
                &endpoint,
                "--token",
                &token,
                "--owner",
                "fake-gui",
            ],
        );
        assert_eq!(unavailable.status.code(), Some(4));
        assert!(unavailable.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&unavailable.stderr);
        assert!(stderr.contains("network_unavailable"));
        assert!(stderr.contains(expected), "{locale}: {stderr}");
    }
}

#[test]
fn cli_reads_redacted_status_action_history_and_both_human_locales() {
    let isolated = tempfile::TempDir::new().expect("isolated CLI history");
    let audit = isolated.path().join("DevSweep/audit/v1/status.jsonl");
    std::fs::create_dir_all(audit.parent().expect("parent")).expect("directory");
    std::fs::write(
        &audit,
        format!(
            "{}\n",
            serde_json::json!({
                "schema_version":1, "domain":"status", "operation_id":"awake-cli-fixture",
                "timestamp_unix_ms": 1, "action":"start", "status_code":"dispatch_started",
                "duration_ms": 1000, "preview_digest": format!("sha256:{}", "a".repeat(64))
            })
        ),
    )
    .expect("fixture journal");
    let output = cli(
        isolated.path(),
        &["history", "list", "--domain", "status", "--format", "json"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).expect("history JSON");
    assert_eq!(document["data"]["operations"][0]["domain"], "status");
    assert_eq!(
        document["data"]["operations"][0]["outcome_code"],
        "unknown_after_dispatch"
    );
    assert_eq!(document["data"]["operations"][0]["unknown"], true);
    for locale in ["en", "zh-CN"] {
        let human = cli(
            isolated.path(),
            &[
                "--language",
                locale,
                "history",
                "show",
                "--operation-id",
                "awake-cli-fixture",
            ],
        );
        assert!(
            human.status.success(),
            "{}",
            String::from_utf8_lossy(&human.stderr)
        );
        assert!(String::from_utf8_lossy(&human.stdout).contains("awake-cli-fixture"));
    }
    assert!(
        !audit.with_file_name("status.lock").exists(),
        "read-only history must not create locks"
    );
}
