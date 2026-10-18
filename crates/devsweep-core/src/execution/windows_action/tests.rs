use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use super::{
    ActionCapability, HelperFault, InjectedElevation, RefuseReason, RejectReason, UnknownKind,
    WindowsActionDispatch, WindowsActionOutcome, execute_windows_action, prepare_windows_fixture,
};
use super::{audit, registry};

const FILE_A: &[u8] = b"file-a\0\xff";
const REG_A: &[u8] = b"reg-a\0\xff";

fn helper_exe() -> PathBuf {
    let mut path = std::env::current_exe().expect("test executable");
    path.pop();
    if path.file_name().and_then(|name| name.to_str()) == Some("deps") {
        path.pop();
    }
    path.push(format!(
        "windows-action-helper{}",
        std::env::consts::EXE_SUFFIX
    ));
    assert!(
        path.is_file(),
        "windows-action-helper is missing at {}",
        path.display()
    );
    path
}

fn setup(
    file_bytes: &[u8],
    registry_bytes: &[u8],
) -> (tempfile::TempDir, super::PreparedFixture, PathBuf) {
    let directory = tempfile::TempDir::new().expect("temp fixture");
    let helper = helper_exe();
    let prepared = prepare_windows_fixture(directory.path(), file_bytes, registry_bytes, &helper)
        .expect("prepare isolated fixture");
    (directory, prepared, helper)
}

fn request(prepared: &super::PreparedFixture, operation_id: &str, operation: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schema_version": 1,
        "operation_id": operation_id,
        "operation": operation,
        "preview_digest": prepared.preview_digest(),
        "caller_user_sid": prepared.caller_user_sid(),
        "tool_identity_hash": prepared.tool_identity_hash(),
        "target_identity": prepared.target_identity(),
    }))
    .expect("request json")
}

fn dispatch<'a>(
    helper: &'a Path,
    elevation: InjectedElevation,
    timeout: Duration,
    fault: Option<HelperFault>,
    gate: Option<&'a Path>,
    cancel: bool,
) -> WindowsActionDispatch<'a> {
    WindowsActionDispatch {
        helper_path: helper,
        elevation,
        timeout,
        fault,
        gate_dir: gate,
        cancel_before_dispatch: cancel,
    }
}

fn consented(helper: &Path) -> WindowsActionDispatch<'_> {
    dispatch(
        helper,
        InjectedElevation::Consented,
        Duration::from_secs(20),
        None,
        None,
        false,
    )
}

fn subject_file(prepared: &super::PreparedFixture) -> PathBuf {
    prepared.root().join("subject.bin")
}

fn subject_hive(prepared: &super::PreparedFixture) -> PathBuf {
    prepared.root().join("subject.hiv")
}

fn overwrite(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .expect("open subject");
    file.write_all(bytes).expect("write subject");
    file.sync_all().expect("sync subject");
}

fn replace_file(path: &Path, bytes: &[u8]) {
    fs::remove_file(path).expect("remove subject");
    fs::write(path, bytes).expect("recreate subject");
}

fn starts(prepared: &super::PreparedFixture, operation_id: &str) -> usize {
    audit::read(prepared.root())
        .expect("journal")
        .iter()
        .filter(|record| {
            record.operation_id == operation_id
                && record.phase == audit::AuditPhase::DispatchStarted
        })
        .count()
}

fn assert_unchanged(prepared: &super::PreparedFixture, file_bytes: &[u8], registry_bytes: &[u8]) {
    assert_eq!(fs::read(subject_file(prepared)).expect("file"), file_bytes);
    assert_eq!(
        registry::read_value(&subject_hive(prepared)).expect("registry value"),
        registry_bytes
    );
}

