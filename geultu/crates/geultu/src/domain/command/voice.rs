//! Pure voice-profile activation decision.

use super::super::Event;
use crate::error::{Error, Result};
use crate::model::{Project, VoiceProfile};
use crate::voice::validate_profile;

pub(super) fn activate_profile(
    project: &Project,
    profile: VoiceProfile,
    expected_project_revision: u64,
) -> Result<Vec<Event>> {
    if project.revision != expected_project_revision {
        return Err(Error::Conflict(format!(
            "project revision mismatch: expected {expected_project_revision}, actual {}",
            project.revision
        )));
    }
    validate_profile(&profile)?;
    Ok(vec![Event::VoiceProfileActivated { profile }])
}
