use std::{
    env, fs,
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    execution::{
        CleanAuditError, CleanAuditRecordV1, ProtectionErrorCode, ProtectionMutationAction,
        ProtectionOutcomeCode, append_clean_audit_record, identity_sha256,
    },
    filesystem::{normalize_path_for_compare, paths_equal},
};

#[cfg(not(windows))]
use super::resolve_home_dir;

const USER_PROTECTION_VERSION: u32 = 1;
const LOCK_RETRIES: usize = 500;
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(10);

/// Versioned user protection list stored in OS app-data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProtectionList {
    path: PathBuf,
    entries: Vec<PathBuf>,
    /// Test-only in-memory mode never touches the real config path.
    in_memory: bool,
}

/// Result of one add/remove transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtectionMutationReport {
    pub operation_id: String,
    pub action: ProtectionMutationAction,
    pub outcome_code: ProtectionOutcomeCode,
    pub error_code: Option<ProtectionErrorCode>,
    pub identity_sha256: String,
    pub display_path: String,
    pub changed: bool,
}

/// Fail-closed protection-store errors. Corrupt bytes are never cleared.
#[derive(Debug)]
pub enum ProtectionError {
    StoreUnavailable {
        path: PathBuf,
        reason: &'static str,
    },
    LockUnavailable(PathBuf),
    TargetMissing {
        display: String,
    },
    AuditBlocked,
    AuditUnknown,
    Io {
        stage: &'static str,
        path: PathBuf,
        source: io::Error,
    },
}

impl ProtectionError {
    pub fn is_store_unavailable(&self) -> bool {
        matches!(
            self,
            Self::StoreUnavailable { .. } | Self::LockUnavailable(_) | Self::Io { .. }
        )
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::StoreUnavailable { .. } => "protection_store_unavailable",
            Self::LockUnavailable(_) => "protection_store_unavailable",
            Self::TargetMissing { .. } => "protection_path_missing",
            Self::AuditBlocked => "protection_audit_blocked",
            Self::AuditUnknown => "protection_audit_unknown",
            Self::Io { .. } => "protection_store_unavailable",
        }
    }
}

impl std::fmt::Display for ProtectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StoreUnavailable { path, reason } => write!(
                formatter,
                "protection store unavailable at {} ({reason})",
                path.display()
            ),
            Self::LockUnavailable(path) => write!(
                formatter,
                "protection store lock is unavailable at {}",
                path.display()
            ),
            Self::TargetMissing { display } => {
                write!(formatter, "protect add requires an existing path: {display}")
            }
            Self::AuditBlocked => formatter.write_str(
                "protection mutation was blocked because the Clean V1 audit could not be written",
            ),
            Self::AuditUnknown => formatter.write_str(
                "protection store was replaced but the Clean V1 committed audit could not be flushed",
            ),
            Self::Io {
                stage,
                path,
                source,
            } => write!(
                formatter,
                "protection {stage} failed for {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ProtectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct UserProtectionDocument {
    version: u32,
    paths: Vec<String>,
}

struct SidecarLock {
    _file: std::fs::File,
}

impl SidecarLock {
    fn acquire(path: PathBuf) -> Result<Self, ProtectionError> {
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;

            for attempt in 0..LOCK_RETRIES {
                match OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .share_mode(0)
                    .open(&path)
                {
                    Ok(file) => return Ok(Self { _file: file }),
                    Err(error) if lock_error_is_contention(&error) => {
                        if attempt + 1 == LOCK_RETRIES {
                            return Err(ProtectionError::LockUnavailable(path));
                        }
                        std::thread::sleep(LOCK_RETRY_DELAY);
                    }
                    Err(error) => return Err(io_error("acquire_lock", &path, error)),
                }
            }
            Err(ProtectionError::LockUnavailable(path))
        }
        #[cfg(not(windows))]
        {
            use std::os::fd::AsRawFd;

            let file = io_result(
                "open_lock",
                &path,
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&path),
            )?;
            for attempt in 0..LOCK_RETRIES {
                let result =
                    unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
                if result == 0 {
                    return Ok(Self { _file: file });
                }
                let error = io::Error::last_os_error();
                if !lock_error_is_contention(&error) {
                    return Err(io_error("acquire_lock", &path, error));
                }
                if attempt + 1 == LOCK_RETRIES {
                    return Err(ProtectionError::LockUnavailable(path));
                }
                std::thread::sleep(LOCK_RETRY_DELAY);
            }
            Err(ProtectionError::LockUnavailable(path))
        }
    }
}

