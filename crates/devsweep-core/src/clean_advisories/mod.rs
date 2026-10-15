//! Read-only project capacity advice. This module creates no cleanup authority.

mod preferences;

use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    filesystem::normalize_absolute_path,
    model::{
        CLEANUP_PLAN_VERSION, SCAN_REPORT_VERSION, ScanCoverageScope, ScanCoverageStatus,
        ScanReport, Scope, TargetId, TargetKind,
    },
    project_footprints::{FootprintAttribution, FootprintCategory, project_footprints},
};

pub use preferences::{load_clean_advisory_preferences, save_clean_advisory_preferences};

/// Binary GiB; settings accept positive whole units only.
pub const GIB: u64 = 1 << 30;
/// Default warning is inclusive at five GiB of project build/test/tool artifacts.
pub const DEFAULT_THRESHOLD_BYTES: u64 = 5 * GIB;
const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;
/// Largest whole GiB value representable as exact bytes by the desktop boundary.
pub const MAX_THRESHOLD_GIB: u64 = MAX_SAFE_INTEGER / GIB;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanAdvisoryPreferencesV1 {
    pub version: u8,
    pub threshold_bytes: u64,
}

impl Default for CleanAdvisoryPreferencesV1 {
    fn default() -> Self {
        Self {
            version: 1,
            threshold_bytes: DEFAULT_THRESHOLD_BYTES,
        }
    }
}

