use std::collections::BTreeMap;

use super::super::common::take_object_array;
use super::{decode_u8, push_optional_string, take_u8};
use crate::json::{
    DecodeError, JsonValue, into_object, reject_unknown, take_optional, take_optional_f64,
    take_optional_string, take_optional_u64, take_string,
};

fn take_optional_u8(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<u8>, DecodeError> {
    take_optional_u64(fields, key, path)?
        .map(|value| decode_u8(value, path, key))
        .transpose()
}

fn push_optional_unsigned<T: Into<u64>>(
    fields: &mut Vec<(&'static str, JsonValue)>,
    key: &'static str,
    value: Option<T>,
) {
    if let Some(value) = value {
        fields.push((key, JsonValue::from(value.into())));
    }
}

fn number_from_f64(value: f64) -> JsonValue {
    let mut text = value.to_string();
    if !text.contains(['.', 'e', 'E']) {
        text.push_str(".0");
    }
    JsonValue::Number(text)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraSpec {
    pub field_of_view: String,
    pub perspective: String,
    pub depth_of_field: String,
    pub focus: String,
    pub motion_rendering: String,
}

impl CameraSpec {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            field_of_view: take_string(&mut fields, "field_of_view", path)?,
            perspective: take_string(&mut fields, "perspective", path)?,
            depth_of_field: take_string(&mut fields, "depth_of_field", path)?,
            focus: take_string(&mut fields, "focus", path)?,
            motion_rendering: take_string(&mut fields, "motion_rendering", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "depth_of_field",
                JsonValue::from(self.depth_of_field.clone()),
            ),
            ("field_of_view", JsonValue::from(self.field_of_view.clone())),
            ("focus", JsonValue::from(self.focus.clone())),
            (
                "motion_rendering",
                JsonValue::from(self.motion_rendering.clone()),
            ),
            ("perspective", JsonValue::from(self.perspective.clone())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightingSpec {
    pub key_direction: String,
    pub key_quality: String,
    pub key_temperature_kelvin: Option<u16>,
    pub key_color_hex: Option<String>,
    pub key_to_fill_ratio: Option<f64>,
    pub fill_description: String,
    pub rim_description: Option<String>,
    pub shadow_character: String,
    pub exposure: String,
}

impl LightingSpec {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let kelvin = take_optional_u64(&mut fields, "key_temperature_kelvin", path)?
            .map(|value| {
                u16::try_from(value).map_err(|_| {
                    DecodeError::new(
                        format!("{path}.key_temperature_kelvin"),
                        "value exceeds u16",
                    )
                })
            })
            .transpose()?;
        let result = Self {
            key_direction: take_string(&mut fields, "key_direction", path)?,
            key_quality: take_string(&mut fields, "key_quality", path)?,
            key_temperature_kelvin: kelvin,
            key_color_hex: take_optional_string(&mut fields, "key_color_hex", path)?,
            key_to_fill_ratio: take_optional_f64(&mut fields, "key_to_fill_ratio", path)?,
            fill_description: take_string(&mut fields, "fill_description", path)?,
            rim_description: take_optional_string(&mut fields, "rim_description", path)?,
            shadow_character: take_string(&mut fields, "shadow_character", path)?,
            exposure: take_string(&mut fields, "exposure", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("exposure", JsonValue::from(self.exposure.clone())),
            (
                "fill_description",
                JsonValue::from(self.fill_description.clone()),
            ),
            ("key_direction", JsonValue::from(self.key_direction.clone())),
            ("key_quality", JsonValue::from(self.key_quality.clone())),
            (
                "shadow_character",
                JsonValue::from(self.shadow_character.clone()),
            ),
        ];
        if let Some(value) = &self.key_color_hex {
            fields.push(("key_color_hex", JsonValue::from(value.clone())));
        }
        if let Some(value) = self.key_temperature_kelvin {
            fields.push(("key_temperature_kelvin", JsonValue::from(u64::from(value))));
        }
        if let Some(value) = self.key_to_fill_ratio {
            fields.push(("key_to_fill_ratio", number_from_f64(value)));
        }
        if let Some(value) = &self.rim_description {
            fields.push(("rim_description", JsonValue::from(value.clone())));
        }
        JsonValue::object(fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteEntry {
    pub hex: String,
    pub proportion_percent: u8,
    pub usage: String,
}

impl PaletteEntry {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            hex: take_string(&mut fields, "hex", path)?,
            proportion_percent: take_u8(&mut fields, "proportion_percent", path)?,
            usage: take_string(&mut fields, "usage", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("hex", JsonValue::from(self.hex.clone())),
            (
                "proportion_percent",
                JsonValue::from(u64::from(self.proportion_percent)),
            ),
            ("usage", JsonValue::from(self.usage.clone())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhotoLutSelection {
    pub preset: String,
    pub strength_percent: u8,
    pub white_balance_kelvin: Option<u16>,
    pub tint: Option<String>,
    pub tone_curve: Option<String>,
    pub black_response: Option<String>,
    pub contrast: Option<String>,
    pub saturation_percent: Option<u8>,
    pub shadow_bias_hex: Option<String>,
    pub highlight_bias_hex: Option<String>,
    pub highlight_rolloff: Option<String>,
    pub skin_tone_policy: Option<String>,
    pub grain_size: Option<String>,
    pub grain_amount_percent: Option<u8>,
    pub halation_percent: Option<u8>,
    pub vignette_percent: Option<u8>,
}

impl PhotoLutSelection {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let kelvin = take_optional_u64(&mut fields, "white_balance_kelvin", path)?
            .map(|value| {
                u16::try_from(value).map_err(|_| {
                    DecodeError::new(format!("{path}.white_balance_kelvin"), "value exceeds u16")
                })
            })
            .transpose()?;
        let result = Self {
            preset: take_string(&mut fields, "preset", path)?,
            strength_percent: take_optional_u8(&mut fields, "strength_percent", path)?
                .unwrap_or(100),
            white_balance_kelvin: kelvin,
            tint: take_optional_string(&mut fields, "tint", path)?,
            tone_curve: take_optional_string(&mut fields, "tone_curve", path)?,
            black_response: take_optional_string(&mut fields, "black_response", path)?,
            contrast: take_optional_string(&mut fields, "contrast", path)?,
            saturation_percent: take_optional_u8(&mut fields, "saturation_percent", path)?,
            shadow_bias_hex: take_optional_string(&mut fields, "shadow_bias_hex", path)?,
            highlight_bias_hex: take_optional_string(&mut fields, "highlight_bias_hex", path)?,
            highlight_rolloff: take_optional_string(&mut fields, "highlight_rolloff", path)?,
            skin_tone_policy: take_optional_string(&mut fields, "skin_tone_policy", path)?,
            grain_size: take_optional_string(&mut fields, "grain_size", path)?,
            grain_amount_percent: take_optional_u8(&mut fields, "grain_amount_percent", path)?,
            halation_percent: take_optional_u8(&mut fields, "halation_percent", path)?,
            vignette_percent: take_optional_u8(&mut fields, "vignette_percent", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("preset", JsonValue::from(self.preset.clone())),
            (
                "strength_percent",
                JsonValue::from(u64::from(self.strength_percent)),
            ),
        ];
        push_optional_unsigned(
            &mut fields,
            "white_balance_kelvin",
            self.white_balance_kelvin,
        );
        push_optional_string(&mut fields, "tint", self.tint.as_ref());
        push_optional_string(&mut fields, "tone_curve", self.tone_curve.as_ref());
        push_optional_string(&mut fields, "black_response", self.black_response.as_ref());
        push_optional_string(&mut fields, "contrast", self.contrast.as_ref());
        push_optional_unsigned(&mut fields, "saturation_percent", self.saturation_percent);
        push_optional_string(
            &mut fields,
            "shadow_bias_hex",
            self.shadow_bias_hex.as_ref(),
        );
        push_optional_string(
            &mut fields,
            "highlight_bias_hex",
            self.highlight_bias_hex.as_ref(),
        );
        push_optional_string(
            &mut fields,
            "highlight_rolloff",
            self.highlight_rolloff.as_ref(),
        );
        push_optional_string(
            &mut fields,
            "skin_tone_policy",
            self.skin_tone_policy.as_ref(),
        );
        push_optional_string(&mut fields, "grain_size", self.grain_size.as_ref());
        push_optional_unsigned(
            &mut fields,
            "grain_amount_percent",
            self.grain_amount_percent,
        );
        push_optional_unsigned(&mut fields, "halation_percent", self.halation_percent);
        push_optional_unsigned(&mut fields, "vignette_percent", self.vignette_percent);
        JsonValue::object(fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorSpec {
    pub palette: Vec<PaletteEntry>,
    pub harmony: String,
    pub contrast: String,
    pub saturation: String,
    pub photo_lut: Option<PhotoLutSelection>,
}

impl ColorSpec {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            palette: take_object_array(&mut fields, "palette", path, PaletteEntry::from_json)?,
            harmony: take_string(&mut fields, "harmony", path)?,
            contrast: take_string(&mut fields, "contrast", path)?,
            saturation: take_string(&mut fields, "saturation", path)?,
            photo_lut: take_optional(&mut fields, "photo_lut")
                .map(|value| PhotoLutSelection::from_json(value, &format!("{path}.photo_lut")))
                .transpose()?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("contrast", JsonValue::from(self.contrast.clone())),
            ("harmony", JsonValue::from(self.harmony.clone())),
            (
                "palette",
                JsonValue::array(self.palette.iter().map(PaletteEntry::to_json)),
            ),
            ("saturation", JsonValue::from(self.saturation.clone())),
        ];
        if let Some(value) = &self.photo_lut {
            fields.push(("photo_lut", value.to_json()));
        }
        JsonValue::object(fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceSpec {
    pub id: String,
    pub material: String,
    pub finish: String,
    pub micro_detail: String,
    pub light_response: String,
}

impl SurfaceSpec {
    pub(super) fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            id: take_string(&mut fields, "id", path)?,
            material: take_string(&mut fields, "material", path)?,
            finish: take_string(&mut fields, "finish", path)?,
            micro_detail: take_string(&mut fields, "micro_detail", path)?,
            light_response: take_string(&mut fields, "light_response", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("finish", JsonValue::from(self.finish.clone())),
            ("id", JsonValue::from(self.id.clone())),
            (
                "light_response",
                JsonValue::from(self.light_response.clone()),
            ),
            ("material", JsonValue::from(self.material.clone())),
            ("micro_detail", JsonValue::from(self.micro_detail.clone())),
        ])
    }
}
