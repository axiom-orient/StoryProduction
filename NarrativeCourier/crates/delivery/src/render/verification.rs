//! Artifact-byte semantic and technical verification for renderer outputs.

use super::document::{graph_value, html_escape};
use super::process::RunControl;
use super::process::{ffprobe_video, identify_dimensions};
use narrative_courier_contracts::{
    DeliveryPlan, DeliveryStatus, DeliveryVerification, OutputFormat, OutputRequest,
    VerificationLayer, VerificationStatus,
};
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::Path;

fn exact_html_semantics(path: &Path, delivery: &DeliveryPlan) -> Result<bool> {
    let html = fs::read_to_string(path)?;
    let mut cursor = 0usize;
    for unit in &delivery.units {
        let marker = format!("<section data-unit=\"{}\">", html_escape(&unit.unit_id));
        let Some(section_at) = html[cursor..].find(&marker).map(|i| cursor + i) else {
            return Ok(false);
        };
        let expected = format!(
            "<pre>{}</pre>",
            html_escape(unit.content.as_deref().unwrap_or(""))
        );
        if html[cursor..section_at].contains("<section data-unit=") {
            return Ok(false);
        }
        let body_start = section_at + marker.len();
        let Some(section_end) = html[body_start..]
            .find("</section>")
            .map(|i| body_start + i)
        else {
            return Ok(false);
        };
        if !html[body_start..section_end].ends_with(&expected) {
            return Ok(false);
        }
        cursor = section_end + "</section>".len();
    }
    Ok(!html[cursor..].contains("<section data-unit="))
}

fn exact_graph_semantics(path: &Path, delivery: &DeliveryPlan) -> Result<bool> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let expected = graph_value(delivery);
    Ok(value == expected)
}

pub(super) fn verification_for(
    format: OutputFormat,
    artifact_path: &Path,
    delivery: &DeliveryPlan,
) -> Result<(DeliveryVerification, DeliveryStatus)> {
    let pass = VerificationLayer {
        status: VerificationStatus::Pass,
        findings: vec![],
    };
    let not_applicable = VerificationLayer {
        status: VerificationStatus::NotApplicable,
        findings: vec![],
    };
    match format {
        OutputFormat::Html => {
            let semantic_ok = exact_html_semantics(artifact_path, delivery)?;
            let has_assets = delivery.units.iter().any(|u| !u.asset_refs.is_empty());
            let semantic = if !semantic_ok {
                VerificationLayer { status: VerificationStatus::Fail, findings: vec!["HTML artifact does not exactly preserve planned unit content".into()] }
            } else if has_assets {
                VerificationLayer { status: VerificationStatus::Warn, findings: vec!["attached USER/DERIVED asset semantics require Human preview".into()] }
            } else { pass.clone() };
            let status = if semantic_ok { DeliveryStatus::NeedsReview } else { DeliveryStatus::Blocked };
            Ok((DeliveryVerification { semantic, visual: VerificationLayer { status: VerificationStatus::Warn, findings: vec!["browser visual qualification requires preview".into()] }, technical: pass }, status))
        }
        OutputFormat::Json => {
            let semantic_ok = exact_graph_semantics(artifact_path, delivery)?;
            let semantic = if semantic_ok { pass.clone() } else { VerificationLayer { status: VerificationStatus::Fail, findings: vec!["graph artifact does not exactly preserve planned nodes/lineage/content".into()] } };
            let status = if semantic_ok { DeliveryStatus::Pass } else { DeliveryStatus::Blocked };
            Ok((DeliveryVerification { semantic, visual: not_applicable, technical: pass }, status))
        }
        OutputFormat::Pdf => {
            let semantic = match narrative_courier_runtime::document::extract_document_text(artifact_path) {
                Ok(text) if delivery.units.iter().all(|unit| {
                    unit.narrative_refs.iter().all(|reference| text.contains(reference))
                }) => pass.clone(),
                Ok(_) => VerificationLayer {
                    status: VerificationStatus::Warn,
                    findings: vec!["PDF text extraction could not prove every SCN lineage marker; preview required".into()],
                },
                Err(error) => VerificationLayer {
                    status: VerificationStatus::Warn,
                    findings: vec![format!("PDF text extraction failed: {error}; preview required")],
                },
            };
            Ok((DeliveryVerification { semantic, visual: VerificationLayer { status: VerificationStatus::Warn, findings: vec!["PDF visual layout requires preview".into()] }, technical: pass }, DeliveryStatus::NeedsReview))
        }
        OutputFormat::Png | OutputFormat::Webp | OutputFormat::Mp4 | OutputFormat::Webm => Ok((DeliveryVerification {
            semantic: VerificationLayer { status: VerificationStatus::Warn, findings: vec!["raster/video observable semantics require human preview; deterministic input lineage is preserved".into()] },
            visual: VerificationLayer { status: VerificationStatus::Warn, findings: vec!["visual result requires preview".into()] },
            technical: pass,
        }, DeliveryStatus::NeedsReview)),
        _ => Err(Error::Contract(format!("UNSUPPORTED_CAPABILITY: verification not implemented for {format:?}"))),
    }
}

pub(super) fn technical_verify(
    path: &Path,
    output: &OutputRequest,
    control: &RunControl,
) -> Result<()> {
    super::process::check_cancel(control)?;
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
        return Err(Error::Contract(
            "TECHNICAL_FAIL: artifact missing or empty".into(),
        ));
    }
    match output.format {
        OutputFormat::Html => {
            let s = fs::read_to_string(path)?;
            if !s.contains("<!doctype html>") {
                return Err(Error::Contract("TECHNICAL_FAIL: invalid HTML".into()));
            }
        }
        OutputFormat::Json => {
            let _: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
        }
        OutputFormat::Pdf => {
            if !fs::read(path)?.starts_with(b"%PDF-") {
                return Err(Error::Contract("TECHNICAL_FAIL: invalid PDF header".into()));
            }
        }
        OutputFormat::Png | OutputFormat::Webp => {
            let (w, h, format) = identify_dimensions(path, control)?;
            let expected_format = if output.format == OutputFormat::Png {
                "PNG"
            } else {
                "WEBP"
            };
            if format != expected_format || Some(w) != output.width || Some(h) != output.height {
                return Err(Error::Contract(format!(
                    "TECHNICAL_FAIL: image dimensions {w}x{h} do not match request"
                )));
            }
        }
        OutputFormat::Mp4 => {
            let video = ffprobe_video(path, control)?;
            let fps = output.fps.as_ref().ok_or_else(|| {
                Error::Contract("TECHNICAL_FAIL: requested video fps missing".into())
            })?;
            if Some(video.width) != output.width
                || Some(video.height) != output.height
                || Some(video.frames) != output.duration_frames
                || u64::from(video.fps_num) * u64::from(fps.den)
                    != u64::from(fps.num) * u64::from(video.fps_den)
            {
                return Err(Error::Contract(
                    "TECHNICAL_FAIL: video dimensions/frames/rate do not match request".into(),
                ));
            }
        }
        _ => {
            return Err(Error::Contract(format!(
                "UNSUPPORTED_CAPABILITY: technical verifier not implemented for {:?}",
                output.format
            )))
        }
    }
    super::process::check_cancel(control)
}
