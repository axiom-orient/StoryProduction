//! Workspace-owned project creation and canonical project read projection.

use super::file_io::ensure_directory;
use super::journal::{append_envelopes, journal_entries};
use super::lock::acquire_lock;
use super::{now_ms, validate_project_id, CommitResult, Workspace, SCHEMA_VERSION};
use crate::admission::snapshot::ensure_snapshot_conformance;
use crate::domain::validation::{normalize_project_metadata, validate_id};
use crate::domain::{evolve, Event, EventEnvelope};
use crate::error::{Error, Result};
use crate::model::{Project, ProjectSummary};
use std::fs::{self, File};

impl Workspace {
    pub fn create_project(&self, id: &str, title: &str, command_id: &str) -> Result<CommitResult> {
        self.create_project_with_metadata(id, title, None, None, command_id)
    }

    pub(crate) fn create_project_with_metadata(
        &self,
        id: &str,
        title: &str,
        display_label: Option<&str>,
        summary: Option<&str>,
        command_id: &str,
    ) -> Result<CommitResult> {
        validate_project_id(id)?;
        validate_id("command id", command_id)?;
        let (title, display_label, summary) =
            normalize_project_metadata(title, display_label, summary)?;
        let project_dir = self.project_dir(id);
        ensure_directory(&project_dir)?;
        let projects_root = project_dir
            .parent()
            .ok_or_else(|| Error::InvalidInput("project directory has no parent".into()))?;
        File::open(projects_root)?.sync_all()?;
        let _project_lock = acquire_lock(&project_dir.join("lock"))?;
        if self.events_path(id).try_exists()? {
            let first = journal_entries(&self.events_path(id))?
                .next()
                .ok_or_else(|| Error::Corrupt("project journal is empty".into()))??;
            if first.command_id != command_id
                || !matches!(
                    first.event,
                    Event::ProjectCreated {
                        id: ref created_id,
                        title: ref created_title,
                        display_label: ref created_label,
                        summary: ref created_summary,
                    }
                        if created_id == id
                            && created_title == &title
                            && created_label == &display_label
                            && created_summary == &summary
                )
            {
                return Err(Error::Conflict(
                    "project.create identity differs from the original creation request".into(),
                ));
            }
            let (event_count, existing) = self.load_project_with_count(id)?;
            let revision = *existing.applied_commands.get(command_id).ok_or_else(|| {
                Error::Corrupt("creation command is missing from project state".into())
            })?;
            self.persist_committed_projection(event_count, &existing)?;
            return Ok(CommitResult {
                project_id: id.into(),
                revision,
                event_count: 0,
                idempotent: true,
            });
        }
        // An interrupted directory reservation has no authoritative state. Only
        // an empty directory with this call's lock can be initialized; preserve
        // unknown files and require explicit inspection instead of overwriting.
        for entry in fs::read_dir(&project_dir)? {
            if entry?.file_name() != "lock" {
                return Err(Error::Conflict(
                    "uninitialized project directory contains existing data".into(),
                ));
            }
        }
        let envelope = EventEnvelope {
            schema_version: SCHEMA_VERSION,
            seq: 1,
            command_id: command_id.into(),
            timestamp_ms: now_ms(),
            event: Event::ProjectCreated {
                id: id.into(),
                title,
                display_label,
                summary,
            },
        };
        let mut project = evolve(None, &envelope.event)?;
        project
            .applied_commands
            .insert(command_id.into(), project.revision);
        ensure_snapshot_conformance(&project, 1)?;
        append_envelopes(&self.events_path(id), std::slice::from_ref(&envelope))?;
        self.persist_committed_projection(1, &project)?;
        Ok(CommitResult {
            project_id: id.into(),
            revision: project.revision,
            event_count: 1,
            idempotent: false,
        })
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectSummary>> {
        let meta = self.metadata()?;
        Ok(meta.projects.into_values().collect())
    }

    pub fn load_project(&self, id: &str) -> Result<Project> {
        validate_project_id(id)?;
        let (_, project) = self.load_project_with_count(id)?;
        Ok(project)
    }
}
