#![warn(unreachable_pub)]
#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

//! Reusable scanning, planning, inventory, and cleanup execution logic.

/// Opt-in AI-tool classification, recovery, and confirmed trash moves.
pub mod ai;
/// Read-only bounded Analyze traversal and versioned snapshots.
pub mod analysis;

/// Neutral Cargo metadata probing and diagnostics.
pub mod cargo_metadata;
/// Pure project capacity advice and its independent warning preference store.
pub mod clean_advisories;
/// Validated desktop appearance and bounded runtime preferences.
pub mod desktop_preferences;
mod digest_encoding;
/// Cleanup execution orchestration and safety enforcement.
pub mod execution;
/// Shared filesystem identity, containment, reparse, and sizing operations.
pub mod filesystem;
/// Versioned, non-replayable Clean/Software/Optimize audit history.
pub mod history;
/// Read-only capacity inventory operations.
pub mod inventory;
/// Cleanup plans and scan report domain models.
pub mod model;
/// Closed standard-user Windows maintenance catalogue and execution.
pub mod optimize;
/// Untrusted-plan validation and canonical plan identity.
pub mod plan;
/// Locale-only presentation preferences shared by interactive frontends.
pub mod presentation_settings;
/// Bounded external-process execution and cooperative cancellation.
pub mod process;
mod project_footprints;
/// Built-in cleanup rule declarations and trusted action reconstruction.
pub mod rules;
/// Project and global cleanup target discovery.
pub mod scan;
pub mod scan_preferences;
/// Frontend-neutral scan, inventory, and cleanup service boundaries.
pub mod services;
/// Read-only installed-software inventory and immutable preview planning.
pub mod software;
/// Bounded read-only Windows Status snapshots and live sampling.
pub mod status;
