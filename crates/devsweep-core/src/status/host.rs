//! Static machine identity and GPU adapter names, read once per sampler.
//!
//! CPU model and the release label come from read-only HKLM registry values,
//! the kernel version from `RtlGetVersion`, boot time from `GetTickCount64`,
//! and adapter names from the D3DKMT driver description joined by LUID. No
//! process, WMI, COM, service or elevation is involved.

#![cfg_attr(not(windows), allow(dead_code))]

use super::{AvailabilityV1, GpuV1, HostV1};

/// Raw identity readings before projection into `HostV1`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HostReadings {
    pub(crate) cpu_model: Option<String>,
    pub(crate) os_version: Option<String>,
    pub(crate) boot_time_unix_ms: Option<u64>,
}

/// All fields present: `available`. Some missing: `partial` with one reason
/// per missing field. None present: `unavailable` / `api_unavailable`.
pub(crate) fn host_availability(
    readings: HostReadings,
    sampled_at_unix_ms: u64,
) -> AvailabilityV1<HostV1> {
    let mut reasons = Vec::new();
    if readings.cpu_model.is_none() {
        reasons.push("cpu_model_unavailable".to_string());
    }
    if readings.os_version.is_none() {
        reasons.push("os_version_unavailable".to_string());
    }
    if readings.boot_time_unix_ms.is_none() {
        reasons.push("boot_time_unavailable".to_string());
    }
    if reasons.len() == 3 {
        return AvailabilityV1::unavailable("api_unavailable");
    }
    let value = HostV1 {
        cpu_model: readings.cpu_model,
        os_version: readings.os_version,
        boot_time_unix_ms: readings.boot_time_unix_ms,
    };
    if reasons.is_empty() {
        AvailabilityV1::available(sampled_at_unix_ms, 0, value)
    } else {
        AvailabilityV1::partial(sampled_at_unix_ms, 0, value, reasons)
    }
}

/// Trim the registry brand string and collapse internal runs of spaces.
pub(crate) fn normalize_cpu_model(raw: &str) -> Option<String> {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    (!normalized.is_empty()).then_some(normalized)
}

/// `major.minor.build[.revision][ (label)]`. The label is the registry
/// `DisplayVersion`; no product name is derived from the build number.
pub(crate) fn format_os_version(
    major: u32,
    minor: u32,
    build: u32,
    revision: Option<u32>,
    label: Option<&str>,
) -> String {
    let mut text = format!("{major}.{minor}.{build}");
    if let Some(revision) = revision {
        text.push_str(&format!(".{revision}"));
    }
    if let Some(label) = label.map(str::trim).filter(|label| !label.is_empty()) {
        text.push_str(&format!(" ({label})"));
    }
    text
}

/// Boot time from wall clock minus uptime, rounded to whole seconds so that
/// consecutive samples do not jitter.
pub(crate) fn boot_time_from(now_unix_ms: u64, uptime_ms: u64) -> Option<u64> {
    let boot = now_unix_ms.checked_sub(uptime_ms)?;
    Some((boot + 500) / 1_000 * 1_000)
}

/// Read the identity once. Non-Windows hosts are `unsupported`.
pub(crate) fn sample_host() -> AvailabilityV1<HostV1> {
    #[cfg(windows)]
    {
        host_availability(windows_readings(), super::unix_now_ms())
    }
    #[cfg(not(windows))]
    {
        AvailabilityV1::unsupported("platform_unsupported")
    }
}

/// Adapter names keyed by the lowercase PDH LUID prefix
/// `luid_0x<high>_0x<low>`.
pub(crate) type AdapterNames = Vec<(String, String)>;

pub(crate) fn luid_key(high: i32, low: u32) -> String {
    format!("luid_0x{:08x}_0x{low:08x}", high as u32)
}

fn name_for<'a>(adapter_id: &str, names: &'a AdapterNames) -> Option<&'a str> {
    let id = adapter_id.to_ascii_lowercase();
    names
        .iter()
        .find(|(key, _)| {
            id.strip_prefix(key.as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('_'))
        })
        .map(|(_, name)| name.as_str())
}

