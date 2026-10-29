use std::{ffi::OsString, fs, path::Path, time::Duration};

use super::{
    AiClass, AiDisposition, AiError, AiProvider, AiRow, check_cancel, io_unavailable,
    protected_component,
    scan::{content_hash, entry_is_safe, lock_present, read_dir_sorted, utf8_name},
};
use crate::{
    filesystem::{path_is_within, paths_equal},
    process::{CancelObserver, CwdPolicy, ProcessRequest, ProcessRunner, ProcessStatus},
};

pub(crate) fn classify_worktrees(
    home: &Path,
    provider: AiProvider,
    cancel: &dyn CancelObserver,
    rows: &mut Vec<AiRow>,
) -> Result<(), AiError> {
    let root = home.join("worktrees");
    if lock_present(&root) {
        rows.push(worktree_row(
            provider,
            "worktrees",
            AiDisposition::InspectOnly,
            "locked",
            None,
        ));
        return Ok(());
    }
    for entry in
        read_dir_sorted(&root).map_err(|error| io_unavailable(error, "failed to read worktrees"))?
    {
        check_cancel(cancel)?;
        let Some(name) = utf8_name(&entry) else {
            continue;
        };
        let relative = format!("worktrees/{name}");
        let path = entry.path();
        if protected_name(&name) || !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            rows.push(worktree_row(
                provider,
                &relative,
                AiDisposition::UnsupportedVersion,
                "unsupported_version",
                None,
            ));
            continue;
        }
        if !entry_is_safe(&path) {
            rows.push(worktree_row(
                provider,
                &relative,
                AiDisposition::InspectOnly,
                "unsupported_version",
                None,
            ));
            continue;
        }
        let reason = worktree_refusal(&path, cancel);
        if reason.is_empty() {
            match content_hash(&path) {
                Ok(hash) => rows.push(worktree_row(
                    provider,
                    &relative,
                    AiDisposition::Candidate,
                    "selectable",
                    Some(hash),
                )),
                Err(hash_reason) => rows.push(worktree_row(
                    provider,
                    &relative,
                    AiDisposition::InspectOnly,
                    hash_reason,
                    None,
                )),
            }
        } else {
            rows.push(worktree_row(
                provider,
                &relative,
                AiDisposition::InspectOnly,
                reason,
                None,
            ));
        }
    }
    Ok(())
}

fn protected_name(name: &str) -> bool {
    protected_component(name)
}

/// True when the directory carries a Git marker: a `.git` directory in a main
/// repository or a `.git` file naming a linked worktree's gitdir. Presence
/// alone does not prove validity; the git checks still judge the marker.
pub(crate) fn git_marker_present(worktree: &Path) -> bool {
    fs::metadata(worktree.join(".git"))
        .map(|metadata| metadata.is_dir() || metadata.is_file())
        .unwrap_or(false)
}

