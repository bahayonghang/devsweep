//! CPU, physical memory, fixed-volume, and battery adapters.

use super::{AcState, AvailabilityV1, MemoryV1, PowerV1, VolumeV1, VolumesV1, unix_now_ms};

/// Kernel/user/idle times in 100-nanosecond units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuTimes {
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
}

/// Why a counter delta cannot become a numeric rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaIssue {
    ZeroElapsed,
    CounterReset,
    SampleGap,
}

/// Whole-system CPU from two `GetSystemTimes` samples.
///
/// Kernel time includes idle. Zero elapsed, reset, and sleep-sized gaps never
/// become a synthesized zero utilization.
pub fn cpu_utilization_basis_points(
    first: CpuTimes,
    second: CpuTimes,
    expected_window_ms: u32,
    actual_window_ms: u32,
) -> Result<u32, DeltaIssue> {
    if actual_window_ms == 0 {
        return Err(DeltaIssue::ZeroElapsed);
    }
    if is_sleep_sized_gap(expected_window_ms, actual_window_ms) {
        return Err(DeltaIssue::SampleGap);
    }
    if second.idle < first.idle || second.kernel < first.kernel || second.user < first.user {
        return Err(DeltaIssue::CounterReset);
    }
    let idle = second.idle - first.idle;
    let kernel = second.kernel - first.kernel;
    let user = second.user - first.user;
    let total = kernel.saturating_add(user);
    if total == 0 {
        return Err(DeltaIssue::ZeroElapsed);
    }
    let busy = total.saturating_sub(idle);
    let points = busy.saturating_mul(10_000) / total;
    Ok(u32::try_from(points.min(10_000)).unwrap_or(10_000))
}

/// Physical memory from totals. `available > total` is rejected rather than
/// clamped into a fake used-zero.
pub fn memory_from_physical(total_bytes: u64, available_bytes: u64) -> Option<MemoryV1> {
    if total_bytes == 0 || available_bytes > total_bytes {
        return None;
    }
    Some(MemoryV1 {
        total_bytes,
        available_bytes,
        used_bytes: total_bytes - available_bytes,
    })
}

#[must_use]
pub(crate) fn is_sleep_sized_gap(expected_ms: u32, actual_ms: u32) -> bool {
    if expected_ms == 0 {
        return actual_ms > 10_000;
    }
    actual_ms > expected_ms.saturating_mul(4).max(10_000)
}

pub(crate) fn cpu_availability(
    first: CpuTimes,
    second: CpuTimes,
    expected_window_ms: u32,
    actual_window_ms: u32,
    sampled_at_unix_ms: u64,
) -> AvailabilityV1<super::CpuV1> {
    match cpu_utilization_basis_points(first, second, expected_window_ms, actual_window_ms) {
        Ok(system_utilization_basis_points) => AvailabilityV1::available(
            sampled_at_unix_ms,
            0,
            super::CpuV1 {
                system_utilization_basis_points,
            },
        ),
        Err(DeltaIssue::ZeroElapsed) => AvailabilityV1::unavailable("zero_elapsed"),
        Err(DeltaIssue::CounterReset) => AvailabilityV1::unavailable("counter_reset"),
        Err(DeltaIssue::SampleGap) => AvailabilityV1::unavailable("sample_gap"),
    }
}

pub(crate) fn sample_memory() -> AvailabilityV1<MemoryV1> {
    #[cfg(windows)]
    {
        windows_memory()
    }
    #[cfg(not(windows))]
    {
        AvailabilityV1::unsupported("platform_unsupported")
    }
}

pub(crate) fn sample_cpu_times() -> Result<CpuTimes, AvailabilityFailure> {
    #[cfg(windows)]
    {
        windows_cpu_times()
    }
    #[cfg(not(windows))]
    {
        Err(AvailabilityFailure::Unsupported)
    }
}

pub(crate) fn sample_volumes() -> AvailabilityV1<VolumesV1> {
    #[cfg(windows)]
    {
        windows_volumes()
    }
    #[cfg(not(windows))]
    {
        AvailabilityV1::unsupported("platform_unsupported")
    }
}