/// True when some measured adapter has no name in `names`.
pub(crate) fn has_unnamed(gpu: &AvailabilityV1<GpuV1>, names: &AdapterNames) -> bool {
    match gpu {
        AvailabilityV1::Available { value, .. } | AvailabilityV1::Partial { value, .. } => value
            .adapters
            .iter()
            .any(|adapter| name_for(&adapter.adapter_id, names).is_none()),
        _ => false,
    }
}

/// Join names into measured adapters. An adapter without a matching LUID
/// keeps `name: null`.
pub(crate) fn name_adapters(
    gpu: AvailabilityV1<GpuV1>,
    names: &AdapterNames,
) -> AvailabilityV1<GpuV1> {
    let attach = |mut value: GpuV1| {
        for adapter in &mut value.adapters {
            adapter.name = name_for(&adapter.adapter_id, names).map(str::to_string);
        }
        value
    };
    match gpu {
        AvailabilityV1::Available {
            sampled_at_unix_ms,
            age_ms,
            value,
        } => AvailabilityV1::Available {
            sampled_at_unix_ms,
            age_ms,
            value: attach(value),
        },
        AvailabilityV1::Partial {
            sampled_at_unix_ms,
            age_ms,
            value,
            reason_codes,
        } => AvailabilityV1::Partial {
            sampled_at_unix_ms,
            age_ms,
            value: attach(value),
            reason_codes,
        },
        other => other,
    }
}

/// Enumerate adapter names. A failure gives an empty list, so every adapter
/// keeps `name: null`.
pub(crate) fn sample_adapter_names() -> AdapterNames {
    #[cfg(windows)]
    {
        windows_adapter_names()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn windows_readings() -> HostReadings {
    use windows_sys::Win32::System::SystemInformation::GetTickCount64;

    let cpu_model = registry::read_string(
        r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
        "ProcessorNameString",
    )
    .and_then(|raw| normalize_cpu_model(&raw));
    // SAFETY: GetTickCount64 has no preconditions.
    let uptime_ms = unsafe { GetTickCount64() };
    HostReadings {
        cpu_model,
        os_version: windows_os_version(),
        boot_time_unix_ms: boot_time_from(super::unix_now_ms(), uptime_ms),
    }
}

#[cfg(windows)]
fn windows_os_version() -> Option<String> {
    use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
    use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;

    // SAFETY: zeroed OSVERSIONINFOW is valid once its size field is set.
    let mut info: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
    info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
    // SAFETY: `info` remains valid for the duration of the call.
    if unsafe { RtlGetVersion(&mut info) } != 0 {
        return None;
    }
    const CURRENT_VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let revision = registry::read_dword(CURRENT_VERSION, "UBR");
    let label = registry::read_string(CURRENT_VERSION, "DisplayVersion");
    Some(format_os_version(
        info.dwMajorVersion,
        info.dwMinorVersion,
        info.dwBuildNumber,
        revision,
        label.as_deref(),
    ))
}

#[cfg(windows)]
mod registry {
    use std::ffi::c_void;

    type Hkey = *mut c_void;
    const HKEY_LOCAL_MACHINE: Hkey = -2_147_483_646_i32 as isize as Hkey;
    const RRF_RT_REG_SZ: u32 = 0x0000_0002;
    const RRF_RT_REG_DWORD: u32 = 0x0000_0010;
    const RRF_SUBKEY_WOW6464KEY: u32 = 0x0001_0000;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_MORE_DATA: i32 = 234;
    const MAX_VALUE_BYTES: u32 = 4 * 1024;

    // Read-only registry access follows the local-declaration precedent in
    // `software/arp.rs`; the windows-sys registry feature is not enabled.
    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegGetValueW(
            key: Hkey,
            subkey: *const u16,
            value: *const u16,
            flags: u32,
            value_type: *mut u32,
            data: *mut c_void,
            data_len: *mut u32,
        ) -> i32;
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }

    pub(super) fn read_string(subkey: &str, value: &str) -> Option<String> {
        let subkey = wide(subkey);
        let value = wide(value);
        let mut size = 512_u32;
        for _ in 0..2 {
            let mut buffer = vec![0_u16; (size as usize).div_ceil(2)];
            let mut len = size;
            // SAFETY: both names are NUL-terminated; `buffer` holds `len` bytes.
            let status = unsafe {
                RegGetValueW(
                    HKEY_LOCAL_MACHINE,
                    subkey.as_ptr(),
                    value.as_ptr(),
                    RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr().cast(),
                    &mut len,
                )
            };
            if status == ERROR_MORE_DATA && len <= MAX_VALUE_BYTES {
                size = len;
                continue;
            }
            if status != ERROR_SUCCESS {
                return None;
            }
            let units = (len as usize / 2).min(buffer.len());
            let end = buffer[..units]
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(units);
            let text = String::from_utf16_lossy(&buffer[..end]);
            return (!text.trim().is_empty()).then_some(text);
        }
        None
    }

    pub(super) fn read_dword(subkey: &str, value: &str) -> Option<u32> {
        let subkey = wide(subkey);
        let value = wide(value);
        let mut data = 0_u32;
        let mut len = std::mem::size_of::<u32>() as u32;
        // SAFETY: both names are NUL-terminated; `data` holds 4 bytes.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                subkey.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_DWORD | RRF_SUBKEY_WOW6464KEY,
                std::ptr::null_mut(),
                (&mut data as *mut u32).cast(),
                &mut len,
            )
        };
        (status == ERROR_SUCCESS).then_some(data)
    }
}

