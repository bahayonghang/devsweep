//! Shared read-only physical-file binding for Software host commands.

use crate::{
    digest_encoding::hex_digest,
    filesystem::{
        PathIdentity, PathReparseProbe, PathSafety, SystemPathReparseProbe, capture_path_identity,
        inspect_entry_no_follow,
    },
};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub(super) struct BoundFile {
    pub path: PathBuf,
    pub digest: String,
    _held: Vec<File>,
}

impl BoundFile {
    pub(super) fn open(path: &Path) -> Result<Self, ()> {
        Self::open_with_probe(path, &SystemPathReparseProbe)
    }

    fn open_with_probe(path: &Path, probe: &dyn PathReparseProbe) -> Result<Self, ()> {
        if !path.is_absolute() {
            return Err(());
        }
        #[cfg(windows)]
        {
            let root = local_disk_root(path).ok_or(())?;
            // Reject UNC, device namespaces and mapped network drives before any
            // path/file probe. Only fixed local disks are command authority.
            if unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(root.as_ptr()) } != 3
            {
                return Err(());
            }
        }
        let mut held = Vec::new();
        for ancestor in path.ancestors().filter(|path| !path.as_os_str().is_empty()) {
            let entry = inspect_entry_no_follow(ancestor, probe).map_err(|_| ())?;
            if entry.safety != PathSafety::Safe || (ancestor != path && !entry.is_dir) {
                return Err(());
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                // Retain every name: deny deletion/rename; the leaf also denies writes.
                let file = OpenOptions::new()
                    .read(true)
                    .access_mode(if ancestor == path { 0x8000_0000 } else { 0 })
                    .share_mode(1)
                    .custom_flags(0x0020_0000 | 0x0200_0000)
                    .open(ancestor)
                    .map_err(|_| ())?;
                held.push(file);
            }
        }
        // Recheck after retaining every Windows name. A reparse introduced
        // between an earlier probe and its open must not become authority.
        for ancestor in path.ancestors().filter(|path| !path.as_os_str().is_empty()) {
            let entry = inspect_entry_no_follow(ancestor, probe).map_err(|_| ())?;
            if entry.safety != PathSafety::Safe || (ancestor != path && !entry.is_dir) {
                return Err(());
            }
        }
        let path = path.canonicalize().map_err(|_| ())?;
        #[cfg(windows)]
        let mut file = held.first().ok_or(())?.try_clone().map_err(|_| ())?;
        #[cfg(not(windows))]
        let mut file = OpenOptions::new().read(true).open(&path).map_err(|_| ())?;
        let metadata = file.metadata().map_err(|_| ())?;
        if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
            return Err(());
        }
        let identity: PathIdentity = capture_path_identity(&path).map_err(|_| ())?;
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Storage::FileSystem::{
                BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
            };
            let mut opened = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
            // SAFETY: the retained read-only handle is live and opened is writable.
            if unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, opened.as_mut_ptr()) }
                == 0
            {
                return Err(());
            }
            let opened = unsafe { opened.assume_init() };
            let index = (u64::from(opened.nFileIndexHigh) << 32) | u64::from(opened.nFileIndexLow);
            if identity.value != format!("{:x}:{index:x}", opened.dwVolumeSerialNumber) {
                return Err(());
            }
        }
        let mut hash = Sha256::new();
        let mut bytes = [0; 64 * 1024];
        file.rewind().map_err(|_| ())?;
        loop {
            let count = file.read(&mut bytes).map_err(|_| ())?;
            if count == 0 {
                break;
            }
            hash.update(&bytes[..count]);
        }
        let digest = digest(&(
            "devsweep.software.file.v1",
            &path,
            identity,
            hex_digest(hash.finalize()),
        ))?;
        held.push(file);
        Ok(Self {
            path,
            digest,
            _held: held,
        })
    }
}

#[cfg(test)]
#[test]
fn reparse_introduced_before_names_are_retained_is_refused() {
    use crate::filesystem::ReparseProbeResult;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct ChangedPath {
        path: PathBuf,
        probes: AtomicUsize,
    }
    impl PathReparseProbe for ChangedPath {
        fn probe(&self, path: &Path) -> ReparseProbeResult {
            if path == self.path && self.probes.fetch_add(1, Ordering::SeqCst) > 0 {
                ReparseProbeResult::ReparsePoint { tag: 0xA000_0003 }
            } else {
                ReparseProbeResult::NotReparsePoint
            }
        }
    }
    let directory = crate::test_support::tempdir().unwrap();
    let path = directory.path().join("tool.exe");
    std::fs::write(&path, b"isolated tool").unwrap();
    for changed in [&path, directory.path()] {
        let probe = ChangedPath {
            path: changed.to_path_buf(),
            probes: AtomicUsize::new(0),
        };
        assert!(BoundFile::open_with_probe(&path, &probe).is_err());
    }
    assert!(BoundFile::open(&path).is_ok());
}

#[cfg(windows)]
fn local_disk_root(path: &Path) -> Option<[u16; 4]> {
    use std::path::{Component, Prefix};
    match path.components().next()? {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) if letter.is_ascii_alphabetic() => {
                Some([u16::from(letter), u16::from(b':'), u16::from(b'\\'), 0])
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(all(test, windows))]
#[test]
fn disk_root_policy_rejects_unc_devices_and_relative_names_without_probing_them() {
    for value in [
        r"\\server\share\tool.exe",
        r"\\?\UNC\server\share\tool.exe",
        r"\\.\NUL",
        "tool.exe",
    ] {
        assert!(local_disk_root(Path::new(value)).is_none());
    }
    assert_eq!(
        local_disk_root(Path::new(r"C:\Apps\tool.exe")),
        Some([67, 58, 92, 0])
    );
    assert_eq!(
        local_disk_root(Path::new(r"\\?\C:\Apps\tool.exe")),
        Some([67, 58, 92, 0])
    );
}

pub(super) fn digest(value: &impl serde::Serialize) -> Result<String, ()> {
    let bytes = serde_json::to_vec(value).map_err(|_| ())?;
    Ok(format!("sha256:{}", hex_digest(Sha256::digest(bytes))))
}
