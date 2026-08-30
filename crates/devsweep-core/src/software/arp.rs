//! Allowlisted Add/Remove Programs registry inventory.

use super::{SourceInventory, model::*, reported_size};

#[cfg(windows)]
const UNINSTALL_ROOT: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

pub(super) fn inventory(observed_at_unix_ms: u64) -> SourceInventory {
    #[cfg(windows)]
    {
        windows_inventory(observed_at_unix_ms)
    }
    #[cfg(not(windows))]
    {
        let _ = observed_at_unix_ms;
        let mut evidence = Vec::new();
        for hive in [RegistryHive::CurrentUser, RegistryHive::LocalMachine] {
            for view in [RegistryView::Registry32, RegistryView::Registry64] {
                evidence.push(SoftwareSourceEvidence::unavailable(
                    SoftwareSourceId::Arp { hive, view },
                    SoftwareSourceState::Unsupported,
                    "windows_only_source",
                ));
            }
        }
        SourceInventory {
            evidence,
            observations: Vec::new(),
        }
    }
}

#[cfg(windows)]
fn windows_inventory(observed_at_unix_ms: u64) -> SourceInventory {
    let mut evidence = Vec::new();
    let mut observations = Vec::new();
    for hive in [RegistryHive::CurrentUser, RegistryHive::LocalMachine] {
        for view in [RegistryView::Registry32, RegistryView::Registry64] {
            let source = SoftwareSourceId::Arp { hive, view };
            let result = enumerate_source(hive, view);
            let complete = result.state == SoftwareSourceState::Available;
            evidence.push(if complete {
                SoftwareSourceEvidence::available(source.clone())
            } else {
                SoftwareSourceEvidence::unavailable(
                    source.clone(),
                    result.state,
                    result.reason_code,
                )
            });
            observations.extend(result.records.into_iter().map(|record| {
                observation_from_record(
                    hive,
                    view,
                    source.clone(),
                    record,
                    complete,
                    observed_at_unix_ms,
                )
            }));
        }
    }
    SourceInventory {
        evidence,
        observations,
    }
}

#[cfg(windows)]
struct EnumeratedArp {
    state: SoftwareSourceState,
    reason_code: &'static str,
    records: Vec<ArpRecord>,
}

#[cfg(windows)]
#[derive(Default)]
struct ArpRecord {
    subkey: String,
    display_name: Option<String>,
    display_version: Option<String>,
    publisher: Option<String>,
    estimated_size_kib: Option<u64>,
    no_remove: bool,
    system_component: bool,
    release_type: Option<String>,
    parent_key_name: Option<String>,
}

#[cfg(windows)]
fn observation_from_record(
    hive: RegistryHive,
    view: RegistryView,
    source: SoftwareSourceId,
    record: ArpRecord,
    source_complete: bool,
    observed_at_unix_ms: u64,
) -> SoftwareObservation {
    let protected =
        super::is_protected_product(record.display_name.as_deref(), record.publisher.as_deref());
    let release_type = record
        .release_type
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let system_or_update = record.parent_key_name.is_some()
        || matches!(
            release_type.as_str(),
            "update" | "hotfix" | "security update" | "service pack"
        );
    SoftwareObservation {
        identity: SoftwareIdentity::Arp {
            hive,
            view,
            subkey: record.subkey,
        },
        scope: if hive == RegistryHive::CurrentUser {
            SoftwareScope::CurrentUser
        } else {
            SoftwareScope::Machine
        },
        display_name: record.display_name.clone(),
        publisher: record.publisher,
        version: record.display_version,
        provenance: vec![source],
        size: reported_size(
            record.estimated_size_kib,
            SoftwareSizeSourceCode::ArpEstimatedSizeKib,
            observed_at_unix_ms,
        ),
        flags: EligibilityFlags {
            protected,
            source_incomplete: !source_complete,
            no_remove: record.no_remove,
            hidden: record.system_component || record.display_name.is_none(),
            system_or_update,
            ..EligibilityFlags::default()
        },
    }
}

