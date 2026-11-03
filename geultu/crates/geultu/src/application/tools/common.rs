//! Shared tool-definition and object-schema constructors.

use super::super::ToolDefinition;
use serde_json::{json, Value};

pub(super) fn project_id_schema() -> Value {
    json!({"project_id":{"type":"string"}})
}

pub(super) fn object_schema(required: &[&str], properties: Value) -> Value {
    json!({"type":"object", "additionalProperties":false, "required":required, "properties":properties})
}

pub(super) fn tool(
    name: &'static str,
    title: &'static str,
    description: &'static str,
    input_schema: Value,
) -> ToolDefinition {
    ToolDefinition {
        name,
        title,
        description,
        input_schema,
    }
}
