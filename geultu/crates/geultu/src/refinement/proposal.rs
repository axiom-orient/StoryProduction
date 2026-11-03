//! Same-language patch proposal and domain-decision construction.

use super::diagnose::{apply_replacements, diagnose_text};
use super::{
    PatchProposal, PatchProposalInput, RefinementPack, REFINEMENT_CANDIDATE_KIND,
    REFINEMENT_PROVIDER_ID,
};
use crate::contracts::{DomainDecision, DomainDecisionKind};
use crate::domain::validation::validate_id;
use crate::error::{Error, Result};
use crate::model::{
    Candidate, CandidateStatus, ChangeOperation, ChangeSet, ChangeSetStatus, Receipt,
};

pub fn propose_refinement_patch(
    workspace: &crate::store::Workspace,
    pack: &RefinementPack,
    project: &crate::model::Project,
    input: &PatchProposalInput,
) -> Result<PatchProposal> {
    validate_id("command id", &input.command_id)?;
    pack.catalog.validate()?;
    if !pack.catalog.policy.allows("plan_patch") {
        return Err(Error::InvalidInput(
            "refinement catalog policy does not grant plan_patch".into(),
        ));
    }
    let scene = project
        .scenes
        .get(&input.scene_id)
        .ok_or_else(|| Error::NotFound(format!("scene '{}' was not found", input.scene_id)))?;
    let findings = diagnose_text(&input.scene_id, &scene.text);
    if findings.iter().all(|finding| finding.replacement.is_none()) {
        return Err(Error::InvalidInput(format!(
            "scene '{}' has no mechanically patchable findings; \
             diagnose-only findings require human editing",
            input.scene_id
        )));
    }
    let patched = apply_replacements(&scene.text, &findings);

    let patch_digest = crate::contracts::digest::jcs_sha256(&serde_json::json!({
        "catalog_id": pack.catalog.catalog_id,
        "project_id": project.id,
        "scene_id": input.scene_id,
        "source_scene_revision": scene.revision,
        "patched_text": patched,
    }))?;
    let candidate_id = input
        .candidate_id
        .clone()
        .unwrap_or_else(|| format!("refine-{}", &patch_digest[..16]));
    let changeset_id = input
        .changeset_id
        .clone()
        .unwrap_or_else(|| format!("{candidate_id}-cs"));

    let reason_codes: Vec<String> = findings
        .iter()
        .filter(|finding| finding.replacement.is_some())
        .map(|finding| finding.code.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    let patched_chars = patched.chars().count();
    let candidate = Candidate {
        id: candidate_id.clone(),
        kind: REFINEMENT_CANDIDATE_KIND.into(),
        source_revision: project.content_revision,
        writing_type: None,
        stage: None,
        content: patched.clone(),
        receipt: Receipt {
            provider: REFINEMENT_PROVIDER_ID.into(),
            model: pack.catalog.catalog_id.clone(),
            request_id: Some(patch_digest.clone()),
            input_hash: Some(patch_digest.clone()),
            created_at_ms: crate::store::now_ms(),
        },
        status: CandidateStatus::Pending,
    };
    let changeset = ChangeSet {
        id: changeset_id.clone(),
        candidate_id: candidate_id.clone(),
        source_revision: project.content_revision,
        operations: vec![ChangeOperation::ReplaceSceneText {
            scene_id: input.scene_id.clone(),
            expected_scene_revision: scene.revision,
            text: patched,
        }],
        status: ChangeSetStatus::Pending,
    };

    let expected_candidate = candidate_binding(&candidate)?;
    let expected_changeset = crate::contracts::digest::serde_json_sha256(&changeset)?;
    let changeset_command = changeset_command_id(&input.command_id);
    let proposal = repair_decision(&changeset_command, project.revision, &reason_codes)?;
    workspace.transact(
        &project.id,
        &input.command_id,
        crate::domain::Command::AddCandidate { candidate },
    )?;
    workspace.transact(
        &project.id,
        &changeset_command,
        crate::domain::Command::AddChangeSet { changeset },
    )?;

    // Generic transaction idempotency is keyed by command ID. Prove that the
    // artifacts actually present after both calls are this proposal's artifacts.
    let current = workspace.load_project(&project.id)?;
    let stored_candidate = current.candidates.get(&candidate_id).ok_or_else(|| {
        Error::Conflict("refinement command did not publish the requested candidate".into())
    })?;
    let stored_changeset = current.changesets.get(&changeset_id).ok_or_else(|| {
        Error::Conflict("refinement command did not publish the requested changeset".into())
    })?;
    if candidate_binding(stored_candidate)? != expected_candidate
        || crate::contracts::digest::serde_json_sha256(stored_changeset)? != expected_changeset
    {
        return Err(Error::Conflict(
            "refinement command IDs are already bound to different or decided artifacts".into(),
        ));
    }

    Ok(PatchProposal {
        proposal,
        candidate_id,
        changeset_id,
        scene_id: input.scene_id.clone(),
        source_scene_revision: scene.revision,
        applied_codes: reason_codes,
        patched_chars,
        source_chars: scene.text.chars().count(),
    })
}

// Creation time is an observation of the first durable write, not caller input.
// Excluding it here allows an identical pending proposal to resume idempotently;
// candidate_subject still binds the complete stored receipt for human review.
fn candidate_binding(candidate: &Candidate) -> Result<String> {
    let mut value = serde_json::to_value(candidate)?;
    value["receipt"]["created_at_ms"] = serde_json::json!(0);
    crate::contracts::digest::jcs_sha256(&value)
}

/// Builds the domain-decision proposal document with
/// `decision_id = "decision_" + jcs_sha256(document minus decision_id)`,
/// mirroring the receipt/decision identity derivations.
///
/// # Errors
/// Returns [`Error::Json`] when serialization fails or the assembled
/// decision violates the domain-decision contract.
pub fn repair_decision(
    command_id: &str,
    state_revision: u64,
    reason_codes: &[String],
) -> Result<DomainDecision> {
    let mut decision = DomainDecision {
        decision_id: String::new(),
        command_id: command_id.into(),
        state_revision,
        decision: DomainDecisionKind::Repair,
        selected_receipts: Vec::new(),
        reason_codes: reason_codes.to_vec(),
    };
    let mut body = serde_json::to_value(&decision)?;
    body.as_object_mut()
        .ok_or_else(|| Error::Corrupt("serialized decision is not a JSON object".into()))?
        .remove("decision_id");
    decision.decision_id = format!(
        "{}{}",
        crate::contracts::DOMAIN_DECISION_ID_PREFIX,
        crate::contracts::digest::jcs_sha256(&body)?
    );
    decision.validate()?;
    Ok(decision)
}

/// Derive a 64-character transaction ID from the entire caller command identity.
/// Domain separation distinguishes this producer purpose without truncating caller IDs.
fn changeset_command_id(base: &str) -> String {
    let mut input = b"geultu-refinement-changeset-command-v2\0".to_vec();
    input.extend_from_slice(base.as_bytes());
    crate::contracts::digest::sha256_hex(&input)
}
