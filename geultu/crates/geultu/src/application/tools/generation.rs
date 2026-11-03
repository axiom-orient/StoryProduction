//! Generation tool metadata and input schemas.

use super::super::ToolDefinition;
use super::common::{object_schema, tool};
use serde_json::json;

pub(super) fn tools() -> Vec<ToolDefinition> {
    vec![
        tool(
            "writing.types.list",
            "List writing types",
            "List registered type adapters and generation providers",
            object_schema(&[], json!({})),
        ),
        tool(
            "generation.prepare",
            "Prepare generation",
            "Build a type-specific writing plan without changing canonical project state",
            object_schema(
                &["project_id", "writing_type", "brief"],
                json!({
                    "project_id":{"type":"string"},
                    "writing_type":{"type":"string"},
                    "stage":{"type":"string"},
                    "brief":{"type":"string"},
                    "parameters":{"type":"object"},
                    "expected_content_revision":{"type":"integer","minimum":0}
                }),
            ),
        ),
        tool(
            "generation.generate",
            "Generate draft",
            "Execute a registered external generation provider and return an untrusted draft",
            object_schema(
                &["project_id", "writing_type", "brief", "provider_id"],
                json!({
                    "project_id":{"type":"string"},
                    "writing_type":{"type":"string"},
                    "stage":{"type":"string"},
                    "brief":{"type":"string"},
                    "parameters":{"type":"object"},
                    "expected_content_revision":{"type":"integer","minimum":0},
                    "provider_id":{"type":"string"}
                }),
            ),
        ),
    ]
}
