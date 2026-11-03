//! Pure Command-to-Event dispatch, grouped by canonical domain owner.

mod narrative;
mod receipts;
mod review;
mod run;
mod visual;
mod voice;

use super::{Command, Event};
use crate::error::Result;
use crate::model::Project;

pub fn decide(project: &Project, command: Command) -> Result<Vec<Event>> {
    match command {
        Command::Authoring { mutation } => {
            crate::authoring::apply(project, &mutation)?;
            Ok(vec![Event::AuthoringApplied { mutation }])
        }
        Command::AddScene { scene } => narrative::add_scene(project, scene),
        Command::SetSceneText {
            scene_id,
            text,
            expected_scene_revision,
            run_id,
        } => narrative::set_scene_text(project, scene_id, text, expected_scene_revision, run_id),
        Command::ImportScenes { scenes } => narrative::import_scenes(project, scenes),
        Command::AddFact { fact } => narrative::add_fact(project, fact),
        Command::AddTimeline { entry } => narrative::add_timeline(project, entry),
        Command::AddThread { thread } => narrative::add_thread(project, thread),
        Command::SetThreadStatus { thread_id, status } => {
            narrative::set_thread_status(project, thread_id, status)
        }
        Command::AddCandidate { candidate } => review::add_candidate(project, candidate),
        Command::PrepareSceneCandidate {
            candidate,
            scene_id,
            expected_scene_revision,
            changeset_id,
        } => review::prepare_scene_candidate(
            project,
            candidate,
            scene_id,
            expected_scene_revision,
            changeset_id,
        ),
        Command::RejectCandidate {
            candidate_id,
            decision,
        } => review::reject_candidate(project, candidate_id, decision),
        Command::AddChangeSet { changeset } => review::add_changeset(project, changeset),
        Command::ApproveChangeSet {
            changeset_id,
            expected_project_revision,
            run_id,
            decision,
        } => review::approve_changeset(
            project,
            changeset_id,
            expected_project_revision,
            run_id,
            decision,
        ),
        Command::RejectChangeSet {
            changeset_id,
            decision,
        } => review::reject_changeset(project, changeset_id, decision),
        Command::AddVisualPlan { plan } => visual::add_plan(project, plan),
        Command::AdmitVisualPackage { run, receipt } => {
            visual::admit_package(project, run, receipt)
        }
        Command::ActivateVoiceProfile {
            profile,
            expected_project_revision,
        } => voice::activate_profile(project, profile, expected_project_revision),
        Command::RecordValidationReceipt { receipt } => receipts::record(project, receipt),
        Command::OpenTextRun { run } => run::open(project, run),
        Command::SetProjectMetadata {
            title,
            display_label,
            summary,
        } => {
            let (title, display_label, summary) =
                crate::domain::validation::normalize_project_metadata(
                    &title,
                    display_label.as_deref(),
                    summary.as_deref(),
                )?;
            if project.title == title
                && project.display_label == display_label
                && project.summary == summary
            {
                return Ok(Vec::new());
            }
            Ok(vec![Event::ProjectMetadataSet {
                title,
                display_label,
                summary,
            }])
        }
    }
}
