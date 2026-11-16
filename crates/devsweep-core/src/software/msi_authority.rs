//! MSI evidence and closed system-tool resolution.

use super::MsiRegistrationEvidenceV1;
pub(super) use super::file_authority::{BoundFile, digest};
use std::path::PathBuf;

pub(super) fn valid_sid(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix("S-1-").filter(|_| value.len() <= 184) else {
        return false;
    };
    let decimal = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let mut parts = suffix.split('-');
    let Some(authority) = parts.next() else {
        return false;
    };
    if !decimal(authority)
        || !authority
            .parse::<u64>()
            .is_ok_and(|value| value < (1 << 48))
    {
        return false;
    }
    let subauthorities: Vec<_> = parts.collect();
    (1..=15).contains(&subauthorities.len())
        && subauthorities
            .iter()
            .all(|part| decimal(part) && part.parse::<u32>().is_ok())
}

#[cfg(test)]
#[test]
fn sid_evidence_requires_windows_numeric_bounds() {
    for sid in ["S-1-5-21-100", "S-1-0-0", "S-1-281474976710655-4294967295"] {
        assert!(valid_sid(sid), "{sid}");
    }
    for sid in [
        "S-1-+5-21-100".to_string(),
        "S-1-5-+21-100".to_string(),
        "S-1-5-4294967296".to_string(),
        "S-1-281474976710656-1".to_string(),
        format!("S-1-5-{}", ["1"; 16].join("-")),
        "S-1-5".to_string(),
    ] {
        assert!(!valid_sid(&sid), "{sid}");
    }
}

pub(super) fn valid_evidence(evidence: &MsiRegistrationEvidenceV1) -> bool {
    valid_sid(&evidence.user_sid)
        && super::launch::valid_product_code(&evidence.package_code)
        && [&evidence.registration_digest, &evidence.tool_digest]
            .into_iter()
            .all(|value| {
                value.strip_prefix("sha256:").is_some_and(|hex| {
                    hex.len() == 64
                        && hex
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
            })
}

pub(super) fn system_tool() -> Result<BoundFile, ()> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
        let mut buffer = vec![0; 32768];
        // SAFETY: the output buffer is valid for its supplied UTF-16 capacity.
        let len = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
        if len == 0 || len as usize >= buffer.len() {
            return Err(());
        }
        let directory = PathBuf::from(String::from_utf16(&buffer[..len as usize]).map_err(|_| ())?);
        let bound = BoundFile::open(&directory.join("msiexec.exe"))?;
        // Canonicalization observes the same WOW64 view used for the retained leaf;
        // dispatch uses this physical path, with no redirection-mode changes.
        if bound.path.parent() != Some(directory.canonicalize().map_err(|_| ())?.as_path()) {
            return Err(());
        }
        Ok(bound)
    }
    #[cfg(not(windows))]
    {
        Err(())
    }
}
