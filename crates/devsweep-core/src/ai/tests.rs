use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::{Duration, SystemTime},
};

use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::{
    AiClass, AiDisposition, AiError, AiExit, AiPlanV1, AiProvider, AiScanRequest,
    SESSION_ENUM_LIMIT, execute_ai, plan_ai, preview_ai, restore_plan, scan_ai, sha256_hex,
};
use crate::{
    execution::TrashRunner,
    process::{FlagCancelObserver, NoopCancelObserver},
};

struct RecordingTrash {
    calls: Mutex<Vec<PathBuf>>,
    fail: bool,
}

impl TrashRunner for RecordingTrash {
    fn move_to_trash(&self, path: &Path) -> anyhow::Result<()> {
        self.calls
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(path.to_path_buf());
        if self.fail {
            anyhow::bail!("trash failed");
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("target");
        fs::rename(path, path.with_file_name(format!("{name}.trashed-by-test")))?;
        Ok(())
    }
}

fn scan_home(provider: AiProvider, home: &Path) -> super::AiObservationV1 {
    scan_ai(
        &AiScanRequest {
            provider: Some(provider),
            home: Some(home.to_path_buf()),
        },
        &NoopCancelObserver,
    )
    .expect("scan")
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
    super::hex_encode(hasher.finalize().as_slice())
}

fn walk(root: &Path, current: &Path, files: &mut Vec<(String, Vec<u8>)>) {
    let relative = current
        .strip_prefix(root)
        .unwrap_or(current)
        .to_string_lossy()
        .replace('\\', "/");
    let metadata = fs::symlink_metadata(current).expect("metadata");
    if metadata.is_file() {
        files.push((relative, fs::read(current).expect("read")));
        return;
    }
    files.push((format!("{relative}/"), Vec::new()));
    let mut children = fs::read_dir(current)
        .expect("read_dir")
        .collect::<Result<Vec<_>, _>>()
        .expect("entries");
    children.sort_by_key(|entry| entry.file_name());
    for entry in children {
        walk(root, &entry.path(), files);
    }
}

fn write_session(home: &Path, id: &str, active: bool, body: &str) {
    fs::create_dir_all(home.join("sessions")).unwrap();
    let document = serde_json::json!({
        "version": 1,
        "id": id,
        "active": active,
        "body": body
    });
    fs::write(
        home.join("sessions").join(format!("{id}.json")),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
}

fn write_class_fixture(home: &Path) {
    fs::create_dir_all(home.join("cache/regen-a")).unwrap();
    fs::write(home.join("cache/regen-a/data.bin"), b"cache-bytes").unwrap();
    fs::write(home.join("cache/other.bin"), b"keep").unwrap();
    fs::create_dir_all(home.join("versions/1.0")).unwrap();
    fs::write(home.join("versions/1.0/payload.bin"), b"old").unwrap();
    fs::create_dir_all(home.join("versions/2.0")).unwrap();
    fs::write(home.join("versions/2.0/payload.bin"), b"current").unwrap();
    fs::write(home.join("versions/current.txt"), b"2.0\n").unwrap();
    write_session(home, "s1", false, "active-body");
    write_session(home, "s2", false, "archive-body");
    write_session(home, "live", true, "still-open");
    fs::write(
        home.join("sessions/bad.json"),
        br#"{"version":1,"id":"bad","active":false,"body":"x","extra":1}"#,
    )
    .unwrap();
    fs::write(home.join("sessions/active"), b"s1\n").unwrap();
    fs::write(home.join("settings.json"), b"{}\n").unwrap();
    fs::write(home.join("credentials"), b"secret").unwrap();
    fs::write(home.join("memory.md"), b"memory").unwrap();
    fs::create_dir_all(home.join("skills")).unwrap();
    fs::write(home.join("skills/skill.md"), b"skill").unwrap();
    fs::create_dir_all(home.join("mcp")).unwrap();
    fs::write(home.join("mcp/server.json"), b"{}").unwrap();
    fs::create_dir_all(home.join("models")).unwrap();
    fs::write(home.join("models/weights.bin"), b"model").unwrap();
    fs::create_dir_all(home.join("projects")).unwrap();
    fs::write(home.join("projects/code.txt"), b"project").unwrap();
    fs::create_dir_all(home.join("worktrees/plain")).unwrap();
    fs::write(home.join("worktrees/plain/file.txt"), b"not-a-repo").unwrap();
    fs::write(home.join("install.json"), br#"{"version":"1.2.3"}"#).unwrap();
    let newer = fs::File::options()
        .write(true)
        .open(home.join("cache/other.bin"))
        .unwrap();
    newer.set_modified(SystemTime::now()).unwrap();
    let older = fs::File::options()
        .write(true)
        .open(home.join("cache/regen-a/data.bin"))
        .unwrap();
    older
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(5))
        .unwrap();
}

fn selected(plan: &AiPlanV1) -> Vec<String> {
    plan.targets
        .iter()
        .map(|target| target.relative_path.clone())
        .collect()
}

fn assert_code(error: &AiError, exit: AiExit, code: &str) {
    assert_eq!(error.exit, exit, "{}", error.message);
    assert_eq!(error.code, code, "{}", error.message);
}

// The fixture builds Windows-style absolute paths (`C:\Users\fixture`), which
// `Path::is_absolute` rejects on POSIX, so the explicit-home branch would report
// `invalid_home` instead of the `source_unavailable` this test asserts.
#[cfg(windows)]
#[test]
fn default_and_custom_roots_do_not_scan_a_profile() {
    let profile = Path::new(r"C:\Users\fixture");
    let appdata = Path::new(r"C:\Users\fixture\AppData\Roaming");
    assert_eq!(
        super::default_home_path(AiProvider::ClaudeCode, Some(profile), Some(appdata)),
        Some(profile.join(".claude"))
    );
    assert_eq!(
        super::default_home_path(AiProvider::Codex, Some(profile), None),
        Some(profile.join(".codex"))
    );
    assert_eq!(
        super::default_home_path(AiProvider::Cursor, None, Some(appdata)),
        Some(appdata.join("Cursor"))
    );
    assert!(super::default_home_path(AiProvider::ClaudeCode, None, Some(appdata)).is_none());
    assert!(super::default_home_path(AiProvider::Cursor, Some(profile), None).is_none());

    let missing = PathBuf::from(r"C:\devsweep-ai-test-missing-home");
    let error = scan_ai(
        &AiScanRequest {
            provider: Some(AiProvider::Codex),
            home: Some(missing),
        },
        &NoopCancelObserver,
    )
    .expect_err("missing explicit home");
    assert_code(&error, AiExit::Unavailable, "source_unavailable");
}

#[test]
fn ac1_ac2_three_providers_share_class_rules() {
    let temp = TempDir::new().unwrap();
    for provider in AiProvider::ALL {
        let home = temp.path().join(provider.as_str());
        fs::create_dir_all(&home).unwrap();
        write_class_fixture(&home);
        let observation = scan_home(provider, &home);
        let entry = &observation.providers[0];
        assert_eq!(entry.state, "present");
        let tool = entry.installed_tool.as_ref().expect("install identity");
        assert_eq!(tool.provider, provider);
        assert_eq!(tool.version, "1.2.3");
        assert_eq!(tool.install_kind, "install_json");
        assert!(
            entry
                .rows
                .iter()
                .any(|row| row.relative_path == "sessions/bad.json"
                    && row.reason == "unsupported_version"
                    && row.disposition == AiDisposition::UnsupportedVersion),
            "unsupported session must explain the refusal"
        );

        let cache = plan_ai(&observation, AiClass::RegenerableCache).unwrap();
        assert_eq!(selected(&cache), vec!["cache/regen-a".to_string()]);
        let versions = plan_ai(&observation, AiClass::SupersededVersion).unwrap();
        assert_eq!(selected(&versions), vec!["versions/1.0".to_string()]);
        let sessions = plan_ai(&observation, AiClass::SessionArchive).unwrap();
        assert_eq!(selected(&sessions), vec!["sessions/s2.json".to_string()]);
        let worktrees = plan_ai(&observation, AiClass::Worktree).unwrap();
        assert!(selected(&worktrees).is_empty(), "{}", provider.as_str());
        let installed = plan_ai(&observation, AiClass::InstalledTool).expect_err("never");
        assert_code(&installed, AiExit::Usage, "class_not_selectable");

        let protected = [
            "settings.json",
            "credentials",
            "memory.md",
            "skills/skill.md",
            "mcp/server.json",
            "models/weights.bin",
            "projects/code.txt",
            "versions/2.0",
            "sessions/s1.json",
            "sessions/live.json",
            "sessions/bad.json",
            "cache/other.bin",
            "worktrees/plain",
            "install.json",
        ];
        for plan in [&cache, &versions, &sessions, &worktrees] {
            for target in &plan.targets {
                assert!(
                    protected
                        .iter()
                        .all(|needle| !target.relative_path.contains(needle)),
                    "{} selected {}",
                    provider.as_str(),
                    target.relative_path
                );
            }
        }
    }
}

#[test]
fn ac1_locked_and_malformed_are_not_targets() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_session(home, "kept", false, "body");
    fs::write(home.join("sessions/index.lock"), b"locked").unwrap();
    fs::write(
        home.join("install.json"),
        br#"{"version":"9","extra":true}"#,
    )
    .unwrap();
    let observation = scan_home(AiProvider::Cursor, home);
    let sessions = plan_ai(&observation, AiClass::SessionArchive).unwrap();
    assert!(selected(&sessions).is_empty());
    assert_eq!(
        observation.providers[0]
            .rows
            .iter()
            .find(|row| row.relative_path == "sessions")
            .map(|row| row.reason.as_str()),
        Some("locked")
    );
    assert_eq!(
        observation.providers[0]
            .installed_tool
            .as_ref()
            .unwrap()
            .version,
        "unknown"
    );
}

#[test]
fn newer_mtime_is_not_authority() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_class_fixture(home);
    let observation = scan_home(AiProvider::ClaudeCode, home);
    let cache = plan_ai(&observation, AiClass::RegenerableCache).unwrap();
    assert_eq!(selected(&cache), vec!["cache/regen-a".to_string()]);
    assert!(
        cache
            .refused
            .iter()
            .any(|row| row.relative_path == "cache/other.bin")
    );
}

