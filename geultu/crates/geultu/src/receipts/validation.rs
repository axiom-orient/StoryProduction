//! Review/proof findings and hash-bound validation receipt issuance.

use super::{validator_version, PROOF_VALIDATOR_ID, REVIEW_VALIDATOR_ID};
use crate::contracts::digest::{jcs_sha256, sha256_hex};
use crate::contracts::{
    ArtifactRef, ReceiptFinding, ReceiptOutcome, ReceiptSeverity, ReceiptValidator,
    ValidationReceipt,
};
use crate::error::{Error, Result};
use crate::model::{Finding, Project, ProofReport, ReviewReport, Severity};

/// Issues the validation receipt for a `review.run` report.
///
/// The subject is the canonical text master exactly as in
/// `story_package::text_master_ref`; `binding_digest` restates the subject digest
/// as the explicit binding claim. Outcome is FAIL when any finding carries
/// error severity. `receipt_id` is deterministic for identical state.
///
/// # Errors
/// Returns [`Error::InvalidInput`] when the mapped findings violate the
/// validation-receipt contract (e.g. more than 10000 findings), or
/// [`Error::Json`] when serialization fails.
pub fn review_receipt(project: &Project, report: &ReviewReport) -> Result<ValidationReceipt> {
    let subject = crate::story_package::text_master_ref(project);
    let findings = map_findings(&report.findings)?;
    finish_receipt(
        REVIEW_VALIDATOR_ID,
        subject,
        outcome(&report.findings),
        findings,
    )
}

/// Issues the validation receipt for a `proof.verify` report.
///
/// The subject is the verified candidate text (`proof:candidate`); its
/// revision is anchored to `content_revision + 1` when a project anchor is
/// supplied and defaults to 1 for context-less verification. The digest is
/// the raw SHA-256 over the candidate UTF-8 bytes.
///
/// # Errors
/// Returns [`Error::InvalidInput`] when the mapped findings violate the
/// validation-receipt contract, or [`Error::Json`] when serialization fails.
pub fn proof_receipt(
    candidate_text: &str,
    project_anchor: Option<&Project>,
    report: &ProofReport,
) -> Result<ValidationReceipt> {
    let revision = match project_anchor {
        Some(project) => project.content_revision + 1,
        None => 1,
    };
    let subject = ArtifactRef {
        id: "proof:candidate".into(),
        kind: "proof_candidate".into(),
        revision,
        digest: format!("sha256:{}", sha256_hex(candidate_text.as_bytes())),
    };
    finish_receipt(
        PROOF_VALIDATOR_ID,
        subject,
        outcome(&report.findings),
        map_findings(&report.findings)?,
    )
}

/// Assembles a receipt with the M2 identity derivation
/// (`vr_` + jcs_sha256 of the document minus its own id) and validates it.
/// Shared by review/proof receipts and the H7 admission receipt.
///
/// # Errors
/// Returns [`Error::Json`] when serialization fails or the assembled
/// receipt violates the validation-receipt contract (fail closed).
pub(crate) fn finish_receipt(
    validator_id: &str,
    subject: ArtifactRef,
    outcome: ReceiptOutcome,
    findings: Vec<ReceiptFinding>,
) -> Result<ValidationReceipt> {
    let mut receipt = ValidationReceipt {
        receipt_id: String::new(),
        validator: ReceiptValidator {
            id: validator_id.into(),
            version: validator_version(),
        },
        binding_digest: subject.digest.clone(),
        subject,
        outcome,
        findings,
    };
    let mut body = serde_json::to_value(&receipt)?;
    body.as_object_mut()
        .ok_or_else(|| Error::Corrupt("serialized receipt is not a JSON object".into()))?
        .remove("receipt_id");
    receipt.receipt_id = format!("vr_{}", jcs_sha256(&body)?);
    receipt.validate()?;
    Ok(receipt)
}

/// FAIL when any finding carries error severity, PASS otherwise.
#[must_use]
pub fn outcome(findings: &[Finding]) -> ReceiptOutcome {
    if findings
        .iter()
        .any(|finding| finding.severity == Severity::Error)
    {
        ReceiptOutcome::Fail
    } else {
        ReceiptOutcome::Pass
    }
}

/// Maps internal dotted lowercase codes (`scene.empty`) to schema-conforming
/// uppercase codes (`SCENE_EMPTY`). Evidence excerpts are not carried into
/// receipt findings; the scene id becomes `path`.
///
/// # Errors
/// Returns [`Error::InvalidInput`] fail-closed when a code cannot be mapped
/// into the schema pattern.
pub fn map_findings(findings: &[Finding]) -> Result<Vec<ReceiptFinding>> {
    findings
        .iter()
        .map(|finding| {
            let code: String = finding
                .code
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() {
                        character.to_ascii_uppercase()
                    } else {
                        '_'
                    }
                })
                .collect();
            let severity = match finding.severity {
                Severity::Error => ReceiptSeverity::Error,
                Severity::Warning => ReceiptSeverity::Warning,
                Severity::Info => ReceiptSeverity::Info,
            };
            let mapped = ReceiptFinding {
                code,
                severity,
                message: finding.message.clone(),
                path: finding.scene_id.clone(),
            };
            mapped.validate()?;
            Ok(mapped)
        })
        .collect()
}
