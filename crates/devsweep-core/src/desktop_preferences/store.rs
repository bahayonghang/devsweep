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

use super::{DesktopPreferencesPatch, DesktopPreferencesV1};

const LOCK_RETRIES: usize = 500;
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(10);
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Fail-closed desktop preference storage or validation failure.
#[derive(Debug)]
pub enum DesktopPreferencesError {
    /// No supported user storage root is available.
    LocalAppDataUnavailable,
    /// The transaction lock was not acquired within its bounded wait.
    LockUnavailable(PathBuf),
    /// Existing bytes do not contain the supported closed V1 document.
    UnsupportedDocument {
        /// Path of the preserved document.
        path: PathBuf,
        /// Decoder error.
        message: String,
    },
    /// A directly constructed Rust patch has an unsupported numeric value.
    InvalidPatch {
        /// Closed field name.
        field: &'static str,
        /// Rejected numeric value.
        value: u8,
    },
    /// A filesystem operation failed before commit.
    Io {
        /// Transaction stage.
        stage: &'static str,
        /// Path involved in the operation.
        path: PathBuf,
        /// Operating-system error.
        source: io::Error,
    },
}

impl fmt::Display for DesktopPreferencesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalAppDataUnavailable => formatter.write_str(
                "LOCALAPPDATA is unavailable; desktop preference persistence is disabled",
            ),
            Self::LockUnavailable(path) => write!(
                formatter,
                "desktop preference lock is unavailable at {}",
                path.display()
            ),
            Self::UnsupportedDocument { path, message } => write!(
                formatter,
                "unsupported desktop preference document at {}: {message}",
                path.display()
            ),
            Self::InvalidPatch { field, value } => {
                write!(formatter, "unsupported desktop preference {field}: {value}")
            }
            Self::Io {
                stage,
                path,
                source,
            } => write!(
                formatter,
                "desktop preference I/O failed at {stage} for {}: {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for DesktopPreferencesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn io_error(stage: &'static str, path: &Path, source: io::Error) -> DesktopPreferencesError {
    DesktopPreferencesError::Io {
        stage,
        path: path.to_path_buf(),
        source,
    }
}

fn io_result<T>(
    stage: &'static str,
    path: &Path,
    result: io::Result<T>,
) -> Result<T, DesktopPreferencesError> {
    result.map_err(|source| io_error(stage, path, source))
}

/// Resolve the fixed desktop-only settings path. No fallback root is used.
pub fn desktop_preferences_path() -> Result<PathBuf, DesktopPreferencesError> {
    path_from_local_app_data(env::var_os("LOCALAPPDATA"))
}

pub(super) fn path_from_local_app_data(
    local_app_data: Option<OsString>,
) -> Result<PathBuf, DesktopPreferencesError> {
    let root = local_app_data
        .filter(|value| !value.is_empty())
        .ok_or(DesktopPreferencesError::LocalAppDataUnavailable)?;
    Ok(PathBuf::from(root).join("DevSweep/settings/desktop-preferences-v1.json"))
}

/// Load validated desktop preferences. Missing storage uses V1 defaults.
pub fn load_desktop_preferences() -> Result<DesktopPreferencesV1, DesktopPreferencesError> {
    load_from_path(&desktop_preferences_path()?)
}

/// Commit one patch after rereading the current document under the OS lock.
pub fn update_desktop_preferences(
    patch: DesktopPreferencesPatch,
) -> Result<DesktopPreferencesV1, DesktopPreferencesError> {
    update_at_path(&desktop_preferences_path()?, patch)
}

pub(super) fn load_from_path(path: &Path) -> Result<DesktopPreferencesV1, DesktopPreferencesError> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
            DesktopPreferencesError::UnsupportedDocument {
                path: path.to_path_buf(),
                message: error.to_string(),
            }
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(DesktopPreferencesV1::default())
        }
        Err(error) => Err(io_error("read_document", path, error)),
    }
}

pub(super) fn update_at_path(
    path: &Path,
    patch: DesktopPreferencesPatch,
) -> Result<DesktopPreferencesV1, DesktopPreferencesError> {
    update_with_replace(path, patch, replace_file)
}

pub(super) fn update_with_replace(
    path: &Path,
    patch: DesktopPreferencesPatch,
    replace: fn(&Path, &Path) -> io::Result<()>,
) -> Result<DesktopPreferencesV1, DesktopPreferencesError> {
    let directory = path.parent().ok_or_else(|| {
        io_error(
            "resolve_settings_parent",
            path,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "desktop preference path has no parent",
            ),
        )
    })?;
    io_result(
        "create_settings_directory",
        directory,
        fs::create_dir_all(directory),
    )?;
    let _lock = TransactionLock::acquire(directory)?;
    let preferences = load_from_path(path)?.patched(patch)?;
    let bytes = serde_json::to_vec(&preferences)
        .expect("the closed desktop preference document always serializes");
    atomic_replace(path, &bytes, replace)?;
    Ok(preferences)
}

