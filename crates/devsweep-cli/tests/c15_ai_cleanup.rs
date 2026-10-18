//! Temp-home CLI parity for `clean ai`. These tests pass absolute `--home`.

use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    process::{Command, Output},
};

use devsweep_core::ai::{AiPlanV1, preview_ai};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn devsweep(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_devsweep"))
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("devsweep process runs")
}

fn arg(path: &Path) -> String {
    path.to_str().expect("temp path is utf-8").to_string()
}

fn status(output: &Output) -> Option<i32> {
    output.status.code()
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "json {error}\nstatus {:?}\nstdout {}\nstderr {}",
            status(output),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn assert_status(output: &Output, expected: i32) {
    assert_eq!(
        status(output),
        Some(expected),
        "stdout {}\nstderr {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn session(id: &str, active: bool) -> String {
    format!(r#"{{"version":1,"id":"{id}","active":{active},"body":"text"}}"#)
}

fn write_class_home(home: &Path) {
    fs::create_dir_all(home.join("cache/regen-a")).unwrap();
    fs::write(home.join("cache/regen-a/blob"), b"regen").unwrap();
    fs::write(home.join("cache/other.bin"), b"keep").unwrap();
    fs::create_dir_all(home.join("versions/1.0")).unwrap();
    fs::write(home.join("versions/1.0/tool"), b"old").unwrap();
    fs::create_dir_all(home.join("versions/2.0")).unwrap();
    fs::write(home.join("versions/2.0/tool"), b"current").unwrap();
    fs::write(home.join("versions/current.txt"), b"2.0\n").unwrap();
    fs::create_dir_all(home.join("sessions")).unwrap();
    fs::write(home.join("sessions/active"), b"sactive\n").unwrap();
    fs::write(
        home.join("sessions/sactive.json"),
        session("sactive", false),
    )
    .unwrap();
    fs::write(home.join("sessions/sarch.json"), session("sarch", false)).unwrap();
    fs::write(
        home.join("sessions/bad.json"),
        br#"{"version":1,"extra":true}"#,
    )
    .unwrap();
    fs::write(home.join("settings.json"), b"{}").unwrap();
    fs::write(home.join("credentials"), b"secret").unwrap();
    fs::write(home.join("memory.md"), b"notes").unwrap();
    fs::write(home.join("install.json"), br#"{"version":"1.2.3"}"#).unwrap();
}

fn scan(home: &Path, provider: &str) -> Value {
    let output_path = home.join(format!("scan-{provider}.json"));
    let output = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "scan".to_string(),
        "--provider".to_string(),
        provider.to_string(),
        "--home".to_string(),
        arg(home),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        arg(&output_path),
    ]);
    assert_status(&output, 0);
    let document: Value = serde_json::from_slice(&fs::read(&output_path).unwrap()).unwrap();
    assert_eq!(document["schema_version"], 1);
    assert_eq!(document["command"], "clean.ai.scan");
    assert_eq!(document["outcome"], "succeeded");
    assert!(document["error"].is_null());
    document["data"].clone()
}

fn plan(home: &Path, provider: &str, class: &str) -> AiPlanV1 {
    let scan_path = home.join(format!("scan-{provider}.json"));
    if !scan_path.is_file() {
        let _ = scan(home, provider);
    }
    let plan_path = home.join(format!("plan-{provider}-{class}.json"));
    let output = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "plan".to_string(),
        "--scan".to_string(),
        arg(&scan_path),
        "--class".to_string(),
        class.to_string(),
        "--output".to_string(),
        arg(&plan_path),
    ]);
    assert_status(&output, 0);
    serde_json::from_slice(&fs::read(&plan_path).unwrap()).expect("plan json")
}

fn target_paths(plan: &AiPlanV1) -> BTreeSet<String> {
    plan.targets
        .iter()
        .map(|target| target.relative_path.clone())
        .collect()
}

fn candidate_paths(data: &Value, class: &str) -> BTreeSet<String> {
    data["providers"][0]["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .filter(|row| row["class"] == class && row["disposition"] == "candidate")
        .map(|row| row["relative_path"].as_str().unwrap().to_string())
        .collect()
}

fn row_disposition(data: &Value, relative: &str) -> Option<String> {
    data["providers"][0]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["relative_path"] == relative)
        .and_then(|row| row["disposition"].as_str().map(str::to_string))
}