#[cfg(windows)]
fn enumerate_source(hive: RegistryHive, view: RegistryView) -> EnumeratedArp {
    use native_registry::*;

    let root = match RegKey::open(predefined_hive(hive), UNINSTALL_ROOT, view) {
        Ok(Some(root)) => root,
        Ok(None) => {
            return EnumeratedArp {
                state: SoftwareSourceState::Available,
                reason_code: "empty_source",
                records: Vec::new(),
            };
        }
        Err(ERROR_ACCESS_DENIED) => {
            return EnumeratedArp {
                state: SoftwareSourceState::Permission,
                reason_code: "registry_access_denied",
                records: Vec::new(),
            };
        }
        Err(_) => {
            return EnumeratedArp {
                state: SoftwareSourceState::Partial,
                reason_code: "registry_open_failed",
                records: Vec::new(),
            };
        }
    };

    let mut records = Vec::new();
    let mut partial = false;
    let mut index = 0;
    loop {
        let subkey = match root.enum_subkey(index) {
            Ok(Some(name)) => name,
            Ok(None) => break,
            Err(_) => {
                partial = true;
                index += 1;
                continue;
            }
        };
        index += 1;
        let key = match RegKey::open(root.raw(), &subkey, view) {
            Ok(Some(key)) => key,
            Ok(None) => {
                partial = true;
                continue;
            }
            Err(_) => {
                partial = true;
                continue;
            }
        };
        let mut corrupt = false;
        let mut read_string = |name| match key.string(name) {
            Ok(value) => value,
            Err(()) => {
                corrupt = true;
                None
            }
        };
        let display_name = read_string("DisplayName");
        let display_version = read_string("DisplayVersion");
        let publisher = read_string("Publisher");
        let release_type = read_string("ReleaseType");
        let parent_key_name = read_string("ParentKeyName");
        let estimated_size_kib = match key.dword("EstimatedSize") {
            Ok(value) => value.map(u64::from),
            Err(()) => {
                corrupt = true;
                None
            }
        };
        let no_remove = match key.dword("NoRemove") {
            Ok(value) => value == Some(1),
            Err(()) => {
                corrupt = true;
                false
            }
        };
        let system_component = match key.dword("SystemComponent") {
            Ok(value) => value == Some(1),
            Err(()) => {
                corrupt = true;
                false
            }
        };
        partial |= corrupt;
        records.push(ArpRecord {
            subkey,
            display_name,
            display_version,
            publisher,
            estimated_size_kib,
            no_remove,
            system_component,
            release_type,
            parent_key_name,
        });
    }

    EnumeratedArp {
        state: if partial {
            SoftwareSourceState::Partial
        } else {
            SoftwareSourceState::Available
        },
        reason_code: if partial {
            "registry_entry_incomplete"
        } else {
            "complete"
        },
        records,
    }
}

#[cfg(windows)]
mod native_registry {
    use std::{ffi::c_void, ptr};

    use super::{RegistryHive, RegistryView};

