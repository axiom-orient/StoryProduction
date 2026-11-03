//! Pure text-run opening decision.

use super::super::Event;
use crate::contracts::TextRunState;
use crate::error::{Error, Result};
use crate::model::Project;

pub(super) fn open(project: &Project, run: TextRunState) -> Result<Vec<Event>> {
    run.validate()?;
    if run.project_id != project.id {
        return Err(Error::InvalidInput(format!(
            "text run project_id '{}' does not match project '{}'",
            run.project_id, project.id
        )));
    }
    if project.text_runs.contains_key(&run.run_id) {
        return Err(Error::Conflict(format!(
            "text run '{}' already exists",
            run.run_id
        )));
    }
    Ok(vec![Event::TextRunOpened { run }])
}
