use std::{fs, path::Path};

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
    let metadata_reports_link = is_unsafe_link(metadata);
    match probe.probe(path) {
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
