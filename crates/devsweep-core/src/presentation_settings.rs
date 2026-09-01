//! Locale-only interactive presentation settings.
//!
//! This module deliberately has no knowledge of CLI or desktop locale types.
//! Machine-readable command output must never consult this store.

use std::{
    env,
    ffi::OsString,
    fmt, fs,
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use serde::{Deserialize, Serialize};

#[cfg(not(windows))]
use std::fs::File;

const SCHEMA_VERSION: u8 = 1;
const LOCK_RETRIES: usize = 500;
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(10);
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Stable wire tags accepted by interactive DevSweep frontends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresentationLanguageTag {
    /// English presentation.
    #[serde(rename = "en")]
    En,
    /// Simplified-Chinese presentation.
    #[serde(rename = "zh-CN")]
    ZhCn,
}

impl PresentationLanguageTag {
    /// Return the exact stable wire tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhCn => "zh-CN",
        }
    }
}

/// The only user choice persisted by the V1 presentation store.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationSettingsV1 {
    /// Explicit language choice; `None` delegates to the frontend resolver.
    pub language: Option<PresentationLanguageTag>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PresentationSettingsDocumentV1 {
    schema_version: u8,
    language: Option<PresentationLanguageTag>,
}

impl From<PresentationSettingsV1> for PresentationSettingsDocumentV1 {
    fn from(settings: PresentationSettingsV1) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            language: settings.language,
        }
    }
}

/// A fail-closed presentation-settings failure.
#[derive(Debug)]
pub enum PresentationSettingsError {
    /// `%LOCALAPPDATA%` is unavailable, so no alternative root was selected.
    LocalAppDataUnavailable,
    /// The lock could not be acquired within the bounded wait.
    LockUnavailable(PathBuf),
    /// Stored bytes are not the exact supported V1 shape.
    UnsupportedDocument {
        /// Stable validation stage.
        stage: &'static str,
        /// Exact document path whose bytes were rejected.
        path: PathBuf,
        /// Decoder or version failure.
        message: String,
    },
    /// Filesystem persistence failed.
    Io {
        /// Stable transaction stage for diagnostics and focused tests.
        stage: &'static str,
        /// Exact path involved in the failed operation.
        path: PathBuf,
        /// Operating-system error returned by that operation.
        source: io::Error,
    },
}

impl fmt::Display for PresentationSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalAppDataUnavailable => formatter
                .write_str("LOCALAPPDATA is unavailable; presentation persistence is disabled"),
            Self::LockUnavailable(path) => write!(
                formatter,
                "presentation settings lock is unavailable at {}",
                path.display()
            ),
            Self::UnsupportedDocument {
                stage,
                path,
                message,
            } => {
                write!(
                    formatter,
                    "unsupported presentation settings document at {stage} for {}: {message}",
                    path.display()
                )
            }
            Self::Io {
                stage,
                path,
                source,
            } => write!(
                formatter,
                "presentation settings I/O failed at {stage} for {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for PresentationSettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn io_error(stage: &'static str, path: &Path, source: io::Error) -> PresentationSettingsError {
    PresentationSettingsError::Io {
        stage,
        path: path.to_path_buf(),
        source,
    }
}

fn io_result<T>(
    stage: &'static str,
    path: &Path,
    result: io::Result<T>,
) -> Result<T, PresentationSettingsError> {
    result.map_err(|source| io_error(stage, path, source))
}

/// Resolve the one supported current-user settings path.
pub fn presentation_settings_path() -> Result<PathBuf, PresentationSettingsError> {
    settings_path_from_local_app_data(env::var_os("LOCALAPPDATA"))
}

fn settings_path_from_local_app_data(
    local_app_data: Option<OsString>,
) -> Result<PathBuf, PresentationSettingsError> {
    let root = local_app_data
        .filter(|value| !value.is_empty())
        .ok_or(PresentationSettingsError::LocalAppDataUnavailable)?;
    Ok(PathBuf::from(root)
        .join("DevSweep")
        .join("settings")
        .join("presentation-v1.json"))
}

