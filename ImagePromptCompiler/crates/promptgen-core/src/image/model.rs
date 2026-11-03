use std::collections::BTreeMap;

use crate::json::{DecodeError, JsonValue, take_u64};

mod continuity;
mod request;
mod request_parts;
mod scene;
mod visual;

pub use self::continuity::{
    ImageConsistencyDimension, ImageConsistencySequence, ImageConsistencySpec,
};
pub use self::request::ImagePromptRequest;
pub use self::request_parts::{
    BackgroundMode, ImageChangeContract, ImageConstraints, ImageGenerationParameters, ImageProfile,
    ImageQuality, ImageReference, ImageReferenceRole, ImageRenderProfile, ImageTaskMode,
    TextElement, TextRole,
};
pub use self::scene::{
    CanvasPlacement, CanvasZone, CinematicStoryboardPanel, CinematicStoryboardSpec,
    CompositionSpec, SceneSpec, SubjectSpec, TaxonomySelection, VisualMedium,
};
pub use self::visual::{
    CameraSpec, ColorSpec, LightingSpec, PaletteEntry, PhotoLutSelection, SurfaceSpec,
};

fn take_u8(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<u8, DecodeError> {
    let value = take_u64(fields, key, path)?;
    decode_u8(value, path, key)
}

fn decode_u8(value: u64, path: &str, key: &str) -> Result<u8, DecodeError> {
    u8::try_from(value).map_err(|_| DecodeError::new(format!("{path}.{key}"), "value exceeds u8"))
}

fn push_optional_string(
    fields: &mut Vec<(&'static str, JsonValue)>,
    key: &'static str,
    value: Option<&String>,
) {
    if let Some(value) = value {
        fields.push((key, JsonValue::from(value.clone())));
    }
}

#[cfg(test)]
#[path = "model/tests.rs"]
mod tests;
