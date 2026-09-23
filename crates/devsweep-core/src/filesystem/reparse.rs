use std::{fs, path::Path, time::SystemTime};

#[cfg(any(windows, test))]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// Result of a no-follow reparse-point probe for one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReparseProbeResult {
    NotReparsePoint,
    #[cfg(any(windows, test))]
    ReparsePoint {
        tag: u32,
    },
    #[cfg(any(windows, test))]
    Unsupported {
        detail: String,
    },
    #[cfg(any(windows, test))]
    Error {
        detail: String,
    },
}

/// Platform boundary for path-level reparse checks.
///
/// Windows callers must not infer reparse safety from `Metadata` alone: some
/// Cloud Files providers do not report the reparse attribute through Rust's
/// metadata view. Implementations inspect the path without following it.
pub(crate) trait PathReparseProbe: Send + Sync {
    fn probe(&self, path: &Path) -> ReparseProbeResult;

    /// Serves metadata and the reparse check from one open when the platform
    /// can. `None` sends the caller to `fs::symlink_metadata` plus [`probe`].
    ///
    /// [`probe`]: PathReparseProbe::probe
    fn inspect_entry(&self, _path: &Path) -> Option<InspectedEntry> {
        None
    }
}

/// No-follow metadata and reparse safety for one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectedEntry {
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
    pub modified: Option<SystemTime>,
    pub safety: PathSafety,
}

/// Path-level result used by traversal and live authorization callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathSafety {
    Safe,
    ReparsePoint {
        tag: Option<u32>,
    },
    #[cfg_attr(
        all(not(windows), not(test)),
        expect(dead_code, reason = "native non-Windows probes cannot be unverified")
    )]
    Unverified {
        detail: String,
    },
}

/// Native platform probe used outside tests.
#[derive(Debug, Default)]
pub(crate) struct SystemPathReparseProbe;

impl PathReparseProbe for SystemPathReparseProbe {
    fn probe(&self, path: &Path) -> ReparseProbeResult {
        #[cfg(windows)]
        {
            probe_windows_reparse_point(path)
        }

        #[cfg(not(windows))]
        {
            let _ = path;
            ReparseProbeResult::NotReparsePoint
        }
    }

    #[cfg(windows)]
    fn inspect_entry(&self, path: &Path) -> Option<InspectedEntry> {
        inspect_windows_entry(path)
    }
}

/// Metadata and reparse safety for one path, from one handle where the probe
/// supports it and from `fs::symlink_metadata` plus the probe otherwise.
pub(crate) fn inspect_entry_no_follow(
    path: &Path,
    probe: &dyn PathReparseProbe,
) -> std::io::Result<InspectedEntry> {
    if let Some(entry) = probe.inspect_entry(path) {
        return Ok(entry);
    }
    let metadata = fs::symlink_metadata(path)?;
    Ok(InspectedEntry {
        is_file: metadata.is_file(),
        is_dir: metadata.is_dir(),
        len: metadata.len(),
        modified: metadata.modified().ok(),
        safety: inspect_path_no_follow(path, &metadata, probe),
    })
}

/// Combines portable symlink detection with the path-level reparse probe.
///
/// A probe failure is deliberately not treated as a normal path. Callers must
/// skip traversal or deny authority for [`PathSafety::Unverified`].
pub(crate) fn inspect_path_no_follow(
    path: &Path,
    metadata: &fs::Metadata,
    probe: &dyn PathReparseProbe,
) -> PathSafety {
    path_safety(is_unsafe_link(metadata), probe.probe(path))
}

fn path_safety(metadata_reports_link: bool, probed: ReparseProbeResult) -> PathSafety {
    match probed {
        ReparseProbeResult::NotReparsePoint if metadata_reports_link => {
            PathSafety::ReparsePoint { tag: None }
        }
        ReparseProbeResult::NotReparsePoint => PathSafety::Safe,
        #[cfg(any(windows, test))]
        ReparseProbeResult::ReparsePoint { tag } => PathSafety::ReparsePoint { tag: Some(tag) },
        #[cfg(any(windows, test))]
        ReparseProbeResult::Unsupported { .. } | ReparseProbeResult::Error { .. }
            if metadata_reports_link =>
        {
            PathSafety::ReparsePoint { tag: None }
        }
        #[cfg(any(windows, test))]
        ReparseProbeResult::Unsupported { detail } => PathSafety::Unverified {
            detail: format!("reparse probe unsupported: {detail}"),
        },
        #[cfg(any(windows, test))]
        ReparseProbeResult::Error { detail } => PathSafety::Unverified {
            detail: format!("reparse probe failed: {detail}"),
        },
    }
}

pub(crate) fn is_unsafe_link(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink() || has_windows_reparse_point(metadata)
}

#[cfg(any(windows, test))]
pub(super) fn reparse_probe_result_from_tag_info(
    file_attributes: u32,
    reparse_tag: u32,
) -> ReparseProbeResult {
    if reparse_tag != 0 || file_attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        ReparseProbeResult::ReparsePoint { tag: reparse_tag }
    } else {
        ReparseProbeResult::NotReparsePoint
    }
}