/// Empty string means every safety check passed.
pub(crate) fn worktree_refusal(worktree: &Path, cancel: &dyn CancelObserver) -> &'static str {
    if cancel.is_cancel_requested() {
        return "canceled";
    }
    // Without a `.git` marker git could only report that the directory is not
    // a repository, after an error-reporting console launch on Windows.
    // Refuse first so no git process is spawned for plain directories.
    if !git_marker_present(worktree) {
        return "not_registered";
    }
    let inside = match git(worktree, &["rev-parse", "--is-inside-work-tree"], cancel) {
        GitRead::Missing => return "git_unavailable",
        GitRead::Canceled => return "canceled",
        GitRead::Failed => return "unknown",
        GitRead::Output(text) => text,
    };
    if inside.trim() != "true" {
        return "not_registered";
    }
    let listed = match git(worktree, &["worktree", "list", "--porcelain"], cancel) {
        GitRead::Missing => return "git_unavailable",
        GitRead::Canceled => return "canceled",
        GitRead::Failed => return "unknown",
        GitRead::Output(text) => text,
    };
    if !listed.lines().any(|line| {
        line.strip_prefix("worktree ")
            .is_some_and(|path| paths_equal(Path::new(path.trim()), worktree))
    }) {
        return "not_registered";
    }
    let git_dir = match git(worktree, &["rev-parse", "--absolute-git-dir"], cancel) {
        GitRead::Missing => return "git_unavailable",
        GitRead::Canceled => return "canceled",
        GitRead::Failed => return "unknown",
        GitRead::Output(text) => text,
    };
    let git_dir = Path::new(git_dir.trim());
    if path_is_within(worktree, git_dir) {
        return "shared_git_dir";
    }
    if lock_present(git_dir) {
        return "active_owner";
    }
    if worktree.join(".gitmodules").is_file() {
        return "submodule";
    }
    match git(worktree, &["submodule", "status"], cancel) {
        GitRead::Missing => return "git_unavailable",
        GitRead::Canceled => return "canceled",
        GitRead::Failed => return "unknown",
        GitRead::Output(text) if !text.trim().is_empty() => return "submodule",
        GitRead::Output(_) => {}
    }
    let status = match git(worktree, &["status", "--porcelain=v1", "-uall"], cancel) {
        GitRead::Missing => return "git_unavailable",
        GitRead::Canceled => return "canceled",
        GitRead::Failed => return "unknown",
        GitRead::Output(text) => text,
    };
    if status.lines().any(|line| line.starts_with("??")) {
        return "untracked";
    }
    if !status.trim().is_empty() {
        return "dirty";
    }
    let ignored = match git(
        worktree,
        &["status", "--porcelain=v1", "--ignored=matching", "-uall"],
        cancel,
    ) {
        GitRead::Missing => return "git_unavailable",
        GitRead::Canceled => return "canceled",
        GitRead::Failed => return "unknown",
        GitRead::Output(text) => text,
    };
    if ignored.lines().any(|line| line.starts_with("!!")) {
        return "ignored";
    }
    let unpushed = match git(
        worktree,
        &["rev-list", "--count", "HEAD", "--not", "--remotes"],
        cancel,
    ) {
        GitRead::Missing => return "git_unavailable",
        GitRead::Canceled => return "canceled",
        GitRead::Failed => return "unknown",
        GitRead::Output(text) => text,
    };
    match unpushed.trim().parse::<u64>() {
        Ok(0) => {}
        Ok(_) => return "unpushed",
        Err(_) => return "unknown",
    }
    ""
}

enum GitRead {
    Output(String),
    Missing,
    Failed,
    Canceled,
}

fn git(worktree: &Path, args: &[&str], cancel: &dyn CancelObserver) -> GitRead {
    let mut argv = Vec::with_capacity(args.len() + 2);
    argv.push(OsString::from("-C"));
    argv.push(worktree.as_os_str().to_owned());
    argv.extend(args.iter().copied().map(OsString::from));
    let request = ProcessRequest {
        program: OsString::from("git"),
        args: argv,
        cwd: CwdPolicy::Neutral,
        timeout: Some(Duration::from_secs(30)),
        job_deadline: None,
        cancel,
    };
    let result = ProcessRunner::default().run(&request);
    let stdout = String::from_utf8_lossy(&result.output.stdout).into_owned();
    match result.status {
        ProcessStatus::Success => GitRead::Output(stdout),
        ProcessStatus::NotFound => GitRead::Missing,
        ProcessStatus::Canceled => GitRead::Canceled,
        ProcessStatus::Exit { .. } | ProcessStatus::Timeout | ProcessStatus::InvalidOutput => {
            GitRead::Failed
        }
    }
}

fn worktree_row(
    provider: AiProvider,
    relative: &str,
    disposition: AiDisposition,
    reason: &str,
    content_sha256: Option<String>,
) -> AiRow {
    AiRow {
        id: format!(
            "ai:{}:{}:{relative}",
            provider.as_str(),
            AiClass::Worktree.as_str()
        ),
        class: AiClass::Worktree,
        relative_path: relative.to_string(),
        disposition,
        reason: reason.to_string(),
        content_sha256,
    }
}
