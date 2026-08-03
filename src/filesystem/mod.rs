//! Shared filesystem identity, containment, reparse safety, and bounded sizing.

mod containment;
mod identity;
mod reparse;
mod sizing;

pub(crate) use containment::path_contains_path;
pub use identity::{PathIdentity, PathIdentityKind, capture_path_identity};
pub(crate) use identity::{
    normalize_absolute_path, normalize_path_for_compare, path_is_within, paths_equal,
};
pub(crate) use reparse::{
    PathReparseProbe, PathSafety, SystemPathReparseProbe, inspect_path_no_follow,
};
#[cfg(test)]
pub(crate) use reparse::{ReparseProbeResult, is_unsafe_link};
pub(crate) use sizing::estimate_tree_with_budget_and_cancel_and_probe;
pub use sizing::{
    DEFAULT_SIZE_ENTRY_BUDGET, SizeEstimate, estimate_tree, estimate_tree_with_budget,
    estimate_tree_with_budget_and_cancel,
};