pub(crate) fn sample_power() -> AvailabilityV1<PowerV1> {
    #[cfg(windows)]
    {
        windows_power()
    }
    #[cfg(not(windows))]
    {
        AvailabilityV1::unsupported("platform_unsupported")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum AvailabilityFailure {
    Unavailable,
    PermissionDenied,
    Unsupported,
}

impl AvailabilityFailure {
    pub(crate) fn cpu(self) -> AvailabilityV1<super::CpuV1> {
        match self {
            Self::Unavailable => AvailabilityV1::unavailable("api_unavailable"),
            Self::PermissionDenied => AvailabilityV1::permission_denied("access_denied"),
            Self::Unsupported => AvailabilityV1::unsupported("platform_unsupported"),
        }
    }
}

/// Battery/AC mapping. Flag 128 is "no system battery" and must not become
/// charge 0.
pub(crate) fn power_from_status(
    ac_line: u8,
    battery_flag: u8,
    life_percent: u8,
    life_seconds: u32,
) -> PowerV1 {
    let ac_state = match ac_line {
        0 => AcState::Offline,
        1 => AcState::Online,
        _ => AcState::Unknown,
    };
    let battery_present = battery_flag != 255 && battery_flag & 128 == 0;
    if !battery_present {
        return PowerV1 {
            battery_present: false,
            ac_state,
            charge_basis_points: None,
            remaining_seconds: None,
        };
    }
    let charge_basis_points = if life_percent > 100 {
        None
    } else {
        Some(u32::from(life_percent) * 100)
    };
    let remaining_seconds = if life_seconds == u32::MAX {
        None
    } else {
        Some(u64::from(life_seconds))
    };
    PowerV1 {
        battery_present: true,
        ac_state,
        charge_basis_points,
        remaining_seconds,
    }
}

pub(crate) fn keep_fixed_volume(drive_type: u32) -> bool {
    drive_type == 3
}

#[cfg(windows)]
fn windows_cpu_times() -> Result<CpuTimes, AvailabilityFailure> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetSystemTimes;

    let mut idle = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    // SAFETY: the three FILETIME pointers are valid writable locals.
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        return Err(map_last_error());
    }
    Ok(CpuTimes {
        idle: filetime_u64(idle),
        kernel: filetime_u64(kernel),
        user: filetime_u64(user),
    })
}

#[cfg(windows)]
fn windows_memory() -> AvailabilityV1<MemoryV1> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    // SAFETY: zeroed MEMORYSTATUSEX is valid once `dwLength` is set.
    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    // SAFETY: `status` remains valid for the duration of the call.
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 {
        return match map_last_error() {
            AvailabilityFailure::PermissionDenied => {
                AvailabilityV1::permission_denied("access_denied")
            }
            _ => AvailabilityV1::unavailable("api_unavailable"),
        };
    }
    match memory_from_physical(status.ullTotalPhys, status.ullAvailPhys) {
        Some(value) => AvailabilityV1::available(unix_now_ms(), 0, value),
        None => AvailabilityV1::unavailable("inconsistent_counters"),
    }
}

