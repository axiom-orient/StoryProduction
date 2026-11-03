//! Completed-state NvcReleaseBundle publication and cross-artifact verification.

use super::super::{ensure_atomic_file_target, resolve_regular_file_within_root, Engine};
use crate::model::workflow::{Manifest, NodeKind, ProjectStatus, State};
use crate::NodeStatus;
use narrative_courier_contracts::{
    validate_narrative_package, validate_nvc_release_bundle, ArtifactRef, NarrativePackage,
    NvcReleaseBundle, NVC_RELEASE_BUNDLE_SCHEMA_VERSION,
};
use narrative_courier_runtime::store::{atomic_write, sha256_bytes};
use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

impl Engine {
    pub(in crate::engine) fn release_bundle_path(project: &Path) -> PathBuf {
        project.join("release-bundle.json")
    }

    pub(in crate::engine) fn export_release_bundle(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
        package: &NarrativePackage,
    ) -> Result<NvcReleaseBundle> {
        require_completed_release(state, manifest)?;
        let actual_package = self.verify_narrative_package_at(project, state, manifest)?;
        if actual_package != *package {
            return Err(Error::Contract(
                "release package differs from current STS authority".into(),
            ));
        }
        let package_bytes = fs::read(resolve_regular_file_within_root(
            project,
            "narrative-package.json",
        )?)?;
        let captured_package: NarrativePackage = serde_json::from_slice(&package_bytes)?;
        validate_narrative_package(&captured_package)?;
        if captured_package != *package {
            return Err(Error::Contract(
                "release package changed before bundle capture".into(),
            ));
        }
        let map_bytes = fs::read(resolve_regular_file_within_root(
            project,
            "output/scene-map.json",
        )?)?;
        let map_hash = sha256_bytes(&map_bytes);
        if map_hash != package.scene_map.sha256 {
            return Err(Error::Contract(
                "release scene map differs from package proof".into(),
            ));
        }
        let state_bytes = fs::read(resolve_regular_file_within_root(project, "state.json")?)?;
        let persisted_state: State = serde_json::from_slice(&state_bytes)?;
        require_completed_release(&persisted_state, manifest)?;
        if persisted_state != *state {
            return Err(Error::Contract(
                "release state differs from committed state bytes".into(),
            ));
        }
        let package_hash = sha256_bytes(&package_bytes);
        let bundle = NvcReleaseBundle {
            schema_version: NVC_RELEASE_BUNDLE_SCHEMA_VERSION,
            bundle_id: String::new(),
            project_name: state.project_name.clone(),
            status: "COMPLETED".into(),
            completed_state: ArtifactRef {
                kind: "completed_state".into(),
                path: "state.json".into(),
                sha256: sha256_bytes(&state_bytes),
                mime: Some("application/json".into()),
            },
            narrative_package: ArtifactRef {
                kind: "narrative_package_v2".into(),
                path: "narrative-package.json".into(),
                sha256: package_hash.clone(),
                mime: Some("application/json".into()),
            },
            scene_map: ArtifactRef {
                kind: "narrative_scene_map_v2".into(),
                path: "output/scene-map.json".into(),
                sha256: map_hash,
                mime: Some("application/json".into()),
            },
            package_id: package.package_id.clone(),
            package_sha256: package_hash.clone(),
            source_snapshot_sha256: hash_jcs(
                state
                    .source_snapshot
                    .as_ref()
                    .ok_or_else(|| Error::Contract("source snapshot missing".into()))?,
            )?,
        };
        let mut bundle = bundle;
        bundle.bundle_id = bundle.derived_id()?;
        validate_nvc_release_bundle(&bundle)?;
        let mut bytes = serde_json::to_vec_pretty(&bundle)?;
        bytes.push(b'\n');
        let target = Self::release_bundle_path(project);
        ensure_atomic_file_target(&target, "NvcReleaseBundle")?;
        atomic_write(&target, &bytes)?;
        let written: NvcReleaseBundle = serde_json::from_slice(&fs::read(
            resolve_regular_file_within_root(project, "release-bundle.json")?,
        )?)?;
        validate_nvc_release_bundle(&written)?;
        if written != bundle {
            return Err(Error::Contract(
                "NvcReleaseBundle publish verification failed".into(),
            ));
        }
        self.verify_release_bundle_at(project, state, manifest)?;
        Ok(bundle)
    }

