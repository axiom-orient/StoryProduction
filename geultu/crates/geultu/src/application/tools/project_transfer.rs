//! Project transfer tool metadata and input schemas.

use super::super::ToolDefinition;
use super::common::{object_schema, project_id_schema, tool};
use serde_json::json;

pub(super) fn tools() -> Vec<ToolDefinition> {
    let project = project_id_schema();
    vec![
        tool(
            "project.recover",
            "Recover project",
            "Replay the canonical journal and rebuild its snapshot",
            object_schema(
                &["project_id"],
                json!({"project_id":{"type":"string"},"remove_lock":{"type":"boolean","default":false}}),
            ),
        ),
        tool(
            "project.import_markdown",
            "Import Markdown",
            "Import headings and text as scenes",
            object_schema(
                &["project_id", "markdown"],
                json!({"project_id":{"type":"string"},"markdown":{"type":"string"},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "project.export_markdown",
            "Export Markdown",
            "Export ordered scenes as Markdown",
            object_schema(&["project_id"], project.clone()),
        ),
        tool(
            "synopsis.export_reviewed_handoff",
            "Export reviewed synopsis handoff",
            "Review the exact project.export_markdown revision and SHA-256, then publish those whole-project bytes with a bound human approval. Use authoring handoff for an adopted synopsis document.",
            object_schema(
                &["project_id", "actor_id", "language", "expected_source_revision", "expected_synopsis_sha256"],
                json!({
                    "project_id":{"type":"string"},
                    "actor_id":{"type":"string","minLength":1},
                    "language":{"type":"string","pattern":"^[A-Za-z]{2,3}(?:-[A-Za-z0-9]{1,8})*$"},
                    "reason":{"type":"string","maxLength":4000},
                    "expected_source_revision":{"type":"integer","minimum":1},
                    "expected_synopsis_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}
                }),
            ),
        ),
        tool(
            "story.export",
            "Export Geultu story package",
            "Project canonical scenes into a flat GeultuStoryPackage with a deterministic np_ identity",
            object_schema(
                &["project_id", "language", "logline"],
                json!({
                    "project_id":{"type":"string"},
                    "language":{"type":"string","pattern":"^[A-Za-z]{2,3}(?:-[A-Za-z0-9]{2,8})*$"},
                    "logline":{"type":"string","minLength":1,"maxLength":2000},
                    "global_constraints":{"type":"array","items":{"type":"string"}}
                }),
            ),
        ),
    ]
}
