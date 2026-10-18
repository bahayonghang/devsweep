//! C07 read-only analyze commands compared with the same core calls.
//!
//! Fixtures stay under a temporary directory. These tests do not analyze or
//! trash the user profile.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use devsweep_core::analysis::{
    AnalyzeSnapshotV1, analyze_path, explain_analyze_node, list_analyze_children,
    list_analyze_roots,
};
use tempfile::TempDir;

struct Captured {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_devsweep(args: &[String]) -> Captured {
    let output = Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("devsweep process runs");
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

fn data_of(path: &Path, run: &Captured) -> serde_json::Value {
    let document = read_envelope(path, run);
    assert!(document["error"].is_null(), "{document}");
    assert!(!document.to_string().contains("CleanupPlan"), "{document}");
    assert!(document.get("intent").is_none(), "{document}");
    document["data"].clone()
}

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    missing: PathBuf,
    snapshot_path: PathBuf,
    snapshot: AnalyzeSnapshotV1,
}

fn fixture() -> Fixture {
    let temp = TempDir::new().expect("temp");
    let root = temp.path().join("root");
    fs::create_dir_all(root.join("wsl")).expect("wsl");
    fs::create_dir_all(root.join("models")).expect("models");
    fs::write(root.join("notes.bin"), vec![7_u8; 64]).expect("notes");
    fs::write(root.join("wsl").join("ext4.vhdx"), [1_u8]).expect("wsl disk");
    fs::write(root.join("models").join("demo.gguf"), [2_u8]).expect("model");
    fs::write(root.join("hiberfil.sys"), [3_u8]).expect("hibernation");
    fs::write(root.join("pagefile.sys"), [4_u8]).expect("page file");
    fs::write(root.join("a.bin"), [5_u8]).expect("a");
    let outcome = analyze_path(&root, None, None).expect("analyze temp tree");
    let snapshot = outcome.snapshot().clone();
    let snapshot_path = temp.path().join("snapshot.json");
    fs::write(
        &snapshot_path,
        serde_json::to_vec(&snapshot).expect("snapshot json"),
    )
    .expect("write snapshot");
    Fixture {
        missing: temp.path().join("missing-root"),
        _temp: temp,
        root,
        snapshot_path,
        snapshot,
    }
}

fn node_id(snapshot: &AnalyzeSnapshotV1, name: &str) -> u32 {
    snapshot
        .nodes
        .iter()
        .find(|node| node.name == name)
        .unwrap_or_else(|| panic!("missing node {name}"))
        .id
}

#[test]
fn roots_list_and_explain_match_the_core_service() {
    let fixture = fixture();
    let out_dir = fixture
        .snapshot_path
        .parent()
        .expect("parent")
        .to_path_buf();

    let roots_out = out_dir.join("roots.json");
    let roots_run = run_devsweep(&[
        "analyze".into(),
        "roots".into(),
        "--root".into(),
        path_arg(&fixture.root),
        "--format".into(),
        "json".into(),
        "--output".into(),
        path_arg(&roots_out),
    ]);
    assert_eq!(roots_run.status, Some(0), "{}", roots_run.stderr);
    let service_roots =
        list_analyze_roots(std::slice::from_ref(&fixture.root)).expect("service roots");
    assert_eq!(
        data_of(&roots_out, &roots_run),
        serde_json::to_value(&service_roots).expect("roots json")
    );
    assert!(service_roots.roots.iter().all(|root| root.error.is_none()));

    let mixed_out = out_dir.join("mixed.json");
    let mixed_run = run_devsweep(&[
        "analyze".into(),
        "roots".into(),
        "--root".into(),
        path_arg(&fixture.root),
        "--root".into(),
        path_arg(&fixture.missing),
        "--format".into(),
        "json".into(),
        "--output".into(),
        path_arg(&mixed_out),
    ]);
    assert_eq!(mixed_run.status, Some(5), "{}", mixed_run.stderr);
    let service_mixed =
        list_analyze_roots(&[fixture.root.clone(), fixture.missing.clone()]).expect("mixed");
    let mixed = data_of(&mixed_out, &mixed_run);
    assert_eq!(
        mixed,
        serde_json::to_value(&service_mixed).expect("mixed json")
    );
    assert_eq!(mixed["roots"].as_array().map(Vec::len), Some(2));
    assert_eq!(mixed["roots"][1]["error"], "missing");

    let missing_out = out_dir.join("missing.json");
    let missing_run = run_devsweep(&[
        "analyze".into(),
        "roots".into(),
        "--root".into(),
        path_arg(&fixture.missing),
        "--format".into(),
        "json".into(),
        "--output".into(),
        path_arg(&missing_out),
    ]);
    assert_eq!(missing_run.status, Some(6), "{}", missing_run.stderr);
    let service_missing =
        list_analyze_roots(std::slice::from_ref(&fixture.missing)).expect("missing");
    let missing = data_of(&missing_out, &missing_run);
    assert_eq!(
        missing,
        serde_json::to_value(&service_missing).expect("missing json")
    );
    assert_eq!(missing["roots"].as_array().map(Vec::len), Some(1));
    assert_eq!(missing["roots"][0]["error"], "missing");

    let list_out = out_dir.join("list.json");
    let list_run = run_devsweep(&[
        "analyze".into(),
        "list".into(),
        "--snapshot".into(),
        path_arg(&fixture.snapshot_path),
        "--node".into(),
        "0".into(),
        "--offset".into(),
        "0".into(),
        "--limit".into(),
        "1".into(),
        "--format".into(),
        "json".into(),
        "--output".into(),
        path_arg(&list_out),
    ]);
    assert_eq!(list_run.status, Some(0), "{}", list_run.stderr);
    let service_list = list_analyze_children(&fixture.snapshot, 0, 0, 1).expect("service list");
    let listed = data_of(&list_out, &list_run);
    assert_eq!(
        listed,
        serde_json::to_value(&service_list).expect("list json")
    );
    assert_eq!(listed["children"].as_array().map(Vec::len), Some(1));
    assert!(listed["total_children"].as_u64().unwrap_or(0) > 1);

    for (name, class) in [
        ("ext4.vhdx", "wsl_disk"),
        ("demo.gguf", "ai_model"),
        ("notes.bin", "unknown"),
        ("hiberfil.sys", "hibernation"),
        ("pagefile.sys", "page_file"),
    ] {
        let node = node_id(&fixture.snapshot, name);
        let explain_out = out_dir.join(format!("explain-{name}.json"));
        let explain_run = run_devsweep(&[
            "analyze".into(),
            "explain".into(),
            "--snapshot".into(),
            path_arg(&fixture.snapshot_path),
            "--node".into(),
            node.to_string(),
            "--format".into(),
            "json".into(),
            "--output".into(),
            path_arg(&explain_out),
        ]);
        assert_eq!(
            explain_run.status,
            Some(0),
            "{} {}",
            name,
            explain_run.stderr
        );
        let service = explain_analyze_node(&fixture.snapshot, node).expect("service explain");
        let explained = data_of(&explain_out, &explain_run);
        assert_eq!(
            explained,
            serde_json::to_value(&service).expect("explain json"),
            "{name}"
        );
        assert_eq!(explained["class"], class, "{name}");
        assert_eq!(explained["disposable"], false, "{name}");
        assert_eq!(explained["lock"]["terminates_process"], false, "{name}");
        assert!(explained.get("digest").is_none(), "{name}");
    }

    let unknown_out = out_dir.join("unknown.json");
    let unknown_run = run_devsweep(&[
        "analyze".into(),
        "explain".into(),
        "--snapshot".into(),
        path_arg(&fixture.snapshot_path),
        "--node".into(),
        "999999".into(),
        "--format".into(),
        "json".into(),
        "--output".into(),
        path_arg(&unknown_out),
    ]);
    assert_eq!(unknown_run.status, Some(2), "{}", unknown_run.stderr);
    let unknown = read_envelope(&unknown_out, &unknown_run);
    assert_eq!(unknown["error"]["code"], "unknown_node");
    assert!(explain_analyze_node(&fixture.snapshot, 999_999).is_err());
}

#[test]
fn explain_human_text_is_not_a_cleanup_plan() {
    let fixture = fixture();
    let out_dir = fixture.snapshot_path.parent().expect("parent");
    let node = node_id(&fixture.snapshot, "ext4.vhdx");
    let english_path = out_dir.join("explain-en.txt");
    let chinese_path = out_dir.join("explain-zh.txt");
    let english = run_devsweep(&[
        "analyze".into(),
        "explain".into(),
        "--snapshot".into(),
        path_arg(&fixture.snapshot_path),
        "--node".into(),
        node.to_string(),
        "--output".into(),
        path_arg(&english_path),
    ]);
    assert_eq!(english.status, Some(0), "{}", english.stderr);
    let chinese = run_devsweep(&[
        "--language".into(),
        "zh-CN".into(),
        "analyze".into(),
        "explain".into(),
        "--snapshot".into(),
        path_arg(&fixture.snapshot_path),
        "--node".into(),
        node.to_string(),
        "--output".into(),
        path_arg(&chinese_path),
    ]);
    assert_eq!(chinese.status, Some(0), "{}", chinese.stderr);
    let english_text = fs::read_to_string(&english_path).expect("english");
    let chinese_text = fs::read_to_string(&chinese_path).expect("chinese");
    assert!(english_text.contains("wsl_disk"));
    assert!(english_text.contains("Not a cleanup plan"));
    assert!(chinese_text.contains("不是清理计划"));
    assert!(!english_text.contains("CleanupPlan"));
    assert!(!chinese_text.contains("CleanupPlan"));
    assert!(english_text.contains("no process was terminated"));
}
