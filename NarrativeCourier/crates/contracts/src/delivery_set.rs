//! Delivery receipts, verification layers, and aggregate status invariants.

use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::artifact::{validate_artifact_ref, ArtifactRef};
use super::common::{is_narrative_ref, nonempty, nonempty_items, sha, unique};
use super::request::{validate_delivery_request, DeliveryRequest};
use super::SCHEMA_VERSION;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Pass,
    Warn,
    Fail,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VerificationLayer {
    pub status: VerificationStatus,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeliveryStatus {
    Pass,
    NeedsReview,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeliveryVerification {
    pub semantic: VerificationLayer,
    pub visual: VerificationLayer,
    pub technical: VerificationLayer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeliveryReceipt {
    pub output_request_id: String,
    pub artifact: ArtifactRef,
    pub backend_id: String,
    pub backend_fingerprint: String,
    pub lineage_refs: Vec<String>,
    pub verification: DeliveryVerification,
    pub status: DeliveryStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeliverySet {
    pub schema_version: u32,
    pub set_id: String,
    pub narrative_package_sha256: String,
    pub delivery_request_sha256: String,
    pub director_plan_sha256: String,
    pub asset_index_sha256: String,
    pub status: DeliveryStatus,
    pub deliveries: Vec<DeliveryReceipt>,
}

impl DeliverySet {
    /// Aggregate candidate/review outcomes with failure precedence and no vacuous PASS.
    pub fn aggregate_status(deliveries: &[DeliveryReceipt]) -> Result<DeliveryStatus> {
        if deliveries.is_empty() {
            return Err(Error::Contract(
                "DeliverySet requires at least one receipt".into(),
            ));
        }
        let mut status = DeliveryStatus::Pass;
        for receipt in deliveries {
            match receipt.status {
                DeliveryStatus::Blocked => return Ok(DeliveryStatus::Blocked),
                DeliveryStatus::NeedsReview => status = DeliveryStatus::NeedsReview,
                DeliveryStatus::Pass => {}
            }
        }
        Ok(status)
    }
}

pub fn validate_delivery_set(v: &DeliverySet, request: &DeliveryRequest) -> Result<()> {
    validate_delivery_request(request)?;
    if v.schema_version != SCHEMA_VERSION {
        return Err(Error::Contract(
            "DeliverySet schema_version must be 1".into(),
        ));
    }
    nonempty(&v.set_id, "set_id")?;
    for (name, h) in [
        ("narrative_package_sha256", &v.narrative_package_sha256),
        ("delivery_request_sha256", &v.delivery_request_sha256),
        ("director_plan_sha256", &v.director_plan_sha256),
        ("asset_index_sha256", &v.asset_index_sha256),
    ] {
        sha(h, name)?;
    }
    if v.narrative_package_sha256 != request.narrative_package_sha256
        || v.delivery_request_sha256 != hash_jcs(request)?
    {
        return Err(Error::Contract(
            "HASH_MISMATCH: DeliverySet does not target supplied DeliveryRequest/NarrativePackage"
                .into(),
        ));
    }
    if v.deliveries.len() != request.outputs.len() {
        return Err(Error::Contract(
            "DeliverySet deliveries must match requested outputs".into(),
        ));
    }
    unique(
        v.deliveries.iter().map(|d| &d.output_request_id),
        "delivery output_request_id",
    )?;
    let outputs: BTreeMap<_, _> = request
        .outputs
        .iter()
        .map(|output| (output.output_id.as_str(), output))
        .collect();
    for d in &v.deliveries {
        nonempty(&d.output_request_id, "delivery.output_request_id")?;
        validate_artifact_ref(&d.artifact, "delivery.artifact")?;
        nonempty(&d.backend_id, "delivery.backend_id")?;
        nonempty(&d.backend_fingerprint, "delivery.backend_fingerprint")?;
        let output = outputs.get(d.output_request_id.as_str()).ok_or_else(|| {
            Error::Contract(format!("unexpected delivery: {}", d.output_request_id))
        })?;
        if d.artifact.kind != format!("{:?}", output.format).to_uppercase()
            || output
                .backend_id
                .as_deref()
                .is_some_and(|id| id != d.backend_id)
        {
            return Err(Error::Contract(format!(
                "{} artifact format/backend differs from request",
                d.output_request_id
            )));
        }
        if d.lineage_refs.is_empty()
            || request
                .must_include_refs
                .iter()
                .any(|reference| !d.lineage_refs.contains(reference))
        {
            return Err(Error::Contract(format!(
                "{} missing required lineage",
                d.output_request_id
            )));
        }
        unique(d.lineage_refs.iter(), "delivery lineage_ref")?;
        for r in &d.lineage_refs {
            if !is_narrative_ref(r) {
                return Err(Error::Contract(format!("invalid lineage ref: {r}")));
            }
        }
        for (name, layer) in [
            ("semantic", &d.verification.semantic),
            ("visual", &d.verification.visual),
            ("technical", &d.verification.technical),
        ] {
            nonempty_items(&layer.findings, "verification finding")?;
            match layer.status {
                VerificationStatus::Pass | VerificationStatus::NotApplicable
                    if !layer.findings.is_empty() =>
                {
                    return Err(Error::Contract(format!(
                        "{name} PASS/NOT_APPLICABLE cannot have findings"
                    )))
                }
                VerificationStatus::Warn | VerificationStatus::Fail
                    if layer.findings.is_empty() =>
                {
                    return Err(Error::Contract(format!(
                        "{name} WARN/FAIL requires findings"
                    )))
                }
                _ => {}
            }
        }
        let has_failure = [
            &d.verification.semantic,
            &d.verification.visual,
            &d.verification.technical,
        ]
        .iter()
        .any(|layer| layer.status == VerificationStatus::Fail);
        if has_failure != (d.status == DeliveryStatus::Blocked) {
            return Err(Error::Contract(format!(
                "{} failed verification must remain BLOCKED; BLOCKED requires a failure finding",
                d.output_request_id
            )));
        }
        if d.status == DeliveryStatus::Pass
            && !(d.verification.semantic.status == VerificationStatus::Pass
                && d.verification.technical.status == VerificationStatus::Pass
                && matches!(
                    d.verification.visual.status,
                    VerificationStatus::Pass | VerificationStatus::NotApplicable
                ))
        {
            return Err(Error::Contract(format!(
                "{} false PASS",
                d.output_request_id
            )));
        }
    }
    let expected_status = DeliverySet::aggregate_status(&v.deliveries)?;
    if v.status != expected_status {
        return Err(Error::Contract(
            "DeliverySet status must equal aggregated delivery status".into(),
        ));
    }
    Ok(())
}