#[test]
fn cursor_electron_cache_children_are_not_targets() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    fs::create_dir_all(home.join("Cache/data_0")).unwrap();
    fs::write(home.join("Cache/data_0/blob"), b"electron").unwrap();
    fs::create_dir_all(home.join("Code Cache")).unwrap();
    fs::write(home.join("Code Cache/js"), b"js").unwrap();
    fs::create_dir_all(home.join("GPUCache")).unwrap();
    fs::write(home.join("GPUCache/data"), b"gpu").unwrap();
    fs::create_dir_all(home.join("CachedData")).unwrap();
    fs::write(home.join("CachedData/hash"), b"cached").unwrap();
    let observation = scan_home(AiProvider::Cursor, home);
    for class in [
        AiClass::RegenerableCache,
        AiClass::SupersededVersion,
        AiClass::SessionArchive,
        AiClass::Worktree,
    ] {
        let plan = plan_ai(&observation, class).unwrap();
        assert!(
            plan.targets.is_empty(),
            "{class:?} selected {:?}",
            selected(&plan)
        );
    }
}

#[test]
fn ac3_bundle_failure_leaves_home_unchanged() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_session(home, "s2", false, "archive-body");
    let observation = scan_home(AiProvider::Codex, home);
    let plan = plan_ai(&observation, AiClass::SessionArchive).unwrap();
    let preview = preview_ai(&plan).unwrap();
    fs::write(home.join(".devsweep-recovery"), b"not-a-directory").unwrap();
    let before = fingerprint(home);
    let trash = RecordingTrash {
        calls: Mutex::new(Vec::new()),
        fail: false,
    };
    let error = execute_ai(&plan, &preview.digest, true, &NoopCancelObserver, &trash)
        .expect_err("bundle must fail closed");
    assert_code(&error, AiExit::Failed, "bundle_write_failed");
    assert!(trash.calls.lock().unwrap().is_empty());
    assert_eq!(fingerprint(home), before);
    assert!(home.join("sessions/s2.json").is_file());
}