/// Load locale settings from the fixed current-user path.
pub fn load_presentation_settings() -> Result<PresentationSettingsV1, PresentationSettingsError> {
    load_from_path(&presentation_settings_path()?)
}

/// Store locale settings at the fixed current-user path.
pub fn save_presentation_settings(
    settings: PresentationSettingsV1,
) -> Result<(), PresentationSettingsError> {
    save_to_path(&presentation_settings_path()?, settings)
}

fn load_from_path(path: &Path) -> Result<PresentationSettingsV1, PresentationSettingsError> {
    match fs::read(path) {
        Ok(bytes) => decode_document(path, "validate_loaded_document", &bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(PresentationSettingsV1::default())
        }
        Err(error) => Err(io_error("read_document", path, error)),
    }
}

fn decode_document(
    path: &Path,
    stage: &'static str,
    bytes: &[u8],
) -> Result<PresentationSettingsV1, PresentationSettingsError> {
    let document: PresentationSettingsDocumentV1 =
        serde_json::from_slice(bytes).map_err(|error| {
            PresentationSettingsError::UnsupportedDocument {
                stage,
                path: path.to_path_buf(),
                message: error.to_string(),
            }
        })?;
    if document.schema_version != SCHEMA_VERSION {
        return Err(PresentationSettingsError::UnsupportedDocument {
            stage,
            path: path.to_path_buf(),
            message: format!(
                "schema version {} is not supported",
                document.schema_version
            ),
        });
    }
    Ok(PresentationSettingsV1 {
        language: document.language,
    })
}

fn save_to_path(
    path: &Path,
    settings: PresentationSettingsV1,
) -> Result<(), PresentationSettingsError> {
    save_to_path_with_lock_cleanup(path, settings, remove_lock_path)
}

fn remove_lock_path(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

fn save_to_path_with_lock_cleanup(
    path: &Path,
    settings: PresentationSettingsV1,
    lock_cleanup: fn(&Path) -> io::Result<()>,
) -> Result<(), PresentationSettingsError> {
    let directory = path.parent().ok_or_else(|| {
        io_error(
            "resolve_settings_parent",
            path,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "presentation settings path has no parent",
            ),
        )
    })?;
    io_result(
        "create_settings_directory",
        directory,
        fs::create_dir_all(directory),
    )?;

    let lock_path = directory.join("presentation-v1.lock");
    let _lock = TransactionLock::acquire_with_cleanup(lock_path, lock_cleanup)?;

    // Refuse to overwrite any unreadable/newer document. This read occurs
    // while the cross-process lock is held for the entire transaction.
    if io_result("probe_existing_document", path, path.try_exists())? {
        let original = io_result("read_existing_document", path, fs::read(path))?;
        decode_document(path, "validate_existing_document", &original)?;
    }

    let bytes = serde_json::to_vec(&PresentationSettingsDocumentV1::from(settings))
        .expect("the closed V1 presentation document always serializes");
    // Atomic replacement is the public commit point. The OS owns lock release
    // through this guard's handle lifetime, so there is no fallible cleanup
    // operation that can turn a committed write into an API failure.
    atomic_replace(path, &bytes)
}

struct TransactionLock {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(not(windows))]
    _file: File,
}

impl TransactionLock {
    #[cfg(windows)]
    fn acquire_with_cleanup(
        path: PathBuf,
        cleanup: fn(&Path) -> io::Result<()>,
    ) -> Result<Self, PresentationSettingsError> {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_ABANDONED, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
            System::Threading::{CreateMutexW, WaitForSingleObject},
        };

