//! Repair-only resume for paused projects and already-approved releases.

use super::super::{push_event, Engine};
use crate::model::workflow::ProjectStatus;
use narrative_courier_runtime::store::lock_project;
use narrative_courier_runtime::Result;
use std::fs;

impl Engine {
    pub fn resume(&self, name: &str) -> Result<ProjectStatus> {
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let _lock = lock_project(&project)?;
        let mut state = self.load_state(&project, &manifest)?;
        if state.status == ProjectStatus::Completed {
            // Completed state bytes are already part of the public release proof.
            // Resume must neither rewrite them nor silently bless a corrupt receipt.
            self.verify_source_snapshot(&project, &state)?;
            let package = self.verify_narrative_package_at(&project, &state, &manifest)?;
            match fs::symlink_metadata(Self::release_bundle_path(&project)) {
                Ok(_) => {
                    self.verify_release_bundle_at(&project, &state, &manifest)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // Explicit recovery of a missing derived bundle, never a new Human decision.
                    self.export_release_bundle(&project, &state, &manifest, &package)?;
                    self.verify_release_bundle_at(&project, &state, &manifest)?;
                }
                Err(error) => return Err(error.into()),
            }
            return Ok(state.status);
        }
        if state.status == ProjectStatus::Paused {
            state.status = ProjectStatus::InProgress;
            push_event(&mut state, "resume", []);
        }
        self.update_status(&mut state, &manifest);
        self.save_state(&project, &mut state)?;
        Ok(state.status)
    }
}