#[cfg(windows)]
fn windows_adapter_names() -> AdapterNames {
    use windows_sys::Wdk::Graphics::Direct3D::{
        D3DKMT_ADAPTERINFO, D3DKMT_CLOSEADAPTER, D3DKMT_DRIVER_DESCRIPTION, D3DKMT_ENUMADAPTERS2,
        D3DKMT_QUERYADAPTERINFO, D3DKMTCloseAdapter, D3DKMTEnumAdapters2, D3DKMTQueryAdapterInfo,
        KMTQAITYPE_DRIVER_DESCRIPTION,
    };
    const MAX_ADAPTERS: u32 = 64;

    let mut adapters = Vec::new();
    for _ in 0..3 {
        let mut request = D3DKMT_ENUMADAPTERS2 {
            NumAdapters: 0,
            pAdapters: std::ptr::null_mut(),
        };
        // SAFETY: a null adapter array asks for the adapter count only.
        if unsafe { D3DKMTEnumAdapters2(&mut request) } != 0 {
            return Vec::new();
        }
        let count = request.NumAdapters.min(MAX_ADAPTERS);
        adapters = vec![D3DKMT_ADAPTERINFO::default(); count as usize];
        request.NumAdapters = count;
        request.pAdapters = adapters.as_mut_ptr();
        // SAFETY: `adapters` holds `count` writable entries.
        if unsafe { D3DKMTEnumAdapters2(&mut request) } == 0 {
            adapters.truncate(request.NumAdapters.min(count) as usize);
            break;
        }
        // The adapter set changed between calls; nothing was opened. Retry.
        adapters.clear();
    }

    let mut names = Vec::new();
    let mut description = Box::new(D3DKMT_DRIVER_DESCRIPTION::default());
    for adapter in &adapters {
        description.DriverDescription.fill(0);
        let mut query = D3DKMT_QUERYADAPTERINFO {
            hAdapter: adapter.hAdapter,
            Type: KMTQAITYPE_DRIVER_DESCRIPTION,
            pPrivateDriverData: (&mut *description as *mut D3DKMT_DRIVER_DESCRIPTION).cast(),
            PrivateDriverDataSize: std::mem::size_of::<D3DKMT_DRIVER_DESCRIPTION>() as u32,
        };
        // SAFETY: the handle came from D3DKMTEnumAdapters2; the buffer is
        // writable for its declared size.
        if unsafe { D3DKMTQueryAdapterInfo(&mut query) } == 0 {
            let units = &description.DriverDescription;
            let len = units
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(units.len());
            let name = String::from_utf16_lossy(&units[..len]).trim().to_string();
            if !name.is_empty() {
                names.push((
                    luid_key(adapter.AdapterLuid.HighPart, adapter.AdapterLuid.LowPart),
                    name,
                ));
            }
        }
        let close = D3DKMT_CLOSEADAPTER {
            hAdapter: adapter.hAdapter,
        };
        // SAFETY: every handle returned by D3DKMTEnumAdapters2 is closed once.
        unsafe { D3DKMTCloseAdapter(&close) };
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::GpuAdapterV1;

    #[test]
    fn host_is_available_partial_or_unavailable_by_missing_fields() {
        let full = HostReadings {
            cpu_model: Some("Intel(R) Core(TM) Ultra 9 275HX".into()),
            os_version: Some("10.0.26300.9974 (26H2)".into()),
            boot_time_unix_ms: Some(1_788_000_000_000),
        };
        let AvailabilityV1::Available { value, .. } = host_availability(full.clone(), 5) else {
            panic!("all fields read");
        };
        assert_eq!(
            value.cpu_model.as_deref(),
            Some("Intel(R) Core(TM) Ultra 9 275HX")
        );
        let AvailabilityV1::Partial {
            value,
            reason_codes,
            ..
        } = host_availability(
            HostReadings {
                cpu_model: None,
                ..full
            },
            5,
        )
        else {
            panic!("one field missing");
        };
        assert_eq!(value.cpu_model, None);
        assert_eq!(reason_codes, ["cpu_model_unavailable"]);
        assert_eq!(
            host_availability(HostReadings::default(), 5),
            AvailabilityV1::unavailable("api_unavailable")
        );
        let encoded = serde_json::to_value(host_availability(
            HostReadings {
                os_version: Some("10.0.1".into()),
                ..HostReadings::default()
            },
            5,
        ))
        .unwrap();
        assert!(encoded["value"]["cpu_model"].is_null());
        assert!(encoded["value"]["boot_time_unix_ms"].is_null());
    }

    #[test]
    fn os_version_is_measured_build_with_revision_and_label_only() {
        assert_eq!(
            format_os_version(10, 0, 26_300, Some(9_974), Some("26H2")),
            "10.0.26300.9974 (26H2)"
        );
        assert_eq!(format_os_version(10, 0, 19_045, None, None), "10.0.19045");
        assert_eq!(
            format_os_version(10, 0, 19_045, Some(1), Some("  ")),
            "10.0.19045.1"
        );
    }

    #[test]
    fn cpu_model_is_trimmed_and_blank_is_missing() {
        assert_eq!(
            normalize_cpu_model("  Intel(R) Core(TM) i7-8700 CPU @ 3.20GHz   ").as_deref(),
            Some("Intel(R) Core(TM) i7-8700 CPU @ 3.20GHz")
        );
        assert_eq!(normalize_cpu_model("   "), None);
    }

    #[test]
    fn boot_time_rounds_to_seconds_and_rejects_clock_underflow() {
        assert_eq!(boot_time_from(10_000_499, 1_000), Some(9_999_000));
        assert_eq!(boot_time_from(10_000_500, 1_000), Some(10_000_000));
        assert_eq!(boot_time_from(5, 10), None);
    }

    #[test]
    fn adapter_names_join_by_luid_and_unmatched_stays_null() {
        let names: AdapterNames = vec![
            (
                luid_key(0, 0xC0B6),
                "NVIDIA GeForce RTX 5090 Laptop GPU".into(),
            ),
            (luid_key(0, 0xD1A0), "Intel(R) Graphics".into()),
        ];
        assert_eq!(names[0].0, "luid_0x00000000_0x0000c0b6");
        let gpu = AvailabilityV1::available(
            1,
            0,
            GpuV1 {
                adapters: vec![
                    GpuAdapterV1 {
                        adapter_id: "luid_0x00000000_0x0000C0B6_phys_0".into(),
                        utilization_basis_points: 10,
                        name: None,
                    },
                    GpuAdapterV1 {
                        adapter_id: "luid_0x00000000_0x0001DA3F_phys_0".into(),
                        utilization_basis_points: 0,
                        name: None,
                    },
                ],
            },
        );
        assert!(has_unnamed(&gpu, &names));
        let AvailabilityV1::Available { value, .. } = name_adapters(gpu, &names) else {
            panic!("value kept");
        };
        assert_eq!(
            value.adapters[0].name.as_deref(),
            Some("NVIDIA GeForce RTX 5090 Laptop GPU")
        );
        assert_eq!(value.adapters[1].name, None);
        // A key must match a whole LUID, not a prefix of a longer one.
        assert_eq!(name_for("luid_0x00000000_0x0000c0b60_phys_0", &names), None);
        let missing = AvailabilityV1::<GpuV1>::unavailable("counter_missing");
        assert!(!has_unnamed(&missing, &names));
        assert_eq!(name_adapters(missing.clone(), &names), missing);
    }
}
