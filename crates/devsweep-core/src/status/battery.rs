//! Read-only battery device evidence: health, cycle count and power rate.
//!
//! Battery interfaces are enumerated with SetupDi and opened with
//! `GENERIC_READ` only; the in-box battery class driver answers the query
//! IOCTLs. No driver, service, WMI, process or elevation is involved. The
//! caller runs this on the existing 30 s power cadence and only when the
//! system power status reports a battery.

#![cfg_attr(not(windows), allow(dead_code))]

/// `BATTERY_INFORMATION.Capabilities`: a system (not UPS) battery.
const BATTERY_SYSTEM_BATTERY: u32 = 0x8000_0000;
/// `BATTERY_INFORMATION.Capabilities`: capacities are relative, not mWh.
const BATTERY_CAPACITY_RELATIVE: u32 = 0x4000_0000;
/// `BATTERY_INFORMATION` capacity value meaning "unknown".
const BATTERY_UNKNOWN_CAPACITY: u32 = u32::MAX;
/// `BATTERY_STATUS.Rate` value meaning "unknown" (`0x80000000`).
const BATTERY_UNKNOWN_RATE: i32 = i32::MIN;
/// Upper bound on enumerated battery interfaces.
const MAX_BATTERIES: u32 = 8;

/// Battery fields for `PowerV1`. `None` is reported as `null` and lists the
/// matching unsupported capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct BatteryEvidence {
    pub(crate) full_charge_basis_points: Option<u32>,
    pub(crate) cycle_count: Option<u32>,
    pub(crate) rate_milliwatts: Option<i32>,
}

/// Raw readings of one battery device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BatteryReading {
    pub(crate) capabilities: u32,
    pub(crate) designed_capacity: u32,
    pub(crate) full_charged_capacity: u32,
    pub(crate) cycle_count: u32,
    /// `None` when the status query failed.
    pub(crate) rate: Option<i32>,
}

/// Combine system batteries. UPS batteries are ignored. Health is total full
/// capacity over total design capacity and may exceed 10000. Relative-unit
/// batteries give no health and no rate. A cycle count of 0 is "not tracked".
/// Rates are summed; any unknown rate makes the sum unknown.
pub(crate) fn evidence_from_readings(readings: &[BatteryReading]) -> BatteryEvidence {
    let system = readings
        .iter()
        .filter(|reading| reading.capabilities & BATTERY_SYSTEM_BATTERY != 0)
        .collect::<Vec<_>>();
    if system.is_empty() {
        return BatteryEvidence::default();
    }
    let absolute = system
        .iter()
        .all(|reading| reading.capabilities & BATTERY_CAPACITY_RELATIVE == 0);

    let full_charge_basis_points = absolute.then(|| health(&system)).flatten();
    let cycle_count = system
        .iter()
        .map(|reading| reading.cycle_count)
        .filter(|count| *count > 0)
        .max();
    let rate_milliwatts = absolute
        .then(|| {
            system
                .iter()
                .try_fold(0_i64, |sum, reading| match reading.rate {
                    Some(rate) if rate != BATTERY_UNKNOWN_RATE => Some(sum + i64::from(rate)),
                    _ => None,
                })
                .and_then(|sum| i32::try_from(sum).ok())
        })
        .flatten();
    BatteryEvidence {
        full_charge_basis_points,
        cycle_count,
        rate_milliwatts,
    }
}

fn health(system: &[&BatteryReading]) -> Option<u32> {
    let mut designed = 0_u64;
    let mut full = 0_u64;
    for reading in system {
        if reading.designed_capacity == 0
            || reading.designed_capacity == BATTERY_UNKNOWN_CAPACITY
            || reading.full_charged_capacity == BATTERY_UNKNOWN_CAPACITY
        {
            return None;
        }
        designed += u64::from(reading.designed_capacity);
        full += u64::from(reading.full_charged_capacity);
    }
    if designed == 0 {
        return None;
    }
    u32::try_from(full * 10_000 / designed).ok()
}

/// Query every present battery device. Any failure leaves the affected fields
/// `None`; it never produces a zero.
pub(crate) fn sample_battery() -> BatteryEvidence {
    #[cfg(windows)]
    {
        evidence_from_readings(&windows_readings())
    }
    #[cfg(not(windows))]
    {
        BatteryEvidence::default()
    }
}

