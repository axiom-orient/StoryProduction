//! Exact scene/lineage and provenance preflight for candidate DirectorPlans.

use super::plan::request_policy::{selected_scenes, validate_supported_request};
use super::plan::timing::FrameAllocation;
use super::plan::{director_bundle_sha256, plan_identity, structure};
use super::scene::extract_scenes;
use narrative_courier_contracts::{
    validate_director_plan, DeliveryRequest, DirectorPlan, GenerationMode, ProjectionOperation,
};
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeMap;

pub fn semantic_preflight(
    plan: &DirectorPlan,
    request: &DeliveryRequest,
    screenplay: &str,
) -> Result<Vec<String>> {
    validate_director_plan(plan, request)?;
    validate_supported_request(request)?;
    let bundle_hash = director_bundle_sha256();
    if plan.director_bundle_sha256 != bundle_hash
        || plan.plan_id
            != plan_identity(
                &plan.narrative_package_sha256,
                &plan.delivery_request_sha256,
                &bundle_hash,
            )
    {
        return Err(Error::Contract(
            "STALE_RESULT: DirectorPlan identity/bundle differs from current deterministic planner"
                .into(),
        ));
    }
    let requested_assets: BTreeMap<_, _> = request
        .asset_inputs
        .iter()
        .map(|asset| (asset.asset_id.as_str(), asset))
        .collect();
    if plan.asset_requests.len() != requested_assets.len() {
        return Err(Error::Contract(
            "INVALID_ASSET_REQUEST: plan assets differ from the request".into(),
        ));
    }
    for asset in &plan.asset_requests {
        // The contracts validator already requires source provenance for USER/DERIVED.
        if !matches!(
            asset.generation_mode,
            GenerationMode::User | GenerationMode::Derived
        ) {
            return Err(Error::Contract(format!(
                "INVALID_ASSET_REQUEST: {} is not a provenance-bound USER/DERIVED asset",
                asset.asset_id
            )));
        }
        let input = requested_assets
            .get(asset.asset_id.as_str())
            .ok_or_else(|| {
                Error::Contract(format!(
                    "INVALID_ASSET_REQUEST: undeclared asset {}",
                    asset.asset_id
                ))
            })?;
        if asset.kind != input.kind
            || asset.role != input.role
            || asset.narrative_refs != input.narrative_refs
            || asset.generation_mode != input.generation_mode
            || asset.source.as_ref() != Some(&input.source)
            || asset.consistency_group.is_some()
            || asset.hard_constraints.len() != 1
            || asset.hard_constraints.first().map(String::as_str)
                != Some("preserve source bytes and provenance")
        {
            return Err(Error::Contract(format!(
                "INVALID_ASSET_REQUEST: asset {} differs from requested provenance",
                asset.asset_id
            )));
        }
        let expected_brief = format!(
            "Use supplied {} asset as-is; do not infer or add Narrative facts.",
            asset.role
        );
        if asset.brief != expected_brief {
            return Err(Error::Contract(format!(
                "NARRATIVE_DRIFT: {} asset brief is not deterministic provenance-only projection",
                asset.asset_id
            )));
        }
    }
    let scenes = extract_scenes(screenplay)?;
    let by_id: BTreeMap<_, _> = scenes
        .iter()
        .map(|s| (s.id.as_str(), s.content.as_str()))
        .collect();
    let selected = selected_scenes(&scenes, request)?;
    let expected: Vec<_> = selected.iter().map(|scene| scene.id.as_str()).collect();
    let selected_count = u32::try_from(selected.len())
        .map_err(|_| Error::Contract("selected scene count exceeds u32 capacity".into()))?;
    let outputs: BTreeMap<_, _> = request
        .outputs
        .iter()
        .map(|output| (output.output_id.as_str(), output))
        .collect();
    let mut evidence = Vec::new();
    for delivery in &plan.deliveries {
        let output = outputs
            .get(delivery.output_request_id.as_str())
            .ok_or_else(|| Error::Contract("unknown plan output".into()))?;
        if delivery.presentation_structure != structure(output.format) {
            return Err(Error::Contract(format!(
                "UNSUPPORTED_CAPABILITY: {} structure differs from deterministic output projection",
                delivery.output_request_id
            )));
        }
        let timing = FrameAllocation::for_output(output, selected_count)?;
        let mut refs = Vec::new();
        for unit in &delivery.units {
            if unit.duration_frames != timing.duration(unit.order) {
                return Err(Error::Contract(format!(
                    "{} frame allocation differs from deterministic request budget",
                    unit.unit_id
                )));
            }
            if unit.operation != ProjectionOperation::Preserve || unit.narrative_refs.len() != 1 {
                return Err(Error::Contract(format!(
                    "NARRATIVE_DRIFT: {} is not exact single-scene PRESERVE",
                    unit.unit_id
                )));
            }
            let id = unit.narrative_refs[0].as_str();
            let canonical = by_id
                .get(id)
                .ok_or_else(|| Error::Contract(format!("LINEAGE_MISSING: {id}")))?;
            if unit.content.as_deref() != Some(*canonical) {
                return Err(Error::Contract(format!(
                    "NARRATIVE_DRIFT: {} content differs from canonical scene",
                    unit.unit_id
                )));
            }
            let expected_assets: Vec<_> = request
                .asset_inputs
                .iter()
                .filter(|asset| {
                    asset
                        .narrative_refs
                        .iter()
                        .any(|reference| reference.as_str() == id)
                })
                .map(|asset| asset.asset_id.as_str())
                .collect();
            if !unit
                .asset_refs
                .iter()
                .map(String::as_str)
                .eq(expected_assets)
            {
                return Err(Error::Contract(format!(
                    "INVALID_ASSET_REQUEST: {} has mismatched scene asset bindings",
                    unit.unit_id
                )));
            }
            refs.push(id);
        }
        if refs != expected {
            return Err(Error::Contract(format!(
                "ILLEGAL_OMISSION: output {} does not preserve the requested SCN sequence",
                delivery.output_request_id
            )));
        }
        for must in request
            .must_include_refs
            .iter()
            .filter(|r| r.starts_with("SCN-"))
        {
            if !refs.contains(&must.as_str()) {
                return Err(Error::Contract(format!(
                    "ILLEGAL_OMISSION: required ref {must} missing from {}",
                    delivery.output_request_id
                )));
            }
        }
        evidence.push(format!(
            "{}: exact SCN PRESERVE verified",
            delivery.output_request_id
        ));
    }
    Ok(evidence)
}
