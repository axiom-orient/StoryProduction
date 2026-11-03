//! Revision-budget admission for commands that commit text changes.

use crate::contracts::TextRunState;
use crate::error::{Error, Result};
use crate::model::Project;

/// Units of `revision_remaining` consumed per committed text change.
pub(crate) const REVISION_UNITS_PER_TEXT_CHANGE: u64 = 1;

/// Admits one would-be text change against the referenced run's revision
/// budget. Fails closed before any event is written when the run is unknown,
/// terminal, or out of budget.
///
/// # Errors
/// Returns [`Error::NotFound`] for an unknown `run_id`,
/// [`Error::Conflict`] when the run is `COMPLETED`/`FAILED`, and
/// [`Error::BudgetExhausted`] when `revision_remaining` is below
/// `units`.
pub(crate) fn admit_revision_spend(project: &Project, run_id: &str, units: u64) -> Result<()> {
    let run = project
        .text_runs
        .get(run_id)
        .ok_or_else(|| Error::NotFound(format!("text run '{run_id}' was not found")))?;
    if run.project_id != project.id {
        return Err(Error::Corrupt(format!(
            "text run '{run_id}' belongs to a different project"
        )));
    }
    revision_remaining_after_spend(run, units).map(|_| ())
}

/// Shared pure spend policy for command admission and event reduction.
pub(crate) fn revision_remaining_after_spend(run: &TextRunState, units: u64) -> Result<u64> {
    if units == 0 {
        return Err(Error::InvalidInput(
            "revision budget spend must consume at least one unit".into(),
        ));
    }
    if !run.status.can_spend() {
        return Err(Error::Conflict(format!(
            "text run '{}' is {:?} and cannot spend revision budget",
            run.run_id, run.status
        )));
    }
    run.budgets
        .revision_remaining
        .checked_sub(units)
        .ok_or_else(|| {
            Error::BudgetExhausted(format!(
                "text run '{}' has {} revision_remaining, needs {units}",
                run.run_id, run.budgets.revision_remaining
            ))
        })
}