#[test]
fn ac3_failed_move_is_not_retried() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_session(home, "s2", false, "archive-body");
    let original = fs::read(home.join("sessions/s2.json")).unwrap();
    let observation = scan_home(AiProvider::ClaudeCode, home);
    let plan = plan_ai(&observation, AiClass::SessionArchive).unwrap();
    let preview = preview_ai(&plan).unwrap();
    let trash = RecordingTrash {
        calls: Mutex::new(Vec::new()),
        fail: true,
    };
    let report = execute_ai(&plan, &preview.digest, true, &NoopCancelObserver, &trash).unwrap();
    assert_eq!(report.outcome, "failed");
    assert_eq!(report.code.as_deref(), Some("move_failed"));
    assert_eq!(trash.calls.lock().unwrap().len(), 1);
    assert_eq!(fs::read(home.join("sessions/s2.json")).unwrap(), original);
    let bundle = home
        .join(".devsweep-recovery")
        .join(report.bundle_ids[0].as_str());
    assert_eq!(fs::read(bundle.join("payload")).unwrap(), original);
}

#[test]
fn ac3_disappeared_move_is_unknown_and_not_retried() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_session(home, "s2", false, "archive-body");
    let observation = scan_home(AiProvider::Codex, home);
    let plan = plan_ai(&observation, AiClass::SessionArchive).unwrap();
    let preview = preview_ai(&plan).unwrap();
    let trash = HideThenFailTrash {
        calls: Mutex::new(0),
    };
    let report = execute_ai(&plan, &preview.digest, true, &NoopCancelObserver, &trash).unwrap();
    assert_eq!(report.outcome, "unknown");
    assert_eq!(report.code.as_deref(), Some("unknown_after_dispatch"));
    assert_eq!(*trash.calls.lock().unwrap(), 1);
    assert!(!home.join("sessions/s2.json").exists());
}

