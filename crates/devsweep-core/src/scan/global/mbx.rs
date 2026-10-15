//! Public, bounded mbx discovery only. The reviewed 1.24.0 interface does not
//! provide a workspace footprint/expected-state cleanup contract. No internal
//! metadata, shared store traversal, or executable cleanup target is exposed.

use std::path::{Component, PathBuf};

use serde::Deserialize;

use super::probe::ProviderProbe;
use crate::model::{ScanCoverage, ScanCoverageScope, ScanCoverageStatus};

pub(super) const COVERAGE_ID: &str = "mbx.shared-cache";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheDirectoryV1 {
    version: u32,
    store: PathBuf,
}

pub(super) fn coverage(probe: &impl ProviderProbe) -> ScanCoverage {
    probe.begin_provider();
    let Some(executable) = probe.resolve_executable("mbx") else {
        return unavailable(probe, ScanCoverageStatus::Absent, "mbx tool is absent");
    };
    let Some(version) = probe.command_output(&executable, &["--version"]) else {
        return unavailable(
            probe,
            ScanCoverageStatus::Failed,
            "mbx version query unavailable; capacity is unknown",
        );
    };
    if !matches!(version.trim(), "mbx 1.24.0" | "1.24.0") {
        return entry(
            None,
            ScanCoverageStatus::Unsupported,
            "mbx version is outside the reviewed 1.24.0 reporting contract; capacity is unknown",
        );
    }
    let Some(output) = probe.command_output(&executable, &["cache", "dir", "--json"]) else {
        return unavailable(
            probe,
            ScanCoverageStatus::Failed,
            "mbx shared-store location query unavailable; capacity is unknown",
        );
    };
    let Ok(directory) = serde_json::from_str::<CacheDirectoryV1>(&output) else {
        return entry(
            None,
            ScanCoverageStatus::Failed,
            "mbx shared-store location response is invalid; capacity is unknown",
        );
    };
    if directory.version != 1 {
        return entry(
            None,
            ScanCoverageStatus::Unsupported,
            "mbx shared-store location schema is unsupported; capacity is unknown",
        );
    }
    if !directory.store.is_absolute()
        || directory
            .store
            .to_str()
            .is_none_or(|path| path.chars().any(char::is_control))
        || directory
            .store
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return entry(
            None,
            ScanCoverageStatus::Failed,
            "mbx shared-store location is not an unambiguous absolute path; capacity is unknown",
        );
    }
    entry(
        Some(directory.store),
        ScanCoverageStatus::Unsupported,
        "mbx shared action-store location reported; occupancy was not measured and is not attributed to projects. Workspace ownership and cleanup are not supported by the qualified public interface",
    )
}

fn unavailable(
    probe: &impl ProviderProbe,
    fallback: ScanCoverageStatus,
    detail: &str,
) -> ScanCoverage {
    entry(None, probe.failure().unwrap_or(fallback), detail)
}