    type Hkey = *mut c_void;
    const HKEY_CURRENT_USER: Hkey = -2_147_483_647_i32 as isize as Hkey;
    const HKEY_LOCAL_MACHINE: Hkey = -2_147_483_646_i32 as isize as Hkey;
    const KEY_READ: u32 = 0x0002_0019;
    const KEY_WOW64_64KEY: u32 = 0x0100;
    const KEY_WOW64_32KEY: u32 = 0x0200;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_FILE_NOT_FOUND: i32 = 2;
    pub(super) const ERROR_ACCESS_DENIED: i32 = 5;
    const ERROR_MORE_DATA: i32 = 234;
    const ERROR_NO_MORE_ITEMS: i32 = 259;
    const REG_SZ: u32 = 1;
    const REG_EXPAND_SZ: u32 = 2;
    const REG_DWORD: u32 = 4;
    const MAX_VALUE_BYTES: u32 = 64 * 1024;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            key: Hkey,
            subkey: *const u16,
            options: u32,
            access: u32,
            result: *mut Hkey,
        ) -> i32;
        fn RegCloseKey(key: Hkey) -> i32;
        fn RegEnumKeyExW(
            key: Hkey,
            index: u32,
            name: *mut u16,
            name_len: *mut u32,
            reserved: *mut u32,
            class: *mut u16,
            class_len: *mut u32,
            last_write_time: *mut c_void,
        ) -> i32;
        fn RegQueryValueExW(
            key: Hkey,
            value_name: *const u16,
            reserved: *mut u32,
            value_type: *mut u32,
            data: *mut u8,
            data_len: *mut u32,
        ) -> i32;
    }

    pub(super) fn predefined_hive(hive: RegistryHive) -> Hkey {
        match hive {
            RegistryHive::CurrentUser => HKEY_CURRENT_USER,
            RegistryHive::LocalMachine => HKEY_LOCAL_MACHINE,
        }
    }

    pub(super) struct RegKey(Hkey);

    impl RegKey {
        pub(super) fn open(
            parent: Hkey,
            subkey: &str,
            view: RegistryView,
        ) -> Result<Option<Self>, i32> {
            let subkey = wide(subkey);
            let access = KEY_READ
                | match view {
                    RegistryView::Registry32 => KEY_WOW64_32KEY,
                    RegistryView::Registry64 => KEY_WOW64_64KEY,
                };
            let mut result = ptr::null_mut();
            // SAFETY: all pointers refer to live buffers and `result` is writable.
            let status = unsafe { RegOpenKeyExW(parent, subkey.as_ptr(), 0, access, &mut result) };
            match status {
                ERROR_SUCCESS => Ok(Some(Self(result))),
                ERROR_FILE_NOT_FOUND => Ok(None),
                other => Err(other),
            }
        }

        pub(super) fn raw(&self) -> Hkey {
            self.0
        }

        pub(super) fn enum_subkey(&self, index: u32) -> Result<Option<String>, ()> {
            let mut buffer = [0_u16; 256];
            let mut len = u32::try_from(buffer.len()).expect("registry name bound fits u32");
            // SAFETY: the name buffer is writable for `len` UTF-16 code units.
            let status = unsafe {
                RegEnumKeyExW(
                    self.0,
                    index,
                    buffer.as_mut_ptr(),
                    &mut len,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            match status {
                ERROR_SUCCESS => {
                    let len = usize::try_from(len).map_err(|_| ())?;
                    String::from_utf16(buffer.get(..len).ok_or(())?)
                        .map(Some)
                        .map_err(|_| ())
                }
                ERROR_NO_MORE_ITEMS => Ok(None),
                ERROR_MORE_DATA => Err(()),
                _ => Err(()),
            }
        }

        pub(super) fn string(&self, name: &str) -> Result<Option<String>, ()> {
            let Some((value_type, bytes)) = self.value(name)? else {
                return Ok(None);
            };
            if !matches!(value_type, REG_SZ | REG_EXPAND_SZ) || bytes.len() % 2 != 0 {
                return Err(());
            }
            let mut utf16 = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect::<Vec<_>>();
            while utf16.last() == Some(&0) {
                utf16.pop();
            }
            String::from_utf16(&utf16).map(Some).map_err(|_| ())
        }

        pub(super) fn dword(&self, name: &str) -> Result<Option<u32>, ()> {
            let Some((value_type, bytes)) = self.value(name)? else {
                return Ok(None);
            };
            if value_type != REG_DWORD || bytes.len() != 4 {
                return Err(());
            }
            Ok(Some(u32::from_le_bytes(bytes.try_into().map_err(|_| ())?)))
        }

        fn value(&self, name: &str) -> Result<Option<(u32, Vec<u8>)>, ()> {
            let name = wide(name);
            let mut value_type = 0;
            let mut len = 0;
            // SAFETY: this first call requests only the required byte count.
            let status = unsafe {
                RegQueryValueExW(
                    self.0,
                    name.as_ptr(),
                    ptr::null_mut(),
                    &mut value_type,
                    ptr::null_mut(),
                    &mut len,
                )
            };
            if status == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            if !matches!(status, ERROR_SUCCESS | ERROR_MORE_DATA) || len > MAX_VALUE_BYTES {
                return Err(());
            }
            let mut data = vec![0_u8; usize::try_from(len).map_err(|_| ())?];
            // SAFETY: `data` is writable for exactly the advertised length.
            let status = unsafe {
                RegQueryValueExW(
                    self.0,
                    name.as_ptr(),
                    ptr::null_mut(),
                    &mut value_type,
                    data.as_mut_ptr(),
                    &mut len,
                )
            };
            if status != ERROR_SUCCESS {
                return Err(());
            }
            data.truncate(usize::try_from(len).map_err(|_| ())?);
            Ok(Some((value_type, data)))
        }
    }

    impl Drop for RegKey {
        fn drop(&mut self) {
            // SAFETY: this wrapper uniquely owns the key returned by RegOpenKeyExW.
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn adapter_allowlist_contains_no_command_or_icon_fields() {
        let source = include_str!("arp.rs");
        for forbidden in [
            ["Quiet", "Uninstall", "String"].concat(),
            ["Uninstall", "String"].concat(),
            ["Display", "Icon"].concat(),
        ] {
            assert!(!source.contains(&forbidden));
        }
    }
}