/// Validate a session-only setting without reading or writing any store.
pub fn clean_advisory_preferences(threshold_gib: u64) -> Result<CleanAdvisoryPreferencesV1> {
    let bytes = threshold_gib.checked_mul(GIB);
    if threshold_gib == 0 || bytes.is_none_or(|bytes| bytes > MAX_SAFE_INTEGER) {
        bail!("project warning threshold must be 1..={MAX_THRESHOLD_GIB} whole GiB");
    }
    Ok(CleanAdvisoryPreferencesV1 {
        version: 1,
        threshold_bytes: bytes.unwrap(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityConfidence {
    Exact,
    LowerBound,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityAssessment {
    Oversized,
    BelowThreshold,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectCapacitySubtotalV1 {
    pub known_bytes: u64,
    pub confidence: CapacityConfidence,
    pub target_ids: Vec<TargetId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectCapacityAdvisoryV1 {
    pub owner_root: PathBuf,
    pub artifacts: ProjectCapacitySubtotalV1,
    pub dependencies: ProjectCapacitySubtotalV1,
    pub excluded_target_ids: Vec<TargetId>,
    pub assessment: CapacityAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanAdvisoriesV1 {
    pub version: u8,
    /// Hash of the original report, solely for correlating observations.
    pub report_id: String,
    pub threshold_bytes: u64,
    pub included_kinds: Vec<TargetKind>,
    pub projects: Vec<ProjectCapacityAdvisoryV1>,
}

#[derive(Default)]
struct Subtotal {
    bytes: u64,
    incomplete: bool,
    ids: Vec<TargetId>,
}
impl Subtotal {
    fn include(&mut self, bytes: u64, complete: bool, ids: impl Iterator<Item = TargetId>) {
        let sum = self.bytes.checked_add(bytes);
        self.incomplete |= !complete || sum.is_none_or(|sum| sum > MAX_SAFE_INTEGER);
        self.bytes = sum.unwrap_or(u64::MAX).min(MAX_SAFE_INTEGER);
        self.ids.extend(ids);
    }
    fn finish(mut self) -> ProjectCapacitySubtotalV1 {
        self.ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        self.ids.dedup();
        ProjectCapacitySubtotalV1 {
            known_bytes: self.bytes,
            confidence: if !self.incomplete {
                CapacityConfidence::Exact
            } else if self.bytes > 0 {
                CapacityConfidence::LowerBound
            } else {
                CapacityConfidence::Unknown
            },
            target_ids: self.ids,
        }
    }
}
struct Project {
    root: PathBuf,
    artifacts: Subtotal,
    dependencies: Subtotal,
    excluded: Vec<TargetId>,
}

fn owner_key(root: &Path) -> String {
    normalize_absolute_path(root).unwrap_or_else(|_| format!("unresolved:{}", root.display()))
}

fn covered(coverage: &BTreeMap<String, bool>, root: &Path) -> bool {
    let Ok(mut path) = normalize_absolute_path(root) else {
        return false;
    };
    let mut found = false;
    // Probe component ancestors in the normalized index, rather than comparing
    // every project with every coverage row in a large completed report.
    loop {
        if let Some(complete) = coverage.get(&path) {
            found = true;
            if !complete {
                return false;
            }
        }
        let trimmed = path.trim_end_matches('/');
        let Some(separator) = trimmed.rfind('/') else {
            break;
        };
        let end = if separator == 0 {
            separator + 1
        } else {
            separator
        };
        path = trimmed[..end].to_string();
    }
    found
}

/// Re-evaluate one completed observation. No filesystem, command, timer,
/// selection or plan mutation occurs, including when the threshold changes.
pub fn project_capacity_advisories(
    report: &ScanReport,
    threshold_gib: u64,
) -> Result<CleanAdvisoriesV1> {
    let preferences = clean_advisory_preferences(threshold_gib)?;
    if report.version != SCAN_REPORT_VERSION || report.plan.version != CLEANUP_PLAN_VERSION {
        bail!("unsupported report or plan version for project capacity advice");
    }
    let mut ids = HashSet::new();
    if report
        .plan
        .targets
        .iter()
        .any(|target| target.id.as_str().trim().is_empty() || !ids.insert(&target.id))
    {
        bail!("project capacity advice requires unique nonempty target identities");
    }
    let mut coverage = BTreeMap::new();
    for unit in &report.health.coverage {
        if unit.scope == ScanCoverageScope::Project
            && let Some(path) = unit
                .path
                .as_deref()
                .and_then(|path| normalize_absolute_path(path).ok())
        {
            let complete = matches!(
                unit.status,
                ScanCoverageStatus::Complete | ScanCoverageStatus::Empty
            );
            coverage
                .entry(path)
                .and_modify(|value| *value &= complete)
                .or_insert(complete);
        }
    }
    let mut projects = BTreeMap::new();
    let mut add_owner = |root: &Path| {
        projects.entry(owner_key(root)).or_insert_with(|| {
            let incomplete = !covered(&coverage, root);
            Project {
                root: root.to_path_buf(),
                artifacts: Subtotal {
                    incomplete,
                    ..Default::default()
                },
                dependencies: Subtotal {
                    incomplete,
                    ..Default::default()
                },
                excluded: Vec::new(),
            }
        });
    };
    for target in &report.plan.targets {
        if let Scope::Project { root } = &target.scope {
            add_owner(root);
        }
    }
    for unit in &report.health.coverage {
        if unit.scope == ScanCoverageScope::Project
            && unit.target_count == 0
            && let Some(root) = &unit.path
        {
            add_owner(root);
        }
    }
    for footprint in project_footprints(&report.plan.targets) {
        for owner in &footprint.owner_roots {
            let project = projects
                .get_mut(&owner_key(owner))
                .expect("reported owner was indexed");
            let ids = || {
                footprint
                    .target_indices
                    .iter()
                    .map(|index| report.plan.targets[*index].id.clone())
            };
            if footprint.attribution != FootprintAttribution::Owned {
                project.artifacts.incomplete = true;
                project.dependencies.incomplete = true;
                project.excluded.extend(ids());
                continue;
            }
            let subtotal = match footprint.category {
                FootprintCategory::Artifact => Some(&mut project.artifacts),
                FootprintCategory::Dependency => Some(&mut project.dependencies),
                FootprintCategory::Other => None,
                FootprintCategory::Mixed => {
                    project.artifacts.incomplete = true;
                    project.dependencies.incomplete = true;
                    None
                }
            };
            if let Some(subtotal) = subtotal {
                subtotal.include(footprint.logical_bytes, footprint.size_complete, ids());
            } else {
                project.excluded.extend(ids());
            }
        }
    }
    let projects = projects
        .into_values()
        .map(|mut project| {
            let artifacts = project.artifacts.finish();
            let assessment = if artifacts.known_bytes >= preferences.threshold_bytes {
                CapacityAssessment::Oversized
            } else if artifacts.confidence == CapacityConfidence::Exact {
                CapacityAssessment::BelowThreshold
            } else {
                CapacityAssessment::Incomplete
            };
            project.excluded.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            project.excluded.dedup();
            ProjectCapacityAdvisoryV1 {
                owner_root: project.root,
                artifacts,
                dependencies: project.dependencies.finish(),
                excluded_target_ids: project.excluded,
                assessment,
            }
        })
        .collect();
    Ok(CleanAdvisoriesV1 {
        version: 1,
        report_id: format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(report)?)),
        threshold_bytes: preferences.threshold_bytes,
        included_kinds: vec![
            TargetKind::BuildArtifacts,
            TargetKind::TestCache,
            TargetKind::ToolCache,
        ],
        projects,
    })
}

#[cfg(test)]
mod tests;