struct HideThenFailTrash {
    calls: Mutex<u32>,
}

impl TrashRunner for HideThenFailTrash {
    fn move_to_trash(&self, path: &Path) -> anyhow::Result<()> {
        *self.calls.lock().unwrap_or_else(|error| error.into_inner()) += 1;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("target");
        fs::rename(path, path.with_file_name(format!("{name}.trashed-by-test")))?;
        anyhow::bail!("trash failed after the path left");
    }
}

#[test]
fn ac3_restore_refuses_external_edit() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_session(home, "s2", false, "archive-body");
    let original = fs::read(home.join("sessions/s2.json")).unwrap();
    let observation = scan_home(AiProvider::ClaudeCode, home);
    let plan = plan_ai(&observation, AiClass::SessionArchive).unwrap();
    let preview = preview_ai(&plan).unwrap();
    assert!(preview.digest.starts_with("sha256:"));
    assert_eq!(preview.digest.len(), "sha256:".len() + 64);
    let trash = RecordingTrash {
        calls: Mutex::new(Vec::new()),
        fail: false,
    };
    let report = execute_ai(&plan, &preview.digest, true, &NoopCancelObserver, &trash).unwrap();
    assert_eq!(report.outcome, "succeeded");
    assert!(!home.join("sessions/s2.json").exists());
    assert_eq!(trash.calls.lock().unwrap().len(), 1);
    let bundle = home.join(".devsweep-recovery").join(&report.bundle_ids[0]);
    assert_eq!(fs::read(bundle.join("payload")).unwrap(), original);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(bundle.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["content_sha256"], sha256_hex(&original));

    let restore = restore_plan(home, &report.bundle_ids[0]).unwrap();
    let restore_preview = preview_ai(&restore).unwrap();
    execute_ai(
        &restore,
        &restore_preview.digest,
        true,
        &NoopCancelObserver,
        &trash,
    )
    .unwrap();
    assert_eq!(fs::read(home.join("sessions/s2.json")).unwrap(), original);

    fs::write(home.join("sessions/s2.json"), b"edited-by-someone-else").unwrap();
    let again = restore_plan(home, &report.bundle_ids[0]).unwrap();
    let again_preview = preview_ai(&again).unwrap();
    let error = execute_ai(
        &again,
        &again_preview.digest,
        true,
        &NoopCancelObserver,
        &trash,
    )
    .expect_err("external edit");
    assert_code(&error, AiExit::Stale, "restore_refused");
    assert_eq!(
        fs::read(home.join("sessions/s2.json")).unwrap(),
        b"edited-by-someone-else"
    );
}

