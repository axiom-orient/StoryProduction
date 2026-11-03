//! H6 admission layer: fail-closed command admission against text-run
//! budgets plus snapshot conformance predicates for the project-state SSOT.
//!
//! Budgets bind to the text-run-state concept (`revision_remaining` /
//! `pivot_remaining`): only commands whose payloads reference a text run via
//! `run_id` are gated. Snapshot conformance enforces the project-state
//! contract subset and engine-owned lineage that state can satisfy truthfully;
//! violations fail closed and are never silently repaired. Revision spending uses
//! the same pure policy during admission and journal replay; pivot spending is unwired.

pub(crate) mod budget;
pub(crate) mod snapshot;