#[cfg(windows)]
fn windows_readings() -> Vec<BatteryReading> {
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
        SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiDestroyDeviceInfoList,
        SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
    };
    use windows_sys::Win32::System::Power::GUID_DEVICE_BATTERY;

    // SAFETY: a valid interface-class GUID with null enumerator and window.
    let devices = unsafe {
        SetupDiGetClassDevsW(
            &GUID_DEVICE_BATTERY,
            std::ptr::null(),
            std::ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    };
    if devices == -1 || devices == 0 {
        return Vec::new();
    }
    struct DeviceList(isize);
    impl Drop for DeviceList {
        fn drop(&mut self) {
            // SAFETY: the list came from SetupDiGetClassDevsW and is freed once.
            unsafe { SetupDiDestroyDeviceInfoList(self.0) };
        }
    }
    let _list = DeviceList(devices);

    let mut readings = Vec::new();
    for index in 0..MAX_BATTERIES {
        // SAFETY: zeroed SP_DEVICE_INTERFACE_DATA is valid once cbSize is set.
        let mut interface: SP_DEVICE_INTERFACE_DATA = unsafe { std::mem::zeroed() };
        interface.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
        // SAFETY: `devices` is open and `interface` is writable.
        let found = unsafe {
            SetupDiEnumDeviceInterfaces(
                devices,
                std::ptr::null(),
                &GUID_DEVICE_BATTERY,
                index,
                &mut interface,
            )
        };
        if found == 0 {
            break;
        }
        let mut required = 0_u32;
        // SAFETY: a null detail buffer asks for the required size.
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                devices,
                &interface,
                std::ptr::null_mut(),
                0,
                &mut required,
                std::ptr::null_mut(),
            )
        };
        let header = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
        if (required as usize) < header || required > 64 * 1024 {
            continue;
        }
        let mut buffer = vec![0_u64; (required as usize).div_ceil(8)];
        let detail = buffer
            .as_mut_ptr()
            .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
        // SAFETY: `buffer` is 8-byte aligned and at least `required` bytes.
        unsafe { (*detail).cbSize = header as u32 };
        // SAFETY: `detail` is writable for `required` bytes.
        let ok = unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                devices,
                &interface,
                detail,
                required,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            continue;
        }
        // SAFETY: DevicePath starts inside `buffer`; the path occupies the
        // remaining `required` bytes and is NUL-terminated by SetupDi.
        let path = unsafe {
            let start = std::ptr::addr_of!((*detail).DevicePath).cast::<u16>();
            let offset = start.cast::<u8>().offset_from(buffer.as_ptr().cast::<u8>()) as usize;
            let units = (required as usize).saturating_sub(offset) / 2;
            let slice = std::slice::from_raw_parts(start, units);
            let len = slice.iter().position(|unit| *unit == 0).unwrap_or(units);
            slice[..len]
                .iter()
                .copied()
                .chain([0])
                .collect::<Vec<u16>>()
        };
        if path.len() > 1
            && let Some(reading) = query_battery(&path)
        {
            readings.push(reading);
        }
    }
    readings
}

