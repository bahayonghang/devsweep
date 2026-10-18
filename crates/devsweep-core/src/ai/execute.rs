use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    AiClass, AiError, AiExecuteReportV1, AiPlanKind, AiPlanTarget, AiPlanV1, AiProvider,
    AiRecoveryBundle, AiRecoveryListV1, SCHEMA_EXECUTE, SCHEMA_RECOVERY, SCHEMA_RECOVERY_LIST,
    check_cancel, hex_encode, is_real_dir, normalize_home,
};
use crate::{execution::TrashRunner, process::CancelObserver};

/// Move confirmed cleanup targets to trash after a recovery bundle, or restore
/// a bundle when the destination is absent. A failed move is not retried.
pub fn execute_ai(
    plan: &AiPlanV1,
    preview_digest: &str,
    confirmed: bool,
    cancel: &dyn CancelObserver,
    trash: &dyn TrashRunner,
) -> Result<AiExecuteReportV1, AiError> {
    check_cancel(cancel)?;
    if !confirmed {
        return Err(AiError::usage(
            "confirmation_required",
            "AI execute requires confirmation",
        ));
    }
    let preview = super::plan::preview_ai(plan)?;
    if preview.digest != preview_digest {
        return Err(AiError::stale(
            "stale_confirmation",
            "the live AI identity does not match the preview digest",
        ));
    }
    if plan.kind == AiPlanKind::Cleanup {
        super::plan::still_candidate(plan, cancel)?;
        if plan.class == AiClass::Worktree && !plan.targets.is_empty() {
            return Err(AiError::unavailable(
                "deletion_policy",
                "trash cannot keep Git worktree metadata consistent",
            ));
        }
        return execute_cleanup(plan, trash);
    }
    execute_restore(plan)
}

/// List recovery bundles. Does not restore.
pub fn recovery_list(home: &Path) -> Result<AiRecoveryListV1, AiError> {
    let home = require_home(home)?;
    let normalized = normalize_home(&home)?;
    let root = home.join(".devsweep-recovery");
    if !root.exists() {
        return Ok(empty_list(normalized));
    }
    if !is_real_dir(&root) {
        return Err(AiError::unavailable(
            "source_unavailable",
            "recovery store is unavailable",
        ));
    }
    let mut bundles = Vec::new();
    for entry in super::scan::read_dir_sorted(&root)
        .map_err(|error| super::io_unavailable(error, "failed to read recovery bundles"))?
    {
        let Some(name) = super::scan::utf8_name(&entry) else {
            continue;
        };
        if name.starts_with('.') || !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        if let Ok(bundle) = load_bundle(&home, &name) {
            bundles.push(AiRecoveryBundle {
                id: bundle.manifest.id,
                provider: bundle.manifest.provider,
                class: bundle.manifest.class,
                relative_path: bundle.manifest.relative_path,
                content_sha256: bundle.manifest.content_sha256,
            });
        }
    }
    bundles.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(AiRecoveryListV1 {
        schema: SCHEMA_RECOVERY_LIST.to_string(),
        version: 1,
        home: normalized,
        bundles,
    })
}

/// Build a restore plan. Does not copy bytes back.
pub fn restore_plan(home: &Path, bundle_id: &str) -> Result<AiPlanV1, AiError> {
    let home = require_home(home)?;
    let bundle = load_bundle(&home, bundle_id)?;
    verify_bundle(&bundle)?;
    let bundle_id = bundle.manifest.id.clone();
    Ok(AiPlanV1 {
        schema: super::SCHEMA_PLAN.to_string(),
        version: 1,
        kind: AiPlanKind::Restore,
        provider: bundle.manifest.provider,
        home: normalize_home(&home)?,
        class: bundle.manifest.class,
        bundle_id: Some(bundle_id.clone()),
        targets: vec![AiPlanTarget {
            id: format!("restore:{bundle_id}"),
            relative_path: bundle.manifest.relative_path,
            content_sha256: bundle.manifest.content_sha256,
            action: "restore_bytes".to_string(),
        }],
        refused: Vec::new(),
    })
}

