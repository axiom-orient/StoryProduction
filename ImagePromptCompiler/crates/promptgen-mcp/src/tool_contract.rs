use std::sync::Arc;

use promptgen_core::json::{JsonValue, parse};
use promptgen_core::{compilation_schema, image_schema, interview_schema};
use rmcp::model::{JsonObject, Tool, ToolAnnotations};
use serde_json::{Value, json};

pub(super) fn tool_definitions() -> Vec<Tool> {
    vec![
        tool(
            "promptgen_compile_image",
            "Compile a validated image-generation prompt without executing an image backend.",
            image_schema(),
            Some(compilation_schema()),
        ),
        tool(
            "promptgen_interview",
            "Run the stateless guided interview; returns missing questions or a compiled prompt.",
            interview_schema(),
            Some(interview_output_schema()),
        ),
        tool(
            "promptgen_catalog",
            "Read the immutable promptGen image outcome catalog, optionally filtered by family.",
            catalog_input_schema(),
            None,
        ),
        tool(
            "promptgen_lut_presets",
            "List the resolved photographic LUT presets available to image prompt compilation.",
            empty_object_schema(),
            None,
        ),
    ]
}

fn tool(
    name: &'static str,
    description: &'static str,
    input: JsonValue,
    output: Option<JsonValue>,
) -> Tool {
    let mut tool = Tool::new(name, description, schema_arc(input));
    if let Some(output) = output {
        tool = tool.with_raw_output_schema(schema_arc(output));
    }
    tool.with_annotations(ToolAnnotations::new().read_only(true).idempotent(true))
}

fn schema_arc(value: JsonValue) -> Arc<JsonObject> {
    let value: Value = serde_json::from_str(&value.to_compact_string())
        .expect("canonical promptGen schema must be valid JSON");
    let Value::Object(value) = value else {
        panic!("canonical promptGen schema must be a JSON object");
    };
    Arc::new(value)
}

fn empty_object_schema() -> JsonValue {
    JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        ("type", JsonValue::from("object")),
    ])
}

fn catalog_input_schema() -> JsonValue {
    JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object([(
                "family",
                JsonValue::object([
                    (
                        "enum",
                        JsonValue::array(
                            [
                                "all",
                                "category",
                                "promo",
                                "typography",
                                "look",
                                "movement",
                                "response",
                                "type-art",
                                "editorial",
                            ]
                            .into_iter()
                            .map(JsonValue::from),
                        ),
                    ),
                    ("type", JsonValue::from("string")),
                ]),
            )]),
        ),
        ("type", JsonValue::from("object")),
    ])
}

fn interview_output_schema() -> JsonValue {
    json_to_core_value(json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "additionalProperties": false,
        "properties": {
            "compilation": {"type": ["object", "null"]},
            "inferences": {"type": "array", "items": {"type": "string"}},
            "kind": {"enum": ["image"], "type": "string"},
            "normalized_answers": {"type": "object", "additionalProperties": {"type": "string"}},
            "questions": {"type": "array", "items": {"type": "object"}},
            "request": {"type": ["object", "null"]},
            "status": {"enum": ["needs_input", "ready", "invalid"], "type": "string"}
        },
        "required": ["compilation", "inferences", "kind", "normalized_answers", "questions", "request", "status"],
        "type": "object"
    }))
}

fn json_to_core_value(value: Value) -> JsonValue {
    parse(&value.to_string()).expect("MCP adapter schema must be valid JSON")
}
