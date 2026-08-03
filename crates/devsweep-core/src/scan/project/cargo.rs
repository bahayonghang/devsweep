use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    cargo_metadata::{
        CargoMetadataFailure, CargoMetadataProbe, CargoMetadataProbeResult, CargoMetadataScope,
    },
    filesystem::PathReparseProbe,
};

use super::{is_real_dir, is_real_file};

const MAX_CARGO_METADATA_CACHE_ENTRIES: usize = 256;

/// Scan-lifetime Cargo metadata cache. Successful resolutions are indexed only
/// by canonical manifests that Cargo explicitly reported as workspace members;
/// failures are reusable only for the exact canonical manifest that failed.
#[derive(Debug)]
pub(super) struct CargoWorkspaceCache {
    max_entries: usize,
    workspaces: Vec<CachedCargoWorkspace>,
    member_workspaces: HashMap<PathBuf, usize>,
    failures: HashMap<PathBuf, CargoMetadataFailure>,
}

#[derive(Debug, Clone)]
struct CachedCargoWorkspace {
    canonical_workspace_root: PathBuf,
    workspace_root: PathBuf,
    target_directory: PathBuf,
}

impl CachedCargoWorkspace {
    fn scope(&self) -> CargoMetadataScope {
        CargoMetadataScope {
            workspace_root: self.workspace_root.clone(),
            target_directory: self.target_directory.clone(),
            member_manifests: Vec::new(),
        }
    }
}

impl Default for CargoWorkspaceCache {
    fn default() -> Self {
        Self::with_limit(MAX_CARGO_METADATA_CACHE_ENTRIES)
    }
}

impl CargoWorkspaceCache {
    pub(super) fn with_limit(max_entries: usize) -> Self {
        Self {
            max_entries,
            workspaces: Vec::new(),
            member_workspaces: HashMap::new(),
            failures: HashMap::new(),
        }
    }

    pub(super) fn resolve(
        &mut self,
        manifest: &Path,
        manifest_dir: &Path,
        probe: &dyn CargoMetadataProbe,
        reparse_probe: &dyn PathReparseProbe,
    ) -> CargoMetadataProbeResult {
        let manifest_key = canonical_manifest(manifest, reparse_probe);
        if let Some(manifest_key) = manifest_key.as_ref() {
            if let Some(failure) = self.failures.get(manifest_key) {
                return CargoMetadataProbeResult::Failed(failure.clone());
            }
            if let Some(workspace_index) = self.member_workspaces.get(manifest_key)
                && let Some(workspace) = self.workspaces.get(*workspace_index)
            {
                return CargoMetadataProbeResult::Resolved(workspace.scope());
            }
        }

        let result = probe.probe(manifest_dir);
        match &result {
            CargoMetadataProbeResult::Resolved(scope) => {
                self.cache_success(scope, reparse_probe);
            }
            CargoMetadataProbeResult::Failed(failure) => {
                self.cache_failure(manifest_key, failure);
            }
        }
        result
    }

    fn cache_success(&mut self, scope: &CargoMetadataScope, reparse_probe: &dyn PathReparseProbe) {
        let Some(workspace_root) = canonical_workspace_root(&scope.workspace_root, reparse_probe)
        else {
            return;
        };
        let mut members: Vec<PathBuf> = scope
            .member_manifests
            .iter()
            .filter_map(|member| canonical_manifest(member, reparse_probe))
            .collect();
        members.sort();
        members.dedup();
        if members.is_empty() {
            return;
        }

        let workspace_index = match self
            .workspaces
            .iter()
            .position(|workspace| workspace.canonical_workspace_root == workspace_root)
        {
            Some(index) => index,
            None => {
                // A new workspace needs one entry plus at least one membership
                // entry, otherwise it could never be returned by a lookup.
                if self.entry_count() >= self.max_entries.saturating_sub(1) {
                    return;
                }
                self.workspaces.push(CachedCargoWorkspace {
                    canonical_workspace_root: workspace_root,
                    workspace_root: scope.workspace_root.clone(),
                    target_directory: scope.target_directory.clone(),
                });
                self.workspaces.len() - 1
            }
        };

        for member in members {
            if self.member_workspaces.contains_key(&member) {
                continue;
            }
            if !self.has_capacity() {
                break;
            }
            self.member_workspaces.insert(member, workspace_index);
        }
    }

    fn cache_failure(&mut self, manifest: Option<PathBuf>, failure: &CargoMetadataFailure) {
        let Some(manifest) = manifest else {
            return;
        };
        if !self.failures.contains_key(&manifest) && self.has_capacity() {
            self.failures.insert(manifest, failure.clone());
        }
    }

    pub(super) fn entry_count(&self) -> usize {
        self.workspaces.len() + self.member_workspaces.len() + self.failures.len()
    }

    fn has_capacity(&self) -> bool {
        self.entry_count() < self.max_entries
    }
}

fn canonical_manifest(path: &Path, reparse_probe: &dyn PathReparseProbe) -> Option<PathBuf> {
    is_real_file(path, reparse_probe)
        .then(|| path.canonicalize().ok())
        .flatten()
}

fn canonical_workspace_root(path: &Path, reparse_probe: &dyn PathReparseProbe) -> Option<PathBuf> {
    is_real_dir(path, reparse_probe)
        .then(|| path.canonicalize().ok())
        .flatten()
}
