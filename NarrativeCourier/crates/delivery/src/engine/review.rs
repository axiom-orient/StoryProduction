//! Human review transition for an already-published delivery candidate.

use super::{
    load_contract_set, project_owned_path, read_json, verify_delivery_artifacts,
    verify_released_package, write_json, DeliveryEngine,
};
use narrative_courier_contracts::{
    validate_delivery_set, DeliveryRequest, DeliverySet, DeliveryStatus, NarrativePackage,
    VerificationLayer, VerificationStatus,
};
use narrative_courier_runtime::paths::validate_path_segment;
use narrative_courier_runtime::store::lock_project;
use narrative_courier_runtime::{Error, Result};
use std::path::Path;

impl DeliveryEngine {
    pub fn review(
        project: &Path,
        package: &NarrativePackage,
        request_id: &str,
        output_id: &str,
        approve: bool,
        note: Option<&str>,
    ) -> Result<DeliverySet> {
        validate_path_segment(request_id, "request_id")?;
        validate_path_segment(output_id, "output_id")?;
        let _lock = lock_project(project)?;
        let package_sha = verify_released_package(project, package)?;
        let request_path = format!("visual/requests/{request_id}/delivery-request.json");
        let set_path = format!("visual/requests/{request_id}/delivery-set.json");
        let request: DeliveryRequest = read_json(&project_owned_path(project, &request_path)?)?;
        let mut set = load_contract_set(project, package, request_id)?;
        if set.narrative_package_sha256 != package_sha {
            return Err(Error::Contract(
                "STALE_RESULT: delivery does not target current released NarrativePackage".into(),
            ));
        }
        verify_delivery_artifacts(project, &set)?;
        let receipt = set
            .deliveries
            .iter_mut()
            .find(|d| d.output_request_id == output_id)
            .ok_or_else(|| Error::Contract(format!("delivery output not found: {output_id}")))?;

        if approve {
            if receipt.status == DeliveryStatus::Pass {
                return Ok(set);
            }
            if receipt.status != DeliveryStatus::NeedsReview {
                return Err(Error::Contract(format!(
                    "{} is not awaiting Human review",
                    output_id
                )));
            }
            if receipt.verification.technical.status != VerificationStatus::Pass
                || matches!(
                    receipt.verification.semantic.status,
                    VerificationStatus::Fail
                )
            {
                return Err(Error::Contract(format!(
                    "{} cannot be approved over failed automatic verification",
                    output_id
                )));
            }
            receipt.verification.semantic = VerificationLayer {
                status: VerificationStatus::Pass,
                findings: vec![],
            };
            receipt.verification.visual = VerificationLayer {
                status: VerificationStatus::Pass,
                findings: vec![],
            };
            receipt.status = DeliveryStatus::Pass;
        } else {
            if receipt.status == DeliveryStatus::Blocked {
                return Ok(set);
            }
            if receipt.status != DeliveryStatus::NeedsReview {
                return Err(Error::Contract(format!(
                    "{} is not awaiting Human review",
                    output_id
                )));
            }
            let finding = note
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .unwrap_or("Human visual review rejected the artifact");
            receipt.verification.visual = VerificationLayer {
                status: VerificationStatus::Fail,
                findings: vec![finding.to_string()],
            };
            receipt.status = DeliveryStatus::Blocked;
        }

        set.status = DeliverySet::aggregate_status(&set.deliveries)?;
        validate_delivery_set(&set, &request)?;
        write_json(&project_owned_path(project, &set_path)?, &set)?;
        let persisted = load_contract_set(project, package, request_id)?;
        verify_delivery_artifacts(project, &persisted)?;
        if persisted != set {
            return Err(Error::Contract(
                "delivery set changed during Human review commit".into(),
            ));
        }
        Ok(persisted)
    }
}
