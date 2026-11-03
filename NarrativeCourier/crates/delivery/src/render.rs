use crate::backend::BackendDescriptor;
use crate::project_owned_path;
use narrative_courier_contracts::{
    AssetIndex, DeliveryPlan, DeliveryReceipt, DeliveryStatus, DirectorPlan, OutputFormat,
    OutputRequest, PresentationStructure,
};
use narrative_courier_runtime::paths::{validate_path_segment, validate_relative_path};
use narrative_courier_runtime::store::atomic_publish_file;
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

mod cache;
mod document;
pub(super) mod process;
mod verification;

use cache::{output_build_key, receipt_for_artifact};
use document::{delivery_plan, extension, graph_json, html_document, svg_document};
use process::{check_cancel, run_external, RunControl, StageCleanup};
use verification::technical_verify;

pub fn render_output(
    project: &Path,
    plan: &DirectorPlan,
    asset_index: &AssetIndex,
    output: &OutputRequest,
    backend: &BackendDescriptor,
    control: &RunControl,
) -> Result<DeliveryReceipt> {
    validate_path_segment(&output.output_id, "output_id")?;
    let delivery = delivery_plan(plan, &output.output_id)?;
    let key = output_build_key(plan, delivery, output, backend, asset_index)?;
    let rel = format!(
        "visual/deliveries/{}/{}.{}",
        output.output_id,
        key,
        extension(output.format)
    );
    validate_relative_path(&rel)?;
    let parent_relative = format!("visual/deliveries/{}", output.output_id);
    let parent = project_owned_path(project, &parent_relative)?;
    fs::create_dir_all(&parent)?;
    let target = project_owned_path(project, &rel)?;

    // Existing content-addressed bytes are immutable. A failed admission must remain
    // visible; never delete an artifact and disguise that failure as regeneration.
    match fs::symlink_metadata(&target) {
        Ok(_) => {
            check_cancel(control)?;
            technical_verify(&target, output, control)?;
            let receipt = receipt_for_artifact(&target, rel.clone(), output, backend, delivery)?;
            if receipt.status == DeliveryStatus::Blocked {
                return Err(Error::Contract(
                    "CACHE_ARTIFACT_INVALID: cached artifact semantics differ from plan".into(),
                ));
            }
            check_cancel(control)?;
            return Ok(receipt);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let stage_root = project_owned_path(project, ".delivery-stage")?;
    fs::create_dir_all(&stage_root)?;
    let run_stage = project_owned_path(project, &format!(".delivery-stage/{}", control.run_id))?;
    fs::create_dir(&run_stage)?;
    let mut stage_cleanup = StageCleanup::new(run_stage);
    let rendered = (|| -> Result<DeliveryReceipt> {
        let stage_relative = format!(".delivery-stage/{}/{}", control.run_id, output.output_id);
        let stage = project_owned_path(project, &stage_relative)?;
        fs::create_dir(&stage)?;
        stage_cleanup.own_stage(stage.clone());
        let staged = stage.join(format!("artifact.{}", extension(output.format)));
        check_cancel(control)?;
        match (backend.backend_id.as_str(), output.format) {
            ("internal-html", OutputFormat::Html) => fs::write(
                &staged,
                html_document(project, plan, delivery, asset_index)?,
            )?,
            ("internal-graph", OutputFormat::Json) => fs::write(&staged, graph_json(delivery)?)?,
            ("weasyprint", OutputFormat::Pdf) => {
                let html = stage.join("source.html");
                fs::write(&html, html_document(project, plan, delivery, asset_index)?)?;
                let mut c = Command::new("weasyprint");
                c.arg(&html).arg(&staged);
                run_external(c, 120, control)?;
            }
            ("imagemagick", OutputFormat::Png | OutputFormat::Webp) => {
                let width = output
                    .width
                    .ok_or_else(|| Error::Contract("image width missing".into()))?;
                let height = output
                    .height
                    .ok_or_else(|| Error::Contract("image height missing".into()))?;
                let svg = stage.join("source.svg");
                fs::write(&svg, svg_document(delivery, width, height)?)?;
                let mut c = Command::new("magick");
                c.arg(&svg).arg(&staged);
                run_external(c, 120, control)?;
            }
            ("ffmpeg-text-video", OutputFormat::Mp4) => {
                let width = output
                    .width
                    .ok_or_else(|| Error::Contract("video width missing".into()))?;
                let height = output
                    .height
                    .ok_or_else(|| Error::Contract("video height missing".into()))?;
                let fps = output
                    .fps
                    .as_ref()
                    .ok_or_else(|| Error::Contract("video fps missing".into()))?;
                let total = output
                    .duration_frames
                    .ok_or_else(|| Error::Contract("video duration_frames missing".into()))?;
                let frames_dir = stage.join("frames");
                fs::create_dir_all(&frames_dir)?;
                let mut frame_no = 1u64;
                for unit in &delivery.units {
                    check_cancel(control)?;
                    let one = DeliveryPlan {
                        output_request_id: delivery.output_request_id.clone(),
                        presentation_structure: PresentationStructure::Paged,
                        units: vec![unit.clone()],
                    };
                    let svg_path = stage.join(format!("unit-{:06}.svg", unit.order));
                    let png_path = stage.join(format!("unit-{:06}.png", unit.order));
                    fs::write(&svg_path, svg_document(&one, width, height)?)?;
                    let mut c = Command::new("magick");
                    c.arg(&svg_path).arg(&png_path);
                    run_external(c, 120, control)?;
                    for _ in 0..unit.duration_frames.ok_or_else(|| {
                        Error::Contract("video unit duration_frames missing".into())
                    })? {
                        check_cancel(control)?;
                        let frame = frames_dir.join(format!("frame{frame_no:06}.png"));
                        if fs::hard_link(&png_path, &frame).is_err() {
                            fs::copy(&png_path, &frame)?;
                        }
                        frame_no += 1;
                    }
                }
                if frame_no - 1 != u64::from(total) {
                    return Err(Error::Contract(
                        "TECHNICAL_FAIL: generated frame count differs from plan".into(),
                    ));
                }
                let pattern = frames_dir.join("frame%06d.png");
                let framerate = format!("{}/{}", fps.num, fps.den);
                let frame_count = total.to_string();
                let mut c = Command::new("ffmpeg");
                c.args(["-y", "-v", "error", "-framerate", framerate.as_str(), "-i"])
                    .arg(&pattern)
                    .args([
                        "-frames:v",
                        frame_count.as_str(),
                        "-c:v",
                        "libx264",
                        "-pix_fmt",
                        "yuv420p",
                    ])
                    .arg(&staged);
                run_external(c, 300, control)?;
            }
            (backend_id, format) => {
                return Err(Error::Contract(format!(
                    "UNSUPPORTED_CAPABILITY: backend {backend_id} cannot render {format:?}"
                )))
            }
        }
        check_cancel(control)?;
        technical_verify(&staged, output, control)?;
        let staged_receipt = receipt_for_artifact(&staged, rel.clone(), output, backend, delivery)?;
        if staged_receipt.status == DeliveryStatus::Blocked {
            return Err(Error::Contract(
                "ARTIFACT_INVALID: rendered semantics differ from plan".into(),
            ));
        }
        check_cancel(control)?;
        atomic_publish_file(&staged, &target)?;
        technical_verify(&target, output, control)?;
        let receipt = receipt_for_artifact(&target, rel, output, backend, delivery)?;
        if receipt.artifact.sha256 != staged_receipt.artifact.sha256
            || receipt.status == DeliveryStatus::Blocked
        {
            return Err(Error::Contract(
                "ARTIFACT_CHANGED: published artifact differs from admitted stage".into(),
            ));
        }
        check_cancel(control)?;
        Ok(receipt)
    })();
    let cleanup = stage_cleanup.finish();
    match (rendered, cleanup) {
        (Ok(receipt), Ok(())) => Ok(receipt),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(Error::Executor(format!("RENDER_CLEANUP_FAILED: {error}"))),
        (Err(error), Err(cleanup)) => Err(Error::Executor(format!(
            "{error}; RENDER_CLEANUP_FAILED: {cleanup}"
        ))),
    }
}