// Offline registry fixtures (`registry::create_hive` / `replace_value_in_place`)
// return `Unsupported` off Windows, and the helper binary is a Windows target.
#[cfg(windows)]
#[test]
fn ac1_forged_replayed_stale_cross_user_and_tool_or_path_replacement_fail_before_dispatch() {
    let (_dir, prepared, helper) = setup(FILE_A, REG_A);
    let run = consented(&helper);

    let forged = serde_json::json!({
        "schema_version": 1,
        "operation_id": "forged-op",
        "operation": "fixture.backup",
        "preview_digest": prepared.preview_digest(),
        "caller_user_sid": prepared.caller_user_sid(),
        "tool_identity_hash": prepared.tool_identity_hash(),
        "target_identity": prepared.target_identity(),
        "program": "cmd.exe",
        "argv": ["/c", "whoami"],
        "shell": "cmd /c whoami",
        "registry_path": "HKLM\\SOFTWARE\\DevSweep",
    });
    let report = execute_windows_action(
        &prepared,
        &serde_json::to_vec(&forged).expect("forged json"),
        &run,
    );
    assert_eq!(
        report.outcome,
        WindowsActionOutcome::Rejected(RejectReason::Forged)
    );
    assert!(!report.outcome.is_success());
    assert_eq!(starts(&prepared, "forged-op"), 0);

    let unsupported = request(&prepared, "unsupported-op", "cmd.exe");
    let report = execute_windows_action(&prepared, &unsupported, &run);
    assert_eq!(
        report.outcome,
        WindowsActionOutcome::Rejected(RejectReason::UnsupportedOperation)
    );
    assert_eq!(report.capability, ActionCapability::Unsupported);
    assert_eq!(starts(&prepared, "unsupported-op"), 0);

    let external = request(&prepared, "external-op", "external.confirm");
    let report = execute_windows_action(&prepared, &external, &run);
    assert_eq!(
        report.outcome,
        WindowsActionOutcome::Rejected(RejectReason::ExternalConfirmation)
    );
    assert_eq!(report.capability, ActionCapability::ExternalConfirmation);
    assert_eq!(starts(&prepared, "external-op"), 0);

    let mut stale = request(&prepared, "stale-op", "fixture.identity_check");
    let text = String::from_utf8(stale.clone()).expect("utf8");
    let digest = prepared.preview_digest();
    let flipped = if digest.ends_with('a') { 'b' } else { 'a' };
    let updated = text.replacen(
        digest,
        &format!("{}{flipped}", &digest[..digest.len() - 1]),
        1,
    );
    stale = updated.into_bytes();
    let report = execute_windows_action(&prepared, &stale, &run);
    assert_eq!(
        report.outcome,
        WindowsActionOutcome::Rejected(RejectReason::StalePreview)
    );
    assert_eq!(starts(&prepared, "stale-op"), 0);

    let mut cross_user = serde_json::from_slice::<serde_json::Value>(&request(
        &prepared,
        "cross-user-op",
        "fixture.backup",
    ))
    .expect("request");
    cross_user["caller_user_sid"] = serde_json::Value::String("S-1-0-0".to_string());
    let report = execute_windows_action(
        &prepared,
        &serde_json::to_vec(&cross_user).expect("cross user"),
        &run,
    );
    assert_eq!(
        report.outcome,
        WindowsActionOutcome::Rejected(RejectReason::CrossUser)
    );
    assert_eq!(starts(&prepared, "cross-user-op"), 0);

    let copy_dir = tempfile::TempDir::new().expect("tool copy dir");
    let copy = copy_dir
        .path()
        .join(format!("helper-copy{}", std::env::consts::EXE_SUFFIX));
    fs::copy(&helper, &copy).expect("copy helper");
    let tool_root = tempfile::TempDir::new().expect("tool fixture");
    let tool_plan = prepare_windows_fixture(tool_root.path(), FILE_A, REG_A, &copy)
        .expect("prepare against copied helper");
    let mut tampered = OpenOptions::new()
        .append(true)
        .open(&copy)
        .expect("open copied helper");
    tampered.write_all(&[0]).expect("change helper bytes");
    tampered.sync_all().expect("sync helper");
    let tool_request = request(&tool_plan, "tool-op", "fixture.backup");
    let tool_run = consented(&copy);
    let report = execute_windows_action(&tool_plan, &tool_request, &tool_run);
    assert_eq!(
        report.outcome,
        WindowsActionOutcome::Rejected(RejectReason::ToolReplacement)
    );
    assert_eq!(starts(&tool_plan, "tool-op"), 0);
    assert_unchanged(&tool_plan, FILE_A, REG_A);

    let report = execute_windows_action(
        &prepared,
        &request(&prepared, "check-op", "fixture.identity_check"),
        &dispatch(
            &helper,
            InjectedElevation::Declined,
            Duration::from_secs(20),
            None,
            None,
            false,
        ),
    );
    assert_eq!(report.outcome, WindowsActionOutcome::Succeeded);
    assert_eq!(report.capability, ActionCapability::UserScope);
    let again = execute_windows_action(
        &prepared,
        &request(&prepared, "check-op", "fixture.identity_check"),
        &run,
    );
    assert_eq!(
        again.outcome,
        WindowsActionOutcome::Rejected(RejectReason::Replayed)
    );
    assert!(!again.outcome.is_success());
    assert_eq!(starts(&prepared, "check-op"), 1);

    replace_file(&subject_file(&prepared), b"replaced-subject");
    let report = execute_windows_action(
        &prepared,
        &request(&prepared, "path-op", "fixture.restore"),
        &run,
    );
    assert_eq!(
        report.outcome,
        WindowsActionOutcome::Rejected(RejectReason::PathReplacement)
    );
    assert_eq!(starts(&prepared, "path-op"), 0);
    assert_eq!(
        fs::read(subject_file(&prepared)).expect("replaced file"),
        b"replaced-subject"
    );
}

