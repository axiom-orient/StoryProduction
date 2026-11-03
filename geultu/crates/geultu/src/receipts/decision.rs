//! Human decision issuance bound to the exact candidate artifact.

use super::subject::candidate_subject;
use super::timestamp::rfc3339_utc;
use crate::contracts::digest::jcs_sha256;
use crate::contracts::{ArtifactRef, DecisionKind, HumanDecision};
use crate::error::{Error, Result};
use crate::model::{Candidate, DecisionInput};

pub fn human_decision(
    actor_id: String,
    reason: Option<String>,
    scope: String,
    kind: DecisionKind,
    subject: ArtifactRef,
) -> Result<HumanDecision> {
    let mut decision = HumanDecision {
        decision_id: String::new(),
        actor_id,
        subject,
        scope,
        decision: kind,
        occurred_at: rfc3339_utc(crate::store::now_ms()),
        reason,
    };
    let mut body = serde_json::to_value(&decision)?;
    body.as_object_mut()
        .ok_or_else(|| Error::Corrupt("serialized decision is not a JSON object".into()))?
        .remove("decision_id");
    decision.decision_id = format!("human_{}", jcs_sha256(&body)?);
    decision.validate()?;
    Ok(decision)
}

/// Binds a caller's authority claims to the exact candidate artifact and
/// builds the resulting human decision.
///
/// The declared revision and digest must match [`candidate_subject`]
/// exactly; a malformed digest is an invalid-input error and any mismatch is
/// a fail-closed conflict raised before an event can be journaled.
///
/// # Errors
/// Returns [`Error::InvalidInput`] for a malformed digest claim or a
/// contract violation in the assembled decision, and [`Error::Conflict`]
/// when the declared revision or digest differs from the stored candidate,
/// or [`Error::Json`] when serialization fails.
pub fn bind_decision(
    candidate: &Candidate,
    scope: String,
    kind: DecisionKind,
    input: &DecisionInput,
) -> Result<HumanDecision> {
    let subject = candidate_subject(candidate)?;
    if !crate::contracts::validate::valid_sha256_digest(input.expected_artifact_digest.as_str()) {
        return Err(Error::InvalidInput(format!(
            "expected_artifact_digest '{}' must match ^sha256:[0-9a-f]{{64}}$",
            input.expected_artifact_digest
        )));
    }
    if input.expected_artifact_revision != subject.revision {
        return Err(Error::Conflict(format!(
            "artifact revision mismatch: expected {}, actual {}",
            input.expected_artifact_revision, subject.revision
        )));
    }
    if input.expected_artifact_digest != subject.digest {
        return Err(Error::Conflict(
            "artifact digest mismatch: the declared candidate body differs from the stored candidate"
                .into(),
        ));
    }
    human_decision(
        input.actor_id.clone(),
        input.reason.clone(),
        scope,
        kind,
        subject,
    )
}