    pub(in crate::engine) fn verify_release_bundle_at(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
    ) -> Result<NvcReleaseBundle> {
        require_completed_release(state, manifest)?;
        let path = resolve_regular_file_within_root(project, "release-bundle.json")?;
        let bundle: NvcReleaseBundle = serde_json::from_slice(&fs::read(&path)?)?;
        validate_nvc_release_bundle(&bundle)?;
        if bundle.project_name != state.project_name || bundle.status != "COMPLETED" {
            return Err(Error::Contract(
                "NvcReleaseBundle project/status proof is invalid".into(),
            ));
        }
        if bundle.completed_state.path != "state.json"
            || bundle.narrative_package.path != "narrative-package.json"
            || bundle.scene_map.path != "output/scene-map.json"
        {
            return Err(Error::Contract(
                "NvcReleaseBundle contains a non-canonical artifact path".into(),
            ));
        }
        let state_path = resolve_regular_file_within_root(project, "state.json")?;
        let package_path = resolve_regular_file_within_root(project, "narrative-package.json")?;
        let scene_map_path = resolve_regular_file_within_root(project, "output/scene-map.json")?;
        let state_bytes = fs::read(&state_path)?;
        let persisted_state: State = serde_json::from_slice(&state_bytes)?;
        require_completed_release(&persisted_state, manifest)?;
        if persisted_state != *state || bundle.completed_state.sha256 != sha256_bytes(&state_bytes)
        {
            return Err(Error::Contract(
                "NvcReleaseBundle completed state hash or identity drift".into(),
            ));
        }
        let package = self.verify_narrative_package_at(project, state, manifest)?;
        let package_bytes = fs::read(&package_path)?;
        let captured_package: NarrativePackage = serde_json::from_slice(&package_bytes)?;
        let package_hash = sha256_bytes(&package_bytes);
        if captured_package != package
            || package_hash != bundle.package_sha256
            || package_hash != bundle.narrative_package.sha256
            || package.package_id != bundle.package_id
            || package.source_snapshot_sha256 != bundle.source_snapshot_sha256
        {
            return Err(Error::Contract(
                "NvcReleaseBundle package proof is stale or mismatched".into(),
            ));
        }
        if bundle.scene_map.sha256 != package.scene_map.sha256
            || bundle.scene_map.sha256 != sha256_bytes(&fs::read(&scene_map_path)?)
        {
            return Err(Error::Contract(
                "NvcReleaseBundle scene map hash drift".into(),
            ));
        }
        Ok(bundle)
    }

    pub fn release_bundle(&self, name: &str) -> Result<NvcReleaseBundle> {
        let manifest = self.load_manifest()?;
        let project = self.project_dir(name)?;
        let state = self.load_state(&project, &manifest)?;
        if state.status != ProjectStatus::Completed {
            return Err(Error::Contract(
                "NvcReleaseBundle is available only after final Human release".into(),
            ));
        }
        self.verify_release_bundle_at(&project, &state, &manifest)
    }
}

fn require_completed_release(state: &State, manifest: &Manifest) -> Result<()> {
    if state.status != ProjectStatus::Completed
        || !state.nodes.get("final_release").is_some_and(|record| {
            record.status == NodeStatus::Completed && record.decision.as_deref() == Some("approve")
        })
        || state
            .nodes
            .values()
            .any(|record| record.status != NodeStatus::Completed)
        || manifest
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Human)
            .any(|node| {
                !state
                    .nodes
                    .get(&node.id)
                    .is_some_and(|record| record.decision.as_deref() == Some("approve"))
            })
    {
        return Err(Error::Contract(
            "release bundle requires a committed Completed workflow and approved final_release"
                .into(),
        ));
    }
    Ok(())
}