#[cfg(windows)]
#[test]
fn ac2_decline_cancel_disconnect_timeout_crash_and_audit_failure_do_not_retry() {
    let (_dir, prepared, helper) = setup(FILE_A, REG_A);

    let declined = execute_windows_action(
        &prepared,
        &request(&prepared, "decline-op", "fixture.backup"),
        &dispatch(
            &helper,
            InjectedElevation::Declined,
            Duration::from_secs(20),
            None,
            None,
            false,
        ),
    );
    assert_eq!(
        declined.outcome,
        WindowsActionOutcome::AuthorizationDeclined
    );
    assert!(!declined.outcome.is_success());
    assert_eq!(starts(&prepared, "decline-op"), 0);
    let declined_again = execute_windows_action(
        &prepared,
        &request(&prepared, "decline-op", "fixture.backup"),
        &consented(&helper),
    );
    assert_eq!(
        declined_again.outcome,
        WindowsActionOutcome::Rejected(RejectReason::Replayed)
    );
    assert_eq!(starts(&prepared, "decline-op"), 0);
    assert_unchanged(&prepared, FILE_A, REG_A);

    let missing = execute_windows_action(
        &prepared,
        &request(&prepared, "missing-prompt-op", "fixture.restore"),
        &dispatch(
            &helper,
            InjectedElevation::MissingPrompt,
            Duration::from_secs(20),
            None,
            None,
            false,
        ),
    );
    assert_eq!(missing.outcome, WindowsActionOutcome::ConsentMissing);
    assert!(!missing.outcome.is_success());
    assert_eq!(starts(&prepared, "missing-prompt-op"), 0);

    let canceled = execute_windows_action(
        &prepared,
        &request(&prepared, "cancel-op", "fixture.backup"),
        &dispatch(
            &helper,
            InjectedElevation::Consented,
            Duration::from_secs(20),
            None,
            None,
            true,
        ),
    );
    assert_eq!(
        canceled.outcome,
        WindowsActionOutcome::CanceledBeforeDispatch
    );
    assert_eq!(starts(&prepared, "cancel-op"), 0);
    assert_unchanged(&prepared, FILE_A, REG_A);

    let hung = execute_windows_action(
        &prepared,
        &request(&prepared, "timeout-op", "fixture.backup"),
        &dispatch(
            &helper,
            InjectedElevation::Consented,
            Duration::from_millis(400),
            Some(HelperFault::Hang),
            None,
            false,
        ),
    );
    assert_eq!(
        hung.outcome,
        WindowsActionOutcome::Unknown(UnknownKind::Timeout)
    );
    assert!(!hung.outcome.is_success());
    assert_eq!(starts(&prepared, "timeout-op"), 1);
    let hung_again = execute_windows_action(
        &prepared,
        &request(&prepared, "timeout-op", "fixture.backup"),
        &consented(&helper),
    );
    assert_eq!(
        hung_again.outcome,
        WindowsActionOutcome::Rejected(RejectReason::Replayed)
    );
    assert_eq!(starts(&prepared, "timeout-op"), 1);

    let crashed = execute_windows_action(
        &prepared,
        &request(&prepared, "crash-op", "fixture.backup"),
        &dispatch(
            &helper,
            InjectedElevation::Consented,
            Duration::from_secs(5),
            Some(HelperFault::Crash),
            None,
            false,
        ),
    );
    assert_eq!(
        crashed.outcome,
        WindowsActionOutcome::Unknown(UnknownKind::Crash)
    );
    assert_eq!(starts(&prepared, "crash-op"), 1);

    let disconnected = execute_windows_action(
        &prepared,
        &request(&prepared, "disconnect-op", "fixture.backup"),
        &dispatch(
            &helper,
            InjectedElevation::Consented,
            Duration::from_secs(5),
            Some(HelperFault::Disconnect),
            None,
            false,
        ),
    );
    assert_eq!(
        disconnected.outcome,
        WindowsActionOutcome::Unknown(UnknownKind::Disconnect)
    );
    assert_eq!(starts(&prepared, "disconnect-op"), 1);

    let missing_terminal = execute_windows_action(
        &prepared,
        &request(&prepared, "missing-terminal-op", "fixture.backup"),
        &dispatch(
            &helper,
            InjectedElevation::Consented,
            Duration::from_secs(5),
            Some(HelperFault::MissingTerminal),
            None,
            false,
        ),
    );
    assert_eq!(
        missing_terminal.outcome,
        WindowsActionOutcome::Unknown(UnknownKind::MissingTerminal)
    );
    assert_eq!(starts(&prepared, "missing-terminal-op"), 1);
    assert_unchanged(&prepared, FILE_A, REG_A);

    let (_audit_dir, audit_plan, audit_helper) = setup(FILE_A, REG_A);
    fs::write(audit_plan.root().join("audit"), b"not a directory").expect("block audit");
    let audit_failure = execute_windows_action(
        &audit_plan,
        &request(&audit_plan, "audit-op", "fixture.backup"),
        &consented(&audit_helper),
    );
    assert_eq!(audit_failure.outcome, WindowsActionOutcome::AuditFailed);
    assert!(!audit_failure.outcome.is_success());
    assert_eq!(starts(&audit_plan, "audit-op"), 0);
    assert_unchanged(&audit_plan, FILE_A, REG_A);
}

