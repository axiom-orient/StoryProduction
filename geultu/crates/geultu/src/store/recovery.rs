//! Snapshot reconciliation and explicit project/workspace recovery.

use super::file_io::{atomic_json, read_json};
use super::journal::{
    ensure_created_project, ensure_project_identity, journal_entries, replay_journal,
};
use super::lock::acquire_lock;
use super::{validate_project_id, Snapshot, Workspace, STATE_DIR};
use crate::admission::snapshot::ensure_snapshot_conformance;
use crate::domain::evolve;
use crate::error::{Error, Result};
use crate::model::{Project, ProjectSummary, WorkspaceMeta};
use std::fs::{self, File};

impl Workspace {
    /// Reconcile durable projections before acknowledging an already committed command.
    pub(super) fn persist_committed_projection(
        &self,
        event_count: u64,
        project: &Project,
    ) -> Result<()> {
        let reconcile = || -> Result<()> {
            let journal = self.events_path(&project.id);
            File::open(&journal)?.sync_all()?;
            File::open(self.project_dir(&project.id))?.sync_all()?;
            self.write_snapshot(&project.id, event_count, project)?;
            self.update_project_index(project)
        };
        reconcile().map_err(|error| Error::CommittedButSnapshotFailed {
            revision: project.revision,
            message: error.to_string(),
        })
    }

    pub fn recover_project(&self, id: &str, remove_lock: bool) -> Result<Project> {
        validate_project_id(id)?;
        let lock_path = self.project_dir(id).join("lock");
        if remove_lock && lock_path.exists() {
            fs::remove_file(&lock_path)?;
        }
        let _lock = acquire_lock(&lock_path)?;
        let (event_count, project) = replay_journal(&self.events_path(id))?;
        ensure_project_identity(id, &project)?;
        self.write_snapshot(id, event_count, &project)?;
        self.update_project_index(&project)?;
        Ok(project)
    }

    pub fn recover_index(&self) -> Result<WorkspaceMeta> {
        let _lock = acquire_lock(&self.root.join(STATE_DIR).join("workspace.lock"))?;
        let mut meta = self.metadata()?;
        meta.projects.clear();
        let projects_dir = self.root.join(STATE_DIR).join("projects");
        for entry in fs::read_dir(projects_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().into_owned();
            let project = self.load_project(&id)?;
            meta.projects.insert(
                id.clone(),
                ProjectSummary {
                    id,
                    title: project.title,
                    display_label: project.display_label,
                    summary: project.summary,
                    revision: project.revision,
                },
            );
        }
        atomic_json(&self.meta_path(), &meta)?;
        Ok(meta)
    }

    /// Writes a snapshot after enforcing the project-state contract
    /// (H6 SSOT binding). A non-conforming state is a fail-closed error and
    /// is never silently repaired.
    pub(super) fn write_snapshot(
        &self,
        id: &str,
        event_count: u64,
        project: &Project,
    ) -> Result<()> {
        ensure_snapshot_conformance(project, event_count)?;
        atomic_json(
            &self.snapshot_path(id),
            &Snapshot {
                event_count,
                project,
            },
        )
    }

    pub(super) fn load_project_with_count(&self, id: &str) -> Result<(u64, Project)> {
        let events_path = self.events_path(id);
        if !events_path.exists() {
            return Err(Error::NotFound(format!("project '{id}' was not found")));
        }
        let snapshot_path = self.snapshot_path(id);
        let (mut count, mut project) = if snapshot_path.exists() {
            let snapshot: Snapshot = read_json(&snapshot_path)?;
            ensure_snapshot_conformance(&snapshot.project, snapshot.event_count).map_err(
                |error| {
                    Error::Corrupt(format!(
                        "invalid snapshot at {}: {error}",
                        snapshot_path.display()
                    ))
                },
            )?;
            (snapshot.event_count, Some(snapshot.project))
        } else {
            (0, None)
        };
        let snapshot_count = count;
        let mut journal_count = 0u64;
        for entry in journal_entries(&events_path)? {
            let envelope = entry?;
            if envelope.seq == 1 {
                ensure_created_project(id, &envelope.event)?;
            }
            journal_count = envelope.seq;
            if envelope.seq <= count {
                continue;
            }
            project = Some(evolve(project, &envelope.event)?);
            if let Some(state) = project.as_mut() {
                state
                    .applied_commands
                    .insert(envelope.command_id, state.revision);
            }
            count = envelope.seq;
        }
        if snapshot_count > journal_count {
            return Err(Error::Corrupt(format!(
                "snapshot event_count {snapshot_count} exceeds journal event_count {journal_count}"
            )));
        }
        let project = project.ok_or_else(|| Error::Corrupt("project journal is empty".into()))?;
        ensure_project_identity(id, &project)?;
        ensure_snapshot_conformance(&project, count)?;
        Ok((count, project))
    }

    pub(super) fn update_project_index(&self, project: &Project) -> Result<()> {
        let _lock = acquire_lock(&self.root.join(STATE_DIR).join("workspace.lock"))?;
        let mut meta = self.metadata()?;
        meta.projects.insert(
            project.id.clone(),
            ProjectSummary {
                id: project.id.clone(),
                title: project.title.clone(),
                display_label: project.display_label.clone(),
                summary: project.summary.clone(),
                revision: project.revision,
            },
        );
        atomic_json(&self.meta_path(), &meta)
    }
}
