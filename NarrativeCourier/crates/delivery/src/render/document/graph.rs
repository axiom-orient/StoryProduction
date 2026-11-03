//! One graph projection owner shared by publication and semantic admission.

use narrative_courier_contracts::DeliveryPlan;
use narrative_courier_runtime::Result;
use serde_json::json;

pub(in crate::render) fn graph_value(delivery: &DeliveryPlan) -> serde_json::Value {
    let nodes: Vec<_> = delivery
        .units
        .iter()
        .map(|u| json!({"id":u.unit_id,"refs":u.narrative_refs,"content":u.content}))
        .collect();
    let edges: Vec<_> = delivery
        .units
        .windows(2)
        .map(|w| json!({"from":w[0].unit_id,"to":w[1].unit_id,"kind":"sequence"}))
        .collect();
    json!({"schema_version":1,"nodes":nodes,"edges":edges})
}

pub(in crate::render) fn graph_json(delivery: &DeliveryPlan) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(&graph_value(delivery))?)
}
