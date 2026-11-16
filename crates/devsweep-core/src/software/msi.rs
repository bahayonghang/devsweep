//! Authoritative current-user and machine Windows Installer identity inventory.

use super::{SourceInventory, model::*, reported_size};

pub(super) fn inventory(observed_at_unix_ms: u64) -> SourceInventory {
    #[cfg(windows)]
    {
        windows_inventory(observed_at_unix_ms)
    }
    #[cfg(not(windows))]
    {
        let _ = observed_at_unix_ms;
        SourceInventory {
            evidence: contexts()
                .into_iter()
                .map(|context| {
                    SoftwareSourceEvidence::unavailable(
                        SoftwareSourceId::Msi { context },
                        SoftwareSourceState::Unsupported,
                        "windows_only_source",
                    )
                })
                .collect(),
            observations: Vec::new(),
        }
    }
}

fn contexts() -> [MsiContext; 3] {
    [
        MsiContext::UserUnmanaged,
        MsiContext::UserManaged,
        MsiContext::Machine,
    ]
}

#[cfg(windows)]
fn windows_inventory(observed_at_unix_ms: u64) -> SourceInventory {
    let current_sid = super::msix::current_process_sid();
    let mut evidence = Vec::new();
    let mut observations = Vec::new();
    for context in contexts() {
        let source = SoftwareSourceId::Msi { context };
        let sid = if context == MsiContext::Machine {
            None
        } else {
            match current_sid.as_deref() {
                Ok(sid) => Some(sid),
                Err(_) => {
                    evidence.push(SoftwareSourceEvidence::unavailable(
                        source,
                        SoftwareSourceState::Partial,
                        "current_user_sid_unavailable",
                    ));
                    continue;
                }
            }
        };
        let result = enumerate_context(context, sid);
        let complete = result.state == SoftwareSourceState::Available;
        evidence.push(if complete {
            SoftwareSourceEvidence::available(source.clone())
        } else {
            SoftwareSourceEvidence::unavailable(source.clone(), result.state, result.reason_code)
        });
        observations.extend(result.records.into_iter().map(|record| {
            let protected = super::is_protected_product(
                record.display_name.as_deref(),
                record.publisher.as_deref(),
            );
            SoftwareObservation {
                msi_registration: None,
                identity: SoftwareIdentity::Msi {
                    product_code: record.product_code,
                    context,
                },
                scope: if context == MsiContext::Machine {
                    SoftwareScope::Machine
                } else {
                    SoftwareScope::CurrentUser
                },
                display_name: record.display_name.clone(),
                publisher: record.publisher,
                version: record.version,
                icon: None,
                installed_on: None,
                provenance: vec![source.clone()],
                size: reported_size(
                    record.estimated_size_kib,
                    SoftwareSizeSourceCode::MsiEstimatedSizeKib,
                    observed_at_unix_ms,
                ),
                flags: EligibilityFlags {
                    protected,
                    source_incomplete: !complete,
                    conflicting_identity: record.context_conflict,
                    hidden: record.display_name.is_none(),
                    ..EligibilityFlags::default()
                },
            }
        }));
    }
    let tool = super::msi_authority::system_tool();
    for observation in &mut observations {
        let SoftwareIdentity::Msi {
            product_code,
            context: MsiContext::UserUnmanaged,
        } = &observation.identity
        else {
            continue;
        };
        if let (Ok(mut registration), Ok(tool)) = (bind_registration(product_code), &tool) {
            registration.evidence.tool_digest = tool.digest.clone();
            observation.msi_registration = Some(registration.evidence);
        }
    }
    SourceInventory {
        evidence,
        observations,
    }
}

pub(super) struct MsiRegistrationBinding {
    pub evidence: MsiRegistrationEvidenceV1,
    pub _package: Option<super::msi_authority::BoundFile>,
}