#[test]
fn ac3_stale_digest_does_not_mutate() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_session(home, "s2", false, "archive-body");
    let observation = scan_home(AiProvider::Codex, home);
    let plan = plan_ai(&observation, AiClass::SessionArchive).unwrap();
    let preview = preview_ai(&plan).unwrap();
    fs::write(home.join("sessions/s2.json"), b"changed").unwrap();
    let before = fingerprint(home);
    let trash = RecordingTrash {
        calls: Mutex::new(Vec::new()),
        fail: false,
    };
    let error =
        execute_ai(&plan, &preview.digest, true, &NoopCancelObserver, &trash).expect_err("stale");
    assert_code(&error, AiExit::Stale, "stale_confirmation");
    assert!(trash.calls.lock().unwrap().is_empty());
    assert_eq!(fingerprint(home), before);
}

#[test]
fn ac3_unknown_sessions_active() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_session(home, "s2", false, "archive-body");
    fs::write(home.join("sessions/active"), b"s2\nnot-a-safe-id\n").unwrap();
    let before = fingerprint(home);
    let observation = scan_home(AiProvider::Codex, home);
    let active = observation.providers[0]
        .rows
        .iter()
        .find(|row| row.relative_path == "sessions/active")
        .expect("sessions/active row");
    assert_eq!(active.reason, "unknown_active_owner");
    assert_ne!(active.disposition, AiDisposition::Candidate);
    assert_eq!(active.disposition, AiDisposition::InspectOnly);
    assert!(
        observation.providers[0]
            .rows
            .iter()
            .all(|row| row.relative_path != "sessions/s2.json"
                || row.disposition != AiDisposition::Candidate)
    );
    let plan = plan_ai(&observation, AiClass::SessionArchive).unwrap();
    assert!(
        !selected(&plan).contains(&"sessions/s2.json".to_string()),
        "unknown sessions/active must not select a session archive: {:?}",
        selected(&plan)
    );
    assert_eq!(fingerprint(home), before);
}

#[test]
fn ac3_unregistered_worktree() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    let worktree = home.join("worktrees/orphan");
    fs::create_dir_all(&worktree).unwrap();
    fs::write(worktree.join("file.txt"), b"not-a-registered-worktree").unwrap();
    let git_available = Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if git_available {
        git_ok(temp.path(), &["init"]);
    }
    let before = fingerprint(&home);
    let observation = scan_home(AiProvider::ClaudeCode, &home);
    let row = observation.providers[0]
        .rows
        .iter()
        .find(|row| row.relative_path == "worktrees/orphan")
        .expect("worktree row");
    let refusal = super::worktree::worktree_refusal(&worktree, &NoopCancelObserver);
    assert_eq!(row.disposition, AiDisposition::InspectOnly);
    assert_eq!(row.reason, refusal);
    if refusal == "not_registered" {
        assert_eq!(row.reason, "not_registered");
    } else if refusal == "git_unavailable" {
        assert_eq!(row.reason, "git_unavailable");
    } else {
        assert_eq!(
            row.reason, "not_registered",
            "production worktree_refusal returned {refusal}"
        );
    }
    assert_eq!(fingerprint(&home), before);
    assert_eq!(
        fs::read(worktree.join("file.txt")).unwrap(),
        b"not-a-registered-worktree"
    );
}