fn fingerprint(root: &Path) -> String {
    let mut files = Vec::new();
    walk(root, root, &mut files);
    files.sort();
    let mut hasher = Sha256::new();
    for (relative, bytes) in files {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(&bytes);
    }
    let digest = hasher.finalize();
    hex_encode(digest.as_slice())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn walk(root: &Path, current: &Path, files: &mut Vec<(String, Vec<u8>)>) {
    let relative = current
        .strip_prefix(root)
        .unwrap_or(current)
        .to_string_lossy()
        .replace('\\', "/");
    let metadata = fs::symlink_metadata(current).expect("metadata");
    if metadata.file_type().is_symlink() {
        return;
    }
    if metadata.is_file() {
        files.push((relative, fs::read(current).expect("read")));
        return;
    }
    files.push((format!("{relative}/"), Vec::new()));
    let mut children = fs::read_dir(current)
        .expect("read dir")
        .map(|entry| entry.expect("entry").path())
        .collect::<Vec<_>>();
    children.sort();
    for child in children {
        walk(root, &child, files);
    }
}

fn zero_digest() -> String {
    format!("sha256:{}", "0".repeat(64))
}

#[test]
fn cli_preview_json_matches_core_service() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_class_home(home);
    let saved = plan(home, "codex", "regenerable_cache");
    let core = preview_ai(&saved).expect("core preview");
    assert!(core.digest.starts_with("sha256:"));
    assert_eq!(core.digest.len(), "sha256:".len() + 64);
    let preview_path = home.join("preview.json");
    let output = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "preview".to_string(),
        "--plan".to_string(),
        arg(&home.join("plan-codex-regenerable_cache.json")),
        "--format".to_string(),
        "json".to_string(),
        "--output".to_string(),
        arg(&preview_path),
    ]);
    assert_status(&output, 0);
    let document: Value = serde_json::from_slice(&fs::read(&preview_path).unwrap()).unwrap();
    assert_eq!(document["command"], "clean.ai.preview");
    assert_eq!(document["outcome"], "succeeded");
    assert_eq!(
        document["data"],
        serde_json::to_value(&core).expect("core json")
    );
    assert_eq!(document["data"]["digest"], core.digest);
}

#[test]
fn cli_execute_without_confirm_exits_2_and_home_is_unchanged() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_class_home(home);
    let saved = plan(home, "claude-code", "session_archive");
    assert_eq!(
        target_paths(&saved),
        BTreeSet::from(["sessions/sarch.json".to_string()])
    );
    let plan_path = home.join("plan-claude-code-session_archive.json");
    let before = fingerprint(home);
    let missing_confirm = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "execute".to_string(),
        "--plan".to_string(),
        arg(&plan_path),
        "--preview-digest".to_string(),
        zero_digest(),
        "--format".to_string(),
        "json".to_string(),
    ]);
    assert_status(&missing_confirm, 2);
    let missing_digest = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "execute".to_string(),
        "--plan".to_string(),
        arg(&plan_path),
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ]);
    assert_status(&missing_digest, 2);
    let missing_plan = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "execute".to_string(),
        "--preview-digest".to_string(),
        zero_digest(),
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ]);
    assert_status(&missing_plan, 2);
    assert_eq!(fingerprint(home), before);
    assert!(home.join("sessions/sarch.json").is_file());
}

#[test]
fn cli_stale_digest_exits_3_and_home_is_unchanged() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_class_home(home);
    let saved = plan(home, "cursor", "regenerable_cache");
    let preview = preview_ai(&saved).unwrap();
    fs::write(home.join("cache/regen-a/blob"), b"edited").unwrap();
    let before = fingerprint(home);
    let output = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "execute".to_string(),
        "--plan".to_string(),
        arg(&home.join("plan-cursor-regenerable_cache.json")),
        "--preview-digest".to_string(),
        preview.digest,
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ]);
    assert_status(&output, 3);
    let document = stdout_json(&output);
    assert_eq!(document["error"]["code"], "stale_confirmation");
    assert!(document["error"]["message"].is_null());
    assert_eq!(fingerprint(home), before);
    assert!(!home.join(".devsweep-recovery").exists());
}

#[test]
fn cli_unsupported_session_bytes_are_not_targets() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_class_home(home);
    let data = scan(home, "codex");
    assert_eq!(
        row_disposition(&data, "sessions/bad.json").as_deref(),
        Some("unsupported_version")
    );
    let saved = plan(home, "codex", "session_archive");
    assert!(!target_paths(&saved).contains("sessions/bad.json"));
}

