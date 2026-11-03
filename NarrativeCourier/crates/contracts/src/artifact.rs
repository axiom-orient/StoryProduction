//! Reusable artifact references and their raw-path/hash contract.

use narrative_courier_runtime::paths::validate_relative_path;
use narrative_courier_runtime::Result;
use serde::{Deserialize, Serialize};

use super::common::{nonempty, sha};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub kind: String,
    pub path: String,
    pub sha256: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StsDomainRef {
    pub id: String,
    pub version: String,
    pub bundle_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalNarrative {
    pub kind: String,
    pub artifact: ArtifactRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NarrativeAuthority {
    pub source_ir: ArtifactRef,
    pub narrative_ir: ArtifactRef,
    pub execution_constraints: ArtifactRef,
    pub character_states: ArtifactRef,
    pub scene_ir: ArtifactRef,
    pub verification: ArtifactRef,
}

pub(super) fn validate_artifact_ref(a: &ArtifactRef, field: &str) -> Result<()> {
    nonempty(&a.kind, field)?;
    nonempty(&a.path, field)?;
    validate_relative_path(&a.path)?;
    sha(&a.sha256, field)?;
    if let Some(mime) = &a.mime {
        nonempty(mime, "artifact.mime")?;
    }
    Ok(())
}
