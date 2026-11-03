//! Output scalar, exact ratio, and required raster/video field admission.

use super::super::super::common::{nonempty, nonempty_items, valid_aspect_ratio};
use super::super::{DeliveryRequest, OutputFormat};
use narrative_courier_runtime::paths::validate_path_segment;
use narrative_courier_runtime::{Error, Result};

pub(super) fn validate_outputs(request: &DeliveryRequest) -> Result<()> {
    for o in &request.outputs {
        validate_path_segment(&o.output_id, "output_id")?;
        nonempty_items(&o.constraints, "output constraint")?;
        if let Some(id) = &o.backend_id {
            nonempty(id, "backend_id")?;
        }
        if let Some(width) = o.width {
            if width == 0 {
                return Err(Error::Contract(format!(
                    "{} width must be >= 1",
                    o.output_id
                )));
            }
        }
        if let Some(height) = o.height {
            if height == 0 {
                return Err(Error::Contract(format!(
                    "{} height must be >= 1",
                    o.output_id
                )));
            }
        }
        if let Some(fps) = &o.fps {
            if fps.num == 0 || fps.den == 0 {
                return Err(Error::Contract(format!(
                    "{} fps numerator and denominator must be positive",
                    o.output_id
                )));
            }
        }
        if let Some(frames) = o.duration_frames {
            if frames == 0 {
                return Err(Error::Contract(format!(
                    "{} duration_frames must be >= 1",
                    o.output_id
                )));
            }
        }
        if let Some(ratio) = &o.aspect_ratio {
            if !valid_aspect_ratio(ratio) {
                return Err(Error::Contract(format!(
                    "{} invalid aspect_ratio",
                    o.output_id
                )));
            }
            let (rw, rh) = ratio.split_once(':').expect("validated aspect ratio");
            let rw = rw
                .parse::<u64>()
                .map_err(|_| Error::Contract(format!("{} invalid aspect_ratio", o.output_id)))?;
            let rh = rh
                .parse::<u64>()
                .map_err(|_| Error::Contract(format!("{} invalid aspect_ratio", o.output_id)))?;
            let width = o.width.ok_or_else(|| {
                Error::Contract(format!(
                    "{} aspect_ratio requires width and height",
                    o.output_id
                ))
            })? as u64;
            let height = o.height.ok_or_else(|| {
                Error::Contract(format!(
                    "{} aspect_ratio requires width and height",
                    o.output_id
                ))
            })? as u64;
            let lhs = width.checked_mul(rh).ok_or_else(|| {
                Error::Contract(format!(
                    "{} aspect_ratio overflows validation domain",
                    o.output_id
                ))
            })?;
            let rhs = height.checked_mul(rw).ok_or_else(|| {
                Error::Contract(format!(
                    "{} aspect_ratio overflows validation domain",
                    o.output_id
                ))
            })?;
            if lhs != rhs {
                return Err(Error::Contract(format!(
                    "{} dimensions do not match aspect_ratio",
                    o.output_id
                )));
            }
        }
        if matches!(o.format, OutputFormat::Png | OutputFormat::Webp)
            && (o.width.is_none() || o.height.is_none())
        {
            return Err(Error::Contract(format!(
                "{} requires width and height",
                o.output_id
            )));
        }
        if matches!(o.format, OutputFormat::Mp4 | OutputFormat::Webm)
            && (o.width.is_none()
                || o.height.is_none()
                || o.fps.is_none()
                || o.duration_frames.is_none())
        {
            return Err(Error::Contract(format!(
                "{} requires width, height, fps and duration_frames",
                o.output_id
            )));
        }
    }
    Ok(())
}