        let name = "Local\\DevSweep.PresentationSettings.V1"
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // SAFETY: the name is a live, NUL-terminated UTF-16 buffer. Default
        // security scopes this named kernel object to the current user session.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(io_error(
                "create_lock_object",
                &path,
                io::Error::last_os_error(),
            ));
        }
        // SAFETY: `handle` is a live mutex handle. WAIT_ABANDONED transfers
        // ownership after an abnormal prior owner exits and is therefore a
        // successful recovery state.
        let wait = unsafe {
            WaitForSingleObject(
                handle,
                u32::try_from(LOCK_RETRIES).expect("bounded retries fit u32")
                    * u32::try_from(LOCK_RETRY_DELAY.as_millis())
                        .expect("bounded retry delay fits u32"),
            )
        };
        match wait {
            WAIT_OBJECT_0 | WAIT_ABANDONED => {
                // Remove only a legacy sidecar left by older builds. This name
                // is not the authority; cleanup failure cannot affect mutex
                // ownership or the transaction result.
                match cleanup(&path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(_) => {}
                }
                Ok(Self { handle })
            }
            WAIT_TIMEOUT => {
                // SAFETY: this process owns the live handle but not the mutex.
                unsafe { CloseHandle(handle) };
                Err(PresentationSettingsError::LockUnavailable(path))
            }
            WAIT_FAILED => {
                let error = io::Error::last_os_error();
                // SAFETY: this process owns the live handle but not the mutex.
                unsafe { CloseHandle(handle) };
                Err(io_error("wait_for_lock_object", &path, error))
            }
            unexpected => {
                // SAFETY: this process owns the live handle but not the mutex.
                unsafe { CloseHandle(handle) };
                Err(io_error(
                    "wait_for_lock_object",
                    &path,
                    io::Error::other(format!("unexpected wait result {unexpected}")),
                ))
            }
        }
    }

    #[cfg(not(windows))]
    fn acquire_with_cleanup(
        path: PathBuf,
        _cleanup: fn(&Path) -> io::Result<()>,
    ) -> Result<Self, PresentationSettingsError> {
        use std::os::fd::AsRawFd;

        let directory = path.parent().ok_or_else(|| {
            io_error(
                "resolve_lock_parent",
                &path,
                io::Error::new(io::ErrorKind::InvalidInput, "lock path has no parent"),
            )
        })?;
        let file = io_result("open_lock_directory", directory, File::open(directory))?;
        for attempt in 0..LOCK_RETRIES {
            // SAFETY: the descriptor belongs to `file` and remains open for
            // the lifetime of the returned guard.
            let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if result == 0 {
                return Ok(Self { _file: file });
            }
            let error = io::Error::last_os_error();
            if !lock_error_is_contention(&error) {
                return Err(io_error("acquire_lock", &path, error));
            }
            if attempt + 1 == LOCK_RETRIES {
                return Err(PresentationSettingsError::LockUnavailable(path));
            }
            std::thread::sleep(LOCK_RETRY_DELAY);
        }
        unreachable!("bounded lock loop always returns")
    }
}

#[cfg(any(test, not(windows)))]
fn lock_error_is_contention(error: &io::Error) -> bool {
    if error.kind() == io::ErrorKind::AlreadyExists {
        return true;
    }
    #[cfg(windows)]
    {
        // ERROR_LOCK_VIOLATION plus the observed CreateFile outcomes while a
        // delete-pending or concurrently-created rendezvous file is owned.
        matches!(error.raw_os_error(), Some(5 | 32 | 33 | 80 | 183))
    }
    #[cfg(not(windows))]
    {
        matches!(error.raw_os_error(), Some(libc::EAGAIN))
    }
}

#[cfg(windows)]
impl Drop for TransactionLock {
    fn drop(&mut self) {
        use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::ReleaseMutex};

