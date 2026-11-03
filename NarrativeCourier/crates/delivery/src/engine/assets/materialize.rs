//! Delivery-owned materialization of released USER/DERIVED assets.

use super::super::project_owned_path;
use super::imported_image::{imported_format_details, inspect_imported_image};
use super::{publish_blob, read_source_bytes, MAX_IMAGE_BYTES};
use narrative_courier_contracts::{
    validate_asset_index, AssetIndex, AssetItem, AssetKind, DirectorPlan, GenerationMode,
};
use narrative_courier_runtime::store::sha256_bytes;
use narrative_courier_runtime::{Error, Result};
use std::path::Path;

pub(in crate::engine) fn internal_copy_fingerprint() -> String {
    sha256_bytes(format!("internal-copy:v3:{}", env!("CARGO_PKG_VERSION")).as_bytes())
}

pub(in crate::engine) fn materialize_assets(
    project: &Path,
    plan: &DirectorPlan,
    plan_sha: &str,
) -> Result<AssetIndex> {
    if narrative_courier_runtime::token::hash_jcs(plan)? != plan_sha {
        return Err(Error::Contract(
            "HASH_MISMATCH: materialization plan identity".into(),
        ));
    }
    let fingerprint = internal_copy_fingerprint();
    let mut assets = Vec::new();
    for request in &plan.asset_requests {
        match request.generation_mode {
            GenerationMode::User | GenerationMode::Derived => {
                let source = request.source.as_ref().ok_or_else(|| {
                    Error::Contract(format!(
                        "INVALID_ASSET_REQUEST: {} source missing",
                        request.asset_id
                    ))
                })?;
                let source_path = project_owned_path(project, &source.path)?;
                if request.kind != AssetKind::Image {
                    return Err(Error::Contract(format!("UNSUPPORTED_CAPABILITY: internal-copy presentation cannot materialize {:?}", request.kind)));
                }
                let bytes = read_source_bytes(&source_path, Some(MAX_IMAGE_BYTES))?;
                let actual = sha256_bytes(&bytes);
                if actual != source.sha256 {
                    return Err(Error::Contract(format!(
                        "ASSET_HASH_MISMATCH: {}",
                        request.asset_id
                    )));
                }
                let (format, _, _) = inspect_imported_image(&bytes)?;
                let (extension, mime) = imported_format_details(format);
                if source
                    .mime
                    .as_deref()
                    .is_some_and(|declared| declared != mime)
                {
                    return Err(Error::Contract(format!(
                        "INVALID_IMAGE: MIME differs from actual bytes for {}",
                        request.asset_id
                    )));
                }
                let rel = format!("visual/assets/blobs/{actual}.{extension}");
                publish_blob(project, &rel, &bytes, &actual)?;
                assets.push(AssetItem {
                    asset_id: request.asset_id.clone(),
                    source_request_id: request.asset_id.clone(),
                    kind: request.kind,
                    path: rel,
                    sha256: actual,
                    mime: mime.to_owned(),
                    size_bytes: bytes.len() as u64,
                    backend_id: "internal-copy".into(),
                    backend_fingerprint: fingerprint.clone(),
                });
            }
            GenerationMode::Procedural | GenerationMode::Generative => {
                return Err(Error::Contract(format!(
                    "UNSUPPORTED_CAPABILITY: no materializer for {} {:?}",
                    request.asset_id, request.generation_mode
                )));
            }
        }
    }
    let index = AssetIndex {
        schema_version: 1,
        index_id: format!("INDEX-{plan_sha}"),
        narrative_package_sha256: plan.narrative_package_sha256.clone(),
        director_plan_sha256: plan_sha.into(),
        assets,
    };
    validate_asset_index(&index, plan)?;
    Ok(index)
}

/// Admit persisted results of this concrete copy adapter; generic contract policy stays in contracts.
pub(in crate::engine) fn admit_materialized_index(
    project: &Path,
    index: &AssetIndex,
    plan: &DirectorPlan,
) -> Result<()> {
    validate_asset_index(index, plan)?;
    if index.index_id != format!("INDEX-{}", index.director_plan_sha256) {
        return Err(Error::Contract(
            "INVALID_INVARIANT: internal-copy index identity differs from its plan".into(),
        ));
    }
    let fingerprint = internal_copy_fingerprint();
    let requests: std::collections::BTreeMap<_, _> = plan
        .asset_requests
        .iter()
        .map(|request| (request.asset_id.as_str(), request))
        .collect();
    for item in &index.assets {
        if item.backend_id != "internal-copy" || item.backend_fingerprint != fingerprint {
            return Err(Error::Contract(
                "STALE_RESULT: asset index uses a different materializer policy".into(),
            ));
        }
        let request = requests
            .get(item.source_request_id.as_str())
            .ok_or_else(|| {
                Error::Contract(format!(
                    "unknown source_request_id: {}",
                    item.source_request_id
                ))
            })?;
        let source = request.source.as_ref().ok_or_else(|| {
            Error::Contract("INVALID_ASSET_REQUEST: internal-copy source missing".into())
        })?;
        if !matches!(
            request.generation_mode,
            GenerationMode::User | GenerationMode::Derived
        ) || source.sha256 != item.sha256
            || source.mime.as_deref().is_some_and(|mime| mime != item.mime)
        {
            return Err(Error::Contract(format!(
                "ASSET_HASH_MISMATCH: internal-copy provenance differs for {}",
                item.asset_id
            )));
        }
        super::read_materialized_image(project, item)?;
    }
    Ok(())
}