/// Independent exact current-user registration proof, also used just before spawn.
pub(super) fn bind_registration(product_code: &str) -> Result<MsiRegistrationBinding, ()> {
    #[cfg(windows)]
    {
        if !super::launch::valid_product_code(product_code) {
            return Err(());
        }
        let user_sid = super::msix::current_process_sid()?;
        let mut matches = Vec::new();
        for context in contexts() {
            let found = enumerate_context(
                context,
                if context == MsiContext::Machine {
                    None
                } else {
                    Some(&user_sid)
                },
            );
            if found.state != SoftwareSourceState::Available {
                return Err(());
            }
            matches.extend(
                found
                    .records
                    .into_iter()
                    .filter(|record| record.product_code.eq_ignore_ascii_case(product_code))
                    .map(|record| (context, record)),
            );
        }
        let [(MsiContext::UserUnmanaged, record)] = matches.as_slice() else {
            return Err(());
        };
        if record.context_conflict
            || super::is_protected_product(
                record.display_name.as_deref(),
                record.publisher.as_deref(),
            )
        {
            return Err(());
        }
        let property = |name| {
            native_msi::product_property(
                product_code,
                MsiContext::UserUnmanaged,
                Some(&user_sid),
                name,
            )
            .map_err(|_| ())?
            .ok_or(())
        };
        if native_msi::product_state(product_code, &user_sid)
            .map_err(|_| ())?
            .as_deref()
            != Some("5")
        {
            return Err(());
        }
        let version = property("VersionString")?;
        if version.is_empty() {
            return Err(());
        }
        let package_code = property("PackageCode")?;
        if !super::launch::valid_product_code(&package_code) {
            return Err(());
        }
        let restrictions = super::arp::msi_restrictions(product_code)?;
        let package = super::msi_authority::BoundFile::open(std::path::Path::new(&property(
            "LocalPackage",
        )?))?;
        let (word_count, revision) = native_msi::package_summary(&package.path)?;
        if word_count & 8 == 0 || !revision.eq_ignore_ascii_case(&package_code) {
            return Err(());
        }
        let registration_digest = super::msi_authority::digest(&(
            "devsweep.msi.registration.v1",
            product_code,
            &user_sid,
            MsiContext::UserUnmanaged,
            version,
            &package_code,
            restrictions,
            &package.digest,
            word_count,
        ))?;
        Ok(MsiRegistrationBinding {
            evidence: MsiRegistrationEvidenceV1 {
                user_sid,
                package_code,
                registration_digest,
                tool_digest: String::new(),
            },
            _package: Some(package),
        })
    }
    #[cfg(not(windows))]
    {
        let _ = product_code;
        Err(())
    }
}

