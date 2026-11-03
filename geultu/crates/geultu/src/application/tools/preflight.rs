//! Preflight tool metadata and input schemas.

use super::super::ToolDefinition;
use super::common::{object_schema, tool};
use serde_json::json;

pub(super) fn tools() -> Vec<(ToolDefinition, super::Handler)> {
    vec![
        (
            tool(
            "run.open",
            "Open text run",
            "Open a READY text run with declared inputs and budgets. Each run-referencing text edit or changeset adoption consumes one revision unit; terminal or exhausted runs refuse changes. Pivot budget is recorded but has no spending command.",
            object_schema(
                &["project_id", "run_id", "stage", "budgets", "frozen_inputs"],
                json!({
                    "project_id":{"type":"string"},
                    "run_id":{"type":"string"},
                    "stage":{"type":"string","pattern":"^T(?:0[0-9]|[12][0-9]|3[0-3])$"},
                    "budgets":{"type":"object","required":["revision_remaining","pivot_remaining"],"additionalProperties":false,"properties":{"revision_remaining":{"type":"integer","minimum":0},"pivot_remaining":{"type":"integer","minimum":0}}},
                    "frozen_inputs":{"type":"object","required":["synopsis","style","canon","genre"],"additionalProperties":false,"properties":{"synopsis":{"$ref":"artifact-ref"},"style":{"$ref":"artifact-ref"},"canon":{"$ref":"artifact-ref"},"genre":{"$ref":"artifact-ref"},"sources":{"type":"array","items":{}}}},
                    "accepted_outputs":{"type":"object"},
                    "command_id":{"type":"string"}
                }),
            ),
            ),
            super::super::dispatch::review::call as super::Handler,
        ),
        (
            tool(
            "craftpack.status",
            "Craft pack status",
            "Report craft-pack identity and SHA256SUMS integrity state, including workflow node counts and per-file digests; fails closed on any integrity mismatch",
            object_schema(&[], json!({})),
            ),
            super::super::dispatch::refinement::call as super::Handler,
        ),
    ]
}
