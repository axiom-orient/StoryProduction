//! Voice profile, comparison, and guide routes.

use super::super::requests::common::ProjectRef;
use super::super::requests::voice::{VoiceCompare, VoiceGuideRequest, VoiceProfileCreate};
use super::super::{command_id, commit, parse, Application};
use crate::domain::Command;
use crate::error::{Error, Result};
use crate::voice::{build_guide, build_profile, compare_to_profile};
use serde_json::Value;

pub(in crate::application) fn call(
    app: &Application,
    name: &str,
    arguments: Value,
) -> Result<Value> {
    match name {
        "voice.profile.create" => {
            let req: VoiceProfileCreate = parse(arguments)?;
            let command_id = command_id(req.command_id);
            let project = app.workspace.load_project(&req.project_id)?;
            if let Some(revision) = project.applied_commands.get(&command_id) {
                return Ok(serde_json::to_value(crate::store::CommitResult {
                    project_id: req.project_id,
                    revision: *revision,
                    event_count: 0,
                    idempotent: true,
                })?);
            }
            if project.revision != req.expected_project_revision {
                return Err(Error::Conflict(format!(
                    "project revision mismatch: expected {}, actual {}",
                    req.expected_project_revision, project.revision
                )));
            }
            let samples = req
                .samples
                .into_iter()
                .enumerate()
                .map(|(index, sample)| sample.into_voice_sample(index + 1))
                .collect::<Vec<_>>();
            let profile = build_profile(
                req.profile_id.as_deref().unwrap_or("voice-profile"),
                &samples,
            )?;
            commit(
                &app.workspace,
                &req.project_id,
                Some(command_id),
                Command::ActivateVoiceProfile {
                    profile,
                    expected_project_revision: req.expected_project_revision,
                },
            )
        }
        "voice.profile.get" => {
            let req: ProjectRef = parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            Ok(serde_json::to_value(project.voice_profile)?)
        }
        "voice.compare" => {
            let req: VoiceCompare = parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            let profile = project.voice_profile.as_ref().ok_or_else(|| {
                Error::NotFound("active voice profile was not found; create 글투 first".into())
            })?;
            Ok(serde_json::to_value(compare_to_profile(
                profile,
                &req.candidate_text,
            )?)?)
        }
        "voice.guide" => {
            let req: VoiceGuideRequest = parse(arguments)?;
            let project = app.workspace.load_project(&req.project_id)?;
            Ok(serde_json::to_value(build_guide(&project, &req.scene_id)?)?)
        }
        _ => Err(super::unknown_tool(name)),
    }
}
