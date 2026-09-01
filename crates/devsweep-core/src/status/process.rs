//! Bounded ToolHelp process counters. Name, PID, CPU, memory, and I/O only.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::process::CancelObserver;

use super::{
    AvailabilityV1, PROCESS_DETAIL_BUDGET_MS, PROCESS_ENUMERATION_CEILING, ProcessV1, ProcessesV1,
    clamp_process_limit, unix_now_ms,
};

/// Counters captured for one PID. No path, command line, user, environment,
/// handles, or history are retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessCounters {
    pub pid: u32,
    pub name: String,
    pub cpu_100ns: u64,
    pub private_bytes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub thread_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessEnumeration {
    pub identities: Vec<ProcessIdentity>,
    pub enumerated_count: u32,
    pub hit_ceiling: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessIdentity {
    pub pid: u32,
    pub name: String,
    pub thread_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProcessDetailOutcome {
    pub counters: Vec<ProcessCounters>,
    pub budget_exhausted: bool,
    pub access_denied: bool,
}

/// Rank, truncate, and tag the process group. Metadata is computed from the
/// bounded enumeration before sorting.
pub(crate) fn finalize_processes(
    mut detailed: Vec<ProcessV1>,
    enumerated_count: u32,
    requested_limit: u32,
    budget_exhausted: bool,
    access_denied: bool,
    sampled_at_unix_ms: u64,
) -> AvailabilityV1<ProcessesV1> {
    let requested_limit = clamp_process_limit(requested_limit);
    let truncated_by_limit = enumerated_count > requested_limit;
    detailed.sort_by(|left, right| {
        right
            .cpu_basis_points_of_one_logical_core
            .cmp(&left.cpu_basis_points_of_one_logical_core)
            .then_with(|| right.private_bytes.cmp(&left.private_bytes))
            .then_with(|| left.pid.cmp(&right.pid))
            .then_with(|| left.name.cmp(&right.name))
    });
    if detailed.len() > requested_limit as usize {
        detailed.truncate(requested_limit as usize);
    }
    let value = ProcessesV1 {
        returned_count: detailed.len() as u32,
        items: detailed,
        enumerated_count: enumerated_count.min(PROCESS_ENUMERATION_CEILING),
        requested_limit,
        enumeration_ceiling: PROCESS_ENUMERATION_CEILING,
        detail_budget_ms: PROCESS_DETAIL_BUDGET_MS,
        truncated_by_limit,
        budget_exhausted,
    };
    let mut reasons = Vec::new();
    if truncated_by_limit {
        reasons.push("process_limit".to_string());
    }
    if budget_exhausted {
        reasons.push("detail_budget_exhausted".to_string());
    }
    if access_denied {
        reasons.push("process_access_denied".to_string());
    }
    if reasons.is_empty() {
        AvailabilityV1::available(sampled_at_unix_ms, 0, value)
    } else {
        AvailabilityV1::partial(sampled_at_unix_ms, 0, value, reasons)
    }
}

pub(crate) fn process_rows_from_pair(
    first: &[ProcessCounters],
    second: &[ProcessCounters],
    expected_window_ms: u32,
    actual_window_ms: u32,
) -> Vec<ProcessV1> {
    if actual_window_ms == 0
        || super::system::is_sleep_sized_gap(expected_window_ms, actual_window_ms)
    {
        return Vec::new();
    }
    let previous: HashMap<u32, &ProcessCounters> = first.iter().map(|row| (row.pid, row)).collect();
    let mut rows = Vec::new();
    for later in second {
        let Some(earlier) = previous.get(&later.pid) else {
            continue;
        };
        if later.cpu_100ns < earlier.cpu_100ns
            || later.read_bytes < earlier.read_bytes
            || later.write_bytes < earlier.write_bytes
        {
            continue;
        }
        let cpu_delta = later.cpu_100ns - earlier.cpu_100ns;
        let cpu_basis_points_of_one_logical_core =
            u32::try_from(cpu_delta / u64::from(actual_window_ms.max(1))).unwrap_or(u32::MAX);
        rows.push(ProcessV1 {
            pid: later.pid,
            name: later.name.clone(),
            cpu_basis_points_of_one_logical_core,
            private_bytes: later.private_bytes,
            read_bytes_per_second: bytes_per_second(
                later.read_bytes - earlier.read_bytes,
                actual_window_ms,
            ),
            write_bytes_per_second: bytes_per_second(
                later.write_bytes - earlier.write_bytes,
                actual_window_ms,
            ),
        });
    }
    rows
}

fn bytes_per_second(delta: u64, interval_ms: u32) -> u64 {
    ((u128::from(delta) * 1_000) / u128::from(interval_ms.max(1))) as u64
}

pub(crate) fn enumerate_identities(
    cancel: Option<&dyn CancelObserver>,
) -> Result<ProcessEnumeration, super::system::AvailabilityFailure> {
    #[cfg(windows)]
    {
        windows_enumerate(cancel)
    }
    #[cfg(not(windows))]
    {
        let _ = cancel;
        Err(super::system::AvailabilityFailure::Unsupported)
    }
}

pub(crate) fn detail_processes(
    identities: &[ProcessIdentity],
    cancel: Option<&dyn CancelObserver>,
    budget: Duration,
) -> ProcessDetailOutcome {
    #[cfg(windows)]
    {
        windows_detail(identities, cancel, budget)
    }
    #[cfg(not(windows))]
    {
        let _ = (identities, cancel, budget);
        ProcessDetailOutcome {
            counters: Vec::new(),
            budget_exhausted: false,
            access_denied: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProcessGroupMeta {
    pub enumerated_count: u32,
    pub budget_exhausted: bool,
    pub access_denied: bool,
}

pub(crate) fn sample_process_group(
    requested_limit: u32,
    first: &[ProcessCounters],
    second: &[ProcessCounters],
    meta: ProcessGroupMeta,
    expected_window_ms: u32,
    actual_window_ms: u32,
) -> AvailabilityV1<ProcessesV1> {
    if actual_window_ms == 0 {
        return AvailabilityV1::unavailable("zero_elapsed");
    }
    if super::system::is_sleep_sized_gap(expected_window_ms, actual_window_ms) {
        return AvailabilityV1::unavailable("sample_gap");
    }
    let rows = process_rows_from_pair(first, second, expected_window_ms, actual_window_ms);
    finalize_processes(
        rows,
        meta.enumerated_count,
        requested_limit,
        meta.budget_exhausted,
        meta.access_denied,
        unix_now_ms(),
    )
}

#[cfg(windows)]
fn windows_enumerate(
    cancel: Option<&dyn CancelObserver>,
) -> Result<ProcessEnumeration, super::system::AvailabilityFailure> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };

    // SAFETY: TH32CS_SNAPPROCESS with pid 0 snapshots every process.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE || snapshot.is_null() {
        return Err(map_last_error());
    }
    struct Snapshot(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Snapshot {
        fn drop(&mut self) {
            // SAFETY: `CreateToolhelp32Snapshot` returned this handle.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let _snapshot = Snapshot(snapshot);

    // SAFETY: PROCESSENTRY32W must advertise its size before Process32FirstW.
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    // SAFETY: `snapshot` is a live ToolHelp handle and `entry` is sized.
    let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };
    if ok == 0 {
        return Err(map_last_error());
    }

    let mut identities = Vec::new();
    let mut hit_ceiling = false;
    loop {
        if cancel.is_some_and(|observer| observer.is_cancel_requested()) {
            break;
        }
        if identities.len() as u32 >= PROCESS_ENUMERATION_CEILING {
            hit_ceiling = true;
            break;
        }
        let name = utf16_lossy(&entry.szExeFile);
        if !name.is_empty() {
            identities.push(ProcessIdentity {
                pid: entry.th32ProcessID,
                name,
                thread_count: entry.cntThreads,
            });
        }
        // SAFETY: the same snapshot/entry pair remains valid.
        ok = unsafe { Process32NextW(snapshot, &mut entry) };
        if ok == 0 {
            break;
        }
    }
    Ok(ProcessEnumeration {
        enumerated_count: identities.len() as u32,
        identities,
        hit_ceiling,
    })
}

#[cfg(windows)]
fn windows_detail(
    identities: &[ProcessIdentity],
    cancel: Option<&dyn CancelObserver>,
    budget: Duration,
) -> ProcessDetailOutcome {
    let started = Instant::now();
    let mut counters = Vec::new();
    let mut budget_exhausted = false;
    let mut access_denied = false;
    for identity in identities {
        if cancel.is_some_and(|observer| observer.is_cancel_requested()) {
            break;
        }
        if started.elapsed() >= budget {
            budget_exhausted = true;
            break;
        }
        match open_counters(identity) {
            Ok(row) => counters.push(row),
            Err(true) => access_denied = true,
            Err(false) => {}
        }
    }
    ProcessDetailOutcome {
        counters,
        budget_exhausted,
        access_denied,
    }
}

#[cfg(windows)]
fn open_counters(identity: &ProcessIdentity) -> Result<ProcessCounters, bool> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows_sys::Win32::System::Threading::{
        GetProcessIoCounters, GetProcessTimes, IO_COUNTERS, OpenProcess,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    // SAFETY: query-limited rights only; no VM_READ, no QUERY_INFORMATION.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, identity.pid) };
    if handle.is_null() {
        return Err(is_access_denied());
    }
    struct ProcessHandle(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            // SAFETY: `OpenProcess` returned this handle.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let handle = ProcessHandle(handle);

    let mut creation = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit = FILETIME {
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
    // SAFETY: the four FILETIME pointers are writable locals.
    let times_ok =
        unsafe { GetProcessTimes(handle.0, &mut creation, &mut exit, &mut kernel, &mut user) };
    if times_ok == 0 {
        return Err(is_access_denied());
    }

    // SAFETY: PROCESS_MEMORY_COUNTERS_EX is valid once cb is set.
    let mut memory: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
    memory.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    // SAFETY: `handle` is a live process handle; `memory` is sized.
    let memory_ok = unsafe {
        GetProcessMemoryInfo(
            handle.0,
            (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX).cast(),
            memory.cb,
        )
    };
    if memory_ok == 0 {
        return Err(is_access_denied());
    }

    // SAFETY: IO_COUNTERS is a valid out-parameter.
    let mut io: IO_COUNTERS = unsafe { std::mem::zeroed() };
    let io_ok = unsafe { GetProcessIoCounters(handle.0, &mut io) };
    if io_ok == 0 {
        return Err(is_access_denied());
    }

    Ok(ProcessCounters {
        pid: identity.pid,
        name: identity.name.clone(),
        cpu_100ns: filetime_u64(kernel).saturating_add(filetime_u64(user)),
        private_bytes: memory.PrivateUsage as u64,
        read_bytes: io.ReadTransferCount,
        write_bytes: io.WriteTransferCount,
        thread_count: identity.thread_count,
    })
}

#[cfg(windows)]
fn filetime_u64(time: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

#[cfg(windows)]
fn utf16_lossy(units: &[u16]) -> String {
    let len = units
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..len])
}

#[cfg(windows)]
fn map_last_error() -> super::system::AvailabilityFailure {
    if is_access_denied() {
        super::system::AvailabilityFailure::PermissionDenied
    } else {
        super::system::AvailabilityFailure::Unavailable
    }
}

#[cfg(windows)]
fn is_access_denied() -> bool {
    use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, GetLastError};

    // SAFETY: reads this thread's last-error immediately after a failed API.
    unsafe { GetLastError() == ERROR_ACCESS_DENIED }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, name: &str, cpu: u32, memory: u64) -> ProcessV1 {
        ProcessV1 {
            pid,
            name: name.to_string(),
            cpu_basis_points_of_one_logical_core: cpu,
            private_bytes: memory,
            read_bytes_per_second: 0,
            write_bytes_per_second: 0,
        }
    }

    #[test]
    fn truncation_metadata_is_set_before_sorting_and_limit() {
        let items = (0..20)
            .map(|index| row(index, &format!("p{index}.exe"), 20 - index, 100))
            .collect::<Vec<_>>();
        let AvailabilityV1::Partial {
            value,
            reason_codes,
            ..
        } = finalize_processes(items, 20, 15, false, false, 1)
        else {
            panic!("enumerated beyond requested limit is partial");
        };
        assert_eq!(value.enumeration_ceiling, 4_096);
        assert_eq!(value.detail_budget_ms, 150);
        assert_eq!(value.enumerated_count, 20);
        assert_eq!(value.returned_count, 15);
        assert_eq!(value.requested_limit, 15);
        assert!(value.truncated_by_limit);
        assert!(!value.budget_exhausted);
        assert_eq!(reason_codes, &["process_limit".to_string()]);
        assert_eq!(value.items[0].pid, 0);
        assert_eq!(value.items.last().unwrap().pid, 14);
    }

    #[test]
    fn ceiling_and_budget_flags_are_exact() {
        let AvailabilityV1::Partial {
            value,
            reason_codes,
            ..
        } = finalize_processes(vec![row(1, "a.exe", 1, 1)], 4_096, 15, true, false, 1)
        else {
            panic!("ceiling plus budget must be partial");
        };
        assert_eq!(value.enumerated_count, 4_096);
        assert!(value.truncated_by_limit);
        assert!(value.budget_exhausted);
        assert!(reason_codes.contains(&"process_limit".to_string()));
        assert!(reason_codes.contains(&"detail_budget_exhausted".to_string()));
    }

    #[test]
    fn requested_limit_is_clamped_to_one_hundred() {
        let items = (0..120)
            .map(|index| row(index, "n.exe", 1, 1))
            .collect::<Vec<_>>();
        let AvailabilityV1::Partial { value, .. } =
            finalize_processes(items, 120, 1_000, false, false, 1)
        else {
            panic!("over-limit request is still truncated");
        };
        assert_eq!(value.requested_limit, 100);
        assert_eq!(value.returned_count, 100);
    }

    #[test]
    fn process_dto_omits_sensitive_fields() {
        let encoded = serde_json::to_string(&row(42, "fixture.exe", 100, 1_024)).unwrap();
        for forbidden in [
            "path",
            "cmdline",
            "command_line",
            "user",
            "environment",
            "handles",
            "history",
            "C:\\\\",
            "/usr",
        ] {
            assert!(
                !encoded.to_lowercase().contains(&forbidden.to_lowercase()),
                "{encoded} contains {forbidden}"
            );
        }
        assert!(encoded.contains("\"pid\":42"));
        assert!(encoded.contains("fixture.exe"));
    }

    #[test]
    fn pair_delta_skips_reset_and_zero_window() {
        let first = [ProcessCounters {
            pid: 1,
            name: "a.exe".into(),
            cpu_100ns: 5_000,
            private_bytes: 10,
            read_bytes: 100,
            write_bytes: 50,
            thread_count: 1,
        }];
        let reset = [ProcessCounters {
            pid: 1,
            name: "a.exe".into(),
            cpu_100ns: 4_000,
            private_bytes: 10,
            read_bytes: 90,
            write_bytes: 40,
            thread_count: 1,
        }];
        assert!(process_rows_from_pair(&first, &reset, 500, 500).is_empty());
        let later = [ProcessCounters {
            pid: 1,
            name: "a.exe".into(),
            cpu_100ns: 10_000,
            private_bytes: 99,
            read_bytes: 1_100,
            write_bytes: 550,
            thread_count: 2,
        }];
        let rows = process_rows_from_pair(&first, &later, 500, 500);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].private_bytes, 99);
        assert_eq!(rows[0].cpu_basis_points_of_one_logical_core, 10);
        assert!(process_rows_from_pair(&first, &later, 500, 0).is_empty());
    }

    #[test]
    fn access_denial_is_partial_not_zero_rows_as_available() {
        let AvailabilityV1::Partial { reason_codes, .. } =
            finalize_processes(Vec::new(), 8, 15, false, true, 1)
        else {
            panic!("access denial must be partial");
        };
        assert!(reason_codes.contains(&"process_access_denied".to_string()));
    }
}
