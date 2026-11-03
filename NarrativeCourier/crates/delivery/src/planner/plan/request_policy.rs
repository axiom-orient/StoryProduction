//! Fail-closed capability admission and ordered scene selection policy.

use super::super::scene::Scene;
use narrative_courier_contracts::{
    AssetKind, Coverage, DeliveryIntent, DeliveryRequest, OutputFormat,
};
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeMap;

pub(crate) fn validate_supported_request(request: &DeliveryRequest) -> Result<()> {
    if request.intent != DeliveryIntent::Presentation {
        return Err(Error::Contract(format!(
            "UNSUPPORTED_CAPABILITY: deterministic DirectorIR currently supports PRESENTATION intent only, not {:?}",
            request.intent
        )));
    }
    if request.audience.is_some() {
        return Err(Error::Contract(
            "UNSUPPORTED_CAPABILITY: audience adaptation requires a reviewed DirectorIR planner"
                .into(),
        ));
    }
    if request.locale.is_some() {
        return Err(Error::Contract(
            "UNSUPPORTED_CAPABILITY: locale adaptation requires a reviewed DirectorIR planner"
                .into(),
        ));
    }
    if !request.constraints.is_empty()
        || request
            .outputs
            .iter()
            .any(|output| !output.constraints.is_empty())
    {
        return Err(Error::Contract("UNSUPPORTED_CAPABILITY: free-form delivery constraints require a reviewed DirectorIR planner".into()));
    }
    if let Some(reference) = request
        .must_include_refs
        .iter()
        .find(|reference| !reference.starts_with("SCN-"))
    {
        return Err(Error::Contract(format!(
            "UNSUPPORTED_CAPABILITY: deterministic DirectorIR cannot prove non-SCN must_include_ref {reference}"
        )));
    }
    if let Some(reference) = request
        .asset_inputs
        .iter()
        .flat_map(|asset| asset.narrative_refs.iter())
        .find(|reference| !reference.starts_with("SCN-"))
    {
        return Err(Error::Contract(format!(
            "UNSUPPORTED_CAPABILITY: deterministic DirectorIR currently scopes USER/DERIVED assets by SCN-* only, not {reference}"
        )));
    }
    if let Some(asset) = request
        .asset_inputs
        .iter()
        .find(|asset| asset.kind != AssetKind::Image)
    {
        return Err(Error::Contract(format!("UNSUPPORTED_CAPABILITY: deterministic presentation cannot represent {:?} asset {}; only image attachments in HTML are supported", asset.kind, asset.asset_id)));
    }
    if !request.asset_inputs.is_empty()
        && request
            .outputs
            .iter()
            .any(|output| output.format != OutputFormat::Html)
    {
        return Err(Error::Contract(
            "UNSUPPORTED_CAPABILITY: image asset inputs are supported only by HTML delivery; choose HTML before running".into(),
        ));
    }
    for output in &request.outputs {
        if !matches!(
            output.format,
            OutputFormat::Html
                | OutputFormat::Json
                | OutputFormat::Pdf
                | OutputFormat::Png
                | OutputFormat::Webp
                | OutputFormat::Mp4
        ) {
            return Err(Error::Contract(format!("UNSUPPORTED_CAPABILITY: deterministic presentation has no {:?} output implementation", output.format)));
        }
        if matches!(
            output.format,
            OutputFormat::Html | OutputFormat::Json | OutputFormat::Pdf
        ) && (output.width.is_some() || output.height.is_some() || output.aspect_ratio.is_some())
        {
            return Err(Error::Contract(format!(
                "UNSUPPORTED_CAPABILITY: {} {:?} output does not consume raster geometry",
                output.output_id, output.format
            )));
        }
        if output.format != OutputFormat::Mp4
            && (output.fps.is_some() || output.duration_frames.is_some())
        {
            return Err(Error::Contract(format!(
                "UNSUPPORTED_CAPABILITY: {} static output does not consume fps/duration_frames",
                output.output_id
            )));
        }
    }
    Ok(())
}

pub(in crate::planner) fn selected_scenes<'a>(
    scenes: &'a [Scene],
    request: &DeliveryRequest,
) -> Result<Vec<&'a Scene>> {
    let all: BTreeMap<_, _> = scenes
        .iter()
        .map(|scene| (scene.id.as_str(), scene))
        .collect();
    for reference in &request.must_include_refs {
        if reference.starts_with("SCN-") && !all.contains_key(reference.as_str()) {
            return Err(Error::Contract(format!(
                "LINEAGE_MISSING: requested scene not found: {reference}"
            )));
        }
    }
    match request.coverage {
        Coverage::Full => Ok(scenes.iter().collect()),
        Coverage::Selection | Coverage::Excerpt => {
            let mut selected = Vec::new();
            for id in request
                .must_include_refs
                .iter()
                .filter(|reference| reference.starts_with("SCN-"))
            {
                selected.push(
                    all.get(id.as_str())
                        .copied()
                        .ok_or_else(|| Error::Contract(format!("LINEAGE_MISSING: {id}")))?,
                );
            }
            if selected.is_empty() {
                return Err(Error::Contract(
                    "SELECTION/EXCERPT requires must_include_refs with at least one SCN-*".into(),
                ));
            }
            Ok(selected)
        }
        Coverage::Summary | Coverage::Mood => Err(Error::Contract(
            "UNSUPPORTED_CAPABILITY: deterministic DirectorIR does not fabricate SUMMARY/MOOD; use an explicit reviewed planner when that capability is implemented".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_supported_request;
    use narrative_courier_contracts::{
        AssetKind, AssetSource, Coverage, DeliveryAssetInput, DeliveryIntent, DeliveryRequest,
        GenerationMode, OutputFormat, OutputRequest,
    };

    fn image_request(format: OutputFormat) -> DeliveryRequest {
        DeliveryRequest {
            schema_version: 1,
            request_id: "REQ-IMAGE-GUARD".into(),
            narrative_package_sha256: "a".repeat(64),
            intent: DeliveryIntent::Presentation,
            coverage: Coverage::Selection,
            audience: None,
            locale: None,
            must_include_refs: vec!["SCN-001".into()],
            constraints: vec![],
            asset_inputs: vec![DeliveryAssetInput {
                asset_id: "IMG-TEST".into(),
                kind: AssetKind::Image,
                role: "approved external visual".into(),
                narrative_refs: vec!["SCN-001".into()],
                generation_mode: GenerationMode::User,
                source: AssetSource {
                    path: "visual/assets/imports/test.png".into(),
                    sha256: "b".repeat(64),
                    mime: Some("image/png".into()),
                },
            }],
            outputs: vec![OutputRequest {
                output_id: "OUT-IMAGE-GUARD".into(),
                format,
                backend_id: None,
                width: None,
                height: None,
                aspect_ratio: None,
                fps: None,
                duration_frames: None,
                constraints: vec![],
            }],
        }
    }

    #[test]
    fn image_assets_require_html_before_any_delivery_side_effect() {
        assert!(validate_supported_request(&image_request(OutputFormat::Png)).is_err());
        assert!(validate_supported_request(&image_request(OutputFormat::Mp4)).is_err());
        assert!(validate_supported_request(&image_request(OutputFormat::Html)).is_ok());
    }
}
