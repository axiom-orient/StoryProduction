//! Request capability policy and deterministic DirectorPlan projection.

pub(crate) mod request_policy;
pub(in crate::planner) mod timing;

use timing::FrameAllocation;

use self::request_policy::{selected_scenes, validate_supported_request};
use super::scene::extract_scenes;
use narrative_courier_contracts::{
    validate_delivery_request, validate_director_plan, validate_narrative_package, AssetRequest,
    DeliveryPlan, DeliveryRequest, DirectorPlan, GrammarAxis, NarrativePackage, OutputFormat,
    PresentationStructure, ProjectionOperation, ProjectionUnit,
};
use narrative_courier_runtime::store::sha256_bytes;
use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeSet;

const DIRECTOR_BUNDLE: &[u8] = include_bytes!("../director_bundle.txt");

pub fn director_bundle_sha256() -> String {
    sha256_bytes(DIRECTOR_BUNDLE)
}

pub(super) fn structure(format: OutputFormat) -> PresentationStructure {
    match format {
        OutputFormat::Html => PresentationStructure::Scroll,
        OutputFormat::Png
        | OutputFormat::Webp
        | OutputFormat::Svg
        | OutputFormat::Pdf
        | OutputFormat::Pptx => PresentationStructure::Paged,
        OutputFormat::Mp4 | OutputFormat::Webm => PresentationStructure::Timed,
        OutputFormat::Json => PresentationStructure::Graph,
        OutputFormat::Otio => PresentationStructure::Linear,
    }
}

pub(super) fn plan_identity(package_hash: &str, request_hash: &str, bundle_hash: &str) -> String {
    format!(
        "PLAN-{}",
        sha256_bytes(format!("{package_hash}\n{request_hash}\n{bundle_hash}").as_bytes())
    )
}

pub fn build_plan(
    package: &NarrativePackage,
    request: &DeliveryRequest,
    screenplay: &str,
) -> Result<DirectorPlan> {
    validate_narrative_package(package)?;
    validate_delivery_request(request)?;
    validate_supported_request(request)?;
    let package_hash = hash_jcs(package)?;
    if request.narrative_package_sha256 != package_hash {
        return Err(Error::Contract(
            "HASH_MISMATCH: DeliveryRequest narrative_package_sha256".into(),
        ));
    }
    if sha256_bytes(screenplay.as_bytes()) != package.canonical_narrative.artifact.sha256 {
        return Err(Error::Contract(
            "HASH_MISMATCH: screenplay bytes differ from NarrativePackage".into(),
        ));
    }
    let request_hash = hash_jcs(request)?;
    let scenes = extract_scenes(screenplay)?;
    let selected = selected_scenes(&scenes, request)?;
    let scene_ids: BTreeSet<&str> = scenes.iter().map(|scene| scene.id.as_str()).collect();
    let selected_ids: BTreeSet<&str> = selected.iter().map(|scene| scene.id.as_str()).collect();
    for input in &request.asset_inputs {
        let mut is_selected = false;
        for reference in &input.narrative_refs {
            if reference.starts_with("SCN-") {
                if !scene_ids.contains(reference.as_str()) {
                    return Err(Error::Contract(format!(
                        "LINEAGE_MISSING: asset {} references unknown scene {reference}",
                        input.asset_id
                    )));
                }
                is_selected |= selected_ids.contains(reference.as_str());
            }
        }
        if !is_selected {
            return Err(Error::Contract(format!(
                "UNUSED_ASSET_INPUT: {} is not scoped to any selected SCN-*",
                input.asset_id
            )));
        }
    }
    let selected_count = u32::try_from(selected.len())
        .map_err(|_| Error::Contract("selected scene count exceeds u32 capacity".into()))?;
    if selected_count == 0 {
        return Err(Error::Contract(
            "delivery requires at least one selected scene".into(),
        ));
    }
    let bundle_hash = director_bundle_sha256();
    let plan_id = plan_identity(&package_hash, &request_hash, &bundle_hash);
    let asset_requests: Vec<AssetRequest> = request
        .asset_inputs
        .iter()
        .map(|input| AssetRequest {
            asset_id: input.asset_id.clone(),
            kind: input.kind,
            role: input.role.clone(),
            narrative_refs: input.narrative_refs.clone(),
            generation_mode: input.generation_mode,
            consistency_group: None,
            brief: format!(
                "Use supplied {} asset as-is; do not infer or add Narrative facts.",
                input.role
            ),
            hard_constraints: vec!["preserve source bytes and provenance".into()],
            source: Some(input.source.clone()),
        })
        .collect();
    let mut deliveries = Vec::new();
    for output in &request.outputs {
        let timing = FrameAllocation::for_output(output, selected_count)?;
        let units = (0..selected_count).zip(&selected).map(|(order, scene)| ProjectionUnit {
            unit_id: format!("{}-{}", output.output_id, scene.id),
            order,
            narrative_refs: vec![scene.id.clone()],
            operation: ProjectionOperation::Preserve,
            direction: "Render the exact approved scene text; presentation may change but Narrative meaning must not.".into(),
            asset_refs: asset_requests.iter()
                .filter(|asset| asset.narrative_refs.iter().any(|r| r == &scene.id))
                .map(|asset| asset.asset_id.clone())
                .collect(),
            content: Some(scene.content.clone()),
            duration_frames: timing.duration(order),
        }).collect();
        deliveries.push(DeliveryPlan {
            output_request_id: output.output_id.clone(),
            presentation_structure: structure(output.format),
            units,
        });
    }
    let visual_grammar = [
        (
            "composition".into(),
            GrammarAxis {
                must: vec![
                    "preserve requested scene order".into(),
                    "keep text readable".into(),
                ],
                avoid: vec!["semantic embellishment".into()],
            },
        ),
        (
            "typography".into(),
            GrammarAxis {
                must: vec!["plain legible typography".into()],
                avoid: vec!["decorative text replacement".into()],
            },
        ),
    ]
    .into();
    let plan = DirectorPlan {
        schema_version: 1,
        plan_id,
        narrative_package_sha256: package_hash,
        delivery_request_sha256: request_hash,
        director_bundle_sha256: bundle_hash,
        visual_grammar,
        asset_requests,
        deliveries,
    };
    validate_director_plan(&plan, request)?;
    Ok(plan)
}
