use super::contracts::{
    array_schema, enum_schema, hex_schema, integer_schema, nullable, number_schema, object_schema,
    string_schema,
};
use crate::json::JsonValue;

pub(super) fn subject_schema() -> JsonValue {
    object_schema(
        &[
            "id",
            "count",
            "description",
            "placement",
            "scale",
            "pose",
            "gaze",
            "action",
        ],
        [
            ("id", string_schema()),
            ("count", integer_schema(1, 65535)),
            ("description", string_schema()),
            ("placement", placement_schema()),
            ("scale", string_schema()),
            ("pose", string_schema()),
            ("gaze", string_schema()),
            ("action", string_schema()),
            ("face", nullable(string_schema())),
            ("hair", nullable(string_schema())),
            ("appearance", nullable(string_schema())),
            ("distinguishing_features", array_schema(string_schema())),
        ],
    )
}

pub(super) fn placement_schema() -> JsonValue {
    JsonValue::object([(
        "oneOf",
        JsonValue::array([
            object_schema(
                &["zone"],
                [(
                    "zone",
                    enum_schema(&[
                        "top_left",
                        "top_center",
                        "top_right",
                        "middle_left",
                        "center",
                        "middle_right",
                        "bottom_left",
                        "bottom_center",
                        "bottom_right",
                    ]),
                )],
            ),
            object_schema(
                &[
                    "zone",
                    "x_percent",
                    "y_percent",
                    "width_percent",
                    "height_percent",
                ],
                [
                    ("zone", enum_schema(&["custom"])),
                    ("x_percent", integer_schema(0, 100)),
                    ("y_percent", integer_schema(0, 100)),
                    ("width_percent", integer_schema(1, 100)),
                    ("height_percent", integer_schema(1, 100)),
                ],
            ),
        ]),
    )])
}

pub(super) fn composition_schema() -> JsonValue {
    object_schema(
        &[
            "framing",
            "viewpoint",
            "camera_angle",
            "balance",
            "visual_hierarchy",
            "depth_layers",
        ],
        [
            ("framing", string_schema()),
            ("viewpoint", string_schema()),
            ("camera_angle", string_schema()),
            ("balance", string_schema()),
            ("visual_hierarchy", array_schema(string_schema())),
            ("depth_layers", array_schema(string_schema())),
            (
                "negative_space",
                array_schema(enum_schema(&[
                    "top_left",
                    "top_center",
                    "top_right",
                    "middle_left",
                    "center",
                    "middle_right",
                    "bottom_left",
                    "bottom_center",
                    "bottom_right",
                ])),
            ),
        ],
    )
}

pub(super) fn camera_schema() -> JsonValue {
    object_schema(
        &[
            "field_of_view",
            "perspective",
            "depth_of_field",
            "focus",
            "motion_rendering",
        ],
        [
            ("field_of_view", string_schema()),
            ("perspective", string_schema()),
            ("depth_of_field", string_schema()),
            ("focus", string_schema()),
            ("motion_rendering", string_schema()),
        ],
    )
}

pub(super) fn lighting_schema() -> JsonValue {
    object_schema(
        &[
            "key_direction",
            "key_quality",
            "fill_description",
            "shadow_character",
            "exposure",
        ],
        [
            ("key_direction", string_schema()),
            ("key_quality", string_schema()),
            (
                "key_temperature_kelvin",
                nullable(integer_schema(1000, 20000)),
            ),
            ("key_color_hex", nullable(hex_schema())),
            ("key_to_fill_ratio", nullable(number_schema(0.01, 100.0))),
            ("fill_description", string_schema()),
            ("rim_description", nullable(string_schema())),
            ("shadow_character", string_schema()),
            ("exposure", string_schema()),
        ],
    )
}