pub(crate) fn live_bundle_hash(plan: &AiPlanV1) -> Result<String, AiError> {
    let home = PathBuf::from(&plan.home);
    let bundle_id = plan
        .bundle_id
        .as_deref()
        .ok_or_else(|| AiError::usage("invalid_plan", "restore plan is missing its bundle"))?;
    let bundle = load_bundle(&home, bundle_id)?;
    verify_bundle(&bundle)?;
    Ok(bundle.manifest.content_sha256)
}

pub(crate) fn restore_index(plan: &AiPlanV1) -> Result<String, AiError> {
    let home = PathBuf::from(&plan.home);
    let bundle_hash = live_bundle_hash(plan)?;
    let mut state = String::from("absent");
    for target in &plan.targets {
        let destination = super::plan::resolve_inside(&home, &target.relative_path)?;
        if destination.exists() {
            state = super::scan::content_hash(&destination)
                .unwrap_or_else(|_| "unreadable".to_string());
        }
    }
    let mut hasher = Sha256::new();
    hasher.update(bundle_hash.as_bytes());
    hasher.update([0]);
    hasher.update(state.as_bytes());
    Ok(hex_encode(hasher.finalize().as_slice()))
}

fn execute_cleanup(plan: &AiPlanV1, trash: &dyn TrashRunner) -> Result<AiExecuteReportV1, AiError> {
    let home = PathBuf::from(&plan.home);
    let mut bundle_ids = Vec::new();
    let mut moved = Vec::new();
    for target in &plan.targets {
        let source = super::plan::resolve_inside(&home, &target.relative_path)?;
        if !source.exists() {
            return Err(AiError::unavailable(
                "source_unavailable",
                "AI target is unavailable",
            ));
        }
        let bundle_id = match write_bundle(&home, plan, target, &source) {
            Ok(bundle_id) => bundle_id,
            Err(error) => {
                if moved.is_empty() {
                    return Err(error);
                }
                return Ok(report(
                    "partial",
                    bundle_ids,
                    moved,
                    Some("partial_execution"),
                ));
            }
        };
        bundle_ids.push(bundle_id);
        match trash.move_to_trash(&source) {
            Ok(()) => moved.push(target.id.clone()),
            Err(_) => {
                let code = if source.exists() {
                    "move_failed"
                } else {
                    "unknown_after_dispatch"
                };
                let outcome = if source.exists() { "failed" } else { "unknown" };
                return Ok(report(outcome, bundle_ids, moved, Some(code)));
            }
        }
    }
    Ok(report("succeeded", bundle_ids, moved, None))
}

fn execute_restore(plan: &AiPlanV1) -> Result<AiExecuteReportV1, AiError> {
    let home = PathBuf::from(&plan.home);
    let bundle_id = plan
        .bundle_id
        .as_deref()
        .ok_or_else(|| AiError::usage("invalid_plan", "restore plan is missing its bundle"))?;
    let bundle = load_bundle(&home, bundle_id)?;
    verify_bundle(&bundle)?;
    for target in &plan.targets {
        if target.content_sha256 != bundle.manifest.content_sha256 {
            return Err(restore_refused());
        }
        let destination = super::plan::resolve_inside(&home, &target.relative_path)?;
        if destination.exists() {
            return Err(restore_refused());
        }
        restore_payload(&bundle.payload, &destination)?;
    }
    Ok(report(
        "succeeded",
        vec![bundle.manifest.id],
        Vec::new(),
        None,
    ))
}

fn restore_refused() -> AiError {
    AiError::stale(
        "restore_refused",
        "restore refused because the destination exists, the bundle hash differs, or the live file changed",
    )
}

