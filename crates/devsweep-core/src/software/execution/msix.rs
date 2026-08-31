//! Exact current-user MSIX removal and identity-only requery adapters.

use std::{sync::Arc, time::Duration};

use super::{
    AdapterEvidence, InstalledStateRequery, MsixRemovalAdapter, RemovalTiming, RequeryObservation,
    SoftwareAuditErrorCode, SoftwareInstalledState, ValidatedSoftwareAction,
};
use crate::process::FlagCancelObserver;

#[derive(Debug, Default)]
pub(super) struct RealMsixAdapter;

impl MsixRemovalAdapter for RealMsixAdapter {
    fn remove(
        &self,
        action: &ValidatedSoftwareAction,
        cancel: Option<&Arc<FlagCancelObserver>>,
        timing: RemovalTiming,
    ) -> AdapterEvidence {
        remove_exact(action.package_full_name(), cancel.cloned(), timing)
    }
}

#[derive(Debug, Default)]
pub(super) struct RealInstalledStateRequery;

impl InstalledStateRequery for RealInstalledStateRequery {
    fn query(&self, package_full_name: &str) -> RequeryObservation {
        query_exact(package_full_name)
    }
}

#[cfg(windows)]
fn remove_exact(
    package_full_name: &str,
    cancel: Option<Arc<FlagCancelObserver>>,
    timing: RemovalTiming,
) -> AdapterEvidence {
    let package_full_name = package_full_name.to_string();
    match super::super::msix::run_joined_mta(move || {
        remove_on_mta(&package_full_name, cancel.as_ref(), timing)
    }) {
        Ok(evidence) => evidence,
        Err(_) => AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterStatusUnavailable),
    }
}

#[cfg(not(windows))]
fn remove_exact(
    _package_full_name: &str,
    _cancel: Option<Arc<FlagCancelObserver>>,
    _timing: RemovalTiming,
) -> AdapterEvidence {
    AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterDispatchFailed)
}

#[cfg(windows)]
fn remove_on_mta(
    package_full_name: &str,
    cancel: Option<&Arc<FlagCancelObserver>>,
    timing: RemovalTiming,
) -> AdapterEvidence {
    use std::time::Instant;

    use windows::{Foundation::AsyncStatus, Management::Deployment::PackageManager, core::HSTRING};

    use crate::process::CancelObserver;

    let manager = match PackageManager::new() {
        Ok(manager) => manager,
        Err(_) => {
            return AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterDispatchFailed);
        }
    };
    let operation = match manager.RemovePackageAsync(&HSTRING::from(package_full_name)) {
        Ok(operation) => operation,
        Err(_) => {
            return AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterDispatchFailed);
        }
    };

    let started = Instant::now();
    let mut cancel_requested = false;
    let mut cancel_deadline = None;
    let mut timed_out = false;
    let mut cancel_error = None;
    loop {
        if cancel_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return AdapterEvidence::unfinished(cancel_error.unwrap_or(if timed_out {
                SoftwareAuditErrorCode::AdapterTimedOut
            } else {
                SoftwareAuditErrorCode::AdapterCanceledAfterDispatch
            }));
        }
        let status = match operation.Status() {
            Ok(status) => status,
            Err(_) => {
                if cancel_requested {
                    std::thread::sleep(timing.poll_interval);
                    continue;
                }
                return AdapterEvidence::unfinished(
                    SoftwareAuditErrorCode::AdapterStatusUnavailable,
                );
            }
        };
        if status != AsyncStatus::Started {
            return match status {
                AsyncStatus::Completed => match operation.GetResults() {
                    Ok(result) => match result.ExtendedErrorCode() {
                        Ok(code) if code.0 == 0 => AdapterEvidence::success(),
                        Ok(code) if requires_reboot(code) => AdapterEvidence::reboot_required(),
                        Ok(_) | Err(_) => {
                            AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed)
                        }
                    },
                    Err(_) => match operation.ErrorCode() {
                        Ok(code) if requires_reboot(code) => AdapterEvidence::reboot_required(),
                        Ok(code) if code.0 != 0 => {
                            AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed)
                        }
                        _ => AdapterEvidence::unfinished(
                            SoftwareAuditErrorCode::AdapterStatusUnavailable,
                        ),
                    },
                },
                AsyncStatus::Error => {
                    AdapterEvidence::failure(SoftwareAuditErrorCode::AdapterOperationFailed)
                }
                AsyncStatus::Canceled => AdapterEvidence::unfinished(
                    SoftwareAuditErrorCode::AdapterCanceledAfterDispatch,
                ),
                _ => AdapterEvidence::unfinished(SoftwareAuditErrorCode::AdapterStatusUnavailable),
            };
        }

        let cooperative_cancel = cancel.is_some_and(|flag| flag.is_cancel_requested());
        let monitor_expired = started.elapsed() >= timing.monitor_timeout;
        if (cooperative_cancel || monitor_expired) && !cancel_requested {
            cancel_requested = true;
            timed_out = monitor_expired && !cooperative_cancel;
            if operation.Cancel().is_err() {
                cancel_error = Some(SoftwareAuditErrorCode::AdapterCancelFailed);
            }
            cancel_deadline = Some(Instant::now() + timing.cancel_grace);
        }
        let remaining_monitor = timing.monitor_timeout.saturating_sub(started.elapsed());
        let remaining_grace = cancel_deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(timing.poll_interval);
        std::thread::sleep(
            timing
                .poll_interval
                .min(remaining_monitor.max(Duration::from_millis(1)))
                .min(remaining_grace.max(Duration::from_millis(1))),
        );
    }
}

