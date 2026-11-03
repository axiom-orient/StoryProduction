//! STS-owned NarrativePackage DTO and its integrity invariants.

use narrative_courier_runtime::{Error, Result};
use serde::{Deserialize, Serialize};

use super::artifact::{
    validate_artifact_ref, ArtifactRef, CanonicalNarrative, NarrativeAuthority, StsDomainRef,
};
use super::common::{body_identity, nonempty, sha, valid_prefixed_sha256};
use super::NARRATIVE_PACKAGE_SCHEMA_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NarrativePackage {
    pub schema_version: u32,
    pub package_id: String,
    pub narrative_id: String,
    pub sts_domain: StsDomainRef,
    pub source_snapshot_sha256: String,
    pub canonical_narrative: CanonicalNarrative,
    pub authority: NarrativeAuthority,
    /// Structured scene authority produced from the final screenplay and
    /// scene blueprint. Consumers must use this artifact; reparsing the
    /// screenplay is not a valid main-edge operation.
    pub scene_map: ArtifactRef,
}

impl NarrativePackage {
    pub fn derived_id(&self) -> Result<String> {
        body_identity(self, "package_id", "nvc2_")
    }

    /// Immutable artifact inventory for contract and filesystem admission consumers.
    pub fn artifacts(&self) -> [(&'static str, &ArtifactRef); 8] {
        [
            (
                "canonical_narrative.artifact",
                &self.canonical_narrative.artifact,
            ),
            ("authority.source_ir", &self.authority.source_ir),
            ("authority.narrative_ir", &self.authority.narrative_ir),
            (
                "authority.execution_constraints",
                &self.authority.execution_constraints,
            ),
            (
                "authority.character_states",
                &self.authority.character_states,
            ),
            ("authority.scene_ir", &self.authority.scene_ir),
            ("authority.verification", &self.authority.verification),
            ("scene_map", &self.scene_map),
        ]
    }
}

pub fn validate_narrative_package(v: &NarrativePackage) -> Result<()> {
    if v.schema_version != NARRATIVE_PACKAGE_SCHEMA_VERSION {
        return Err(Error::Contract(
            "NarrativePackage schema_version must be 2".into(),
        ));
    }
    if !valid_prefixed_sha256(&v.package_id, "nvc2_") {
        return Err(Error::Contract(
            "NarrativePackage package_id must be nvc2_<lowercase SHA-256>".into(),
        ));
    }
    nonempty(&v.narrative_id, "narrative_id")?;
    if v.sts_domain.id != "sts" {
        return Err(Error::Contract("sts_domain.id must be sts".into()));
    }
    nonempty(&v.sts_domain.version, "sts_domain.version")?;
    sha(&v.sts_domain.bundle_sha256, "sts_domain.bundle_sha256")?;
    sha(&v.source_snapshot_sha256, "source_snapshot_sha256")?;
    if v.canonical_narrative.kind != "SCREENPLAY" {
        return Err(Error::Contract(
            "canonical_narrative.kind must be SCREENPLAY".into(),
        ));
    }
    for (name, a) in v.artifacts() {
        validate_artifact_ref(a, name)?;
    }
    if v.scene_map.kind != "narrative_scene_map_v2" {
        return Err(Error::Contract(
            "NarrativePackage scene_map.kind must be narrative_scene_map_v2".into(),
        ));
    }
    if v.package_id != v.derived_id()? {
        return Err(Error::Contract(
            "NarrativePackage package_id is not bound to its body".into(),
        ));
    }
    Ok(())
}
