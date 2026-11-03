//! Narrative tool metadata and input schemas.

use super::super::ToolDefinition;
use super::common::{object_schema, project_id_schema, tool};
use serde_json::json;

pub(super) fn tools() -> Vec<ToolDefinition> {
    let project = project_id_schema();
    vec![
        tool(
            "scene.add",
            "Add scene",
            "Add a scene to a project",
            object_schema(
                &["project_id", "scene_id", "title"],
                json!({"project_id":{"type":"string"},"scene_id":{"type":"string"},"title":{"type":"string"},"text":{"type":"string"},"order":{"type":"integer"},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "scene.list",
            "List scenes",
            "List scenes in narrative order",
            object_schema(&["project_id"], project.clone()),
        ),
        tool(
            "scene.get",
            "Get scene",
            "Read one scene",
            object_schema(
                &["project_id", "scene_id"],
                json!({"project_id":{"type":"string"},"scene_id":{"type":"string"}}),
            ),
        ),
        tool(
            "scene.set_text",
            "Set scene text",
            "Replace scene text with optional revision precondition",
            object_schema(
                &["project_id", "scene_id", "text"],
                json!({"project_id":{"type":"string"},"scene_id":{"type":"string"},"text":{"type":"string"},"expected_scene_revision":{"type":"integer","minimum":0},"run_id":{"type":"string"},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "fact.add",
            "Add fact",
            "Add a typed story fact with evidence",
            object_schema(
                &["project_id", "fact_id", "subject", "predicate", "object"],
                json!({"project_id":{"type":"string"},"fact_id":{"type":"string"},"subject":{"type":"string"},"predicate":{"type":"string"},"object":{"type":"string"},"evidence":{"type":"array","items":{"type":"object"}},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "fact.list",
            "List facts",
            "List canonical story facts",
            object_schema(&["project_id"], project.clone()),
        ),
        tool(
            "timeline.add",
            "Add timeline entry",
            "Add an ordered timeline entry",
            object_schema(
                &["project_id", "entry_id", "order", "label", "description"],
                json!({"project_id":{"type":"string"},"entry_id":{"type":"string"},"order":{"type":"integer"},"label":{"type":"string"},"description":{"type":"string"},"scene_id":{"type":"string"},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "timeline.list",
            "List timeline",
            "List timeline entries in order",
            object_schema(&["project_id"], project.clone()),
        ),
        tool(
            "thread.add",
            "Add story thread",
            "Add a tracked story thread",
            object_schema(
                &["project_id", "thread_id", "title"],
                json!({"project_id":{"type":"string"},"thread_id":{"type":"string"},"title":{"type":"string"},"status":{"enum":["open","resolved","abandoned"]},"evidence":{"type":"array","items":{"type":"object"}},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "thread.set_status",
            "Set thread status",
            "Change story thread status",
            object_schema(
                &["project_id", "thread_id", "status"],
                json!({"project_id":{"type":"string"},"thread_id":{"type":"string"},"status":{"enum":["open","resolved","abandoned"]},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "thread.list",
            "List story threads",
            "List canonical story threads",
            object_schema(&["project_id"], project.clone()),
        ),
    ]
}