        // SAFETY: this guard owns the mutex and its live handle. Even if the
        // explicit release fails, closing the handle cannot turn an already
        // committed document into an API error and the OS releases ownership.
        unsafe {
            ReleaseMutex(self.handle);
            CloseHandle(self.handle);
        }
    }
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), PresentationSettingsError> {
    let directory = path.parent().expect("validated settings path has a parent");
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp_path = directory.join(format!(
        ".presentation-v1.{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    let mut temporary = io_result(
        "create_temporary_document",
        &temp_path,
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path),
    )?;

    let result = (|| {
        io_result(
            "write_temporary_document",
            &temp_path,
            temporary.write_all(bytes),
        )?;
        io_result("flush_temporary_document", &temp_path, temporary.flush())?;
        io_result("sync_temporary_document", &temp_path, temporary.sync_all())?;
        drop(temporary);
        replace_file(&temp_path, path)
    })();
    if let Err(original) = result {
        match fs::remove_file(&temp_path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(io_error(
                    "remove_failed_temporary_document",
                    &temp_path,
                    error,
                ));
            }
        }
        return Err(original);
    }
    Ok(())
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), PresentationSettingsError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    // SAFETY: both paths are valid, NUL-terminated UTF-16 buffers that remain
    // alive for the duration of the call.
    let moved = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(io_error(
            "replace_document",
            destination,
            io::Error::last_os_error(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), PresentationSettingsError> {
    io_result(
        "replace_document",
        destination,
        fs::rename(source, destination),
    )?;
    let parent = destination
        .parent()
        .expect("validated settings path has a parent");
    let directory = io_result("open_settings_directory", parent, File::open(parent))?;
    io_result("sync_settings_directory", parent, directory.sync_all())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::*;

    fn path(root: &Path) -> PathBuf {
        root.join("DevSweep/settings/presentation-v1.json")
    }

    #[test]
    fn fixed_windows_path_never_selects_a_fallback_root() {
        assert_eq!(
            settings_path_from_local_app_data(Some(OsString::from(r"C:\Users\dev\AppData\Local")))
                .unwrap(),
            PathBuf::from(r"C:\Users\dev\AppData\Local")
                .join("DevSweep/settings/presentation-v1.json")
        );
        assert!(matches!(
            settings_path_from_local_app_data(None),
            Err(PresentationSettingsError::LocalAppDataUnavailable)
        ));
        assert!(matches!(
            settings_path_from_local_app_data(Some(OsString::new())),
            Err(PresentationSettingsError::LocalAppDataUnavailable)
        ));
    }

    #[test]
    fn exact_v1_bytes_and_closed_tags_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let settings_path = path(temp.path());
        for (settings, exact) in [
            (
                PresentationSettingsV1 { language: None },
                br#"{"schema_version":1,"language":null}"#.as_slice(),
            ),
            (
                PresentationSettingsV1 {
                    language: Some(PresentationLanguageTag::En),
                },
                br#"{"schema_version":1,"language":"en"}"#.as_slice(),
            ),
            (
                PresentationSettingsV1 {
                    language: Some(PresentationLanguageTag::ZhCn),
                },
                br#"{"schema_version":1,"language":"zh-CN"}"#.as_slice(),
            ),
        ] {
            save_to_path(&settings_path, settings).unwrap();
            assert_eq!(fs::read(&settings_path).unwrap(), exact);
            assert_eq!(load_from_path(&settings_path).unwrap(), settings);
        }
        assert_eq!(PresentationLanguageTag::En.as_str(), "en");
        assert_eq!(PresentationLanguageTag::ZhCn.as_str(), "zh-CN");
    }

    #[test]
    fn missing_file_means_no_explicit_choice() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(
            load_from_path(&path(temp.path())).unwrap(),
            PresentationSettingsV1::default()
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_exclusive_sidecar_errors_are_bounded_lock_contention() {
        for code in [5, 32, 33, 80, 183] {
            assert!(lock_error_is_contention(&io::Error::from_raw_os_error(
                code
            )));
        }
        assert!(!lock_error_is_contention(&io::Error::from_raw_os_error(3)));
    }

    #[test]
    fn corrupt_unknown_and_newer_documents_are_preserved_byte_for_byte() {
        let invalid_documents: &[&[u8]] = &[
            b"\xffnot-utf8",
            br#"{"schema_version":1,"language":"zh-TW"}"#,
            br#"{"schema_version":2,"language":"en"}"#,
            br#"{"schema_version":1,"language":"en","future":true}"#,
        ];
        for original in invalid_documents {
            let temp = tempfile::tempdir().unwrap();
            let settings_path = path(temp.path());
            fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
            fs::write(&settings_path, original).unwrap();
            assert!(load_from_path(&settings_path).is_err());
            assert!(
                save_to_path(
                    &settings_path,
                    PresentationSettingsV1 {
                        language: Some(PresentationLanguageTag::En),
                    }
                )
                .is_err()
            );
            assert_eq!(fs::read(&settings_path).unwrap(), *original);
        }
    }

    #[cfg(windows)]
    fn injected_lock_cleanup_failure(_path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "injected lock-name cleanup failure",
        ))
    }

    #[cfg(windows)]
    #[test]
    fn post_commit_cleanup_failure_is_success_and_next_transaction_recovers() {
        let temp = tempfile::tempdir().unwrap();
        let settings_path = path(temp.path());
        let lock_path = settings_path.with_file_name("presentation-v1.lock");
        fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
        fs::write(&lock_path, b"legacy-sidecar").unwrap();

        save_to_path_with_lock_cleanup(
            &settings_path,
            PresentationSettingsV1 {
                language: Some(PresentationLanguageTag::ZhCn),
            },
            injected_lock_cleanup_failure,
        )
        .expect("a committed document cannot become an API failure after commit");
        assert_eq!(
            fs::read(&settings_path).unwrap(),
            br#"{"schema_version":1,"language":"zh-CN"}"#
        );
        assert!(lock_path.exists(), "injected failure leaves a stale name");

        save_to_path(
            &settings_path,
            PresentationSettingsV1 {
                language: Some(PresentationLanguageTag::En),
            },
        )
        .expect("a stale name is not ownership and cannot block recovery");
        assert_eq!(
            fs::read(&settings_path).unwrap(),
            br#"{"schema_version":1,"language":"en"}"#
        );
        assert!(!lock_path.exists());
    }

    #[cfg(windows)]
    #[test]
    fn dropped_abnormal_owner_releases_kernel_lock_even_when_name_remains() {
        use windows_sys::Win32::Foundation::CloseHandle;

        let temp = tempfile::tempdir().unwrap();
        let settings_path = path(temp.path());
        let directory = settings_path.parent().unwrap();
        fs::create_dir_all(directory).unwrap();
        let lock_path = directory.join("presentation-v1.lock");
        fs::write(&lock_path, b"legacy-sidecar").unwrap();

        let prior_path = lock_path.clone();
        let leaked_handle = thread::spawn(move || {
            let prior =
                TransactionLock::acquire_with_cleanup(prior_path, injected_lock_cleanup_failure)
                    .unwrap();
            let handle = prior.handle as usize;
            std::mem::forget(prior);
            handle
        })
        .join()
        .unwrap();
        assert!(lock_path.exists());

        save_to_path(
            &settings_path,
            PresentationSettingsV1 {
                language: Some(PresentationLanguageTag::ZhCn),
            },
        )
        .expect("OS handle lifetime, not pathname cleanup, releases ownership");
        assert_eq!(
            load_from_path(&settings_path).unwrap().language,
            Some(PresentationLanguageTag::ZhCn)
        );
        assert!(!lock_path.exists());
        // SAFETY: the prior thread transferred this still-live raw handle only
        // so the test can model abandonment without leaking process resources.
        unsafe { CloseHandle(leaked_handle as _) };
    }

    #[test]
    fn concurrent_frontends_serialize_complete_transactions() {
        let temp = tempfile::tempdir().unwrap();
        let settings_path = Arc::new(path(temp.path()));
        let handles = (0..16)
            .map(|index| {
                let settings_path = Arc::clone(&settings_path);
                thread::spawn(move || {
                    let language = if index % 2 == 0 {
                        PresentationLanguageTag::En
                    } else {
                        PresentationLanguageTag::ZhCn
                    };
                    save_to_path(
                        &settings_path,
                        PresentationSettingsV1 {
                            language: Some(language),
                        },
                    )
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.join().unwrap().unwrap();
        }
        let bytes = fs::read(&*settings_path).unwrap();
        assert!(
            bytes == br#"{"schema_version":1,"language":"en"}"#
                || bytes == br#"{"schema_version":1,"language":"zh-CN"}"#
        );
        load_from_path(&settings_path).unwrap();
        let directory = settings_path.parent().unwrap();
        let residue = fs::read_dir(directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .filter(|name| {
                let name = name.to_string_lossy();
                name == "presentation-v1.lock"
                    || (name.starts_with(".presentation-v1.") && name.ends_with(".tmp"))
            })
            .collect::<Vec<_>>();
        assert!(residue.is_empty(), "transaction residue: {residue:?}");
    }
}
