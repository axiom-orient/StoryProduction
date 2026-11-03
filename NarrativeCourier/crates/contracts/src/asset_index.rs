//! Materialized asset index bound to an approved DirectorPlan.

use std::collections::{BTreeMap, BTreeSet};

use narrative_courier_runtime::paths::validate_relative_path;
use narrative_courier_runtime::token::hash_jcs;
use narrative_courier_runtime::{Error, Result};
use serde::{Deserialize, Serialize};

use super::asset_types::AssetKind;
use super::common::{nonempty, sha, unique, valid_asset_id};
use super::director::DirectorPlan;
use super::SCHEMA_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetItem {
    pub asset_id: String,
    pub source_request_id: String,
    pub kind: AssetKind,
    pub path: String,
    pub sha256: String,
    pub mime: String,
    pub size_bytes: u64,
    pub backend_id: String,
    pub backend_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetIndex {
    pub schema_version: u32,
    pub index_id: String,
    pub narrative_package_sha256: String,
    pub director_plan_sha256: String,
    pub assets: Vec<AssetItem>,
}

pub fn validate_asset_index(v: &AssetIndex, plan: &DirectorPlan) -> Result<()> {
    if v.schema_version != SCHEMA_VERSION {
        return Err(Error::Contract(
            "AssetIndex schema_version must be 1".into(),
        ));
    }
    nonempty(&v.index_id, "index_id")?;
    sha(&v.narrative_package_sha256, "narrative_package_sha256")?;
    sha(&v.director_plan_sha256, "director_plan_sha256")?;
    if v.narrative_package_sha256 != plan.narrative_package_sha256
        || v.director_plan_sha256 != hash_jcs(plan)?
    {
        return Err(Error::Contract(
            "HASH_MISMATCH: AssetIndex does not target supplied DirectorPlan/NarrativePackage"
                .into(),
        ));
    }
    unique(
        plan.asset_requests.iter().map(|a| &a.asset_id),
        "DirectorPlan asset_id",
    )?;
    for request in &plan.asset_requests {
        if !valid_asset_id(&request.asset_id) {
            return Err(Error::Contract(format!(
                "invalid asset_id: {}",
                request.asset_id
            )));
        }
    }
    unique(v.assets.iter().map(|a| &a.asset_id), "AssetIndex asset_id")?;
    unique(
        v.assets.iter().map(|a| &a.source_request_id),
        "AssetIndex source_request_id",
    )?;
    let request_map: BTreeMap<_, _> = plan
        .asset_requests
        .iter()
        .map(|a| (a.asset_id.as_str(), a))
        .collect();
    let request_ids: BTreeSet<_> = request_map.keys().copied().collect();
    let source_ids: BTreeSet<_> = v
        .assets
        .iter()
        .map(|a| a.source_request_id.as_str())
        .collect();
    if source_ids != request_ids {
        return Err(Error::Contract(
            "AssetIndex must materialize every DirectorPlan AssetRequest exactly once".into(),
        ));
    }
    for a in &v.assets {
        nonempty(&a.asset_id, "asset.asset_id")?;
        nonempty(&a.source_request_id, "asset.source_request_id")?;
        let source = request_map
            .get(a.source_request_id.as_str())
            .ok_or_else(|| {
                Error::Contract(format!(
                    "unknown source_request_id: {}",
                    a.source_request_id
                ))
            })?;
        if a.asset_id != source.asset_id {
            return Err(Error::Contract(format!(
                "asset identity differs from source request: {}",
                a.source_request_id
            )));
        }
        if a.kind != source.kind {
            return Err(Error::Contract(format!(
                "asset kind differs from source request: {}",
                a.source_request_id
            )));
        }
        nonempty(&a.path, "asset.path")?;
        validate_relative_path(&a.path)?;
        sha(&a.sha256, "asset.sha256")?;
        nonempty(&a.mime, "asset.mime")?;
        nonempty(&a.backend_id, "asset.backend_id")?;
        nonempty(&a.backend_fingerprint, "asset.backend_fingerprint")?;
    }
    Ok(())
}
