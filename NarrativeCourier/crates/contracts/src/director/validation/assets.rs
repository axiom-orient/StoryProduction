//! Plan-local asset request shape and source provenance.

use super::super::super::asset_types::GenerationMode;
use super::super::super::common::{
    is_narrative_ref, nonempty, nonempty_items, sha, unique, valid_asset_id,
};
use super::super::DirectorPlan;
use narrative_courier_runtime::paths::validate_relative_path;
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeSet;

pub(super) fn validate_assets(plan: &DirectorPlan) -> Result<BTreeSet<&str>> {
    let mut assets = BTreeSet::new();
    for a in &plan.asset_requests {
        if !assets.insert(a.asset_id.as_str()) {
            return Err(Error::Contract(format!(
                "duplicate asset_id: {}",
                a.asset_id
            )));
        }
        if !valid_asset_id(&a.asset_id) {
            return Err(Error::Contract(format!("invalid asset_id: {}", a.asset_id)));
        }
        nonempty(&a.role, "asset.role")?;
        nonempty(&a.brief, "asset.brief")?;
        nonempty_items(&a.hard_constraints, "asset hard_constraint")?;
        if a.narrative_refs.is_empty() {
            return Err(Error::Contract(format!(
                "{} has no narrative refs",
                a.asset_id
            )));
        }
        unique(a.narrative_refs.iter(), "asset narrative ref")?;
        for r in &a.narrative_refs {
            if !is_narrative_ref(r) {
                return Err(Error::Contract(format!("invalid narrative ref: {r}")));
            }
        }
        if matches!(
            a.generation_mode,
            GenerationMode::User | GenerationMode::Derived
        ) && a.source.is_none()
        {
            return Err(Error::Contract(format!(
                "{} requires source provenance",
                a.asset_id
            )));
        }
        if let Some(s) = &a.source {
            nonempty(&s.path, "asset.source.path")?;
            validate_relative_path(&s.path)?;
            sha(&s.sha256, "asset.source.sha256")?;
            if let Some(mime) = &s.mime {
                nonempty(mime, "asset.source.mime")?;
            }
        }
    }
    Ok(assets)
}
