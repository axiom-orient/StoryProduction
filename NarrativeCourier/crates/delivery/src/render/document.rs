//! Format metadata, plan lookup, and explicit representation composition.

use narrative_courier_contracts::{DeliveryPlan, DirectorPlan, OutputFormat};
use narrative_courier_runtime::{Error, Result};

mod graph;
mod html;
mod svg;

pub(super) use graph::{graph_json, graph_value};
pub(super) use html::html_document;
pub(super) use svg::svg_document;

pub(super) fn extension(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Html => "html",
        OutputFormat::Png => "png",
        OutputFormat::Webp => "webp",
        OutputFormat::Svg => "svg",
        OutputFormat::Pdf => "pdf",
        OutputFormat::Mp4 => "mp4",
        OutputFormat::Webm => "webm",
        OutputFormat::Json => "json",
        OutputFormat::Otio => "otio",
        OutputFormat::Pptx => "pptx",
    }
}
pub(super) fn mime(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Html => "text/html",
        OutputFormat::Png => "image/png",
        OutputFormat::Webp => "image/webp",
        OutputFormat::Svg => "image/svg+xml",
        OutputFormat::Pdf => "application/pdf",
        OutputFormat::Mp4 => "video/mp4",
        OutputFormat::Webm => "video/webm",
        OutputFormat::Json => "application/json",
        OutputFormat::Otio => "application/json",
        OutputFormat::Pptx => {
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        }
    }
}

pub(super) fn html_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for character in s.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

pub(super) fn delivery_plan<'a>(
    plan: &'a DirectorPlan,
    output_id: &str,
) -> Result<&'a DeliveryPlan> {
    plan.deliveries
        .iter()
        .find(|d| d.output_request_id == output_id)
        .ok_or_else(|| Error::Contract(format!("missing delivery plan: {output_id}")))
}
