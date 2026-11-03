//! Canonical command transaction: validate, append journal, persist snapshot, update index.

use super::journal::{append_envelopes, journal_entries};
use super::lock::acquire_lock;
use super::{now_ms, validate_project_id, CommitResult, Workspace, SCHEMA_VERSION};
use crate::admission::snapshot::ensure_snapshot_conformance;
use crate::domain::validation::validate_id;
use crate::domain::{decide, evolve, Command, Event, EventEnvelope};
use crate::error::{Error, Result};
use crate::model::{Candidate, CandidateStatus, ChangeOperation, ChangeSet, ChangeSetStatus};

impl Workspace {
    pub fn transact(&self, id: &str, command_id: &str, command: Command) -> Result<CommitResult> {
        self.transact_inner(id, None, command_id, command)
    }

    pub(crate) fn transact_if_project_revision(
        &self,
        id: &str,
        expected_project_revision: u64,
        command_id: &str,
        command: Command,
    ) -> Result<CommitResult> {
        self.transact_inner(id, Some(expected_project_revision), command_id, command)
    }

    fn transact_inner(
        &self,
        id: &str,
        expected_project_revision: Option<u64>,
        command_id: &str,
        command: Command,
    ) -> Result<CommitResult> {
        validate_project_id(id)?;
        validate_id("command id", command_id)?;
        let _lock = acquire_lock(&self.project_dir(id).join("lock"))?;
        let (event_count, mut project) = self.load_project_with_count(id)?;
        if let Command::Authoring { mutation } = &command {
            if mutation.command_id != command_id {
                return Err(Error::InvalidInput(
                    "authoring command identity mismatch".into(),
                ));
            }
            if project.applied_commands.contains_key(command_id)
                && project.authoring.requests.get(command_id)
                    != Some(&crate::authoring::digest(mutation))
            {
                return Err(Error::Conflict(
                    "command_id was already used for a different payload".into(),
                ));
            }
        }
        if let Command::OpenTextRun { run } = &command {
            if project.applied_commands.contains_key(command_id) {
                let mut matching_open = false;
                for entry in journal_entries(&self.events_path(id))? {
                    let envelope = entry?;
                    if envelope.command_id == command_id {
                        matching_open = matches!(
                            envelope.event,
                            Event::TextRunOpened { run: recorded } if recorded == *run
                        );
                        break;
                    }
                }
                if !matching_open {
                    return Err(Error::Conflict(
                        "run.open command_id was already used for a different payload".into(),
                    ));
                }
            }
        }
        if let Command::PrepareSceneCandidate {
            candidate,
            scene_id,
            expected_scene_revision,
            changeset_id,
        } = &command
        {
            if project.applied_commands.contains_key(command_id) {
                let mut matching_events = Vec::new();
                for entry in journal_entries(&self.events_path(id))? {
                    let envelope = entry?;
                    if envelope.command_id == command_id {
                        matching_events.push(envelope.event);
                    }
                }
                if !prepare_candidate_payload_matches(
                    &matching_events,
                    candidate,
                    scene_id,
                    *expected_scene_revision,
                    changeset_id,
                ) {
                    return Err(Error::Conflict(
                        "scene.prepare_candidate command_id was already used for a different payload".into(),
                    ));
                }
            }
        }
        // An already committed command remains idempotent after its base revision advances.
        if let Some(revision) = project.applied_commands.get(command_id) {
            self.persist_committed_projection(event_count, &project)?;
            return Ok(CommitResult {
                project_id: id.into(),
                revision: *revision,
                event_count: 0,
                idempotent: true,
            });
        }
        if let Some(expected) = expected_project_revision {
            if project.revision != expected {
                return Err(Error::Conflict(format!(
                    "project revision mismatch: expected {expected}, actual {}",
                    project.revision
                )));
            }
        }
        let events = decide(&project, command)?;
        if events.is_empty() {
            project
                .applied_commands
                .insert(command_id.into(), project.revision);
            self.persist_committed_projection(event_count, &project)?;
            return Ok(CommitResult {
                project_id: id.into(),
                revision: project.revision,
                event_count: 0,
                idempotent: true,
            });
        }
        let envelopes: Vec<_> = events
            .into_iter()
            .enumerate()
            .map(|(offset, event)| EventEnvelope {
                schema_version: SCHEMA_VERSION,
                seq: event_count + offset as u64 + 1,
                command_id: command_id.into(),
                timestamp_ms: now_ms(),
                event,
            })
            .collect();
        // Complete deterministic validation before the irreversible journal append.
        for envelope in &envelopes {
            project = evolve(Some(project), &envelope.event)?;
            project
                .applied_commands
                .insert(envelope.command_id.clone(), project.revision);
        }
        let new_count = event_count + envelopes.len() as u64;
        ensure_snapshot_conformance(&project, new_count)?;
        append_envelopes(&self.events_path(id), &envelopes)?;
        self.persist_committed_projection(new_count, &project)?;
        Ok(CommitResult {
            project_id: id.into(),
            revision: project.revision,
            event_count: envelopes.len(),
            idempotent: false,
        })
    }
}

fn prepare_candidate_payload_matches(
    events: &[Event],
    expected_candidate: &Candidate,
    scene_id: &str,
    expected_scene_revision: u64,
    changeset_id: &str,
) -> bool {
    let [Event::CandidateAdded {
        candidate: recorded_candidate,
    }, Event::ChangeSetAdded {
        changeset: recorded_changeset,
    }] = events
    else {
        return false;
    };

    let mut expected_candidate = expected_candidate.clone();
    expected_candidate.receipt.created_at_ms = recorded_candidate.receipt.created_at_ms;
    let expected_changeset = ChangeSet {
        id: changeset_id.into(),
        candidate_id: expected_candidate.id.clone(),
        source_revision: expected_candidate.source_revision,
        operations: vec![ChangeOperation::ReplaceSceneText {
            scene_id: scene_id.into(),
            expected_scene_revision,
            text: expected_candidate.content.clone(),
        }],
        status: ChangeSetStatus::Pending,
    };
    recorded_candidate == &expected_candidate
        && recorded_candidate.status == CandidateStatus::Pending
        && recorded_changeset == &expected_changeset
}
