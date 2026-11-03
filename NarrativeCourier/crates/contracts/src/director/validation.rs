//! Cross-field DirectorPlan invariants bound to its DeliveryRequest.

use narrative_courier_runtime::{Error, Result};

use super::super::common::{nonempty, nonempty_items, sha};
use super::super::request::{validate_delivery_request, DeliveryRequest};
use super::super::SCHEMA_VERSION;
use super::DirectorPlan;
use narrative_courier_runtime::token::hash_jcs;

mod assets;
mod deliveries;

pub fn validate_director_plan(v: &DirectorPlan, request: &DeliveryRequest) -> Result<()> {
    validate_delivery_request(request)?;
    if v.schema_version != SCHEMA_VERSION {
        return Err(Error::Contract(
            "DirectorPlan schema_version must be 1".into(),
        ));
    }
    nonempty(&v.plan_id, "plan_id")?;
    sha(&v.narrative_package_sha256, "narrative_package_sha256")?;
    sha(&v.delivery_request_sha256, "delivery_request_sha256")?;
    sha(&v.director_bundle_sha256, "director_bundle_sha256")?;
    if v.narrative_package_sha256 != request.narrative_package_sha256
        || v.delivery_request_sha256 != hash_jcs(request)?
    {
        return Err(Error::Contract(
            "HASH_MISMATCH: DirectorPlan does not target supplied DeliveryRequest/NarrativePackage"
                .into(),
        ));
    }
    if v.visual_grammar.is_empty() {
        return Err(Error::Contract("visual_grammar must not be empty".into()));
    }
    const GRAMMAR_KEYS: &[&str] = &[
        "composition",
        "light",
        "material",
        "camera",
        "motion",
        "typography",
        "sound",
    ];
    for (key, axis) in &v.visual_grammar {
        if !GRAMMAR_KEYS.contains(&key.as_str()) {
            return Err(Error::Contract(format!(
                "unknown visual_grammar axis: {key}"
            )));
        }
        nonempty_items(&axis.must, "visual_grammar.must")?;
        nonempty_items(&axis.avoid, "visual_grammar.avoid")?;
    }
    let assets = assets::validate_assets(v)?;
    deliveries::validate_deliveries(v, request, &assets)
}
