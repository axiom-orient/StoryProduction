//! Output-local render identity and immutable artifact receipts.

use super::document::mime;
use super::verification::verification_for;
use crate::backend::BackendDescriptor;
use narrative_courier_contracts::{
    ArtifactRef, AssetIndex, AssetItem, AssetRequest, DeliveryPlan, DeliveryReceipt, DirectorPlan,
    OutputRequest,
};
use narrative_courier_runtime::store::sha256_file;
use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Serialize)]
struct BuildAssetKey<'a> {
    request: &'a AssetRequest,
    item: &'a AssetItem,
}

#[derive(Serialize)]
struct BuildKey<'a> {
    renderer_version: &'static str,
    director_bundle_sha256: &'a str,
    delivery: &'a DeliveryPlan,
    output: &'a OutputRequest,
    backend_fingerprint: &'a str,
    assets: Vec<BuildAssetKey<'a>>,
}

pub(super) fn output_build_key(
    plan: &DirectorPlan,
    delivery: &DeliveryPlan,
    output: &OutputRequest,
    backend: &BackendDescriptor,
    asset_index: &AssetIndex,
) -> Result<String> {
    let assets = relevant_asset_keys(plan, delivery, asset_index)?;
    hash_jcs(&BuildKey {
        renderer_version: concat!(env!("CARGO_PKG_VERSION"), ":document-projection-v3"),
        director_bundle_sha256: &plan.director_bundle_sha256,
        delivery,
        output,
        backend_fingerprint: &backend.fingerprint,
        assets,
    })
}

fn relevant_asset_keys<'a>(
    plan: &'a DirectorPlan,
    delivery: &DeliveryPlan,
    index: &'a AssetIndex,
) -> Result<Vec<BuildAssetKey<'a>>> {
    let ids: BTreeSet<&str> = delivery
        .units
        .iter()
        .flat_map(|u| u.asset_refs.iter().map(String::as_str))
        .collect();
    ids.into_iter()
        .map(|id| {
            let request = plan
                .asset_requests
                .iter()
                .find(|a| a.asset_id == id)
                .ok_or_else(|| Error::Contract(format!("dangling asset request: {id}")))?;
            let item = index
                .assets
                .iter()
                .find(|a| a.asset_id == id)
                .ok_or_else(|| Error::Contract(format!("dangling materialized asset: {id}")))?;
            Ok(BuildAssetKey { request, item })
        })
        .collect()
}

pub(super) fn receipt_for_artifact(
    target: &Path,
    rel: String,
    output: &OutputRequest,
    backend: &BackendDescriptor,
    delivery: &DeliveryPlan,
) -> Result<DeliveryReceipt> {
    let sha = sha256_file(target)?;
    let (verification, status) = verification_for(output.format, target, delivery)?;
    let mut lineage = Vec::new();
    for unit in &delivery.units {
        for r in &unit.narrative_refs {
            if !lineage.contains(r) {
                lineage.push(r.clone());
            }
        }
    }
    Ok(DeliveryReceipt {
        output_request_id: output.output_id.clone(),
        artifact: ArtifactRef {
            kind: format!("{:?}", output.format).to_uppercase(),
            path: rel,
            sha256: sha,
            mime: Some(mime(output.format).into()),
        },
        backend_id: backend.backend_id.clone(),
        backend_fingerprint: backend.fingerprint.clone(),
        lineage_refs: lineage,
        verification,
        status,
    })
}
