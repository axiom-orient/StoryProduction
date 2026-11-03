//! DeliveryRequest cross-field, path, and output capability-independent invariants.

use narrative_courier_runtime::paths::validate_path_segment;
use narrative_courier_runtime::{Error, Result};

use super::super::common::{is_narrative_ref, nonempty, nonempty_items, sha, unique};
use super::super::SCHEMA_VERSION;
use super::DeliveryRequest;

mod assets;
mod outputs;

pub fn validate_delivery_request(v: &DeliveryRequest) -> Result<()> {
    if v.schema_version != SCHEMA_VERSION {
        return Err(Error::Contract(
            "DeliveryRequest schema_version must be 1".into(),
        ));
    }
    validate_path_segment(&v.request_id, "request_id")?;
    sha(&v.narrative_package_sha256, "narrative_package_sha256")?;
    if v.outputs.is_empty() {
        return Err(Error::Contract("outputs must not be empty".into()));
    }
    unique(v.outputs.iter().map(|o| &o.output_id), "output_id")?;
    unique(v.must_include_refs.iter(), "must_include_ref")?;
    for r in &v.must_include_refs {
        if !is_narrative_ref(r) {
            return Err(Error::Contract(format!("invalid narrative ref: {r}")));
        }
    }
    nonempty_items(&v.constraints, "request constraint")?;
    for (field, value) in [("audience", &v.audience), ("locale", &v.locale)] {
        if let Some(value) = value {
            nonempty(value, field)?;
        }
    }
    assets::validate_assets(v)?;
    outputs::validate_outputs(v)
}
