//! Refinement tool metadata and input schemas.

use super::super::ToolDefinition;
use super::common::{object_schema, tool};
use serde_json::json;

pub(super) fn tools() -> Vec<ToolDefinition> {
    vec![
        tool(
            "refinement.diagnose",
            "Diagnose refinement findings",
            "Run deterministic catalog-policy-governed checks over scene or project text; read-only and translation-forbidden by construction",
            object_schema(
                &["project_id"],
                json!({"project_id":{"type":"string"},"scene_id":{"type":"string"}}),
            ),
        ),
        tool(
            "refinement.propose_patch",
            "Propose refinement patch",
            "Create a pending candidate plus changeset carrying mechanically safe same-language rewrites for one scene; application still requires changeset.approve with a bound human decision",
            object_schema(
                &["project_id", "scene_id"],
                json!({
                    "project_id":{"type":"string"},
                    "scene_id":{"type":"string"},
                    "candidate_id":{"type":"string"},
                    "changeset_id":{"type":"string"},
                    "command_id":{"type":"string"}
                }),
            ),
        ),
    ]
}
