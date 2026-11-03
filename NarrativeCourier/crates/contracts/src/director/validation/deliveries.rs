//! Ordered unit, lineage, and exact frame-budget admission.

use super::super::super::common::{is_narrative_ref, nonempty, nonempty_items, unique};
use super::super::super::request::{DeliveryRequest, OutputFormat};
use super::super::{DirectorPlan, PresentationStructure, ProjectionOperation};
use narrative_courier_runtime::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_deliveries(
    v: &DirectorPlan,
    request: &DeliveryRequest,
    assets: &BTreeSet<&str>,
) -> Result<()> {
    if v.deliveries.len() != request.outputs.len() {
        return Err(Error::Contract(
            "DirectorPlan deliveries must match DeliveryRequest outputs".into(),
        ));
    }
    unique(
        v.deliveries.iter().map(|d| &d.output_request_id),
        "plan output_request_id",
    )?;
    let request_map: BTreeMap<_, _> = request
        .outputs
        .iter()
        .map(|o| (o.output_id.as_str(), o))
        .collect();
    let mut global_units = BTreeSet::new();
    for d in &v.deliveries {
        let out = request_map
            .get(d.output_request_id.as_str())
            .ok_or_else(|| {
                Error::Contract(format!(
                    "unknown output_request_id: {}",
                    d.output_request_id
                ))
            })?;
        if matches!(out.format, OutputFormat::Mp4 | OutputFormat::Webm)
            && d.presentation_structure != PresentationStructure::Timed
        {
            return Err(Error::Contract(format!(
                "{} video delivery requires TIMED structure",
                d.output_request_id
            )));
        }
        if d.presentation_structure == PresentationStructure::Timed && out.fps.is_none() {
            return Err(Error::Contract(format!(
                "{} timed delivery requires requested fps",
                d.output_request_id
            )));
        }
        if d.units.is_empty() {
            return Err(Error::Contract(format!(
                "{} has no units",
                d.output_request_id
            )));
        }

        let mut frame_sum: u64 = 0;
        for (index, u) in d.units.iter().enumerate() {
            nonempty(&u.unit_id, "unit_id")?;
            nonempty(&u.direction, "unit.direction")?;
            if !global_units.insert(&u.unit_id) {
                return Err(Error::Contract(format!(
                    "duplicate global unit_id: {}",
                    u.unit_id
                )));
            }
            let expected_order = u32::try_from(index)
                .map_err(|_| Error::Contract("unit count exceeds u32 capacity".into()))?;
            if u.order != expected_order {
                return Err(Error::Contract(format!(
                    "{} unit order must match array position from 0",
                    d.output_request_id
                )));
            }
            if u.narrative_refs.is_empty() {
                return Err(Error::Contract(format!(
                    "{} has no narrative refs",
                    u.unit_id
                )));
            }
            unique(u.narrative_refs.iter(), "unit narrative ref")?;
            for r in &u.narrative_refs {
                if !is_narrative_ref(r) {
                    return Err(Error::Contract(format!("invalid narrative ref: {r}")));
                }
            }
            if u.operation != ProjectionOperation::Omit
                && u.content.as_deref().unwrap_or("").trim().is_empty()
            {
                return Err(Error::Contract(format!("{} requires content", u.unit_id)));
            }
            if let Some(content) = &u.content {
                nonempty(content, "unit.content")?;
            }
            if let Some(frames) = u.duration_frames {
                if frames == 0 {
                    return Err(Error::Contract(format!(
                        "{} duration_frames must be >= 1",
                        u.unit_id
                    )));
                }
            }
            unique(u.asset_refs.iter(), "unit asset_ref")?;
            nonempty_items(&u.asset_refs, "unit asset_ref")?;
            for a in &u.asset_refs {
                if !assets.contains(a.as_str()) {
                    return Err(Error::Contract(format!("dangling asset ref: {a}")));
                }
            }
            if d.presentation_structure == PresentationStructure::Timed {
                let frames = u.duration_frames.ok_or_else(|| {
                    Error::Contract(format!("{} requires duration_frames", u.unit_id))
                })?;
                frame_sum = frame_sum
                    .checked_add(u64::from(frames))
                    .ok_or_else(|| Error::Contract("unit frame sum exceeds u64 capacity".into()))?;
            }
        }
        if d.presentation_structure == PresentationStructure::Timed
            && frame_sum
                != u64::from(out.duration_frames.ok_or_else(|| {
                    Error::Contract(format!(
                        "{} timed delivery requires requested duration_frames",
                        d.output_request_id
                    ))
                })?)
        {
            return Err(Error::Contract(format!(
                "{} frame sum mismatch",
                d.output_request_id
            )));
        }
    }
    Ok(())
}