pub(super) fn query_registration(
    identity: &SoftwareIdentity,
    user_sid: &str,
) -> super::SoftwareInstalledState {
    use super::SoftwareInstalledState as State;
    #[cfg(windows)]
    {
        let SoftwareIdentity::Msi {
            product_code,
            context: MsiContext::UserUnmanaged,
        } = identity
        else {
            return State::Conflicting;
        };
        if !super::launch::valid_product_code(product_code)
            || super::msix::current_process_sid().as_deref() != Ok(user_sid)
        {
            return State::Conflicting;
        }
        match native_msi::product_state(product_code, user_sid) {
            Ok(None) => State::Absent,
            Ok(Some(state)) if state == "5" || state == "1" => State::Present,
            Ok(Some(_)) => State::Conflicting,
            Err(_) => State::Unavailable,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (identity, user_sid);
        State::Unavailable
    }
}

#[cfg(windows)]
struct EnumeratedMsi {
    state: SoftwareSourceState,
    reason_code: &'static str,
    records: Vec<MsiRecord>,
}

#[cfg(windows)]
struct MsiRecord {
    product_code: String,
    display_name: Option<String>,
    publisher: Option<String>,
    version: Option<String>,
    estimated_size_kib: Option<u64>,
    context_conflict: bool,
}

#[cfg(windows)]
fn enumerate_context(context: MsiContext, current_sid: Option<&str>) -> EnumeratedMsi {
    use native_msi::*;

    let mut records = Vec::new();
    let mut partial = false;
    let mut permission = false;
    let mut index = 0;
    loop {
        match enum_product(context, current_sid, index) {
            Ok(Some(product)) => {
                index += 1;
                let mut property_failed = false;
                let mut property = |name| match product_property(
                    &product.product_code,
                    context,
                    current_sid,
                    name,
                ) {
                    Ok(value) => value,
                    Err(PropertyError::AccessDenied) => {
                        permission = true;
                        property_failed = true;
                        None
                    }
                    Err(PropertyError::Unavailable) => {
                        property_failed = true;
                        None
                    }
                };
                let display_name = property("ProductName");
                let publisher = property("Publisher");
                let version = property("VersionString");
                let estimated_size_kib =
                    property("EstimatedSize").and_then(|value| value.parse::<u64>().ok());
                partial |= property_failed;
                records.push(MsiRecord {
                    product_code: product.product_code,
                    display_name,
                    publisher,
                    version,
                    estimated_size_kib,
                    context_conflict: product.actual_context != context,
                });
            }
            Ok(None) => break,
            Err(EnumError::AccessDenied) => {
                permission = true;
                break;
            }
            Err(EnumError::Failed) => {
                partial = true;
                break;
            }
        }
    }
    EnumeratedMsi {
        state: if permission && records.is_empty() {
            SoftwareSourceState::Permission
        } else if permission || partial {
            SoftwareSourceState::Partial
        } else {
            SoftwareSourceState::Available
        },
        reason_code: if permission {
            "msi_access_denied"
        } else if partial {
            "msi_inventory_incomplete"
        } else {
            "complete"
        },
        records,
    }
}

#[cfg(windows)]
mod native_msi {
    use std::ptr;

    use super::MsiContext;

    const ERROR_SUCCESS: u32 = 0;
    const ERROR_ACCESS_DENIED: u32 = 5;
    const ERROR_MORE_DATA: u32 = 234;
    const ERROR_NO_MORE_ITEMS: u32 = 259;
    const ERROR_UNKNOWN_PRODUCT: u32 = 1605;
    const ERROR_UNKNOWN_PROPERTY: u32 = 1608;
    const MAX_PROPERTY_CHARS: u32 = 32 * 1024;

    #[link(name = "msi")]
    unsafe extern "system" {
        fn MsiEnumProductsExW(
            product_code_filter: *const u16,
            user_sid: *const u16,
            context: u32,
            index: u32,
            installed_product_code: *mut u16,
            installed_context: *mut u32,
            sid: *mut u16,
            sid_len: *mut u32,
        ) -> u32;
        fn MsiGetProductInfoExW(
            product_code: *const u16,
            user_sid: *const u16,
            context: u32,
            property: *const u16,
            value: *mut u16,
            value_len: *mut u32,
        ) -> u32;
        fn MsiGetSummaryInformationW(
            database: u32,
            path: *const u16,
            update_count: u32,
            summary: *mut u32,
        ) -> u32;
        fn MsiSummaryInfoGetPropertyW(
            summary: u32,
            property: u32,
            data_type: *mut u32,
            integer: *mut i32,
            filetime: *mut core::ffi::c_void,
            value: *mut u16,
            length: *mut u32,
        ) -> u32;
        fn MsiCloseHandle(handle: u32) -> u32;
    }

    pub(super) fn package_summary(path: &std::path::Path) -> Result<(i32, String), ()> {
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let mut handle = 0;
        // SAFETY: valid terminated path; zero updates opens the summary read-only.
        if unsafe { MsiGetSummaryInformationW(0, path.as_ptr(), 0, &mut handle) } != ERROR_SUCCESS {
            return Err(());
        }
        struct Summary(u32);
        impl Drop for Summary {
            fn drop(&mut self) {
                unsafe {
                    MsiCloseHandle(self.0);
                }
            }
        }
        let summary = Summary(handle);
        let mut kind = 0;
        let mut count = 0;
        // SAFETY: PID_WORDCOUNT (15) is an integer; all supplied outputs are valid.
        if unsafe {
            MsiSummaryInfoGetPropertyW(
                summary.0,
                15,
                &mut kind,
                &mut count,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        } != ERROR_SUCCESS
            || kind != 3
        {
            return Err(());
        }
        let mut length = 0;
        // SAFETY: a null string buffer requests the size of PID_REVNUMBER (9).
        let status = unsafe {
            MsiSummaryInfoGetPropertyW(
                summary.0,
                9,
                &mut kind,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                &mut length,
            )
        };
        if !matches!(status, ERROR_SUCCESS | ERROR_MORE_DATA) || kind != 30 || length > 1024 {
            return Err(());
        }
        let mut value = vec![0; length as usize + 1];
        length += 1;
        // SAFETY: value is writable for length UTF-16 characters, including NUL.
        if unsafe {
            MsiSummaryInfoGetPropertyW(
                summary.0,
                9,
                &mut kind,
                ptr::null_mut(),
                ptr::null_mut(),
                value.as_mut_ptr(),
                &mut length,
            )
        } != ERROR_SUCCESS
        {
            return Err(());
        }
        Ok((
            count,
            String::from_utf16(value.get(..length as usize).ok_or(())?).map_err(|_| ())?,
        ))
    }

    pub(super) enum EnumError {
        AccessDenied,
        Failed,
    }

    pub(super) enum PropertyError {
        AccessDenied,
        Unavailable,
    }

    pub(super) struct EnumeratedProduct {
        pub product_code: String,
        pub actual_context: MsiContext,
    }

    pub(super) fn enum_product(
        context: MsiContext,
        current_sid: Option<&str>,
        index: u32,
    ) -> Result<Option<EnumeratedProduct>, EnumError> {
        let sid = current_sid.map(wide);
        let sid_ptr = sid.as_ref().map_or(ptr::null(), |value| value.as_ptr());
        let mut product_code = [0_u16; 39];
        let mut actual_context = 0;
        // SAFETY: output buffers have the Windows Installer documented sizes;
        // null filter/SID output pointers intentionally request only this context.
        let status = unsafe {
            MsiEnumProductsExW(
                ptr::null(),
                sid_ptr,
                context_flag(context),
                index,
                product_code.as_mut_ptr(),
                &mut actual_context,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        match status {
            ERROR_SUCCESS => {
                let length = product_code
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(product_code.len());
                let product_code =
                    String::from_utf16(&product_code[..length]).map_err(|_| EnumError::Failed)?;
                if !is_product_code(&product_code) {
                    return Err(EnumError::Failed);
                }
                Ok(Some(EnumeratedProduct {
                    product_code,
                    actual_context: context_from_flag(actual_context).ok_or(EnumError::Failed)?,
                }))
            }
            ERROR_NO_MORE_ITEMS => Ok(None),
            ERROR_ACCESS_DENIED => Err(EnumError::AccessDenied),
            _ => Err(EnumError::Failed),
        }
    }

    pub(super) fn product_property(
        product_code: &str,
        context: MsiContext,
        current_sid: Option<&str>,
        property: &str,
    ) -> Result<Option<String>, PropertyError> {
        read_property(product_code, context, current_sid, property, true)
    }

    pub(super) fn product_state(
        product_code: &str,
        current_sid: &str,
    ) -> Result<Option<String>, PropertyError> {
        // INSTALLPROPERTY_PRODUCTSTATE is "State" in the Windows SDK, not
        // "ProductState". Only ERROR_UNKNOWN_PRODUCT can prove exact absence.
        read_property(
            product_code,
            MsiContext::UserUnmanaged,
            Some(current_sid),
            "State",
            false,
        )
    }

    fn read_property(
        product_code: &str,
        context: MsiContext,
        current_sid: Option<&str>,
        property: &str,
        optional_property: bool,
    ) -> Result<Option<String>, PropertyError> {
        let product_code = wide(product_code);
        let sid = current_sid.map(wide);
        let sid_ptr = sid.as_ref().map_or(ptr::null(), |value| value.as_ptr());
        let property = wide(property);
        let mut len = 0;
        // SAFETY: first call requests only the property length.
        let status = unsafe {
            MsiGetProductInfoExW(
                product_code.as_ptr(),
                sid_ptr,
                context_flag(context),
                property.as_ptr(),
                ptr::null_mut(),
                &mut len,
            )
        };
        if status == ERROR_UNKNOWN_PRODUCT
            || (optional_property && status == ERROR_UNKNOWN_PROPERTY)
        {
            return Ok(None);
        }
        if status == ERROR_ACCESS_DENIED {
            return Err(PropertyError::AccessDenied);
        }
        if !matches!(status, ERROR_SUCCESS | ERROR_MORE_DATA) || len > MAX_PROPERTY_CHARS {
            return Err(PropertyError::Unavailable);
        }
        let capacity = len.checked_add(1).ok_or(PropertyError::Unavailable)?;
        let mut value =
            vec![0_u16; usize::try_from(capacity).map_err(|_| PropertyError::Unavailable)?];
        let mut output_len = capacity;
        // SAFETY: output buffer contains `capacity` UTF-16 units and the API is
        // given its documented character count excluding the terminator.
        let status = unsafe {
            MsiGetProductInfoExW(
                product_code.as_ptr(),
                sid_ptr,
                context_flag(context),
                property.as_ptr(),
                value.as_mut_ptr(),
                &mut output_len,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(if status == ERROR_ACCESS_DENIED {
                PropertyError::AccessDenied
            } else {
                PropertyError::Unavailable
            });
        }
        let output_len = usize::try_from(output_len).map_err(|_| PropertyError::Unavailable)?;
        String::from_utf16(value.get(..output_len).ok_or(PropertyError::Unavailable)?)
            .map(Some)
            .map_err(|_| PropertyError::Unavailable)
    }

    fn context_flag(context: MsiContext) -> u32 {
        match context {
            MsiContext::UserManaged => 1,
            MsiContext::UserUnmanaged => 2,
            MsiContext::Machine => 4,
        }
    }

    fn context_from_flag(flag: u32) -> Option<MsiContext> {
        match flag {
            1 => Some(MsiContext::UserManaged),
            2 => Some(MsiContext::UserUnmanaged),
            4 => Some(MsiContext::Machine),
            _ => None,
        }
    }

    fn is_product_code(value: &str) -> bool {
        let bytes = value.as_bytes();
        bytes.len() == 38
            && bytes.first() == Some(&b'{')
            && bytes.last() == Some(&b'}')
            && [9, 14, 19, 24]
                .into_iter()
                .all(|index| bytes[index] == b'-')
            && bytes[1..37]
                .iter()
                .enumerate()
                .all(|(index, byte)| [8, 13, 18, 23].contains(&index) || byte.is_ascii_hexdigit())
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_msi_contexts_are_explicit_and_never_infer_execution_support() {
        assert_eq!(contexts().len(), 3);
        assert!(contexts().contains(&MsiContext::Machine));
        assert!(contexts().contains(&MsiContext::UserManaged));
        assert!(contexts().contains(&MsiContext::UserUnmanaged));
    }

    #[cfg(windows)]
    #[test]
    fn native_summary_reads_the_bound_cached_package_without_installing_it() {
        use std::{os::windows::ffi::OsStrExt, ptr};

        #[link(name = "msi")]
        unsafe extern "system" {
            fn MsiOpenDatabaseW(path: *const u16, mode: *const u16, database: *mut u32) -> u32;
            fn MsiGetSummaryInformationW(
                database: u32,
                path: *const u16,
                updates: u32,
                summary: *mut u32,
            ) -> u32;
            fn MsiSummaryInfoSetPropertyW(
                summary: u32,
                property: u32,
                kind: u32,
                integer: i32,
                filetime: *const core::ffi::c_void,
                value: *const u16,
            ) -> u32;
            fn MsiSummaryInfoPersist(summary: u32) -> u32;
            fn MsiDatabaseCommit(database: u32) -> u32;
            fn MsiCloseHandle(handle: u32) -> u32;
        }
        struct Handle(u32);
        impl Drop for Handle {
            fn drop(&mut self) {
                // SAFETY: the fixture owns each successful Installer handle.
                unsafe { MsiCloseHandle(self.0) };
            }
        }
        let directory = crate::test_support::tempdir().unwrap();
        let path = directory.path().join("cached package 缓存.msi");
        let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let code = "{12345678-1234-1234-1234-1234567890AB}";
        let code_wide: Vec<u16> = code.encode_utf16().chain([0]).collect();
        let mut database = 0;
        // SAFETY: MSIDBOPEN_CREATE is the documented integer pointer 3; this
        // creates only a temporary database, without registering/installing it.
        assert_eq!(
            unsafe {
                MsiOpenDatabaseW(
                    path_wide.as_ptr(),
                    ptr::without_provenance(3),
                    &mut database,
                )
            },
            0
        );
        let database = Handle(database);
        let mut summary = 0;
        // SAFETY: the database is live; two fixture summary fields will change.
        assert_eq!(
            unsafe { MsiGetSummaryInformationW(database.0, ptr::null(), 2, &mut summary) },
            0
        );
        let summary = Handle(summary);
        // SAFETY: PID_WORDCOUNT/VT_I4 and PID_REVNUMBER/VT_LPSTR use the
        // documented integer/string inputs. Both strings/buffers remain live.
        unsafe {
            assert_eq!(
                MsiSummaryInfoSetPropertyW(summary.0, 15, 3, 8, ptr::null(), ptr::null()),
                0
            );
            assert_eq!(
                MsiSummaryInfoSetPropertyW(summary.0, 9, 30, 0, ptr::null(), code_wide.as_ptr()),
                0
            );
            assert_eq!(MsiSummaryInfoPersist(summary.0), 0);
            assert_eq!(MsiDatabaseCommit(database.0), 0);
        }
        drop(summary);
        drop(database);
        let bound = super::super::msi_authority::BoundFile::open(&path).unwrap();
        assert_eq!(
            native_msi::package_summary(&bound.path).unwrap(),
            (8, code.to_string())
        );
    }
}
