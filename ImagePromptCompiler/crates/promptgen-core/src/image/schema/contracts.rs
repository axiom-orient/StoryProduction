use crate::json::JsonValue;

pub(super) fn reference_schema() -> JsonValue {
    object_schema(
        &["index", "role", "description", "use_for"],
        [
            ("index", integer_schema(1, 255)),
            (
                "role",
                enum_schema(&[
                    "base", "pose", "subject", "style", "product", "layout", "palette", "mask",
                    "other",
                ]),
            ),
            ("description", string_schema()),
            (
                "use_for",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(1_u64)),
                    ("items", string_schema()),
                ]),
            ),
        ],
    )
}

pub(super) fn change_contract_schema() -> JsonValue {
    object_schema(
        &["change_only", "preserve"],
        [
            ("change_only", array_schema(string_schema())),
            ("preserve", array_schema(string_schema())),
        ],
    )
}
pub(super) fn output_schema() -> JsonValue {
    object_schema(
        &[
            "model",
            "width",
            "height",
            "quality",
            "background",
            "format",
        ],
        [
            (
                "model",
                enum_schema(&[
                    "gpt-image-2",
                    "gpt-image-2.6-sunburst",
                    "gpt-image-2.6-flare",
                ]),
            ),
            ("width", integer_schema(16, 3840)),
            ("height", integer_schema(16, 3840)),
            ("quality", enum_schema(&["auto", "low", "medium", "high"])),
            ("background", enum_schema(&["auto", "opaque"])),
            ("format", enum_schema(&["png", "gif"])),
            ("size_policy", enum_schema(&["exact", "provider"])),
            ("frame_count", integer_schema(2, 16)),
            ("frame_duration_ms", integer_schema(20, 2000)),
        ],
    )
}

pub(super) fn string_schema() -> JsonValue {
    JsonValue::object([
        ("minLength", JsonValue::from(1_u64)),
        ("pattern", JsonValue::from("\\S")),
        ("type", JsonValue::from("string")),
    ])
}

pub(super) fn hex_schema() -> JsonValue {
    JsonValue::object([
        ("pattern", JsonValue::from("^#[0-9A-Fa-f]{6}$")),
        ("type", JsonValue::from("string")),
    ])
}

pub(super) fn enum_schema(values: &[&str]) -> JsonValue {
    JsonValue::object([
        (
            "enum",
            JsonValue::array(values.iter().map(|value| JsonValue::from(*value))),
        ),
        ("type", JsonValue::from("string")),
    ])
}

pub(super) fn array_schema(items: JsonValue) -> JsonValue {
    JsonValue::object([("items", items), ("type", JsonValue::from("array"))])
}

pub(super) fn integer_schema(minimum: u64, maximum: u64) -> JsonValue {
    JsonValue::object([
        ("maximum", JsonValue::from(maximum)),
        ("minimum", JsonValue::from(minimum)),
        ("type", JsonValue::from("integer")),
    ])
}

pub(super) fn number_schema(minimum: f64, maximum: f64) -> JsonValue {
    JsonValue::object([
        ("maximum", JsonValue::Number(maximum.to_string())),
        ("minimum", JsonValue::Number(minimum.to_string())),
        ("type", JsonValue::from("number")),
    ])
}

pub(super) fn nullable(schema: JsonValue) -> JsonValue {
    JsonValue::object([(
        "anyOf",
        JsonValue::array([
            schema,
            JsonValue::object([("type", JsonValue::from("null"))]),
        ]),
    )])
}

pub(super) fn object_schema<const N: usize>(
    required: &[&str],
    properties: [(&str, JsonValue); N],
) -> JsonValue {
    JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object(
                properties
                    .into_iter()
                    .map(|(key, value)| (key.to_owned(), value)),
            ),
        ),
        (
            "required",
            JsonValue::array(required.iter().map(|value| JsonValue::from(*value))),
        ),
        ("type", JsonValue::from("object")),
    ])
}
