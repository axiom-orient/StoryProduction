use std::collections::BTreeMap;

use crate::diagnostic::Diagnostic;
use crate::image::model::CanvasPlacement;

pub(super) fn validate_placement(
    placement: &CanvasPlacement,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let CanvasPlacement::Custom {
        x_percent,
        y_percent,
        width_percent,
        height_percent,
    } = placement
    {
        if *width_percent == 0 || *height_percent == 0 {
            diagnostics.push(Diagnostic::error(
                "IMG_PLACEMENT_SIZE",
                path,
                "custom placement width and height must be positive",
            ));
        }
        if u16::from(*x_percent) + u16::from(*width_percent) > 100
            || u16::from(*y_percent) + u16::from(*height_percent) > 100
        {
            diagnostics.push(Diagnostic::error(
                "IMG_PLACEMENT_BOUNDS",
                path,
                "custom placement must remain inside the 0..=100 canvas",
            ));
        }
    }
}

pub(super) fn register_id(
    ids: &mut BTreeMap<String, String>,
    id: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_nonempty(path, id, diagnostics);
    if let Some(previous) = ids.insert(id.to_owned(), path.to_owned()) {
        diagnostics.push(Diagnostic::error(
            "IMG_ELEMENT_ID_DUPLICATE",
            path,
            format!("element id {id:?} was already declared at {previous}"),
        ));
    }
}

pub(in crate::image::validate) fn validate_nonempty(
    path: &str,
    value: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if value.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_EMPTY_FIELD",
            path,
            "value must not be empty",
        ));
    }
}

pub(super) fn validate_hex(path: &str, value: &str, diagnostics: &mut Vec<Diagnostic>) {
    let bytes = value.as_bytes();
    if bytes.len() != 7
        || bytes.first() != Some(&b'#')
        || !bytes[1..].iter().all(u8::is_ascii_hexdigit)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_HEX",
            path,
            "expected a six-digit color such as #1A2B3C",
        ));
    }
}