fn lock_error_is_contention(error: &io::Error) -> bool {
    if error.kind() == io::ErrorKind::PermissionDenied
        || error.kind() == io::ErrorKind::WouldBlock
        || error.kind() == io::ErrorKind::AlreadyExists
    {
        return true;
    }
    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(5 | 32 | 33 | 80 | 183))
    }
    #[cfg(not(windows))]
    {
        matches!(error.raw_os_error(), Some(libc::EAGAIN | libc::EACCES))
    }
}

fn io_error(stage: &'static str, path: &Path, source: io::Error) -> ProtectionError {
    ProtectionError::Io {
        stage,
        path: path.to_path_buf(),
        source,
    }
}

fn io_result<T>(
    stage: &'static str,
    path: &Path,
    result: io::Result<T>,
) -> Result<T, ProtectionError> {
    result.map_err(|source| io_error(stage, path, source))
}

/// SHA-256 preimage for a protected path. Never persist this string in audit.
pub fn canonical_protection_identity(path: &Path) -> String {
    let normalized = normalize_path_for_compare(path);
    #[cfg(windows)]
    {
        normalized
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase()
    }
    #[cfg(not(windows))]
    {
        normalized.to_string_lossy().into_owned()
    }
}

impl UserProtectionList {
    /// Returns the platform-specific protection-list path.
    pub(crate) fn config_path() -> Result<PathBuf, ProtectionError> {
        Ok(app_data_dir()?.join("protected-paths.json"))
    }

    /// Loads the persisted protection list or returns an empty list when absent.
    pub fn load() -> Result<Self, ProtectionError> {
        Self::load_from_path(Self::config_path()?)
    }

    pub(crate) fn load_from_path(path: PathBuf) -> Result<Self, ProtectionError> {
        if !path.exists() {
            return Ok(Self {
                path,
                entries: Vec::new(),
                in_memory: false,
            });
        }
        let raw = io_result("read_protection_store", &path, fs::read(&path))?;
        let entries = decode_store_bytes(&path, &raw)?;
        Ok(Self {
            path,
            entries,
            in_memory: false,
        })
    }

    /// In-memory empty list for unit tests that must not touch app-data.
    pub(crate) fn empty_in_memory_for_tests_only() -> Self {
        Self {
            path: PathBuf::from("memory://protected-paths.json"),
            entries: Vec::new(),
            in_memory: true,
        }
    }

    /// Returns all normalized protected paths.
    pub(crate) fn paths(&self) -> &[PathBuf] {
        &self.entries
    }

    /// Returns all normalized protected paths for display.
    pub fn list(&self) -> &[PathBuf] {
        &self.entries
    }

    /// Adds an existing path and persists the updated list under the sidecar lock.
    pub fn add(&mut self, path: &Path) -> Result<ProtectionMutationReport, ProtectionError> {
        self.mutate(ProtectionMutationAction::Add, path, |record| {
            append_clean_audit_record(record)
        })
    }

    /// Removes a path and persists the updated list under the sidecar lock.
    pub fn remove(&mut self, path: &Path) -> Result<ProtectionMutationReport, ProtectionError> {
        self.mutate(ProtectionMutationAction::Remove, path, |record| {
            append_clean_audit_record(record)
        })
    }

    /// Test helper that appends mutation audit to a caller-owned journal.
    #[cfg(test)]
    pub(crate) fn add_with_audit_journal(
        &mut self,
        path: &Path,
        journal: &Path,
    ) -> Result<ProtectionMutationReport, ProtectionError> {
        self.mutate(ProtectionMutationAction::Add, path, |record| {
            super::super::audit::append_clean_audit_record_to(journal, record)
        })
    }

    /// Test helper that appends mutation audit to a caller-owned journal.
    #[cfg(test)]
    pub(crate) fn remove_with_audit_journal(
        &mut self,
        path: &Path,
        journal: &Path,
    ) -> Result<ProtectionMutationReport, ProtectionError> {
        self.mutate(ProtectionMutationAction::Remove, path, |record| {
            super::super::audit::append_clean_audit_record_to(journal, record)
        })
    }

