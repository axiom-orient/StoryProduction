//! Current NarrativePackage and persisted Delivery contract-chain verification.

use super::storage::read_json_bytes;
use super::{project_owned_path, read_json};
use narrative_courier_contracts::{
    validate_delivery_request, validate_delivery_set, validate_director_plan,
    validate_narrative_package, validate_nvc_release_bundle, ArtifactRef, AssetIndex,
    DeliveryRequest, DeliverySet, DirectorPlan, NarrativePackage, NvcReleaseBundle,
};
use narrative_courier_runtime::paths::validate_path_segment;
use narrative_courier_runtime::store::{sha256_bytes, sha256_file};
use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::Path;

pub(super) fn verify_artifact(project: &Path, artifact: &ArtifactRef) -> Result<()> {
    let path = project_owned_path(project, &artifact.path)?;
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::Contract(format!(
            "MISSING_ARTIFACT: {}",
            artifact.path
        )));
    }
    if sha256_file(&path)? != artifact.sha256 {
        return Err(Error::Contract(format!("HASH_MISMATCH: {}", artifact.path)));
    }
    Ok(())
}

pub(super) fn verify_delivery_artifacts(project: &Path, set: &DeliverySet) -> Result<()> {
    for delivery in &set.deliveries {
        verify_artifact(project, &delivery.artifact)?;
    }
    Ok(())
}

pub(super) fn verify_package_files(project: &Path, package: &NarrativePackage) -> Result<()> {
    validate_narrative_package(package)?;
    for (_, a) in package.artifacts() {
        verify_artifact(project, a)?;
    }
    let verification = fs::read_to_string(project_owned_path(
        project,
        &package.authority.verification.path,
    )?)?;
    if sha256_bytes(verification.as_bytes()) != package.authority.verification.sha256 {
        return Err(Error::Contract(
            "HASH_MISMATCH: verification bytes changed during read".into(),
        ));
    }
    if verification.matches("STS_STATUS: READY").count() != 1
        || verification.contains("STS_STATUS: BLOCKED")
    {
        return Err(Error::Contract(
            "NARRATIVE_DRIFT: NarrativePackage verification is not final READY".into(),
        ));
    }
    Ok(())
}

pub(super) fn verify_released_package(
    project: &Path,
    package: &NarrativePackage,
) -> Result<String> {
    validate_narrative_package(package)?;
    let canonical_path = project_owned_path(project, "narrative-package.json")?;
    let (canonical, canonical_bytes): (NarrativePackage, _) = read_json_bytes(&canonical_path)?;
    let supplied_sha = hash_jcs(package)?;
    if hash_jcs(&canonical)? != supplied_sha {
        return Err(Error::Contract(
            "HASH_MISMATCH: supplied package is not the current canonical release".into(),
        ));
    }
    verify_package_files(project, &canonical)?;
    let bundle: NvcReleaseBundle = read_json(&project_owned_path(project, "release-bundle.json")?)?;
    validate_nvc_release_bundle(&bundle)?;
    if bundle.project_name != canonical.narrative_id
        || bundle.package_id != canonical.package_id
        || bundle.package_sha256 != sha256_bytes(&canonical_bytes)
        || bundle.scene_map.sha256 != canonical.scene_map.sha256
        || bundle.source_snapshot_sha256 != canonical.source_snapshot_sha256
    {
        return Err(Error::Contract(
            "HASH_MISMATCH: STS release bundle does not bind the canonical package".into(),
        ));
    }
    verify_artifact(project, &bundle.scene_map)?;
    let state_path = project_owned_path(project, &bundle.completed_state.path)?;
    let (state, state_bytes): (serde_json::Value, _) = read_json_bytes(&state_path)?;
    if sha256_bytes(&state_bytes) != bundle.completed_state.sha256 {
        return Err(Error::Contract(
            "HASH_MISMATCH: STS completed state differs from release proof".into(),
        ));
    }
    let nodes = state["nodes"]
        .as_object()
        .ok_or_else(|| Error::Contract("STS completed state has no nodes".into()))?;
    if state["status"] != "completed"
        || state["project_name"] != bundle.project_name
        || nodes.is_empty()
        || nodes.values().any(|record| record["status"] != "completed")
        || nodes
            .get("final_release")
            .and_then(|record| record["decision"].as_str())
            != Some("approve")
    {
        return Err(Error::Contract(
            "STS release proof has no Completed/approved final gate".into(),
        ));
    }
    let snapshot = state
        .get("source_snapshot")
        .filter(|value| !value.is_null())
        .ok_or_else(|| Error::Contract("STS completed state has no source snapshot".into()))?;
    if hash_jcs(snapshot)? != bundle.source_snapshot_sha256 {
        return Err(Error::Contract(
            "HASH_MISMATCH: STS source snapshot differs from release proof".into(),
        ));
    }
    Ok(supplied_sha)
}

pub(super) fn load_contract_set(
    project: &Path,
    package: &NarrativePackage,
    request_id: &str,
) -> Result<DeliverySet> {
    validate_path_segment(request_id, "request_id")?;
    let run_root = format!("visual/requests/{request_id}");
    let request: DeliveryRequest = read_json(&project_owned_path(
        project,
        &format!("{run_root}/delivery-request.json"),
    )?)?;
    if request.request_id != request_id {
        return Err(Error::Contract(
            "INVALID_INVARIANT: delivery directory/request_id mismatch".into(),
        ));
    }
    validate_delivery_request(&request)?;
    let plan: DirectorPlan = read_json(&project_owned_path(
        project,
        &format!("{run_root}/director-plan.json"),
    )?)?;
    validate_director_plan(&plan, &request)?;
    if request.narrative_package_sha256 != hash_jcs(package)?
        || plan.director_bundle_sha256 != crate::planner::plan::director_bundle_sha256()
    {
        return Err(Error::Contract(
            "STALE_RESULT: persisted request/planner does not target current authority".into(),
        ));
    }
    let screenplay = fs::read_to_string(project_owned_path(
        project,
        &package.canonical_narrative.artifact.path,
    )?)?;
    if sha256_bytes(screenplay.as_bytes()) != package.canonical_narrative.artifact.sha256 {
        return Err(Error::Contract(
            "HASH_MISMATCH: persisted plan source changed during read".into(),
        ));
    }
    crate::planner::preflight::semantic_preflight(&plan, &request, &screenplay)?;
    let asset_index: AssetIndex = read_json(&project_owned_path(
        project,
        &format!("{run_root}/asset-index.json"),
    )?)?;
    super::assets::materialize::admit_materialized_index(project, &asset_index, &plan)?;
    let set: DeliverySet = read_json(&project_owned_path(
        project,
        &format!("{run_root}/delivery-set.json"),
    )?)?;
    validate_delivery_set(&set, &request)?;

    let plan_sha = hash_jcs(&plan)?;
    let asset_index_sha = hash_jcs(&asset_index)?;
    if set.director_plan_sha256 != plan_sha || set.asset_index_sha256 != asset_index_sha {
        return Err(Error::Contract(
            "HASH_MISMATCH: persisted delivery contract chain".into(),
        ));
    }
    Ok(set)
}