#[cfg(windows)]
fn query_battery(path: &[u16]) -> Option<BatteryReading> {
    use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Power::{
        BATTERY_INFORMATION, BATTERY_QUERY_INFORMATION, BATTERY_STATUS, BATTERY_WAIT_STATUS,
        BatteryInformation, IOCTL_BATTERY_QUERY_INFORMATION, IOCTL_BATTERY_QUERY_STATUS,
        IOCTL_BATTERY_QUERY_TAG,
    };

    // SAFETY: `path` is NUL-terminated. Read access only; closed below.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return None;
    }
    struct Handle(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            // SAFETY: the handle was opened above and is closed once.
            unsafe { CloseHandle(self.0) };
        }
    }
    let _handle = Handle(handle);

    fn ioctl<I, O>(
        handle: windows_sys::Win32::Foundation::HANDLE,
        code: u32,
        input: &I,
        output: &mut O,
    ) -> bool {
        let mut returned = 0_u32;
        // SAFETY: both buffers are valid for their sizes during the call.
        let ok = unsafe {
            DeviceIoControl(
                handle,
                code,
                (input as *const I).cast(),
                std::mem::size_of::<I>() as u32,
                (output as *mut O).cast(),
                std::mem::size_of::<O>() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        ok != 0
    }

    let wait_ms = 0_u32;
    let mut tag = 0_u32;
    if !ioctl(handle, IOCTL_BATTERY_QUERY_TAG, &wait_ms, &mut tag) || tag == 0 {
        return None;
    }
    let query = BATTERY_QUERY_INFORMATION {
        BatteryTag: tag,
        InformationLevel: BatteryInformation,
        AtRate: 0,
    };
    let mut information = BATTERY_INFORMATION::default();
    if !ioctl(
        handle,
        IOCTL_BATTERY_QUERY_INFORMATION,
        &query,
        &mut information,
    ) {
        return None;
    }
    let wait = BATTERY_WAIT_STATUS {
        BatteryTag: tag,
        ..BATTERY_WAIT_STATUS::default()
    };
    let mut status = BATTERY_STATUS::default();
    let rate = ioctl(handle, IOCTL_BATTERY_QUERY_STATUS, &wait, &mut status).then_some(status.Rate);
    Some(BatteryReading {
        capabilities: information.Capabilities,
        designed_capacity: information.DesignedCapacity,
        full_charged_capacity: information.FullChargedCapacity,
        cycle_count: information.CycleCount,
        rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system(designed: u32, full: u32, cycles: u32, rate: Option<i32>) -> BatteryReading {
        BatteryReading {
            capabilities: BATTERY_SYSTEM_BATTERY,
            designed_capacity: designed,
            full_charged_capacity: full,
            cycle_count: cycles,
            rate,
        }
    }

    #[test]
    fn one_system_battery_reports_health_cycles_and_signed_rate() {
        // 89 160 / 99 072 = 0.899951..., truncated to basis points.
        let evidence = evidence_from_readings(&[system(99_072, 89_160, 312, Some(-12_500))]);
        assert_eq!(evidence.full_charge_basis_points, Some(8_999));
        assert_eq!(evidence.cycle_count, Some(312));
        assert_eq!(evidence.rate_milliwatts, Some(-12_500));
        // AC online and not charging is a measured zero rate.
        assert_eq!(
            evidence_from_readings(&[system(99_072, 99_072, 5, Some(0))]).rate_milliwatts,
            Some(0)
        );
    }

    #[test]
    fn full_charge_above_design_is_reported_as_measured() {
        let evidence = evidence_from_readings(&[system(50_000, 51_000, 1, Some(4_000))]);
        assert_eq!(evidence.full_charge_basis_points, Some(10_200));
    }

    #[test]
    fn zero_cycles_unknown_rate_and_unknown_capacity_are_null_not_zero() {
        let evidence = evidence_from_readings(&[system(99_072, 99_072, 0, Some(i32::MIN))]);
        assert_eq!(evidence.cycle_count, None);
        assert_eq!(evidence.rate_milliwatts, None);
        assert_eq!(evidence.full_charge_basis_points, Some(10_000));
        let failed_status = evidence_from_readings(&[system(99_072, 99_072, 3, None)]);
        assert_eq!(failed_status.rate_milliwatts, None);
        for (designed, full) in [(0, 99_072), (u32::MAX, 1), (99_072, u32::MAX)] {
            let evidence = evidence_from_readings(&[system(designed, full, 3, Some(1))]);
            assert_eq!(evidence.full_charge_basis_points, None, "{designed} {full}");
        }
    }

    #[test]
    fn relative_units_ups_batteries_and_no_device_give_no_values() {
        let relative = BatteryReading {
            capabilities: BATTERY_SYSTEM_BATTERY | BATTERY_CAPACITY_RELATIVE,
            ..system(100, 90, 7, Some(-5))
        };
        let evidence = evidence_from_readings(&[relative]);
        assert_eq!(evidence.full_charge_basis_points, None);
        assert_eq!(evidence.rate_milliwatts, None);
        assert_eq!(evidence.cycle_count, Some(7));
        let ups = BatteryReading {
            capabilities: 0,
            ..system(100, 90, 7, Some(-5))
        };
        assert_eq!(evidence_from_readings(&[ups]), BatteryEvidence::default());
        assert_eq!(evidence_from_readings(&[]), BatteryEvidence::default());
    }

    #[test]
    fn two_batteries_sum_capacity_and_rate_and_keep_the_highest_cycle_count() {
        let evidence = evidence_from_readings(&[
            system(50_000, 45_000, 100, Some(-6_000)),
            system(30_000, 30_000, 0, Some(-2_000)),
        ]);
        assert_eq!(evidence.full_charge_basis_points, Some(9_375));
        assert_eq!(evidence.cycle_count, Some(100));
        assert_eq!(evidence.rate_milliwatts, Some(-8_000));
        let one_unknown = evidence_from_readings(&[
            system(50_000, 45_000, 100, Some(-6_000)),
            system(30_000, 30_000, 0, Some(i32::MIN)),
        ]);
        assert_eq!(one_unknown.rate_milliwatts, None);
    }
}
