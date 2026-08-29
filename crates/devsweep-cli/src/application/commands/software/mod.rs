//! Compiler-registered Software handler hierarchy.

mod execution;
mod inventory;

#[cfg(test)]
pub(super) struct Route;

#[cfg(test)]
pub(super) fn assert_leaf_routes_registered() {
    let _ = execution::Route;
    let _ = inventory::Route;
}