fn entry(path: Option<PathBuf>, status: ScanCoverageStatus, detail: &str) -> ScanCoverage {
    ScanCoverage {
        id: COVERAGE_ID.to_string(),
        scope: ScanCoverageScope::Provider,
        path,
        status,
        target_count: 0,
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, path::Path};

    use crate::{
        filesystem::SizeEstimate,
        model::{ScanHealth, ScanReport, UntrustedPlan},
    };

    use super::*;

    struct Probe {
        installed: bool,
        version: &'static str,
        directory: String,
        failure: Option<ScanCoverageStatus>,
        calls: RefCell<Vec<Vec<String>>>,
    }

    impl Default for Probe {
        fn default() -> Self {
            Self {
                installed: true,
                version: "mbx 1.24.0\n",
                directory: serde_json::json!({
                    "version": 1,
                    "store": if cfg!(windows) { r"Q:\fixture\actions" } else { "/fixture/actions" },
                })
                .to_string(),
                failure: None,
                calls: RefCell::new(vec![]),
            }
        }
    }

    impl ProviderProbe for Probe {
        fn failure(&self) -> Option<ScanCoverageStatus> {
            self.failure
        }
        fn resolve_executable(&self, program: &str) -> Option<PathBuf> {
            assert_eq!(program, "mbx");
            self.installed.then(|| PathBuf::from("fixture-mbx"))
        }
        fn command_output(&self, _: &Path, args: &[&str]) -> Option<String> {
            self.calls
                .borrow_mut()
                .push(args.iter().map(|arg| arg.to_string()).collect());
            if self.failure.is_some() {
                return None;
            }
            match args {
                ["--version"] => Some(self.version.into()),
                ["cache", "dir", "--json"] => Some(self.directory.clone()),
                _ => panic!("unexpected command: {args:?}"),
            }
        }
        fn env_path(&self, _: &str) -> Option<PathBuf> {
            panic!("no environment inspection")
        }
        fn home_dir(&self) -> Option<PathBuf> {
            panic!("no home inspection")
        }
        fn is_dir(&self, _: &Path) -> bool {
            panic!("no store traversal")
        }
        fn estimate_path_size(&self, _: &Path) -> SizeEstimate {
            panic!("no store sizing")
        }
    }

    #[test]
    fn location_only_is_unknown_capacity_without_any_cleanup_target() {
        let probe = Probe::default();
        let observed = coverage(&probe);
        assert_eq!(observed.status, ScanCoverageStatus::Unsupported);
        assert!(observed.path.is_some());
        assert_eq!(observed.target_count, 0);
        assert!(observed.detail.contains("occupancy was not measured"));
        assert!(observed.detail.contains("not attributed to projects"));
        assert_eq!(
            *probe.calls.borrow(),
            vec![vec!["--version"], vec!["cache", "dir", "--json"]]
        );
        let mut health = ScanHealth::complete();
        health.record_coverage(observed);
        let report = ScanReport::new(UntrustedPlan::empty(), health);
        let serialized = serde_json::to_string(&report).expect("report serializes");
        assert_eq!(
            serde_json::from_str::<ScanReport>(&serialized).expect("same V1 decoder"),
            report
        );
        assert!(report.plan.targets.is_empty());
    }

    #[test]
    fn absent_and_unsupported_versions_do_not_query_storage() {
        let probe = Probe {
            installed: false,
            ..Probe::default()
        };
        assert_eq!(coverage(&probe).status, ScanCoverageStatus::Absent);
        assert!(probe.calls.borrow().is_empty());
        for version in ["mbx 1.23.0", "mbx 1.24.1", "mbx 2.0.0", "1.24.0\nextra"] {
            let probe = Probe {
                version,
                ..Probe::default()
            };
            let observed = coverage(&probe);
            assert_eq!(observed.status, ScanCoverageStatus::Unsupported);
            assert!(observed.path.is_none());
            assert_eq!(probe.calls.borrow().len(), 1);
        }
    }

    #[test]
    fn malformed_ambiguous_and_future_reports_never_become_paths_or_empty_success() {
        for (directory, status) in [
            ("{}", ScanCoverageStatus::Failed),
            (
                "{\"version\":1,\"store\":\"relative/actions\"}",
                ScanCoverageStatus::Failed,
            ),
            ("{\"version\":1,\"store\":null}", ScanCoverageStatus::Failed),
            (
                "{\"version\":2,\"store\":\"/fixture/actions\"}",
                ScanCoverageStatus::Unsupported,
            ),
            (
                "{\"version\":1,\"store\":\"/fixture/actions\",\"command\":\"clean\"}",
                ScanCoverageStatus::Failed,
            ),
            (
                "{\"version\":1,\"store\":\"/fixture/actions\"} trailing",
                ScanCoverageStatus::Failed,
            ),
        ] {
            let probe = Probe {
                directory: directory.into(),
                ..Probe::default()
            };
            let observed = coverage(&probe);
            assert_eq!(observed.status, status);
            assert!(observed.path.is_none());
        }
    }

    #[test]
    fn runner_failures_keep_their_typed_status_without_fallback_queries() {
        for failure in [
            ScanCoverageStatus::Timeout,
            ScanCoverageStatus::OutputTruncated,
            ScanCoverageStatus::Canceled,
            ScanCoverageStatus::BudgetLimited,
            ScanCoverageStatus::Failed,
            ScanCoverageStatus::Busy,
        ] {
            let probe = Probe {
                failure: Some(failure),
                ..Probe::default()
            };
            let observed = coverage(&probe);
            assert_eq!(observed.status, failure);
            assert!(observed.path.is_none());
            assert_eq!(probe.calls.borrow().len(), 1);
        }
    }
}
