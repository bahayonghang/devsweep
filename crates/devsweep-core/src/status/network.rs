//! `GetIfTable2` topology and byte-counter deltas.

use super::{AvailabilityV1, NetworkInterfaceV1, NetworkV1, system::DeltaIssue, unix_now_ms};

/// One interface sample identified by a 64-bit LUID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InterfaceCounters {
    pub luid: u64,
    pub name: String,
    pub in_octets: u64,
    pub out_octets: u64,
}

pub(crate) fn sample_interfaces()
-> Result<Vec<InterfaceCounters>, super::system::AvailabilityFailure> {
    #[cfg(windows)]
    {
        windows_interfaces()
    }
    #[cfg(not(windows))]
    {
        Err(super::system::AvailabilityFailure::Unsupported)
    }
}

/// Per-interface rates from two `GetIfTable2` snapshots.
///
/// Topology churn and counter resets yield partial/unavailable rather than
/// negative or spiked values. Missing interfaces are omitted, never zeroed.
pub(crate) fn network_from_pair(
    first: &[InterfaceCounters],
    second: &[InterfaceCounters],
    expected_window_ms: u32,
    actual_window_ms: u32,
) -> AvailabilityV1<NetworkV1> {
    if actual_window_ms == 0 {
        return AvailabilityV1::unavailable("zero_elapsed");
    }
    if super::system::is_sleep_sized_gap(expected_window_ms, actual_window_ms) {
        return AvailabilityV1::unavailable("sample_gap");
    }

    let mut interfaces = Vec::new();
    let mut reasons: Vec<String> = Vec::new();
    let mut matched = 0_u32;
    for later in second {
        let Some(earlier) = first.iter().find(|row| row.luid == later.luid) else {
            push_reason(&mut reasons, "interface_churn");
            continue;
        };
        matched += 1;
        match rate_pair(
            earlier.in_octets,
            later.in_octets,
            earlier.out_octets,
            later.out_octets,
            actual_window_ms,
        ) {
            Ok((rx_bytes_per_second, tx_bytes_per_second)) => {
                interfaces.push(NetworkInterfaceV1 {
                    interface_luid: later.luid.to_string(),
                    name: later.name.clone(),
                    rx_bytes_per_second,
                    tx_bytes_per_second,
                });
            }
            Err(DeltaIssue::CounterReset) => push_reason(&mut reasons, "counter_reset"),
            Err(DeltaIssue::ZeroElapsed) => push_reason(&mut reasons, "zero_elapsed"),
            Err(DeltaIssue::SampleGap) => push_reason(&mut reasons, "sample_gap"),
        }
    }
    for earlier in first {
        if second.iter().all(|row| row.luid != earlier.luid) {
            push_reason(&mut reasons, "interface_churn");
        }
    }

    if matched == 0 && !first.is_empty() {
        return AvailabilityV1::unavailable("interface_churn");
    }

    let value = NetworkV1 {
        interval_ms: actual_window_ms,
        interfaces,
    };
    let sampled = unix_now_ms();
    if reasons.is_empty() {
        AvailabilityV1::available(sampled, 0, value)
    } else {
        AvailabilityV1::partial(sampled, 0, value, reasons)
    }
}

fn rate_pair(
    in_first: u64,
    in_second: u64,
    out_first: u64,
    out_second: u64,
    interval_ms: u32,
) -> Result<(u64, u64), DeltaIssue> {
    if interval_ms == 0 {
        return Err(DeltaIssue::ZeroElapsed);
    }
    if in_second < in_first || out_second < out_first {
        return Err(DeltaIssue::CounterReset);
    }
    Ok((
        bytes_per_second(in_second - in_first, interval_ms),
        bytes_per_second(out_second - out_first, interval_ms),
    ))
}

fn bytes_per_second(delta: u64, interval_ms: u32) -> u64 {
    ((u128::from(delta) * 1_000) / u128::from(interval_ms.max(1))) as u64
}

fn push_reason(reasons: &mut Vec<String>, code: &str) {
    if !reasons.iter().any(|existing| existing == code) {
        reasons.push(code.to_string());
    }
}

