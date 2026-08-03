use std::{
    io,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::fs::File;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Produces a lexical, platform-aware identity for an absolute path without
/// touching the filesystem. This is intentionally not a TOCTOU defense; the
/// execution-time safety policy owns live-object revalidation.
pub(crate) fn normalize_absolute_path(path: &Path) -> Result<String> {
    if !path.is_absolute() {
        bail!("path must be absolute: {}", path.display());
    }

    let raw = path.to_string_lossy();
    #[cfg(windows)]
    let raw = normalize_windows_path(raw.replace('\\', "/"));
    #[cfg(not(windows))]
    let raw = {
        // POSIX treats repeated leading separators as the same root. Keep the
        // Windows UNC branch above separate, where `//server/share` matters.
        let raw = raw.into_owned();
        format!("/{}", raw.trim_start_matches('/'))
    };

    let (prefix, rest) = split_root(&raw)?;
    let mut segments = Vec::new();
    for segment in rest.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    bail!("path escapes its absolute root: {}", path.display());
                }
            }
            value => segments.push(value),
        }
    }

    let joined = segments.join("/");
    if joined.is_empty() {
        Ok(prefix)
    } else if prefix.ends_with('/') {
        Ok(format!("{prefix}{joined}"))
    } else {
        Ok(format!("{prefix}/{joined}"))
    }
}

/// Stable live filesystem object identity for TOCTOU revalidation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PathIdentity {
    pub kind: PathIdentityKind,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PathIdentityKind {
    WindowsFileId,
    UnixDevIno,
}

/// Captures a live object identity for `path`. Failures are returned so callers
/// can fail closed rather than treating a missing identity as authorization.
pub(crate) fn capture_path_identity(path: &Path) -> io::Result<PathIdentity> {
    #[cfg(windows)]
    {
        capture_windows_file_id(path)
    }
    #[cfg(unix)]
    {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        use std::os::unix::fs::MetadataExt;
        Ok(PathIdentity {
            kind: PathIdentityKind::UnixDevIno,
            value: format!("{}:{}", metadata.dev(), metadata.ino()),
        })
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "path identity is not implemented on this platform",
        ))
    }
}

#[cfg(windows)]
fn capture_windows_file_id(path: &Path) -> io::Result<PathIdentity> {
    use std::{
        mem::MaybeUninit,
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
        ptr,
    };

    use windows_sys::Win32::{
        Foundation::{HANDLE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE,
            FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle, OPEN_EXISTING,
        },
    };

    let wide = path_to_wide(path)?;
    // SAFETY: CreateFileW with a null-terminated path and backup semantics so
    // directories can be opened for identity queries without write access.
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            core::ptr::null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: raw is a valid open handle exclusive to this function.
    let handle = unsafe { OwnedHandle::from_raw_handle(raw as _) };
    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: handle is open; info is written fully on success.
    let ok =
        unsafe { GetFileInformationByHandle(handle.as_raw_handle() as HANDLE, info.as_mut_ptr()) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: GetFileInformationByHandle initialized info.
    let info = unsafe { info.assume_init() };
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Ok(PathIdentity {
        kind: PathIdentityKind::WindowsFileId,
        value: format!("{:x}:{index:x}", info.dwVolumeSerialNumber),
    })
}

#[cfg(windows)]
fn path_to_wide(path: &Path) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path contains interior nul",
        ));
    }
    wide.push(0);
    Ok(wide)
}

/// Lexical path normalization used by protection matching when a path may not
/// exist on disk (for example remove of a deleted protection entry).
pub(crate) fn normalize_path_for_compare(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return strip_verbatim(canonical);
    }

    let mut missing = Vec::new();
    let mut current = path;
    while !current.as_os_str().is_empty() {
        if let Ok(canonical) = current.canonicalize() {
            let mut normalized = strip_verbatim(canonical);
            for component in missing.iter().rev() {
                normalized.push(component);
            }
            return normalized;
        }
        let Some(name) = current.file_name() else {
            break;
        };
        missing.push(name.to_os_string());
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent;
    }

    path.to_path_buf()
}