#[cfg(windows)]
fn windows_volumes() -> AvailabilityV1<VolumesV1> {
    use std::collections::BTreeMap;

    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW,
    };

    let required = unsafe { GetLogicalDriveStringsW(0, std::ptr::null_mut()) };
    if required == 0 {
        return match map_last_error() {
            AvailabilityFailure::PermissionDenied => {
                AvailabilityV1::permission_denied("access_denied")
            }
            _ => AvailabilityV1::unavailable("api_unavailable"),
        };
    }
    let mut buffer = vec![0_u16; required as usize];
    // SAFETY: `buffer` is writable for `required` UTF-16 units.
    let written = unsafe { GetLogicalDriveStringsW(required, buffer.as_mut_ptr()) };
    if written == 0 || (written as usize) >= buffer.len() {
        return AvailabilityV1::unavailable("api_unavailable");
    }

    let mut grouped: BTreeMap<String, VolumeV1> = BTreeMap::new();
    let mut complete = true;
    let mut saw_fixed = false;
    let mut offset = 0;
    while offset < written as usize {
        let Some(end) = buffer[offset..].iter().position(|unit| *unit == 0) else {
            break;
        };
        if end == 0 {
            break;
        }
        let mount = String::from_utf16_lossy(&buffer[offset..offset + end]);
        offset += end + 1;
        let mut mount_wide: Vec<u16> = mount.encode_utf16().collect();
        if !mount_wide.ends_with(&[0]) {
            mount_wide.push(0);
        }
        // SAFETY: `mount_wide` is a NUL-terminated drive root.
        let drive_type = unsafe { GetDriveTypeW(mount_wide.as_ptr()) };
        if !keep_fixed_volume(drive_type) {
            continue;
        }
        saw_fixed = true;
        let mut total = 0_u64;
        let mut available = 0_u64;
        // SAFETY: output pointers are valid u64 locals; mount is a drive root.
        let ok = unsafe {
            GetDiskFreeSpaceExW(
                mount_wide.as_ptr(),
                std::ptr::null_mut(),
                &mut total,
                &mut available,
            )
        };
        if ok == 0 || total == 0 || available > total {
            complete = false;
            continue;
        }
        let volume_id = volume_guid(&mount_wide).unwrap_or_else(|| format!("volume:{mount}"));
        if volume_guid(&mount_wide).is_none() {
            complete = false;
        }
        grouped
            .entry(volume_id.clone())
            .and_modify(|existing| {
                if !existing.mount_points.iter().any(|point| point == &mount) {
                    existing.mount_points.push(mount.clone());
                }
            })
            .or_insert(VolumeV1 {
                volume_id,
                mount_points: vec![mount],
                total_bytes: total,
                available_bytes: available,
            });
    }

    if !saw_fixed && grouped.is_empty() {
        return AvailabilityV1::available(
            unix_now_ms(),
            0,
            VolumesV1 {
                items: Vec::new(),
                complete: true,
            },
        );
    }

    let items = grouped.into_values().collect::<Vec<_>>();
    let value = VolumesV1 { items, complete };
    if complete {
        AvailabilityV1::available(unix_now_ms(), 0, value)
    } else {
        AvailabilityV1::partial(
            unix_now_ms(),
            0,
            value,
            vec!["volume_incomplete".to_string()],
        )
    }
}

#[cfg(windows)]
fn volume_guid(mount_wide: &[u16]) -> Option<String> {
    use windows_sys::Win32::Storage::FileSystem::GetVolumeNameForVolumeMountPointW;

    let mut name = [0_u16; 128];
    // SAFETY: `name` is writable; `mount_wide` is a NUL-terminated root.
    let ok = unsafe {
        GetVolumeNameForVolumeMountPointW(mount_wide.as_ptr(), name.as_mut_ptr(), name.len() as u32)
    };
    if ok == 0 {
        return None;
    }
    let len = name
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(name.len());
    let guid = String::from_utf16_lossy(&name[..len]);
    if guid.is_empty() {
        None
    } else {
        Some(guid.trim_end_matches('\\').to_string())
    }
}

#[cfg(windows)]
fn windows_power() -> AvailabilityV1<PowerV1> {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    // SAFETY: zeroed SYSTEM_POWER_STATUS is a valid out-parameter.
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    // SAFETY: `status` is writable for the duration of the call.
    let ok = unsafe { GetSystemPowerStatus(&mut status) };
    if ok == 0 {
        return match map_last_error() {
            AvailabilityFailure::PermissionDenied => {
                AvailabilityV1::permission_denied("access_denied")
            }
            _ => AvailabilityV1::unavailable("api_unavailable"),
        };
    }
    AvailabilityV1::available(
        unix_now_ms(),
        0,
        power_from_status(
            status.ACLineStatus,
            status.BatteryFlag,
            status.BatteryLifePercent,
            status.BatteryLifeTime,
        ),
    )
}