#[cfg(windows)]
#[test]
fn ac3_backup_readback_conditional_restore_and_post_handoff_refusal() {
    let (_dir, prepared, helper) = setup(FILE_A, REG_A);
    let hive = fs::read(subject_hive(&prepared)).expect("hive bytes");
    assert_eq!(&hive[..4], b"regf");
    assert!(registry::hive_is_offline_file(&subject_hive(&prepared)).expect("hive header"));

    let backup = execute_windows_action(
        &prepared,
        &request(&prepared, "backup-1", "fixture.backup"),
        &consented(&helper),
    );
    assert_eq!(backup.outcome, WindowsActionOutcome::Succeeded);
    assert_eq!(backup.capability, ActionCapability::Elevated);
    assert_eq!(starts(&prepared, "backup-1"), 1);
    assert_eq!(
        fs::read(prepared.root().join("backup/v1/file.bin")).expect("backup file"),
        FILE_A
    );
    assert_eq!(
        fs::read(prepared.root().join("backup/v1/registry-value.bin")).expect("backup value"),
        REG_A
    );
    assert_unchanged(&prepared, FILE_A, REG_A);
    let journal =
        fs::read_to_string(prepared.root().join("audit/v1/actions.jsonl")).expect("journal");
    for line in journal.lines() {
        let value: serde_json::Value = serde_json::from_str(line).expect("audit json");
        let object = value.as_object().expect("audit object");
        for key in [
            "program",
            "argv",
            "args",
            "shell",
            "registry_path",
            "path",
            "command",
        ] {
            assert!(!object.contains_key(key), "{key} leaked into the journal");
        }
    }

    let identity_before = prepared.target_identity().to_string();
    overwrite(&subject_file(&prepared), b"file-b");
    registry::replace_value_in_place(&subject_hive(&prepared), b"reg-b").expect("rewrite hive");
    let live = super::read_subject(prepared.root()).expect("live subject");
    assert_eq!(
        live.target_identity, identity_before,
        "in-place edit changed identity"
    );
    assert_eq!(live.registry_value, b"reg-b");

    let restored = execute_windows_action(
        &prepared,
        &request(&prepared, "restore-1", "fixture.restore"),
        &consented(&helper),
    );
    assert_eq!(restored.outcome, WindowsActionOutcome::Succeeded);
    assert_unchanged(&prepared, FILE_A, REG_A);

    overwrite(&subject_file(&prepared), b"file-c");
    registry::replace_value_in_place(&subject_hive(&prepared), b"reg-c").expect("second rewrite");
    let second = execute_windows_action(
        &prepared,
        &request(&prepared, "backup-2", "fixture.backup"),
        &consented(&helper),
    );
    assert_eq!(second.outcome, WindowsActionOutcome::Succeeded);
    overwrite(&subject_file(&prepared), b"file-d");
    registry::replace_value_in_place(&subject_hive(&prepared), b"reg-d").expect("third rewrite");
    let latest = execute_windows_action(
        &prepared,
        &request(&prepared, "restore-2", "fixture.restore"),
        &consented(&helper),
    );
    assert_eq!(latest.outcome, WindowsActionOutcome::Succeeded);
    assert_unchanged(&prepared, b"file-c", b"reg-c");
    assert_eq!(
        fs::read(prepared.root().join("backup/v1/file.bin")).expect("old backup"),
        FILE_A
    );

    let gate = prepared.root().join("gate");
    fs::create_dir(&gate).expect("gate dir");
    let ready = gate.join("ready");
    let go = gate.join("go");
    let file_path = subject_file(&prepared);
    let watcher = thread::spawn(move || {
        let started = std::time::Instant::now();
        while !ready.exists() && started.elapsed() < Duration::from_secs(15) {
            thread::sleep(Duration::from_millis(20));
        }
        if ready.exists() {
            let _ = fs::remove_file(&file_path);
            let _ = fs::write(&file_path, b"replaced-during-gate");
            let _ = fs::write(&go, b"go");
        }
    });
    let handed_off = execute_windows_action(
        &prepared,
        &request(&prepared, "restore-4", "fixture.restore"),
        &dispatch(
            &helper,
            InjectedElevation::Consented,
            Duration::from_secs(20),
            None,
            Some(&gate),
            false,
        ),
    );
    watcher.join().expect("gate watcher");
    assert_eq!(
        handed_off.outcome,
        WindowsActionOutcome::Refused(RefuseReason::RestoreSubjectChanged)
    );
    assert!(!handed_off.outcome.is_success());
    assert_eq!(starts(&prepared, "restore-4"), 1);
    assert_eq!(
        fs::read(subject_file(&prepared)).expect("gate replacement"),
        b"replaced-during-gate"
    );
    let records = audit::read(prepared.root()).expect("journal");
    let terminal = audit::terminal_for(&records, "restore-4").expect("refusal terminal");
    assert_eq!(terminal.outcome.as_deref(), Some("refused"));
    assert_eq!(terminal.reason.as_deref(), Some("restore_subject_changed"));
}