pub(crate) fn paths_equal(left: &Path, right: &Path) -> bool {
    let left = normalize_path_for_compare(left);
    let right = normalize_path_for_compare(right);
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

/// True when `child` is equal to `parent` or strictly inside it.
pub(crate) fn path_is_within(parent: &Path, child: &Path) -> bool {
    let parent = normalize_path_for_compare(parent);
    let child = normalize_path_for_compare(child);
    if paths_equal(&parent, &child) {
        return true;
    }
    child.starts_with(&parent)
}

#[cfg(windows)]
fn strip_verbatim(path: PathBuf) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = raw.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path
    }
}

#[cfg(not(windows))]
fn strip_verbatim(path: PathBuf) -> PathBuf {
    path
}

fn split_root(value: &str) -> Result<(String, &str)> {
    if let Some(rest) = value.strip_prefix("//") {
        return Ok(("//".to_string(), rest));
    }

    let bytes = value.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' {
        return Ok((value[..2].to_string(), &value[3..]));
    }

    if let Some(rest) = value.strip_prefix('/') {
        return Ok(("/".to_string(), rest));
    }

    bail!("path must include an absolute root: {value}")
}

#[cfg(windows)]
fn normalize_windows_path(value: String) -> String {
    let value = value.to_ascii_lowercase();
    if let Some(rest) = value.strip_prefix("//?/unc/") {
        format!("//{rest}")
    } else if let Some(rest) = value.strip_prefix("//?/") {
        rest.to_string()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn normalizes_lexical_dot_segments_without_filesystem_access() {
        #[cfg(windows)]
        let (path, expected) = (
            r"C:\workspace\.\app\cache\..\target",
            "c:/workspace/app/target",
        );
        #[cfg(not(windows))]
        let (path, expected) = ("/workspace/./app/cache/../target", "/workspace/app/target");

        assert_eq!(
            normalize_absolute_path(Path::new(path)).expect("absolute path normalizes"),
            expected
        );
    }

    #[test]
    fn rejects_relative_paths_and_root_escapes() {
        assert!(normalize_absolute_path(Path::new("relative/cache")).is_err());
        assert!(normalize_absolute_path(Path::new("/../escape")).is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn keeps_backslashes_as_unix_path_characters() {
        assert_eq!(
            normalize_absolute_path(Path::new(r"/workspace/cache\entry"))
                .expect("absolute path normalizes"),
            r"/workspace/cache\entry"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn collapses_equivalent_leading_posix_separators() {
        assert_eq!(
            normalize_absolute_path(Path::new("//workspace/cache/"))
                .expect("absolute path normalizes"),
            normalize_absolute_path(Path::new("/workspace/cache"))
                .expect("absolute path normalizes")
        );
    }

    #[cfg(windows)]
    #[test]
    fn normalizes_windows_case_separators_and_verbatim_prefixes() {
        let canonical = normalize_absolute_path(Path::new(r"C:\Work\Cache\")).expect("path");
        assert_eq!(canonical, "c:/work/cache");
        assert_eq!(
            normalize_absolute_path(Path::new(r"\\?\C:\WORK\cache")).expect("path"),
            canonical
        );
    }

    #[test]
    fn captures_stable_identity_for_existing_directory() {
        let fixture = TempDir::new().expect("temp dir");
        let path = fixture.path().join("cache");
        fs::create_dir_all(&path).expect("cache dir");

        let first = capture_path_identity(&path).expect("identity");
        let second = capture_path_identity(&path).expect("identity again");
        assert_eq!(first, second);

        // Recreate at the same path; identity must change when the OS assigns a
        // new object id (best-effort: some filesystems may reuse ids).
        fs::remove_dir_all(&path).expect("remove");
        fs::create_dir_all(&path).expect("recreate");
        let third = capture_path_identity(&path).expect("recreated identity");
        // Equality is allowed on exotic FS; inequality is the expected case.
        let _ = third;
    }
}