#[cfg(windows)]
fn filetime_u64(time: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

#[cfg(windows)]
fn map_last_error() -> AvailabilityFailure {
    use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, GetLastError};

    // SAFETY: reads this thread's last-error code immediately after a failed API.
    let code = unsafe { GetLastError() };
    if code == ERROR_ACCESS_DENIED {
        AvailabilityFailure::PermissionDenied
    } else {
        AvailabilityFailure::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_busy_fraction_uses_kernel_minus_idle() {
        let first = CpuTimes {
            idle: 1_000,
            kernel: 2_000,
            user: 500,
        };
        let second = CpuTimes {
            idle: 1_500,
            kernel: 3_000,
            user: 1_000,
        };
        // idle Δ=500, kernel Δ=1000, user Δ=500, total=1500, busy=1000 → 6666 bp
        assert_eq!(
            cpu_utilization_basis_points(first, second, 500, 500).unwrap(),
            6_666
        );
    }

    #[test]
    fn cpu_zero_elapsed_and_reset_are_not_zero_utilization() {
        let sample = CpuTimes {
            idle: 10,
            kernel: 20,
            user: 5,
        };
        assert_eq!(
            cpu_utilization_basis_points(sample, sample, 500, 0),
            Err(DeltaIssue::ZeroElapsed)
        );
        let earlier = CpuTimes {
            idle: 5,
            kernel: 10,
            user: 2,
        };
        assert_eq!(
            cpu_utilization_basis_points(sample, earlier, 500, 500),
            Err(DeltaIssue::CounterReset)
        );
        assert_eq!(
            cpu_utilization_basis_points(
                sample,
                CpuTimes {
                    idle: 11,
                    kernel: 21,
                    user: 6
                },
                500,
                20_000
            ),
            Err(DeltaIssue::SampleGap)
        );
        assert!(!matches!(
            cpu_availability(sample, sample, 500, 0, 1),
            AvailabilityV1::Available { .. }
        ));
    }

    #[test]
    fn memory_rejects_zero_total_and_inconsistent_available() {
        assert_eq!(memory_from_physical(0, 0), None);
        assert_eq!(memory_from_physical(100, 101), None);
        let value = memory_from_physical(100, 40).unwrap();
        assert_eq!(value.used_bytes, 60);
        assert_eq!(value.available_bytes, 40);
    }

    #[test]
    fn missing_battery_is_present_false_with_null_charge() {
        let power = power_from_status(1, 128, 0, 0);
        assert!(!power.battery_present);
        assert_eq!(power.ac_state, AcState::Online);
        assert_eq!(power.charge_basis_points, None);
        assert_eq!(power.remaining_seconds, None);
        let present = power_from_status(0, 1, 55, 120);
        assert!(present.battery_present);
        assert_eq!(present.charge_basis_points, Some(5_500));
        assert_eq!(present.remaining_seconds, Some(120));
        let unknown_life = power_from_status(255, 1, 255, u32::MAX);
        assert!(unknown_life.battery_present);
        assert_eq!(unknown_life.ac_state, AcState::Unknown);
        assert_eq!(unknown_life.charge_basis_points, None);
        assert_eq!(unknown_life.remaining_seconds, None);
    }

    #[test]
    fn non_fixed_volume_types_are_excluded() {
        assert!(keep_fixed_volume(3));
        for drive_type in [0_u32, 1, 2, 4, 5, 6] {
            assert!(!keep_fixed_volume(drive_type), "type {drive_type}");
        }
    }

    #[test]
    fn sleep_gap_threshold_is_four_times_expected_or_ten_seconds() {
        assert!(!is_sleep_sized_gap(500, 500));
        assert!(!is_sleep_sized_gap(500, 9_999));
        assert!(is_sleep_sized_gap(500, 10_001));
        assert!(!is_sleep_sized_gap(2_000, 8_001));
        assert!(is_sleep_sized_gap(2_000, 10_001));
    }
}
