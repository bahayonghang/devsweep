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
    SourceInventory {
        evidence,
        observations,
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
        if matches!(status, ERROR_UNKNOWN_PRODUCT | ERROR_UNKNOWN_PROPERTY) {
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
}