#[cfg(windows)]
fn windows_interfaces() -> Result<Vec<InterfaceCounters>, super::system::AvailabilityFailure> {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        FreeMibTable, GetIfTable2, MIB_IF_TABLE2,
    };

    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: `table` receives an allocated MIB_IF_TABLE2 that the caller frees.
    let status = unsafe { GetIfTable2(&mut table) };
    if status != 0 || table.is_null() {
        return Err(map_status(status));
    }
    struct TableGuard(*mut MIB_IF_TABLE2);
    impl Drop for TableGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: `GetIfTable2` allocated this table for `FreeMibTable`.
                unsafe { FreeMibTable(self.0.cast()) };
            }
        }
    }
    let _guard = TableGuard(table);

    // SAFETY: `table` is a valid `GetIfTable2` allocation for this scope.
    let header = unsafe { &*table };
    let count = header.NumEntries as usize;
    // SAFETY: `Table` is the flexible array of `NumEntries` `MIB_IF_ROW2` rows.
    let rows = unsafe { std::slice::from_raw_parts(header.Table.as_ptr(), count) };
    Ok(rows.iter().map(row_to_counters).collect())
}

#[cfg(windows)]
fn row_to_counters(
    row: &windows_sys::Win32::NetworkManagement::IpHelper::MIB_IF_ROW2,
) -> InterfaceCounters {
    let luid = luid_value(row);
    let name = utf16_lossy(&row.Alias);
    InterfaceCounters {
        luid,
        name: if name.is_empty() {
            format!("interface-{luid}")
        } else {
            name
        },
        in_octets: row.InOctets,
        out_octets: row.OutOctets,
    }
}

#[cfg(windows)]
fn luid_value(row: &windows_sys::Win32::NetworkManagement::IpHelper::MIB_IF_ROW2) -> u64 {
    use windows_sys::Win32::NetworkManagement::Ndis::NET_LUID_LH;
    let luid: NET_LUID_LH = row.InterfaceLuid;
    // SAFETY: the union's `Value` representation is a u64 LUID.
    unsafe { luid.Value }
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
fn map_status(status: u32) -> super::system::AvailabilityFailure {
    use windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED;

    if status == ERROR_ACCESS_DENIED {
        super::system::AvailabilityFailure::PermissionDenied
    } else {
        super::system::AvailabilityFailure::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::AvailabilityV1;

    fn iface(luid: u64, inn: u64, out: u64) -> InterfaceCounters {
        InterfaceCounters {
            luid,
            name: format!("if-{luid}"),
            in_octets: inn,
            out_octets: out,
        }
    }

    #[test]
    fn rates_use_integer_byte_deltas() {
        let first = [iface(1, 1_000, 500)];
        let second = [iface(1, 2_000, 1_000)];
        let AvailabilityV1::Available { value, .. } =
            network_from_pair(&first, &second, 1_000, 1_000)
        else {
            panic!("matched interfaces should be available");
        };
        assert_eq!(value.interval_ms, 1_000);
        assert_eq!(value.interfaces[0].interface_luid, "1");
        assert_eq!(value.interfaces[0].rx_bytes_per_second, 1_000);
        assert_eq!(value.interfaces[0].tx_bytes_per_second, 500);
    }

    #[test]
    fn interface_churn_and_reset_are_partial_not_negative() {
        let first = [iface(1, 5_000, 5_000), iface(2, 10, 10)];
        let second = [iface(1, 4_000, 4_000), iface(3, 100, 100)];
        let AvailabilityV1::Partial {
            value,
            reason_codes,
            ..
        } = network_from_pair(&first, &second, 1_000, 1_000)
        else {
            panic!("churn plus reset must be partial");
        };
        assert!(reason_codes.iter().any(|code| code == "counter_reset"));
        assert!(reason_codes.iter().any(|code| code == "interface_churn"));
        assert!(value.interfaces.is_empty());
    }

    #[test]
    fn total_churn_is_unavailable_not_zero_rates() {
        let first = [iface(1, 10, 10)];
        let second = [iface(2, 20, 20)];
        assert!(matches!(
            network_from_pair(&first, &second, 1_000, 1_000),
            AvailabilityV1::Unavailable { .. }
        ));
    }

    #[test]
    fn zero_elapsed_and_sleep_gap_do_not_synthesize_rates() {
        let pair = [iface(1, 0, 0)];
        assert!(matches!(
            network_from_pair(&pair, &pair, 1_000, 0),
            AvailabilityV1::Unavailable {
                reason_code,
                ..
            } if reason_code == "zero_elapsed"
        ));
        assert!(matches!(
            network_from_pair(&pair, &[iface(1, 50, 50)], 1_000, 20_000),
            AvailabilityV1::Unavailable {
                reason_code,
                ..
            } if reason_code == "sample_gap"
        ));
    }
}
