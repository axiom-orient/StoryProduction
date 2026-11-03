//! Canonical artifact subjects for candidate bodies.

use crate::contracts::digest::jcs_sha256;
use crate::contracts::ArtifactRef;
use crate::error::Result;
use crate::model::Candidate;

pub fn candidate_subject(candidate: &Candidate) -> Result<ArtifactRef> {
    let body = serde_json::to_value(candidate)?;
    Ok(ArtifactRef {
        id: format!("candidate:{}", candidate.id),
        kind: "candidate".into(),
        revision: candidate.source_revision + 1,
        digest: format!("sha256:{}", jcs_sha256(&body)?),
    })
}
