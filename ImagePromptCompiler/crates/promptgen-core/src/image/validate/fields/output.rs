use crate::diagnostic::Diagnostic;
use crate::image::model::ImagePromptRequest;

use super::super::{EXPERIMENTAL_PIXELS, MAX_EDGE, MAX_PIXELS, MIN_PIXELS};

pub(in crate::image::validate) fn validate_output(
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let output = &request.output;
    let animated_model = matches!(
        output.model.as_str(),
        "gpt-image-2.6-sunburst" | "gpt-image-2.6-flare"
    );
    if output.model != "gpt-image-2" && !animated_model {
        diagnostics.push(Diagnostic::error(
            "IMG_MODEL",
            "$.output.model",
            "supported models: gpt-image-2, gpt-image-2.6-sunburst, gpt-image-2.6-flare",
        ));
    }
    if output.format != "png" && !(animated_model && output.format == "gif") {
        diagnostics.push(Diagnostic::error(
            "IMG_FORMAT",
            "$.output.format",
            "GIF requires GPT Image 2.6; PNG remains supported",
        ));
    }
    if output
        .size_policy
        .as_deref()
        .is_some_and(|p| p != "exact" && !(p == "provider" && output.format == "gif"))
    {
        diagnostics.push(Diagnostic::error(
            "IMG_SIZE_POLICY",
            "$.output.size_policy",
            "provider size policy is allowed only for GIF output",
        ));
    }
    if output.format == "gif" {
        if !output.frame_count.is_some_and(|n| (2..=16).contains(&n))
            || !output
                .frame_duration_ms
                .is_some_and(|n| (20..=2000).contains(&n) && n.is_multiple_of(10))
        {
            diagnostics.push(Diagnostic::error(
                "IMG_GIF_TIMING",
                "$.output",
                "GIF requires frame_count=2..16 and frame_duration_ms=20..2000 in 10 ms increments",
            ));
        }
    } else if output.frame_count.is_some() || output.frame_duration_ms.is_some() {
        diagnostics.push(Diagnostic::error(
            "IMG_ANIMATION_REQUIRES_GIF",
            "$.output",
            "frame timing is allowed only with format=gif",
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
    let minimum = if output.format == "gif" {
        256
    } else {
        MIN_PIXELS
    };
    if !(minimum..=MAX_PIXELS).contains(&pixels) {
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
    // Conservative local decoded-frame budget, not a claim about provider entitlement.
    if output.format == "gif" && pixels * u64::from(output.frame_count.unwrap_or(0)) > 16_777_216 {
        diagnostics.push(Diagnostic::error(
            "IMG_GIF_PIXEL_BUDGET",
            "$.output",
            "local GIF profile limits total decoded pixels to 16777216",
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