fn write_bundle(
    home: &Path,
    plan: &AiPlanV1,
    target: &AiPlanTarget,
    source: &Path,
) -> Result<String, AiError> {
    let recovery = home.join(".devsweep-recovery");
    if recovery.exists() && !is_real_dir(&recovery) {
        return Err(AiError::failed(
            "bundle_write_failed",
            "recovery bundle was not durable; the home was left unchanged",
        ));
    }
    fs::create_dir_all(&recovery).map_err(|error| {
        AiError::failed(
            "bundle_write_failed",
            format!("recovery bundle was not durable: {error}"),
        )
    })?;
    let content_sha256 = super::scan::content_hash(source).map_err(|_| {
        AiError::failed("bundle_write_failed", "failed to hash the recovery payload")
    })?;
    let id = new_bundle_id(plan.provider, &target.relative_path, &content_sha256);
    let partial_path = recovery.join(format!(".partial-{id}"));
    if partial_path.exists() {
        let _ = fs::remove_dir_all(&partial_path);
    }
    let mut partial = PartialDir {
        path: partial_path,
        armed: true,
    };
    fs::create_dir(&partial.path).map_err(|error| {
        AiError::failed(
            "bundle_write_failed",
            format!("recovery bundle was not durable: {error}"),
        )
    })?;
    let payload = partial.path.join("payload");
    copy_payload(source, &payload)?;
    let manifest = RecoveryManifest {
        schema: SCHEMA_RECOVERY.to_string(),
        version: 1,
        id: id.clone(),
        provider: plan.provider,
        class: plan.class,
        relative_path: target.relative_path.clone(),
        content_sha256,
        kind: if source.is_file() {
            "file".to_string()
        } else {
            "directory".to_string()
        },
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| {
        AiError::failed(
            "bundle_write_failed",
            format!("recovery bundle was not durable: {error}"),
        )
    })?;
    write_synced(&partial.path.join("manifest.json"), &manifest_bytes)?;
    let published = recovery.join(&id);
    if published.exists() {
        return Err(AiError::failed(
            "bundle_write_failed",
            "recovery bundle id already exists",
        ));
    }
    fs::rename(&partial.path, &published).map_err(|error| {
        AiError::failed(
            "bundle_write_failed",
            format!("recovery bundle was not durable: {error}"),
        )
    })?;
    partial.armed = false;
    Ok(id)
}

fn copy_payload(source: &Path, dest: &Path) -> Result<(), AiError> {
    let metadata = fs::symlink_metadata(source).map_err(|error| {
        AiError::failed(
            "bundle_write_failed",
            format!("recovery bundle was not durable: {error}"),
        )
    })?;
    if metadata.file_type().is_symlink() || !super::scan::entry_is_safe(source) {
        return Err(AiError::failed(
            "bundle_write_failed",
            "recovery bundle refused an unsafe path",
        ));
    }
    if metadata.is_file() {
        let bytes = fs::read(source).map_err(|error| {
            AiError::failed(
                "bundle_write_failed",
                format!("recovery bundle was not durable: {error}"),
            )
        })?;
        write_synced(dest, &bytes)?;
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(AiError::failed(
            "bundle_write_failed",
            "recovery bundle refused an unsupported file type",
        ));
    }
    fs::create_dir(dest).map_err(|error| {
        AiError::failed(
            "bundle_write_failed",
            format!("recovery bundle was not durable: {error}"),
        )
    })?;
    for entry in super::scan::read_dir_sorted(source).map_err(|error| {
        AiError::failed(
            "bundle_write_failed",
            format!("recovery bundle was not durable: {error}"),
        )
    })? {
        let name = entry.file_name();
        copy_payload(&entry.path(), &dest.join(name))?;
    }
    Ok(())
}

fn restore_payload(payload: &Path, destination: &Path) -> Result<(), AiError> {
    if destination.exists() {
        return Err(restore_refused());
    }
    let metadata = fs::symlink_metadata(payload).map_err(|_| {
        AiError::unavailable("source_unavailable", "recovery payload is unavailable")
    })?;
    if metadata.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                AiError::failed("restore_failed", format!("failed to restore: {error}"))
            })?;
        }
        let bytes = fs::read(payload).map_err(|_| {
            AiError::unavailable("source_unavailable", "recovery payload is unavailable")
        })?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|error| {
                if destination.exists() {
                    restore_refused()
                } else {
                    AiError::failed("restore_failed", format!("failed to restore: {error}"))
                }
            })?;
        file.write_all(&bytes).map_err(|error| {
            AiError::failed("restore_failed", format!("failed to restore: {error}"))
        })?;
        file.sync_all().map_err(|error| {
            AiError::failed("restore_failed", format!("failed to restore: {error}"))
        })?;
        return Ok(());
    }
    copy_payload(payload, destination)
        .map_err(|error| AiError::failed("restore_failed", error.message))
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), AiError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            AiError::failed(
                "bundle_write_failed",
                format!("recovery bundle was not durable: {error}"),
            )
        })?;
    file.write_all(bytes).map_err(|error| {
        AiError::failed(
            "bundle_write_failed",
            format!("recovery bundle was not durable: {error}"),
        )
    })?;
    file.sync_all().map_err(|error| {
        AiError::failed(
            "bundle_write_failed",
            format!("recovery bundle was not durable: {error}"),
        )
    })?;
    Ok(())
}

