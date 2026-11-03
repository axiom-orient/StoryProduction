use crate::diagnostic::Diagnostic;
use crate::image::model::ImagePromptRequest;

use super::super::{EXPERIMENTAL_PIXELS, MAX_EDGE, MAX_PIXELS, MIN_PIXELS};

pub(in crate::image::validate) fn validate_output(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let output = &request.output;
    if output.model != "gpt-image-2" {
        diagnostics.push(Diagnostic::error(
            "IMG_MODEL",
            "$.output.model",
            "this compiler profile currently supports gpt-image-2 only",
        ));
    }
    if output.format != "png" {
        diagnostics.push(Diagnostic::error(
            "IMG_FORMAT",
            "$.output.format",
            "Codex artifact verification currently supports png only",
        ));
    }
    if output.width == 0 || output.height == 0 {
        diagnostics.push(Diagnostic::error(
            "IMG_DIMENSION_ZERO",
            "$.output",
            "width and height must be positive",
        ));
        return;
    }
    if !output.width.is_multiple_of(16) || !output.height.is_multiple_of(16) {
        diagnostics.push(Diagnostic::error(
            "IMG_DIMENSION_MULTIPLE",
            "$.output",
            "gpt-image-2 width and height must be multiples of 16",
        ));
    }
    if output.width > MAX_EDGE || output.height > MAX_EDGE {
        diagnostics.push(Diagnostic::error(
            "IMG_DIMENSION_EDGE",
            "$.output",
            format!("each edge must be less than or equal to {MAX_EDGE} pixels"),
        ));
    }
    let pixels = u64::from(output.width) * u64::from(output.height);
    if !(MIN_PIXELS..=MAX_PIXELS).contains(&pixels) {
        diagnostics.push(Diagnostic::error(
            "IMG_PIXEL_COUNT",
            "$.output",
            format!("pixel count must be within {MIN_PIXELS}..={MAX_PIXELS}, found {pixels}"),
        ));
    } else if pixels > EXPERIMENTAL_PIXELS {
        diagnostics.push(Diagnostic::warning(
            "IMG_EXPERIMENTAL_SIZE",
            "$.output",
            "this size is within hard limits but above the documented standard range",
        ));
    }
    let ratio = output.width as f64 / output.height as f64;
    if !(1.0 / 3.0..=3.0).contains(&ratio) {
        diagnostics.push(Diagnostic::error(
            "IMG_ASPECT_RATIO",
            "$.output",
            "aspect ratio must be between 1:3 and 3:1",
        ));
    }
}
