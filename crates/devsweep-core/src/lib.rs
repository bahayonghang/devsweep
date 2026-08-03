#![warn(unreachable_pub)]

//! Reusable scanning, planning, inventory, and cleanup execution logic.

/// Neutral Cargo metadata probing and diagnostics.
pub mod cargo_metadata;
/// Cleanup execution orchestration and safety enforcement.
pub mod execution;
/// Shared filesystem identity, containment, reparse, and sizing operations.
pub mod filesystem;
/// Read-only capacity inventory operations.
pub mod inventory;
/// Cleanup plans and scan report domain models.
pub mod model;
/// Untrusted-plan validation and canonical plan identity.
pub mod plan;
/// Bounded external-process execution and cooperative cancellation.
pub mod process;
/// Built-in cleanup rule declarations and trusted action reconstruction.
pub mod rules;
/// Project and global cleanup target discovery.
pub mod scan;
/// Frontend-neutral scan, inventory, and cleanup service boundaries.
pub mod services;