#[cfg(windows)]
fn requires_reboot(code: windows::core::HRESULT) -> bool {
    matches!(code.0 as u32, 3010 | 3011 | 0x8007_0bc2 | 0x8007_0bc3)
}

#[cfg(windows)]
fn query_exact(package_full_name: &str) -> RequeryObservation {
    let package_full_name = package_full_name.to_string();
    match super::super::msix::run_joined_mta(move || query_on_mta(&package_full_name)) {
        Ok(observation) => observation,
        Err(_) => RequeryObservation::unavailable(),
    }
}

#[cfg(not(windows))]
fn query_exact(_package_full_name: &str) -> RequeryObservation {
    RequeryObservation::unavailable()
}

#[cfg(windows)]
fn query_on_mta(package_full_name: &str) -> RequeryObservation {
    use windows::{Management::Deployment::PackageManager, core::HSTRING};

    let sid = match super::super::msix::current_process_sid() {
        Ok(sid) => sid,
        Err(_) => return RequeryObservation::unavailable(),
    };
    let manager = match PackageManager::new() {
        Ok(manager) => manager,
        Err(_) => return RequeryObservation::unavailable(),
    };
    match manager.FindPackageByUserSecurityIdPackageFullName(
        &HSTRING::from(sid),
        &HSTRING::from(package_full_name),
    ) {
        Ok(package) => match package.Id().and_then(|id| id.FullName()) {
            Ok(found) if found.to_string_lossy() == package_full_name => RequeryObservation {
                state: SoftwareInstalledState::Present,
                error_code: None,
            },
            Ok(_) => RequeryObservation {
                state: SoftwareInstalledState::Conflicting,
                error_code: Some(SoftwareAuditErrorCode::RequeryConflicting),
            },
            Err(_) => RequeryObservation::unavailable(),
        },
        Err(error) if package_absent_hresult(error.code().0 as u32) => RequeryObservation {
            state: SoftwareInstalledState::Absent,
            error_code: None,
        },
        Err(_) => RequeryObservation::unavailable(),
    }
}

#[cfg(windows)]
fn package_absent_hresult(value: u32) -> bool {
    matches!(value, 0x8007_3cf1 | 0x8007_3d35)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn documented_reboot_codes_are_closed() {
        use windows::core::HRESULT;

        assert!(requires_reboot(HRESULT(3010)));
        assert!(requires_reboot(HRESULT(3011)));
        assert!(requires_reboot(HRESULT(0x8007_0bc2_u32 as i32)));
        assert!(!requires_reboot(HRESULT(0x8007_3cf1_u32 as i32)));
    }

    #[test]
    #[cfg(windows)]
    fn exact_absent_hresult_table_is_closed() {
        assert!(package_absent_hresult(0x8007_3cf1));
        assert!(package_absent_hresult(0x8007_3d35));
        assert!(!package_absent_hresult(0x8000_4005));
    }
}
