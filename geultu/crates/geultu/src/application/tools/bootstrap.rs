//! Bootstrap tool metadata and input schemas.

use super::super::ToolDefinition;
use super::common::{object_schema, project_id_schema, tool};
use serde_json::json;

pub(super) fn tools() -> Vec<ToolDefinition> {
    let project = project_id_schema();
    vec![
        tool(
            "workspace.status",
            "Workspace status",
            "Read canonical workspace metadata",
            object_schema(&[], json!({})),
        ),
        tool(
            "workspace.recover_index",
            "Recover workspace index",
            "Explicitly rebuild the project index from journals",
            object_schema(&[], json!({})),
        ),
        tool(
            "project.create",
            "Create project",
            "Create a canonical project journal",
            object_schema(
                &["project_id", "title"],
                json!({"project_id":{"type":"string"},"title":{"type":"string"},"display_label":{"type":"string","maxLength":80},"summary":{"type":"string","maxLength":280},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "project.update_metadata",
            "Update project metadata",
            "Update the story title and optional display context with a project revision precondition",
            object_schema(
                &["project_id", "title", "display_label", "summary", "expected_project_revision"],
                json!({"project_id":{"type":"string"},"title":{"type":"string","minLength":1},"display_label":{"type":"string","maxLength":80},"summary":{"type":"string","maxLength":280},"expected_project_revision":{"type":"integer","minimum":0},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "project.list",
            "List projects",
            "List projects from the workspace index",
            object_schema(&[], json!({})),
        ),
        tool(
            "project.get",
            "Get project",
            "Read the complete canonical project state",
            object_schema(&["project_id"], project.clone()),
        ),
    ]
}
