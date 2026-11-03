//! Request-local asset input identity and immutable source provenance.

use super::super::super::asset_types::GenerationMode;
use super::super::super::common::{is_narrative_ref, nonempty, sha, unique, valid_asset_id};
use super::super::DeliveryRequest;
use narrative_courier_runtime::paths::validate_relative_path;
use narrative_courier_runtime::{Error, Result};

pub(super) fn validate_assets(request: &DeliveryRequest) -> Result<()> {
    unique(
        request.asset_inputs.iter().map(|a| &a.asset_id),
        "asset_input asset_id",
    )?;
    for a in &request.asset_inputs {
        if !valid_asset_id(&a.asset_id) {
            return Err(Error::Contract(format!(
                "invalid asset input id: {}",
                a.asset_id
            )));
        }
        nonempty(&a.role, "asset_input.role")?;
        if a.narrative_refs.is_empty() {
            return Err(Error::Contract(format!(
                "{} has no narrative refs",
                a.asset_id
            )));
        }
        unique(a.narrative_refs.iter(), "asset_input narrative ref")?;
        for r in &a.narrative_refs {
            if !is_narrative_ref(r) {
                return Err(Error::Contract(format!("invalid narrative ref: {r}")));
            }
        }
        if !matches!(
            a.generation_mode,
            GenerationMode::User | GenerationMode::Derived
        ) {
            return Err(Error::Contract(format!(
                "{} delivery asset input must be USER or DERIVED",
                a.asset_id
            )));
        }
        nonempty(&a.source.path, "asset_input.source.path")?;
        validate_relative_path(&a.source.path)?;
        sha(&a.source.sha256, "asset_input.source.sha256")?;
        if let Some(mime) = &a.source.mime {
            nonempty(mime, "asset_input.source.mime")?;
        }
    }
    Ok(())
}