#[test]
fn cli_adjacent_settings_credentials_memory_are_not_targets() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_class_home(home);
    let data = scan(home, "claude-code");
    for relative in ["settings.json", "credentials", "memory.md"] {
        let disposition = row_disposition(&data, relative).unwrap_or_default();
        assert_ne!(
            disposition, "candidate",
            "{relative} disposition {disposition}"
        );
    }
    for class in [
        "regenerable_cache",
        "superseded_version",
        "session_archive",
        "worktree",
    ] {
        let saved = plan(home, "claude-code", class);
        let targets = target_paths(&saved);
        assert!(!targets.iter().any(|path| {
            path == "settings.json" || path == "credentials" || path == "memory.md"
        }));
    }
}

#[test]
fn cli_active_session_id_is_not_a_target() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_class_home(home);
    let data = scan(home, "cursor");
    assert_ne!(
        row_disposition(&data, "sessions/sactive.json").as_deref(),
        Some("candidate")
    );
    let saved = plan(home, "cursor", "session_archive");
    assert_eq!(
        target_paths(&saved),
        BTreeSet::from(["sessions/sarch.json".to_string()])
    );
}

#[test]
fn cli_superseded_version_can_be_planned_and_current_cannot() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_class_home(home);
    let saved = plan(home, "codex", "superseded_version");
    assert_eq!(
        target_paths(&saved),
        BTreeSet::from(["versions/1.0".to_string()])
    );
    assert!(!target_paths(&saved).contains("versions/2.0"));
}

#[test]
fn cli_regen_cache_can_be_planned_and_other_names_cannot() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_class_home(home);
    let saved = plan(home, "claude-code", "regenerable_cache");
    assert_eq!(
        target_paths(&saved),
        BTreeSet::from(["cache/regen-a".to_string()])
    );
    assert!(
        !target_paths(&saved)
            .iter()
            .any(|path| path.contains("other.bin"))
    );
}

#[test]
fn cli_three_providers_use_the_same_classes() {
    let mut observed = Vec::new();
    for provider in ["claude-code", "codex", "cursor"] {
        let temp = TempDir::new().unwrap();
        let home = temp.path().to_path_buf();
        write_class_home(&home);
        let data = scan(&home, provider);
        assert_eq!(data["providers"][0]["provider"], provider);
        assert_eq!(data["providers"][0]["installed_tool"]["version"], "1.2.3");
        let classes = [
            "regenerable_cache",
            "superseded_version",
            "session_archive",
            "worktree",
        ]
        .into_iter()
        .map(|class| (class, candidate_paths(&data, class)))
        .collect::<Vec<_>>();
        observed.push(classes);
        let refused = devsweep(&[
            "clean".to_string(),
            "ai".to_string(),
            "plan".to_string(),
            "--scan".to_string(),
            arg(&home.join(format!("scan-{provider}.json"))),
            "--class".to_string(),
            "installed_tool".to_string(),
            "--output".to_string(),
            arg(&home.join("installed-plan.json")),
        ]);
        assert_status(&refused, 2);
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(
            stderr.contains("class_not_selectable"),
            "stdout {}\nstderr {stderr}",
            String::from_utf8_lossy(&refused.stdout)
        );
        assert!(!home.join("installed-plan.json").is_file());
        drop(temp);
    }
    assert_eq!(observed[0], observed[1]);
    assert_eq!(observed[1], observed[2]);
}

#[test]
fn cli_bundle_write_failure_leaves_home_unchanged() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_class_home(home);
    let saved = plan(home, "codex", "session_archive");
    let preview = preview_ai(&saved).unwrap();
    fs::write(home.join(".devsweep-recovery"), b"not-a-directory").unwrap();
    let before = fingerprint(home);
    let output = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "execute".to_string(),
        "--plan".to_string(),
        arg(&home.join("plan-codex-session_archive.json")),
        "--preview-digest".to_string(),
        preview.digest.clone(),
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ]);
    assert_status(&output, 6);
    let document = stdout_json(&output);
    assert_eq!(document["error"]["code"], "bundle_write_failed");
    assert!(document["error"]["message"].is_null());
    assert_eq!(fingerprint(home), before);
    assert!(home.join("sessions/sarch.json").is_file());
}

