//! Construction, publication, and current-source verification of NarrativePackage.

use super::super::{ensure_atomic_file_target, resolve_regular_file_within_root, Engine};
use crate::model::workflow::{Manifest, ProjectStatus, State};
use narrative_courier_contracts::{
    validate_narrative_package, ArtifactRef, CanonicalNarrative, NarrativeAuthority,
    NarrativePackage, StsDomainRef, NARRATIVE_PACKAGE_SCHEMA_VERSION,
};
use narrative_courier_runtime::store::{atomic_write, sha256_bytes};
use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

impl Engine {
    pub(in crate::engine) fn narrative_package_path(project: &Path) -> PathBuf {
        project.join("narrative-package.json")
    }

    pub(super) fn package_artifact_ref(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
        kind: &str,
        relative: &str,
        mime: &str,
    ) -> Result<ArtifactRef> {
        self.verify_durable_output(project, state, manifest, relative)?;
        let producer = manifest
            .nodes
            .iter()
            .find(|node| node.output.as_deref() == Some(relative))
            .ok_or_else(|| Error::Contract("package artifact producer is missing".into()))?;
        let digest = state
            .nodes
            .get(&producer.id)
            .and_then(|record| record.artifact_sha256.as_ref())
            .ok_or_else(|| Error::Contract("package artifact digest is missing".into()))?;
        Ok(ArtifactRef {
            kind: kind.into(),
            path: relative.into(),
            sha256: digest.clone(),
            mime: Some(mime.into()),
        })
    }

    fn build_narrative_package(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
    ) -> Result<NarrativePackage> {
        self.verify_source_snapshot(project, state)?;
        let snapshot = state.source_snapshot.as_ref().ok_or_else(|| {
            Error::Contract("source snapshot missing; STEP 0 was not committed".into())
        })?;
        let scene_map = self.load_scene_map(project, state, manifest)?;
        let map_bytes = fs::read(resolve_regular_file_within_root(
            project,
            "output/scene-map.json",
        )?)?;
        let captured_map: narrative_courier_contracts::NarrativeSceneMap =
            serde_json::from_slice(&map_bytes)?;
        if captured_map != scene_map {
            return Err(Error::Contract(
                "scene map changed while building the package".into(),
            ));
        }
        let package = NarrativePackage {
            schema_version: NARRATIVE_PACKAGE_SCHEMA_VERSION,
            package_id: String::new(),
            narrative_id: state.project_name.clone(),
            sts_domain: StsDomainRef {
                id: "sts".into(),
                version: state.domain.version.clone(),
                bundle_sha256: state.domain.sha256.clone(),
            },
            source_snapshot_sha256: hash_jcs(snapshot)?,
            canonical_narrative: CanonicalNarrative {
                kind: "SCREENPLAY".into(),
                artifact: self.package_artifact_ref(
                    project,
                    state,
                    manifest,
                    "screenplay",
                    "output/final_screenplay.md",
                    "text/markdown",
                )?,
            },
            authority: NarrativeAuthority {
                source_ir: self.package_artifact_ref(
                    project,
                    state,
                    manifest,
                    "source_ir",
                    "output/step_00_source_ir.md",
                    "text/markdown",
                )?,
                narrative_ir: self.package_artifact_ref(
                    project,
                    state,
                    manifest,
                    "narrative_ir",
                    "output/step_02_narrative_map.md",
                    "text/markdown",
                )?,
                execution_constraints: self.package_artifact_ref(
                    project,
                    state,
                    manifest,
                    "execution_constraints",
                    "output/step_01_screenplay_contract.md",
                    "text/markdown",
                )?,
                character_states: self.package_artifact_ref(
                    project,
                    state,
                    manifest,
                    "character_states",
                    "output/step_04_character_states.md",
                    "text/markdown",
                )?,
                scene_ir: self.package_artifact_ref(
                    project,
                    state,
                    manifest,
                    "scene_ir",
                    "output/step_06_scene_blueprint.md",
                    "text/markdown",
                )?,
                verification: self.package_artifact_ref(
                    project,
                    state,
                    manifest,
                    "verification",
                    "output/step_08_verification.md",
                    "text/markdown",
                )?,
            },
            scene_map: ArtifactRef {
                kind: "narrative_scene_map_v2".into(),
                path: "output/scene-map.json".into(),
                sha256: sha256_bytes(&map_bytes),
                mime: Some("application/json".into()),
            },
        };
        let mut package = package;
        package.package_id = package.derived_id()?;
        validate_narrative_package(&package)?;
        if scene_map.narrative_id != package.narrative_id {
            return Err(Error::Contract(
                "NarrativeSceneMap is not bound to NarrativePackage".into(),
            ));
        }
        Ok(package)
    }

    pub(in crate::engine) fn verify_narrative_package_at(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
    ) -> Result<NarrativePackage> {
        let path = resolve_regular_file_within_root(project, "narrative-package.json")?;
        let package: NarrativePackage = serde_json::from_slice(&fs::read(&path)?)?;
        validate_narrative_package(&package)?;
        let expected = self.build_narrative_package(project, state, manifest)?;
        if package != expected {
            return Err(Error::Contract(
                "NarrativePackage is stale or does not match final STS authority".into(),
            ));
        }
        Ok(package)
    }

    pub(in crate::engine) fn publish_narrative_package(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
    ) -> Result<NarrativePackage> {
        let package = self.build_narrative_package(project, state, manifest)?;
        let mut bytes = serde_json::to_vec_pretty(&package)?;
        bytes.push(b'\n');
        let target = Self::narrative_package_path(project);
        ensure_atomic_file_target(&target, "NarrativePackage")?;
        atomic_write(&target, &bytes)?;
        let written: NarrativePackage = serde_json::from_slice(&fs::read(
            resolve_regular_file_within_root(project, "narrative-package.json")?,
        )?)?;
        if written != package {
            return Err(Error::Contract(
                "NarrativePackage publish verification failed".into(),
            ));
        }
        Ok(package)
    }

    pub fn narrative_package(&self, name: &str) -> Result<NarrativePackage> {
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let state = self.load_state(&project, &manifest)?;
        if state.status != ProjectStatus::Completed {
            return Err(Error::Contract(
                "NarrativePackage is available only after final Human release".into(),
            ));
        }
        self.verify_release_bundle_at(&project, &state, &manifest)?;
        self.verify_narrative_package_at(&project, &state, &manifest)
    }
}