#[test]
fn ac5_session_enumeration_is_bounded() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    for index in 0..=SESSION_ENUM_LIMIT {
        write_session(home, &format!("s{index:04}"), false, "body");
    }
    let observation = scan_home(AiProvider::Cursor, home);
    assert!(observation.providers[0].session_bounded);
    assert!(observation.providers[0].rows.iter().any(|row| {
        row.relative_path == "sessions/s0000.json" && row.disposition == AiDisposition::Candidate
    }));
    assert!(
        observation
            .providers
            .iter()
            .flat_map(|provider| &provider.rows)
            .all(|row| row.relative_path != "sessions/s4096.json")
    );
    let error = plan_ai(&observation, AiClass::SessionArchive).expect_err("bounded");
    assert_code(&error, AiExit::Partial, "incomplete_observation");
}

#[test]
fn ac5_cancel_before_dispatch_does_not_mutate() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();
    write_session(home, "s2", false, "archive-body");
    let observation = scan_home(AiProvider::ClaudeCode, home);
    let plan = plan_ai(&observation, AiClass::SessionArchive).unwrap();
    let before = fingerprint(home);
    let cancel = FlagCancelObserver::new();
    cancel.request_cancel();
    let error = execute_ai(
        &plan,
        "sha256:unused",
        true,
        &cancel,
        &RecordingTrash {
            calls: Mutex::new(Vec::new()),
            fail: false,
        },
    )
    .expect_err("canceled");
    assert_code(&error, AiExit::Canceled, "canceled");
    assert_eq!(fingerprint(home), before);
    let scan_error = scan_ai(
        &AiScanRequest {
            provider: Some(AiProvider::ClaudeCode),
            home: Some(home.to_path_buf()),
        },
        &cancel,
    )
    .expect_err("scan canceled");
    assert_code(&scan_error, AiExit::Canceled, "canceled");
}