fn new_bundle_id(provider: AiProvider, relative: &str, content_sha256: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(provider.as_str().as_bytes());
    hasher.update([0]);
    hasher.update(relative.as_bytes());
    hasher.update([0]);
    hasher.update(content_sha256.as_bytes());
    hasher.update(nanos.to_le_bytes());
    hex_encode(&hasher.finalize()[..16])
}

struct LoadedBundle {
    manifest: RecoveryManifest,
    payload: PathBuf,
}

fn load_bundle(home: &Path, bundle_id: &str) -> Result<LoadedBundle, AiError> {
    if !valid_bundle_id(bundle_id) {
        return Err(AiError::usage(
            "invalid_plan",
            "recovery bundle id is invalid",
        ));
    }
    let dir = home.join(".devsweep-recovery").join(bundle_id);
    if !is_real_dir(&dir) {
        return Err(AiError::unavailable(
            "source_unavailable",
            "recovery bundle is unavailable",
        ));
    }
    let manifest_bytes = fs::read(dir.join("manifest.json")).map_err(|_| {
        AiError::unavailable("source_unavailable", "recovery bundle is unavailable")
    })?;
    let manifest = serde_json::from_slice::<RecoveryManifest>(&manifest_bytes).map_err(|_| {
        AiError::stale(
            "restore_refused",
            "recovery bundle metadata is not the closed schema",
        )
    })?;
    if manifest.version != 1 || manifest.schema != SCHEMA_RECOVERY || manifest.id != bundle_id {
        return Err(AiError::stale(
            "restore_refused",
            "recovery bundle metadata does not match its id",
        ));
    }
    Ok(LoadedBundle {
        manifest,
        payload: dir.join("payload"),
    })
}

fn verify_bundle(bundle: &LoadedBundle) -> Result<(), AiError> {
    let kind_matches = match bundle.manifest.kind.as_str() {
        "file" => bundle.payload.is_file(),
        "directory" => bundle.payload.is_dir(),
        _ => false,
    };
    if !kind_matches {
        return Err(restore_refused());
    }
    let hash = super::scan::content_hash(&bundle.payload).map_err(|_| {
        AiError::unavailable("source_unavailable", "recovery payload is unavailable")
    })?;
    if hash != bundle.manifest.content_sha256 {
        return Err(restore_refused());
    }
    Ok(())
}

fn valid_bundle_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn require_home(home: &Path) -> Result<PathBuf, AiError> {
    if !home.is_absolute() || !is_real_dir(home) {
        return Err(AiError::unavailable(
            "source_unavailable",
            "AI home is unavailable",
        ));
    }
    Ok(home.to_path_buf())
}

fn empty_list(home: String) -> AiRecoveryListV1 {
    AiRecoveryListV1 {
        schema: SCHEMA_RECOVERY_LIST.to_string(),
        version: 1,
        home,
        bundles: Vec::new(),
    }
}

fn report(
    outcome: &str,
    bundle_ids: Vec<String>,
    moved: Vec<String>,
    code: Option<&str>,
) -> AiExecuteReportV1 {
    AiExecuteReportV1 {
        schema: SCHEMA_EXECUTE.to_string(),
        version: 1,
        outcome: outcome.to_string(),
        bundle_ids,
        moved,
        code: code.map(str::to_string),
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryManifest {
    schema: String,
    version: u32,
    id: String,
    provider: AiProvider,
    class: AiClass,
    relative_path: String,
    content_sha256: String,
    kind: String,
}

struct PartialDir {
    path: PathBuf,
    armed: bool,
}

impl Drop for PartialDir {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
