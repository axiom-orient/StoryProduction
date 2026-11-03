//! Publication and freshness checks for the STS-owned NarrativeSceneMap.

use super::super::{
    ensure_atomic_file_target, resolve_directory_within_root, resolve_regular_file_within_root,
    Engine,
};
use crate::model::workflow::{Manifest, State};
use narrative_courier_contracts::{validate_narrative_scene_map, NarrativeSceneMap};
use narrative_courier_runtime::store::{atomic_write, sha256_bytes};
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

impl Engine {
    pub(in crate::engine) fn scene_map_path(project: &Path) -> PathBuf {
        project.join("output/scene-map.json")
    }

    pub(in crate::engine) fn publish_scene_map(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
    ) -> Result<NarrativeSceneMap> {
        let output_dir = resolve_directory_within_root(project, "output")?;
        let map = self.build_current_scene_map(project, state, manifest)?;
        let mut bytes = serde_json::to_vec_pretty(&map)?;
        bytes.push(b'\n');
        let target = output_dir.join("scene-map.json");
        ensure_atomic_file_target(&target, "NarrativeSceneMap")?;
        atomic_write(&target, &bytes)?;
        let written: NarrativeSceneMap = serde_json::from_slice(&fs::read(
            resolve_regular_file_within_root(project, "output/scene-map.json")?,
        )?)?;
        validate_narrative_scene_map(&written)?;
        if written != map {
            return Err(Error::Contract(
                "NarrativeSceneMap publish verification failed".into(),
            ));
        }
        Ok(map)
    }

    pub(super) fn load_scene_map(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
    ) -> Result<NarrativeSceneMap> {
        let path = resolve_regular_file_within_root(project, "output/scene-map.json")?;
        let map: NarrativeSceneMap = serde_json::from_slice(&fs::read(&path)?)?;
        validate_narrative_scene_map(&map)?;
        let expected = self.build_current_scene_map(project, state, manifest)?;
        if map != expected {
            return Err(Error::Contract(
                "NarrativeSceneMap is stale or does not match STS authority".into(),
            ));
        }
        Ok(map)
    }
    fn build_current_scene_map(
        &self,
        project: &Path,
        state: &State,
        manifest: &Manifest,
    ) -> Result<NarrativeSceneMap> {
        let captured = |relative: &str| -> Result<Vec<u8>> {
            let path = self.verify_durable_output(project, state, manifest, relative)?;
            let bytes = fs::read(&path)?;
            let producer = manifest
                .nodes
                .iter()
                .find(|node| node.output.as_deref() == Some(relative))
                .ok_or_else(|| Error::Contract("scene-map source producer is missing".into()))?;
            let record = state
                .nodes
                .get(&producer.id)
                .ok_or_else(|| Error::Contract("scene-map source state is missing".into()))?;
            if record.artifact_sha256.as_deref() != Some(sha256_bytes(&bytes).as_str()) {
                return Err(Error::Contract(format!(
                    "scene-map source bytes changed: {relative}"
                )));
            }
            Ok(bytes)
        };
        let screenplay = captured("output/final_screenplay.md")?;
        let scene_ir = captured("output/step_06_scene_blueprint.md")?;
        crate::scene_map::build_scene_map(&state.project_name, &screenplay, &scene_ir)
    }
}
