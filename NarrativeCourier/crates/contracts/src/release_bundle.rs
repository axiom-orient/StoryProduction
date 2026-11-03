//! Completed NVC release bundle DTO and body-bound identity checks.

use narrative_courier_runtime::{Error, Result};
use serde::{Deserialize, Serialize};

use super::artifact::{validate_artifact_ref, ArtifactRef};
use super::common::{body_identity, nonempty, sha, valid_prefixed_sha256};
use super::NVC_RELEASE_BUNDLE_SCHEMA_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NvcReleaseBundle {
    pub schema_version: u32,
    pub bundle_id: String,
    pub project_name: String,
    pub status: String,
    pub completed_state: ArtifactRef,
    pub narrative_package: ArtifactRef,
    pub scene_map: ArtifactRef,
    pub package_id: String,
    pub package_sha256: String,
    pub source_snapshot_sha256: String,
}

impl NvcReleaseBundle {
    pub fn derived_id(&self) -> Result<String> {
        body_identity(self, "bundle_id", "nvcb_")
    }
}

pub fn validate_nvc_release_bundle(v: &NvcReleaseBundle) -> Result<()> {
    if v.schema_version != NVC_RELEASE_BUNDLE_SCHEMA_VERSION {
        return Err(Error::Contract(
            "NvcReleaseBundle schema_version must be 1".into(),
        ));
    }
    if !valid_prefixed_sha256(&v.bundle_id, "nvcb_") {
        return Err(Error::Contract(
            "NvcReleaseBundle bundle_id must be nvcb_<lowercase SHA-256>".into(),
        ));
    }
    nonempty(&v.project_name, "project_name")?;
    if v.status != "COMPLETED" {
        return Err(Error::Contract(
            "NvcReleaseBundle status must be COMPLETED".into(),
        ));
    }
    for (field, artifact) in [
        ("completed_state", &v.completed_state),
        ("narrative_package", &v.narrative_package),
        ("scene_map", &v.scene_map),
    ] {
        validate_artifact_ref(artifact, field)?;
    }
    if v.scene_map.kind != "narrative_scene_map_v2" {
        return Err(Error::Contract(
            "NvcReleaseBundle scene_map.kind must be narrative_scene_map_v2".into(),
        ));
    }
    if v.completed_state.path != "state.json" {
        return Err(Error::Contract(
            "NvcReleaseBundle completed_state must reference state.json".into(),
        ));
    }
    if v.narrative_package.kind != "narrative_package_v2"
        || v.narrative_package.path != "narrative-package.json"
    {
        return Err(Error::Contract(
            "NvcReleaseBundle narrative_package must reference narrative-package.json as narrative_package_v2".into(),
        ));
    }
    if v.scene_map.path != "output/scene-map.json" {
        return Err(Error::Contract(
            "NvcReleaseBundle scene_map must reference output/scene-map.json".into(),
        ));
    }
    if !valid_prefixed_sha256(&v.package_id, "nvc2_") {
        return Err(Error::Contract(
            "NvcReleaseBundle package_id must be nvc2_<lowercase SHA-256>".into(),
        ));
    }
    sha(&v.package_sha256, "package_sha256")?;
    sha(&v.source_snapshot_sha256, "source_snapshot_sha256")?;

    if v.narrative_package.sha256 != v.package_sha256 {
        return Err(Error::Contract(
            "NvcReleaseBundle package SHA differs from its artifact reference".into(),
        ));
    }
    let expected = v.derived_id()?;
    if v.bundle_id != expected {
        return Err(Error::Contract(
            "NvcReleaseBundle bundle_id is not bound to its body".into(),
        ));
    }
    Ok(())
}