#[test]
fn ac4_worktree_protections_refuse_deletion_policy() {
    let git = Command::new("git").arg("--version").output();
    assert!(
        git.as_ref()
            .map(|output| output.status.success())
            .unwrap_or(false),
        "git is required"
    );
    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    let remote = temp.path().join("remote.git");
    let home = temp.path().join("home");
    fs::create_dir_all(&repo).unwrap();
    fs::create_dir_all(home.join("worktrees")).unwrap();
    git_ok(&repo, &["init"]);
    git_ok(
        &repo,
        &["config", "user.email", "devsweep-test@example.com"],
    );
    git_ok(&repo, &["config", "user.name", "devsweep-test"]);
    git_ok(&repo, &["config", "commit.gpgsign", "false"]);
    git_ok(&repo, &["config", "core.autocrlf", "false"]);
    fs::write(repo.join("file.txt"), b"base\n").unwrap();
    fs::write(repo.join(".gitignore"), b"*.log\n").unwrap();
    git_ok(&repo, &["add", "file.txt", ".gitignore"]);
    git_ok(&repo, &["commit", "-m", "init"]);
    git_ok(temp.path(), &["init", "--bare", "remote.git"]);
    let remote_path = remote.display().to_string();
    git_ok(&repo, &["remote", "add", "origin", &remote_path]);
    git_ok(&repo, &["push", "-u", "origin", "HEAD"]);

    let clean = home.join("worktrees/clean");
    let dirty = home.join("worktrees/dirty");
    let untracked = home.join("worktrees/untracked");
    let ignored = home.join("worktrees/ignored");
    let unpushed = home.join("worktrees/unpushed");
    let submodule = home.join("worktrees/submodule");
    let clean_path = clean.display().to_string();
    let dirty_path = dirty.display().to_string();
    let untracked_path = untracked.display().to_string();
    let ignored_path = ignored.display().to_string();
    let unpushed_path = unpushed.display().to_string();
    let submodule_path = submodule.display().to_string();
    git_ok(&repo, &["worktree", "add", "--detach", &clean_path, "HEAD"]);
    git_ok(&repo, &["worktree", "add", "--detach", &dirty_path, "HEAD"]);
    git_ok(
        &repo,
        &["worktree", "add", "--detach", &untracked_path, "HEAD"],
    );
    git_ok(
        &repo,
        &["worktree", "add", "--detach", &ignored_path, "HEAD"],
    );
    git_ok(
        &repo,
        &["worktree", "add", "--detach", &unpushed_path, "HEAD"],
    );
    git_ok(
        &repo,
        &["worktree", "add", "--detach", &submodule_path, "HEAD"],
    );
    fs::write(dirty.join("file.txt"), b"dirty\n").unwrap();
    fs::write(untracked.join("extra.txt"), b"extra").unwrap();
    fs::write(ignored.join("noise.log"), b"ignored").unwrap();
    fs::write(unpushed.join("local.txt"), b"local").unwrap();
    git_ok(&unpushed, &["add", "local.txt"]);
    git_ok(&unpushed, &["commit", "-m", "local"]);
    fs::write(submodule.join(".gitmodules"), b"[submodule \"vendor\"]\n").unwrap();
    fs::create_dir_all(home.join("worktrees/embedded")).unwrap();
    git_ok(&home.join("worktrees/embedded"), &["init"]);

    let observation = scan_home(AiProvider::ClaudeCode, &home);
    let reason = |relative: &str| {
        observation.providers[0]
            .rows
            .iter()
            .find(|row| row.relative_path == relative)
            .map(|row| row.reason.clone())
            .unwrap_or_else(|| format!("missing {relative}"))
    };
    assert_eq!(
        reason("worktrees/clean"),
        "selectable",
        "{}",
        reason("worktrees/clean")
    );
    assert_eq!(reason("worktrees/dirty"), "dirty");
    assert_eq!(reason("worktrees/untracked"), "untracked");
    assert_eq!(reason("worktrees/ignored"), "ignored");
    assert_eq!(reason("worktrees/unpushed"), "unpushed");
    assert_eq!(reason("worktrees/submodule"), "submodule");
    assert_eq!(reason("worktrees/embedded"), "shared_git_dir");

    let plan = plan_ai(&observation, AiClass::Worktree).unwrap();
    assert_eq!(selected(&plan), vec!["worktrees/clean".to_string()]);
    let preview = preview_ai(&plan).unwrap();
    let before = fingerprint(&home);
    let trash = RecordingTrash {
        calls: Mutex::new(Vec::new()),
        fail: false,
    };
    let error = execute_ai(&plan, &preview.digest, true, &NoopCancelObserver, &trash)
        .expect_err("deletion policy");
    assert_code(&error, AiExit::Unavailable, "deletion_policy");
    assert!(trash.calls.lock().unwrap().is_empty());
    assert_eq!(fingerprint(&home), before);
    assert!(clean.join("file.txt").is_file());
    assert!(!home.join(".devsweep-recovery").exists());

    let git_dir = git_stdout(&clean, &["rev-parse", "--absolute-git-dir"]);
    fs::write(Path::new(git_dir.trim()).join("index.lock"), b"busy").unwrap();
    let locked = scan_home(AiProvider::ClaudeCode, &home);
    assert_eq!(
        locked.providers[0]
            .rows
            .iter()
            .find(|row| row.relative_path == "worktrees/clean")
            .map(|row| row.reason.as_str()),
        Some("active_owner")
    );
    let locked_plan = plan_ai(&locked, AiClass::Worktree).unwrap();
    assert!(selected(&locked_plan).is_empty());
}

fn git_ok(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {args:?} in {} failed: {}",
        cwd.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_stdout(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}