pub(super) fn color_schema() -> JsonValue {
    object_schema(
        &["palette", "harmony", "contrast", "saturation"],
        [
            (
                "palette",
                JsonValue::object([
                    ("items", palette_entry_schema()),
                    ("maxItems", JsonValue::from(5_u64)),
                    ("minItems", JsonValue::from(3_u64)),
                    ("type", JsonValue::from("array")),
                ]),
            ),
            ("harmony", string_schema()),
            ("contrast", string_schema()),
            ("saturation", string_schema()),
            ("photo_lut", nullable(photo_lut_schema())),
        ],
    )
}

pub(super) fn palette_entry_schema() -> JsonValue {
    object_schema(
        &["hex", "proportion_percent", "usage"],
        [
            ("hex", hex_schema()),
            ("proportion_percent", integer_schema(1, 100)),
            ("usage", string_schema()),
        ],
    )
}

pub(super) fn photo_lut_schema() -> JsonValue {
    object_schema(
        &["preset", "strength_percent"],
        [
            (
                "preset",
                enum_schema(&[
                    "clean_neutral",
                    "warm_pastel_filmic",
                    "cool_steel",
                    "restrained_teal_orange",
                    "bleach_bypass",
                    "tungsten_night",
                    "faded_print",
                    "monochrome_high_contrast",
                ]),
            ),
            ("strength_percent", integer_schema(0, 100)),
            (
                "white_balance_kelvin",
                nullable(integer_schema(1000, 20000)),
            ),
            ("tint", nullable(string_schema())),
            ("tone_curve", nullable(string_schema())),
            ("black_response", nullable(string_schema())),
            ("contrast", nullable(string_schema())),
            ("saturation_percent", nullable(integer_schema(0, 100))),
            ("shadow_bias_hex", nullable(hex_schema())),
            ("highlight_bias_hex", nullable(hex_schema())),
            ("highlight_rolloff", nullable(string_schema())),
            ("skin_tone_policy", nullable(string_schema())),
            ("grain_size", nullable(string_schema())),
            ("grain_amount_percent", nullable(integer_schema(0, 100))),
            ("halation_percent", nullable(integer_schema(0, 100))),
            ("vignette_percent", nullable(integer_schema(0, 100))),
        ],
    )
}

pub(super) fn surface_schema() -> JsonValue {
    object_schema(
        &["id", "material", "finish", "micro_detail", "light_response"],
        [
            ("id", string_schema()),
            ("material", string_schema()),
            ("finish", string_schema()),
            ("micro_detail", string_schema()),
            ("light_response", string_schema()),
        ],
    )
}

pub(super) fn text_element_schema() -> JsonValue {
    object_schema(
        &[
            "id",
            "role",
            "lines",
            "language",
            "placement",
            "font_family",
            "weight",
            "alignment",
            "size_percent",
            "color_hex",
            "letter_spacing",
            "treatment",
        ],
        [
            ("id", string_schema()),
            (
                "role",
                enum_schema(&[
                    "headline", "subhead", "callout", "caption", "badge", "cta", "wordmark",
                ]),
            ),
            ("lines", array_schema(string_schema())),
            ("language", string_schema()),
            ("placement", placement_schema()),
            ("font_family", string_schema()),
            ("weight", string_schema()),
            ("alignment", string_schema()),
            ("size_percent", integer_schema(1, 100)),
            ("color_hex", hex_schema()),
            ("letter_spacing", string_schema()),
            ("treatment", string_schema()),
            ("spelling_hint", nullable(string_schema())),
        ],
    )
}

pub(super) fn constraints_schema() -> JsonValue {
    object_schema(
        &["required_elements", "excluded_elements", "safety_tier"],
        [
            ("required_elements", array_schema(string_schema())),
            ("excluded_elements", array_schema(string_schema())),
            ("safety_tier", integer_schema(0, 2)),
            (
                "adult_subjects_only",
                JsonValue::object([("type", JsonValue::from("boolean"))]),
            ),
            (
                "original_characters_only",
                JsonValue::object([("type", JsonValue::from("boolean"))]),
            ),
            (
                "clean_unbranded_finish",
                JsonValue::object([("type", JsonValue::from("boolean"))]),
            ),
        ],
    )
}
