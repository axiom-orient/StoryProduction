use crate::catalog::MAX_SUPPORTING_DIRECTIONS;
use crate::json::JsonValue;

mod contracts;
mod fields;
mod storyboard;

use self::contracts::{
    array_schema, change_contract_schema, enum_schema, nullable, object_schema, output_schema,
    reference_schema, string_schema,
};
#[cfg(test)]
use self::fields::placement_schema;
use self::fields::{
    camera_schema, color_schema, composition_schema, constraints_schema, lighting_schema,
    subject_schema, surface_schema, text_element_schema,
};
use self::storyboard::{consistency_schema, storyboard_schema};

pub fn image_schema() -> JsonValue {
    let mut schema = object_schema(
        &[
            "use_case",
            "medium",
            "taxonomy",
            "scene",
            "subjects",
            "composition",
            "lighting",
            "color",
            "constraints",
            "output",
        ],
        [
            ("language", enum_schema(&["ko", "en"])),
            (
                "render_profile",
                enum_schema(&[
                    "structured",
                    "editorial_flat",
                    "pose_transfer",
                    "cinematic_storyboard",
                ]),
            ),
            (
                "profile",
                enum_schema(&[
                    "standard",
                    "logo_identity",
                    "app_icon",
                    "app_web_ui",
                    "information_design",
                    "character_pose",
                    "poomsae_pose",
                ]),
            ),
            ("task_mode", enum_schema(&["generate", "edit", "composite"])),
            (
                "references",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("maxItems", JsonValue::from(255_u64)),
                    ("items", reference_schema()),
                ]),
            ),
            ("change_contract", change_contract_schema()),
            ("use_case", string_schema()),
            (
                "medium",
                enum_schema(&["photo", "illustration", "3d", "graphic_design", "mixed"]),
            ),
            (
                "taxonomy",
                object_schema(
                    &[],
                    [
                        ("category", nullable(string_schema())),
                        (
                            "patterns",
                            JsonValue::object([
                                ("type", JsonValue::from("array")),
                                (
                                    "maxItems",
                                    JsonValue::from(MAX_SUPPORTING_DIRECTIONS as u64),
                                ),
                                ("items", string_schema()),
                            ]),
                        ),
                    ],
                ),
            ),
            (
                "scene",
                object_schema(
                    &["environment", "background", "atmosphere"],
                    [
                        ("environment", string_schema()),
                        ("background", string_schema()),
                        ("atmosphere", string_schema()),
                        ("time_of_day", nullable(string_schema())),
                        ("weather", nullable(string_schema())),
                    ],
                ),
            ),
            ("subjects", array_schema(subject_schema())),
            ("composition", composition_schema()),
            ("storyboard", nullable(storyboard_schema())),
            ("consistency", nullable(consistency_schema())),
            ("camera", nullable(camera_schema())),
            ("lighting", lighting_schema()),
            ("color", color_schema()),
            ("surfaces", array_schema(surface_schema())),
            ("text_elements", array_schema(text_element_schema())),
            ("constraints", constraints_schema()),
            ("output", output_schema()),
        ],
    );
    let JsonValue::Object(root) = &mut schema else {
        unreachable!("object_schema must produce an object");
    };
    root.insert(
        "$schema".to_owned(),
        JsonValue::from("https://json-schema.org/draft/2020-12/schema"),
    );
    root.insert("title".to_owned(), JsonValue::from("ImagePromptRequest"));
    schema
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_is_optional_like_the_runtime_decoder() {
        let schema = image_schema();
        let required = schema
            .as_object()
            .and_then(|root| root.get("required"))
            .and_then(JsonValue::as_array)
            .expect("required");
        assert!(
            !required
                .iter()
                .any(|value| value.as_str() == Some("language"))
        );
    }

    #[test]
    fn free_text_schema_rejects_whitespace_only_values() {
        let schema = string_schema();
        let object = schema.as_object().expect("string schema");
        assert_eq!(object.get("minLength").and_then(JsonValue::as_u64), Some(1));
        assert_eq!(
            object.get("pattern").and_then(JsonValue::as_str),
            Some("\\S")
        );
    }

    #[test]
    fn schema_is_valid_json_and_rejects_transparent_enum() {
        let text = image_schema().to_pretty_string();
        let parsed = crate::json::parse(&text).expect("schema must parse");
        assert_eq!(
            parsed
                .as_object()
                .and_then(|root| root.get("$schema"))
                .and_then(JsonValue::as_str),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
        assert!(!text.contains("transparent"));
        assert!(text.contains("gpt-image-2"));
    }

    #[test]
    fn schema_matches_gpt_image_two_task_and_output_contract() {
        let schema = image_schema();
        let root = schema.as_object().expect("root object");
        let properties = root
            .get("properties")
            .and_then(JsonValue::as_object)
            .expect("root properties");
        for key in [
            "task_mode",
            "references",
            "change_contract",
            "consistency",
            "text_elements",
        ] {
            assert!(properties.contains_key(key), "missing {key}");
        }
        assert!(schema.to_pretty_string().contains("\"multi_view\""));

        let output = properties
            .get("output")
            .and_then(JsonValue::as_object)
            .expect("output schema");
        let output_properties = output
            .get("properties")
            .and_then(JsonValue::as_object)
            .expect("output properties");
        let quality = output_properties
            .get("quality")
            .and_then(JsonValue::as_object)
            .and_then(|value| value.get("enum"))
            .and_then(JsonValue::as_array)
            .expect("quality enum");
        assert_eq!(
            quality,
            &[
                JsonValue::from("auto"),
                JsonValue::from("low"),
                JsonValue::from("medium"),
                JsonValue::from("high"),
            ]
        );
        for dimension in ["width", "height"] {
            let maximum = output_properties
                .get(dimension)
                .and_then(JsonValue::as_object)
                .and_then(|value| value.get("maximum"));
            assert_eq!(maximum, Some(&JsonValue::from(3840_u64)));
        }

        assert!(schema.to_pretty_string().contains("spelling_hint"));
    }

    #[test]
    fn placement_union_keeps_branch_level_property_authority() {
        let schema = placement_schema();
        let root = schema
            .as_object()
            .expect("placement schema must be an object");
        assert!(!root.contains_key("additionalProperties"));
        let branches = root
            .get("oneOf")
            .and_then(JsonValue::as_array)
            .expect("placement schema must contain oneOf");
        assert_eq!(branches.len(), 2);
        for branch in branches {
            let object = branch.as_object().expect("branch must be an object");
            assert_eq!(
                object
                    .get("additionalProperties")
                    .and_then(JsonValue::as_bool),
                Some(false)
            );
        }
    }
}
