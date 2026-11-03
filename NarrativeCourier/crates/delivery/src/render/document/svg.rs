//! Conservative bounded text layout for raster/video input, without silent clipping.

use super::html_escape;
use narrative_courier_contracts::DeliveryPlan;
use narrative_courier_runtime::{Error, Result};

fn xml_escape(s: &str) -> String {
    html_escape(s).replace('\'', "&apos;")
}

fn wrapped_lines(text: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.split('\n') {
        if raw.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut count = 0;
        for character in raw.chars() {
            if count == max_chars {
                out.push(std::mem::take(&mut line));
                count = 0;
            }
            line.push(character);
            count += 1;
        }
        if !line.is_empty() {
            out.push(line);
        }
    }
    out
}

pub(in crate::render) fn svg_document(
    delivery: &DeliveryPlan,
    width: u32,
    height: u32,
) -> Result<String> {
    let mut joined = String::new();
    for (index, unit) in delivery.units.iter().enumerate() {
        if !unit.asset_refs.is_empty() {
            return Err(Error::Contract(
                "UNSUPPORTED_CAPABILITY: text raster cannot represent attached assets; choose HTML"
                    .into(),
            ));
        }
        if index > 0 {
            joined.push_str("\n\n");
        }
        joined.push_str(unit.content.as_deref().unwrap_or(""));
    }
    if joined.chars().any(|c| matches!(c, '\t' | '\r') || !matches!(c, '\n' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')) {
        return Err(Error::Contract("UNSUPPORTED_CAPABILITY: raster text contains unsupported layout/XML characters; choose HTML".into()));
    }
    let max_chars = width.checked_sub(80).map(|pixels| pixels / 18).filter(|count| *count > 0)
        .ok_or_else(|| Error::Contract("UNSUPPORTED_CAPABILITY: raster width leaves no readable text area; increase width or choose HTML".into()))? as usize;
    let lines = wrapped_lines(&joined, max_chars);
    let capacity = height.checked_sub(80).map(|pixels| pixels / 24).filter(|count| *count > 0)
        .ok_or_else(|| Error::Contract("UNSUPPORTED_CAPABILITY: raster height leaves no readable text area; increase height or choose HTML".into()))? as usize;
    if lines.len() > capacity {
        return Err(Error::Contract(format!("UNSUPPORTED_CAPABILITY: requested raster size {width}x{height} exceeds the conservative text layout budget ({} lines > {capacity})", lines.len())));
    }
    let mut text = String::new();
    for (i, line) in lines.iter().enumerate() {
        text.push_str(&format!(
            "<text x=\"40\" y=\"{}\" font-size=\"18\" font-family=\"monospace\" xml:space=\"preserve\">{}</text>\n",
            48 + i as u32 * 24,
            xml_escape(line)
        ));
    }
    Ok(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/><g fill=\"#171717\">{text}</g></svg>"))
}