    /// Test helper that injects a failing or counting audit sink.
    #[cfg(test)]
    pub(crate) fn mutate_with_audit_sink<F>(
        &mut self,
        action: ProtectionMutationAction,
        path: &Path,
        audit: F,
    ) -> Result<ProtectionMutationReport, ProtectionError>
    where
        F: FnMut(&CleanAuditRecordV1) -> Result<(), CleanAuditError>,
    {
        self.mutate(action, path, audit)
    }

    /// Replaces all entries with one validated, atomically persisted snapshot.
    pub fn replace(&mut self, paths: Vec<PathBuf>) -> Result<(), ProtectionError> {
        if self.in_memory {
            self.entries = validate_replacement(paths)?;
            return Ok(());
        }
        let _lock = self.acquire_lock()?;
        self.reload_locked()?;
        let entries = validate_replacement(paths)?;
        let previous = std::mem::replace(&mut self.entries, entries);
        if let Err(error) = self.persist_locked() {
            self.entries = previous;
            return Err(error);
        }
        Ok(())
    }

    fn mutate<F>(
        &mut self,
        action: ProtectionMutationAction,
        path: &Path,
        mut audit: F,
    ) -> Result<ProtectionMutationReport, ProtectionError>
    where
        F: FnMut(&CleanAuditRecordV1) -> Result<(), CleanAuditError>,
    {
        if self.in_memory {
            return self.mutate_in_memory(action, path);
        }

        let _lock = self.acquire_lock()?;
        self.reload_locked()?;

        let (canonical, display, identity, prepare_error) = match prepare_identity(action, path) {
            Ok(prepared) => prepared,
            Err(error) => {
                return Err(error);
            }
        };
        if let Some(error) = prepare_error {
            return Err(error);
        }

        let operation_id = new_operation_id();
        let requested = CleanAuditRecordV1::protection(
            &operation_id,
            1,
            ProtectionOutcomeCode::Requested,
            None,
            action,
            identity.clone(),
        );
        if let Err(error) = audit(&requested) {
            let _ = error;
            return Err(ProtectionError::AuditBlocked);
        }

        let changed = match action {
            ProtectionMutationAction::Add => {
                if self
                    .entries
                    .iter()
                    .any(|existing| paths_equal(existing, &canonical))
                {
                    false
                } else {
                    self.entries.push(canonical);
                    true
                }
            }
            ProtectionMutationAction::Remove => {
                let before = self.entries.len();
                self.entries
                    .retain(|existing| !paths_equal(existing, &canonical));
                self.entries.len() != before
            }
        };

        if changed && let Err(error) = self.persist_locked() {
            let failed = CleanAuditRecordV1::protection(
                &operation_id,
                2,
                ProtectionOutcomeCode::Failed,
                Some(ProtectionErrorCode::Unknown),
                action,
                identity.clone(),
            );
            let _ = audit(&failed);
            return Err(error);
        }

        let committed = CleanAuditRecordV1::protection(
            &operation_id,
            2,
            ProtectionOutcomeCode::Committed,
            None,
            action,
            identity.clone(),
        );
        if let Err(error) = audit(&committed) {
            let _ = error;
            return Err(ProtectionError::AuditUnknown);
        }

        Ok(ProtectionMutationReport {
            operation_id,
            action,
            outcome_code: ProtectionOutcomeCode::Committed,
            error_code: None,
            identity_sha256: identity,
            display_path: display,
            changed,
        })
    }

    fn mutate_in_memory(
        &mut self,
        action: ProtectionMutationAction,
        path: &Path,
    ) -> Result<ProtectionMutationReport, ProtectionError> {
        let (canonical, display, identity, prepare_error) = prepare_identity(action, path)?;
        if let Some(error) = prepare_error {
            return Err(error);
        }
        let changed = match action {
            ProtectionMutationAction::Add => {
                if self
                    .entries
                    .iter()
                    .any(|existing| paths_equal(existing, &canonical))
                {
                    false
                } else {
                    self.entries.push(canonical);
                    true
                }
            }
            ProtectionMutationAction::Remove => {
                let before = self.entries.len();
                self.entries
                    .retain(|existing| !paths_equal(existing, &canonical));
                self.entries.len() != before
            }
        };
        Ok(ProtectionMutationReport {
            operation_id: "op-memory".to_string(),
            action,
            outcome_code: ProtectionOutcomeCode::Committed,
            error_code: None,
            identity_sha256: identity,
            display_path: display,
            changed,
        })
    }