#[cfg(windows)]
pub(super) fn has_windows_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(windows)]
fn probe_windows_reparse_point(path: &Path) -> ReparseProbeResult {
    use std::{
        mem::{MaybeUninit, size_of},
        os::windows::ffi::OsStrExt,
    };

    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, FileAttributeTagInfo, GetFileInformationByHandleEx, OPEN_EXISTING,
        },
    };

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: wide_path is null-terminated and lives through this call. The
    // access flags request metadata only and OPEN_REPARSE_POINT avoids follow.
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return classify_windows_probe_error("could not open path without following it");
    }

    let mut tag_info = MaybeUninit::<FILE_ATTRIBUTE_TAG_INFO>::zeroed();
    // SAFETY: handle is valid and tag_info has the exact size and alignment
    // required by FileAttributeTagInfo for the duration of the call.
    let queried = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            tag_info.as_mut_ptr().cast(),
            size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    };
    let error = if queried == 0 {
        Some(std::io::Error::last_os_error())
    } else {
        None
    };
    // SAFETY: handle was returned by CreateFileW above and is closed exactly
    // once on every path after successful creation.
    unsafe {
        let _ = CloseHandle(handle);
    }

    if let Some(error) = error {
        return classify_windows_probe_error_with("could not read path reparse tag", error);
    }

    // SAFETY: a nonzero query result means Windows initialized tag_info.
    let tag_info = unsafe { tag_info.assume_init() };
    reparse_probe_result_from_tag_info(tag_info.FileAttributes, tag_info.ReparseTag)
}

/// One `CreateFileW` open serves both the metadata and the reparse tag query,
/// with the same no-follow flags as [`probe_windows_reparse_point`]. Any open
/// or query failure returns `None`, so the caller takes the two-step path and
/// its error classification unchanged.
#[cfg(windows)]
fn inspect_windows_entry(path: &Path) -> Option<InspectedEntry> {
    use std::{
        mem::{MaybeUninit, size_of},
        os::windows::ffi::OsStrExt,
    };

    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_ATTRIBUTE_DIRECTORY,
            FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
            FileAttributeTagInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
            OPEN_EXISTING,
        },
    };

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: wide_path is null-terminated and lives through this call. The
    // access flags request metadata only and OPEN_REPARSE_POINT avoids follow.
    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }

    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    let mut tag_info = MaybeUninit::<FILE_ATTRIBUTE_TAG_INFO>::zeroed();
    // SAFETY: handle is valid, and each buffer has the exact size and alignment
    // its query requires for the duration of the call.
    let queried = unsafe {
        GetFileInformationByHandle(handle, info.as_mut_ptr()) != 0
            && GetFileInformationByHandleEx(
                handle,
                FileAttributeTagInfo,
                tag_info.as_mut_ptr().cast(),
                size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
            ) != 0
    };
    // SAFETY: handle was returned by CreateFileW above and is closed exactly
    // once on every path after successful creation.
    unsafe {
        let _ = CloseHandle(handle);
    }
    if !queried {
        return None;
    }

    // SAFETY: both queries returned nonzero, so Windows initialized both.
    let (info, tag_info) = unsafe { (info.assume_init(), tag_info.assume_init()) };
    let attributes = info.dwFileAttributes;
    // Same rule as `std::fs::FileType`: only a name-surrogate reparse tag
    // (symlink, junction) makes a symlink.
    let is_symlink =
        attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 && tag_info.ReparseTag & 0x2000_0000 != 0;
    let is_dir = !is_symlink && attributes & FILE_ATTRIBUTE_DIRECTORY != 0;
    let metadata_reports_link = is_symlink || attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    Some(InspectedEntry {
        is_file: !is_symlink && !is_dir,
        is_dir,
        len: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        modified: system_time_from_filetime(
            (u64::from(info.ftLastWriteTime.dwHighDateTime) << 32)
                | u64::from(info.ftLastWriteTime.dwLowDateTime),
        ),
        safety: path_safety(
            metadata_reports_link,
            reparse_probe_result_from_tag_info(tag_info.FileAttributes, tag_info.ReparseTag),
        ),
    })
}

/// FILETIME counts 100 ns intervals since 1601-01-01 UTC.
#[cfg(windows)]
fn system_time_from_filetime(intervals: u64) -> Option<SystemTime> {
    use std::time::{Duration, UNIX_EPOCH};

    const UNIX_EPOCH_INTERVALS: u64 = 116_444_736_000_000_000;
    const INTERVALS_PER_SECOND: u64 = 10_000_000;
    let to_duration = |value: u64| {
        Duration::new(
            value / INTERVALS_PER_SECOND,
            ((value % INTERVALS_PER_SECOND) * 100) as u32,
        )
    };
    if intervals >= UNIX_EPOCH_INTERVALS {
        UNIX_EPOCH.checked_add(to_duration(intervals - UNIX_EPOCH_INTERVALS))
    } else {
        UNIX_EPOCH.checked_sub(to_duration(UNIX_EPOCH_INTERVALS - intervals))
    }
}

#[cfg(windows)]
fn classify_windows_probe_error(operation: &str) -> ReparseProbeResult {
    classify_windows_probe_error_with(operation, std::io::Error::last_os_error())
}

#[cfg(windows)]
fn classify_windows_probe_error_with(operation: &str, error: std::io::Error) -> ReparseProbeResult {
    let detail = format!("{operation}: {error}");
    match error.raw_os_error() {
        // These indicate that the filesystem or OS cannot service the tag query.
        Some(1 | 50 | 87) => ReparseProbeResult::Unsupported { detail },
        _ => ReparseProbeResult::Error { detail },
    }
}

#[cfg(not(windows))]
pub(super) fn has_windows_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}
