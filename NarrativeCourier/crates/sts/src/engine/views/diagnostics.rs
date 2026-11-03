//! Read-only integrity diagnosis of STS-owned project state and release artifacts.

use super::super::Engine;
use crate::model::views::DoctorReport;
use crate::model::workflow::{Manifest, NodeKind, NodeStatus, ProjectStatus, State};
use narrative_courier_runtime::Result;
use std::fs;
use std::path::Path;

impl Engine {
    pub fn doctor(&self, name: &str) -> Result<DoctorReport> {
        let manifest = match self.load_manifest() {
            Ok(v) => v,
            Err(e) => {
                return Ok(DoctorReport {
                    ok: false,
                    errors: vec![e.to_string()],
                    warnings: vec![],
                })
            }
        };
        let project = self.project_dir(name)?;
        let state = match self.load_state(&project, &manifest) {
            Ok(v) => v,
            Err(e) => {
                return Ok(DoctorReport {
                    ok: false,
                    errors: vec![e.to_string()],
                    warnings: vec![],
                })
            }
        };
        self.doctor_at(&project, &state, &manifest)
    }

    pub(in crate::engine::views) fn doctor_at(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
    ) -> Result<DoctorReport> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        if let Err(e) = self.load_config(project).and_then(|c| {
            self.resolve_synopsis(project, &c)?;
            self.resolve_references(project, &c)?;
            Ok(())
        }) {
            errors.push(e.to_string());
        }
        if let Err(error) = self.verify_source_snapshot(project, state) {
            errors.push(error.to_string());
        }
        for relative in state.feedback_sha256.keys() {
            if let Err(error) = self.verify_feedback(project, state, relative) {
                errors.push(error.to_string());
            }
        }
        for node in &manifest.nodes {
            let rec = &state.nodes[&node.id];
            if node.kind == NodeKind::Agent {
                let relative = node.output.as_deref().ok_or_else(|| {
                    narrative_courier_runtime::Error::Contract("Agent output is missing".into())
                })?;
                if rec.status == NodeStatus::Completed {
                    if let Err(error) =
                        self.verify_durable_output(project, state, manifest, relative)
                    {
                        errors.push(error.to_string());
                    }
                } else {
                    match fs::symlink_metadata(project.join(relative)) {
                        Ok(_) => {
                            match super::super::resolve_regular_file_within_root(project, relative)
                            {
                                Ok(_) => warnings.push(format!(
                                    "{}: {relative}",
                                    if rec.version == 0 {
                                        "orphan artifact before state commit"
                                    } else {
                                        "invalidated previous artifact still exists"
                                    }
                                )),
                                Err(error) => errors.push(error.to_string()),
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => errors.push(error.to_string()),
                    }
                }
            }
        }
        let package_path = Self::narrative_package_path(project);
        let scene_map_path = Self::scene_map_path(project);
        let release_bundle_path = Self::release_bundle_path(project);
        if state.status == ProjectStatus::Completed {
            if let Err(e) = self.verify_release_bundle_at(project, state, manifest) {
                errors.push(e.to_string());
            }
        } else {
            for (path, relative, warning) in [
                (
                    &package_path,
                    "narrative-package.json",
                    "orphan NarrativePackage before final state commit",
                ),
                (
                    &scene_map_path,
                    "output/scene-map.json",
                    "orphan NarrativeSceneMap before final state commit",
                ),
                (
                    &release_bundle_path,
                    "release-bundle.json",
                    "release bundle exists before Completed state",
                ),
            ] {
                match fs::symlink_metadata(path) {
                    Ok(_) => {
                        match super::super::resolve_regular_file_within_root(project, relative) {
                            Ok(_) => warnings.push(warning.into()),
                            Err(error) => errors.push(error.to_string()),
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => errors.push(error.to_string()),
                }
            }
        }
        Ok(DoctorReport {
            ok: errors.is_empty(),
            errors,
            warnings,
        })
    }
}