    fn acquire_lock(&self) -> Result<SidecarLock, ProtectionError> {
        let parent = self.path.parent().ok_or_else(|| {
            io_error(
                "resolve_protection_parent",
                &self.path,
                io::Error::new(io::ErrorKind::InvalidInput, "protection path has no parent"),
            )
        })?;
        io_result(
            "create_protection_directory",
            parent,
            fs::create_dir_all(parent),
        )?;
        SidecarLock::acquire(parent.join("protected-paths.lock"))
    }

    fn reload_locked(&mut self) -> Result<(), ProtectionError> {
        if !self.path.exists() {
            self.entries.clear();
            return Ok(());
        }
        let raw = io_result("read_protection_store", &self.path, fs::read(&self.path))?;
        self.entries = decode_store_bytes(&self.path, &raw)?;
        Ok(())
    }

    fn persist_locked(&self) -> Result<(), ProtectionError> {
        let doc = UserProtectionDocument {
            version: USER_PROTECTION_VERSION,
            paths: self
                .entries
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
        };
        let payload = serde_json::to_vec_pretty(&doc).map_err(|error| {
            io_error(
                "encode_protection_store",
                &self.path,
                io::Error::other(error.to_string()),
            )
        })?;
        let parent = self
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let temp_path = parent.join(format!(
            ".protected-paths.{}.{}.tmp",
            std::process::id(),
            unix_epoch_ms()
        ));
        {
            let mut file = io_result(
                "create_temporary_protection_store",
                &temp_path,
                fs::File::create(&temp_path),
            )?;
            file.write_all(&payload)
                .and_then(|_| file.write_all(b"\n"))
                .and_then(|_| file.sync_all())
                .map_err(|source| {
                    io_error("write_temporary_protection_store", &temp_path, source)
                })?;
        }
        io_result(
            "replace_protection_store",
            &self.path,
            fs::rename(&temp_path, &self.path),
        )?;
        Ok(())
    }
}

fn prepare_identity(
    action: ProtectionMutationAction,
    path: &Path,
) -> Result<(PathBuf, String, String, Option<ProtectionError>), ProtectionError> {
    let display = path.display().to_string();
    match action {
        ProtectionMutationAction::Add => {
            if !path.exists() {
                return Ok((
                    PathBuf::new(),
                    display.clone(),
                    identity_sha256(&canonical_protection_identity(path)),
                    Some(ProtectionError::TargetMissing { display }),
                ));
            }
            let canonical = normalize_path_for_compare(&io_result(
                "canonicalize_protection_path",
                path,
                path.canonicalize(),
            )?);
            let identity = identity_sha256(&canonical_protection_identity(&canonical));
            Ok((
                canonical.clone(),
                canonical.display().to_string(),
                identity,
                None,
            ))
        }
        ProtectionMutationAction::Remove => {
            let candidate = if path.exists() {
                normalize_path_for_compare(&io_result(
                    "canonicalize_protection_path",
                    path,
                    path.canonicalize(),
                )?)
            } else if path.is_absolute() {
                normalize_path_for_compare(path)
            } else {
                let joined =
                    io_result("resolve_cwd", path, env::current_dir()).map(|cwd| cwd.join(path))?;
                normalize_path_for_compare(&joined)
            };
            let identity = identity_sha256(&canonical_protection_identity(&candidate));
            Ok((candidate.clone(), display, identity, None))
        }
    }
}