// Use the same OS-owned transaction pattern as the language store, with a
// separate Windows mutex name. No lock-path cleanup follows the commit point.
struct TransactionLock {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(not(windows))]
    _directory: fs::File,
}

impl TransactionLock {
    #[cfg(windows)]
    fn acquire(directory: &Path) -> Result<Self, DesktopPreferencesError> {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_ABANDONED, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
            System::Threading::{CreateMutexW, WaitForSingleObject},
        };
        let name = r"Local\DevSweep.DesktopPreferences.V1"
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // SAFETY: the live name buffer is NUL-terminated. Default security
        // scopes the kernel mutex to the current user session.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(io_error(
                "create_lock_object",
                directory,
                io::Error::last_os_error(),
            ));
        }
        // SAFETY: handle is live. An abandoned mutex transfers ownership.
        let wait = unsafe {
            WaitForSingleObject(
                handle,
                u32::try_from(LOCK_RETRIES).expect("bounded retries fit u32")
                    * u32::try_from(LOCK_RETRY_DELAY.as_millis()).expect("bounded delay fits u32"),
            )
        };
        match wait {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Ok(Self { handle }),
            failure => {
                let error = if failure == WAIT_FAILED {
                    io::Error::last_os_error()
                } else {
                    io::Error::other(format!("unexpected lock wait result {failure}"))
                };
                // SAFETY: this process owns the handle but not the mutex.
                unsafe { CloseHandle(handle) };
                if failure == WAIT_TIMEOUT {
                    Err(DesktopPreferencesError::LockUnavailable(
                        directory.to_path_buf(),
                    ))
                } else {
                    Err(io_error("wait_for_lock_object", directory, error))
                }
            }
        }
    }

    #[cfg(not(windows))]
    fn acquire(directory: &Path) -> Result<Self, DesktopPreferencesError> {
        use std::os::fd::AsRawFd;
        let file = io_result("open_lock_directory", directory, fs::File::open(directory))?;
        for attempt in 0..LOCK_RETRIES {
            // SAFETY: file owns the descriptor until the guard is dropped.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                return Ok(Self { _directory: file });
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::EAGAIN) {
                return Err(io_error("acquire_lock", directory, error));
            }
            if attempt + 1 == LOCK_RETRIES {
                return Err(DesktopPreferencesError::LockUnavailable(
                    directory.to_path_buf(),
                ));
            }
            std::thread::sleep(LOCK_RETRY_DELAY);
        }
        unreachable!("bounded lock loop always returns")
    }
}

#[cfg(windows)]
impl Drop for TransactionLock {
    fn drop(&mut self) {
        use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::ReleaseMutex};
        // SAFETY: the guard owns the mutex and its handle on this thread.
        unsafe {
            ReleaseMutex(self.handle);
            CloseHandle(self.handle);
        }
    }
}

fn atomic_replace(
    path: &Path,
    bytes: &[u8],
    replace: fn(&Path, &Path) -> io::Result<()>,
) -> Result<(), DesktopPreferencesError> {
    let directory = path
        .parent()
        .expect("validated preference path has a parent");
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary_path = directory.join(format!(
        ".desktop-preferences-v1.{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    let mut temporary = io_result(
        "create_temporary_document",
        &temporary_path,
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path),
    )?;
    let result = (|| {
        io_result(
            "write_temporary_document",
            &temporary_path,
            temporary.write_all(bytes),
        )?;
        io_result(
            "flush_temporary_document",
            &temporary_path,
            temporary.flush(),
        )?;
        io_result(
            "sync_temporary_document",
            &temporary_path,
            temporary.sync_all(),
        )?;
        drop(temporary);
        io_result("replace_document", path, replace(&temporary_path, path))
    })();
    if result.is_err() {
        // Cleanup cannot change whether replacement committed. Preserve the
        // transaction error if removing an uncommitted temporary file fails.
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
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
    // SAFETY: both path buffers are valid NUL-terminated UTF-16 for the call.
    if unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    let directory = fs::File::open(destination.parent().expect("validated preference parent"))?;
    fs::rename(source, destination)?;
    // Rename is the commit point. A later directory-sync failure must not
    // report rollback after the committed bytes became visible.
    let _ = directory.sync_all();
    Ok(())
}