#[test]
fn cli_restore_after_external_edit_refuses() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    fs::create_dir_all(home.join("cache/regen-a")).unwrap();
    let original = b"regen-bytes";
    fs::write(home.join("cache/regen-a/blob"), original).unwrap();
    let _ = scan(home, "claude-code");
    let saved = plan(home, "claude-code", "regenerable_cache");
    assert_eq!(
        target_paths(&saved),
        BTreeSet::from(["cache/regen-a".to_string()])
    );
    let plan_path = home.join("plan-claude-code-regenerable_cache.json");
    let preview = preview_ai(&saved).unwrap();
    let executed = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "execute".to_string(),
        "--plan".to_string(),
        arg(&plan_path),
        "--preview-digest".to_string(),
        preview.digest,
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ]);
    assert_status(&executed, 0);
    let report = stdout_json(&executed);
    assert_eq!(report["outcome"], "succeeded");
    assert!(!home.join("cache/regen-a").exists());
    let bundle = report["data"]["bundle_ids"][0]
        .as_str()
        .expect("bundle id")
        .to_string();
    assert_eq!(
        fs::read(
            home.join(".devsweep-recovery")
                .join(&bundle)
                .join("payload")
                .join("blob")
        )
        .expect("payload"),
        original
    );

    let restore_path = home.join("restore.json");
    let restore = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "restore-plan".to_string(),
        "--home".to_string(),
        arg(home),
        "--bundle".to_string(),
        bundle.clone(),
        "--output".to_string(),
        arg(&restore_path),
    ]);
    assert_status(&restore, 0);
    let restore_plan: AiPlanV1 =
        serde_json::from_slice(&fs::read(&restore_path).unwrap()).expect("restore plan");
    let restore_preview = preview_ai(&restore_plan).unwrap();
    let restored = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "execute".to_string(),
        "--plan".to_string(),
        arg(&restore_path),
        "--preview-digest".to_string(),
        restore_preview.digest,
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ]);
    assert_status(&restored, 0);
    assert_eq!(fs::read(home.join("cache/regen-a/blob")).unwrap(), original);

    fs::write(home.join("cache/regen-a/blob"), b"edited-by-someone-else").unwrap();
    let again_path = home.join("restore-again.json");
    let again = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "restore-plan".to_string(),
        "--home".to_string(),
        arg(home),
        "--bundle".to_string(),
        bundle,
        "--output".to_string(),
        arg(&again_path),
    ]);
    assert_status(&again, 0);
    let again_plan: AiPlanV1 =
        serde_json::from_slice(&fs::read(&again_path).unwrap()).expect("second restore plan");
    let again_preview = preview_ai(&again_plan).unwrap();
    let refused = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "execute".to_string(),
        "--plan".to_string(),
        arg(&again_path),
        "--preview-digest".to_string(),
        again_preview.digest,
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ]);
    assert_status(&refused, 3);
    let document = stdout_json(&refused);
    assert_eq!(document["error"]["code"], "restore_refused");
    assert!(document["error"]["message"].is_null());
    assert_eq!(
        fs::read(home.join("cache/regen-a/blob")).unwrap(),
        b"edited-by-someone-else"
    );
}

#[test]
fn cli_recovery_list_does_not_restore() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    fs::create_dir_all(home.join("cache/regen-a")).unwrap();
    fs::write(home.join("cache/regen-a/blob"), b"regen-bytes").unwrap();
    let _ = scan(home, "codex");
    let saved = plan(home, "codex", "regenerable_cache");
    let preview = preview_ai(&saved).unwrap();
    let executed = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "execute".to_string(),
        "--plan".to_string(),
        arg(&home.join("plan-codex-regenerable_cache.json")),
        "--preview-digest".to_string(),
        preview.digest,
        "--confirm".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ]);
    assert_status(&executed, 0);
    assert!(!home.join("cache/regen-a").exists());
    let before = fingerprint(home);
    let output = devsweep(&[
        "clean".to_string(),
        "ai".to_string(),
        "recovery-list".to_string(),
        "--home".to_string(),
        arg(home),
        "--format".to_string(),
        "json".to_string(),
    ]);
    assert_status(&output, 0);
    let document = stdout_json(&output);
    assert_eq!(document["command"], "clean.ai.recovery-list");
    assert_eq!(document["data"]["bundles"].as_array().unwrap().len(), 1);
    assert_eq!(fingerprint(home), before);
    assert!(!home.join("cache/regen-a").exists());
}