fn validate_replacement(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, ProtectionError> {
    let mut entries: Vec<PathBuf> = Vec::with_capacity(paths.len());
    for path in paths {
        if !path.exists() {
            return Err(ProtectionError::TargetMissing {
                display: path.display().to_string(),
            });
        }
        let canonical = normalize_path_for_compare(&io_result(
            "canonicalize_protection_path",
            &path,
            path.canonicalize(),
        )?);
        if !entries
            .iter()
            .any(|existing| paths_equal(existing, &canonical))
        {
            entries.push(canonical);
        }
    }
    Ok(entries)
}

fn decode_store_bytes(path: &Path, raw: &[u8]) -> Result<Vec<PathBuf>, ProtectionError> {
    let text = std::str::from_utf8(raw).map_err(|_| ProtectionError::StoreUnavailable {
        path: path.to_path_buf(),
        reason: "invalid_utf8",
    })?;
    let doc: UserProtectionDocument =
        serde_json::from_str(text).map_err(|_| ProtectionError::StoreUnavailable {
            path: path.to_path_buf(),
            reason: "invalid_json",
        })?;
    if doc.version != USER_PROTECTION_VERSION {
        return Err(ProtectionError::StoreUnavailable {
            path: path.to_path_buf(),
            reason: "unknown_version",
        });
    }
    Ok(doc.paths.into_iter().map(PathBuf::from).collect())
}

fn new_operation_id() -> String {
    format!("protect-{}-{}", unix_epoch_ms(), std::process::id())
}

fn unix_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

fn app_data_dir() -> Result<PathBuf, ProtectionError> {
    #[cfg(windows)]
    {
        let base = env::var_os("APPDATA")
            .map(PathBuf::from)
            .filter(|value| !value.as_os_str().is_empty())
            .ok_or(ProtectionError::StoreUnavailable {
                path: PathBuf::from("<APPDATA>"),
                reason: "appdata_unavailable",
            })?;
        Ok(base.join("devsweep"))
    }
    #[cfg(target_os = "macos")]
    {
        let home = resolve_home_dir().ok_or(ProtectionError::StoreUnavailable {
            path: PathBuf::from("<HOME>"),
            reason: "appdata_unavailable",
        })?;
        Ok(home.join("Library/Application Support/devsweep"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(xdg).join("devsweep"));
        }
        let home = resolve_home_dir().ok_or(ProtectionError::StoreUnavailable {
            path: PathBuf::from("<HOME>"),
            reason: "appdata_unavailable",
        })?;
        Ok(home.join(".config/devsweep"))
    }
    #[cfg(not(any(windows, unix)))]
    {
        Err(ProtectionError::StoreUnavailable {
            path: PathBuf::from("<unsupported>"),
            reason: "unsupported_platform",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::audit::append_clean_audit_record_to;
    use super::*;
    use std::sync::{Arc, Barrier, Mutex};
    use tempfile::TempDir;

    fn journal(dir: &Path) -> PathBuf {
        let path = dir.join("audit").join("v1").join("clean.jsonl");
        fs::create_dir_all(path.parent().expect("parent")).expect("audit dir");
        path
    }

    #[test]
    fn protection_store_preserves_corrupt_and_unknown_bytes() {
        let fixture = TempDir::new().expect("temp");
        let config = fixture.path().join("protected-paths.json");
        let corrupt = b"{not-json";
        fs::write(&config, corrupt).expect("write corrupt");
        let error =
            UserProtectionList::load_from_path(config.clone()).expect_err("corrupt fails closed");
        assert!(error.is_store_unavailable());
        assert_eq!(error.code(), "protection_store_unavailable");
        assert_eq!(fs::read(&config).expect("preserved"), corrupt);

        fs::write(&config, "{\"version\":2,\"paths\":[]}").expect("newer");
        let newer = fs::read(&config).expect("newer bytes");
        let error =
            UserProtectionList::load_from_path(config.clone()).expect_err("newer fails closed");
        assert!(error.is_store_unavailable());
        assert_eq!(fs::read(&config).expect("newer preserved"), newer);

        fs::write(&config, [0xff, 0xfe, 0x00]).expect("invalid utf8");
        let utf8 = fs::read(&config).expect("utf8 bytes");
        let error = UserProtectionList::load_from_path(config.clone())
            .expect_err("invalid utf8 fails closed");
        assert!(error.is_store_unavailable());
        assert_eq!(fs::read(&config).expect("utf8 preserved"), utf8);

        let fixture_corrupt = include_bytes!("../../../tests/fixtures/protection/corrupt.json");
        fs::write(&config, fixture_corrupt).expect("fixture corrupt");
        let error =
            UserProtectionList::load_from_path(config.clone()).expect_err("fixture corrupt");
        assert!(error.is_store_unavailable());
        assert_eq!(
            fs::read(&config).expect("fixture preserved"),
            fixture_corrupt
        );

        let fixture_newer = include_bytes!("../../../tests/fixtures/protection/newer-version.json");
        fs::write(&config, fixture_newer).expect("fixture newer");
        let error = UserProtectionList::load_from_path(config.clone()).expect_err("fixture newer");
        assert!(error.is_store_unavailable());
        assert_eq!(fs::read(&config).expect("newer preserved"), fixture_newer);
    }

    #[test]
    fn protection_add_remove_is_canonical_and_reparse_safe_for_display() {
        let fixture = TempDir::new().expect("temp");
        let config = fixture.path().join("protected-paths.json");
        let keep = fixture.path().join("keep-me");
        fs::create_dir_all(&keep).expect("keep");
        let audit = journal(fixture.path());

        let mut list = UserProtectionList::load_from_path(config.clone()).expect("load");
        let added = list.add_with_audit_journal(&keep, &audit).expect("add");
        assert!(added.changed);
        assert_eq!(added.outcome_code, ProtectionOutcomeCode::Committed);
        assert!(
            !added
                .identity_sha256
                .chars()
                .any(|ch| ch == '\\' || ch == '/')
        );
        assert_eq!(added.identity_sha256.len(), 64);
        assert!(
            !fs::read_to_string(&audit)
                .expect("audit")
                .contains(&keep.display().to_string())
        );

        let reloaded = UserProtectionList::load_from_path(config.clone()).expect("reload");
        assert_eq!(reloaded.list().len(), 1);

        fs::remove_dir_all(&keep).expect("delete");
        let mut reloaded = UserProtectionList::load_from_path(config.clone()).expect("reload");
        let removed = reloaded
            .remove_with_audit_journal(&keep, &audit)
            .expect("stale remove");
        assert!(removed.changed);
        assert!(reloaded.list().is_empty());
    }

    #[test]
    fn protection_duplicate_add_is_idempotent() {
        let fixture = TempDir::new().expect("temp");
        let config = fixture.path().join("protected-paths.json");
        let keep = fixture.path().join("keep-me");
        fs::create_dir_all(&keep).expect("keep");
        let audit = journal(fixture.path());
        let mut list = UserProtectionList::load_from_path(config).expect("load");
        assert!(
            list.add_with_audit_journal(&keep, &audit)
                .expect("add")
                .changed
        );
        assert!(
            !list
                .add_with_audit_journal(&keep, &audit)
                .expect("duplicate")
                .changed
        );
        assert_eq!(list.list().len(), 1);
    }

    #[test]
    fn protection_missing_add_target_is_refused_before_persist() {
        let fixture = TempDir::new().expect("temp");
        let config = fixture.path().join("protected-paths.json");
        let missing = fixture.path().join("missing");
        let audit = journal(fixture.path());
        let mut list = UserProtectionList::load_from_path(config.clone()).expect("load");
        let error = list
            .add_with_audit_journal(&missing, &audit)
            .expect_err("missing");
        assert!(matches!(error, ProtectionError::TargetMissing { .. }));
        assert!(!config.exists());
        assert!(!audit.exists() || fs::read_to_string(&audit).unwrap_or_default().is_empty());
    }

    #[test]
    fn protection_pre_mutation_audit_failure_blocks_store() {
        let fixture = TempDir::new().expect("temp");
        let config = fixture.path().join("protected-paths.json");
        let keep = fixture.path().join("keep-me");
        fs::create_dir_all(&keep).expect("keep");
        let mut list = UserProtectionList::load_from_path(config.clone()).expect("load");
        let error = list
            .mutate_with_audit_sink(ProtectionMutationAction::Add, &keep, |_| {
                Err(CleanAuditError::ForbiddenPayload)
            })
            .expect_err("blocked");
        assert!(matches!(error, ProtectionError::AuditBlocked));
        assert!(!config.exists() || fs::read(&config).expect("bytes").is_empty());
    }

    #[test]
    fn protection_post_replace_audit_failure_is_unknown_without_rollback() {
        let fixture = TempDir::new().expect("temp");
        let config = fixture.path().join("protected-paths.json");
        let keep = fixture.path().join("keep-me");
        fs::create_dir_all(&keep).expect("keep");
        let mut list = UserProtectionList::load_from_path(config.clone()).expect("load");
        let calls = Mutex::new(0_u8);
        let error = list
            .mutate_with_audit_sink(ProtectionMutationAction::Add, &keep, |_| {
                let mut calls = calls.lock().expect("calls");
                *calls += 1;
                if *calls == 1 {
                    Ok(())
                } else {
                    Err(CleanAuditError::ForbiddenPayload)
                }
            })
            .expect_err("unknown");
        assert!(matches!(error, ProtectionError::AuditUnknown));
        let reloaded = UserProtectionList::load_from_path(config).expect("store remains");
        assert_eq!(reloaded.list().len(), 1);
    }

    #[test]
    fn protection_concurrent_writers_do_not_lose_updates() {
        let fixture = TempDir::new().expect("temp");
        let config = Arc::new(fixture.path().join("protected-paths.json"));
        let first = fixture.path().join("keep-a");
        let second = fixture.path().join("keep-b");
        fs::create_dir_all(&first).expect("a");
        fs::create_dir_all(&second).expect("b");
        let audit_a = journal(&fixture.path().join("a"));
        let audit_b = journal(&fixture.path().join("b"));
        let barrier = Arc::new(Barrier::new(2));
        let config_a = Arc::clone(&config);
        let config_b = Arc::clone(&config);
        let barrier_a = Arc::clone(&barrier);
        let barrier_b = Arc::clone(&barrier);
        let left = std::thread::spawn(move || {
            barrier_a.wait();
            let mut list = UserProtectionList::load_from_path((*config_a).clone()).expect("load a");
            list.add_with_audit_journal(&first, &audit_a)
        });
        let right = std::thread::spawn(move || {
            barrier_b.wait();
            let mut list = UserProtectionList::load_from_path((*config_b).clone()).expect("load b");
            list.add_with_audit_journal(&second, &audit_b)
        });
        left.join().expect("join a").expect("add a");
        right.join().expect("join b").expect("add b");
        let reloaded = UserProtectionList::load_from_path((*config).clone()).expect("final");
        assert_eq!(reloaded.list().len(), 2, "lost-update must not occur");
    }

    #[test]
    fn protection_replace_rejects_invalid_snapshot() {
        let fixture = TempDir::new().expect("temp");
        let config = fixture.path().join("protected-paths.json");
        let keep = fixture.path().join("keep-me");
        fs::create_dir_all(&keep).expect("keep");
        let audit = journal(fixture.path());
        let mut list = UserProtectionList::load_from_path(config.clone()).expect("load");
        list.add_with_audit_journal(&keep, &audit).expect("add");
        let missing = fixture.path().join("missing");
        list.replace(vec![keep.clone(), missing])
            .expect_err("invalid replacement is rejected before persistence");
        let unchanged = UserProtectionList::load_from_path(config).expect("reload");
        assert_eq!(unchanged.list().len(), 1);
    }

    #[test]
    fn protection_identity_hash_never_embeds_raw_path() {
        let path = PathBuf::from(r"C:\Users\secret\keep");
        let hashed = identity_sha256(&canonical_protection_identity(&path));
        assert!(!hashed.contains("secret"));
        assert!(!hashed.contains("Users"));
        assert_eq!(hashed.len(), 64);
    }

    #[test]
    fn protection_audit_sink_accepts_valid_records() {
        let fixture = TempDir::new().expect("temp");
        let record = CleanAuditRecordV1::protection(
            "op-test",
            1,
            ProtectionOutcomeCode::Requested,
            None,
            ProtectionMutationAction::Add,
            "a".repeat(64),
        );
        let path = journal(fixture.path());
        append_clean_audit_record_to(&path, &record).expect("append");
        let text = fs::read_to_string(&path).expect("read");
        assert!(text.contains("protection_mutation"));
        assert!(!text.contains("path"));
        assert!(!text.contains("argv"));
    }
}
