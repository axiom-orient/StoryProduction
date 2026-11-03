//! Visual tool metadata and input schemas.

use super::super::ToolDefinition;
use super::common::{object_schema, project_id_schema, tool};
use serde_json::json;

pub(super) fn tools() -> Vec<ToolDefinition> {
    let project = project_id_schema();
    vec![
        tool(
            "visual.plan",
            "Plan a local visual draft",
            "Store a deterministic paragraph-based visual draft from current canonical scene text",
            object_schema(
                &["project_id", "scene_id"],
                json!({"project_id":{"type":"string"},"scene_id":{"type":"string"},"plan_id":{"type":"string"},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "visual.list",
            "List visual plans",
            "List stored visual plans",
            object_schema(&["project_id"], project.clone()),
        ),
        tool(
            "visual.admit_package",
            "Prepare a source-bound visual run",
            "Admit the required inline GeultuStoryPackage matching current Geultu source and projection; record a local READY run and preflight receipt under source CAS. NVC execution and scene release remain separate owner actions.",
            object_schema(
                &["project_id", "run_id", "stage", "package", "route_admission"],
                json!({
                    "project_id":{"type":"string"},
                    "run_id":{"type":"string"},
                    "stage":{"type":"string","pattern":"^S(?:0[0-9]|1[0-2])$"},
                    "package":{"type":"object"},
                    "route_admission":{"type":"object","required":["route_id","command_id","input_refs","required_receipts","capabilities","budget"],"additionalProperties":false,"properties":{
                        "route_id":{"enum":["NVC"]},
                        "command_id":{"type":"string"},
                        "input_refs":{"type":"object"},
                        "required_receipts":{"type":"array"},
                        "capabilities":{"type":"array"},
                        "budget":{"type":"object","required":["time_ms","output_bytes"],"additionalProperties":false,"properties":{"time_ms":{"type":"integer","minimum":1},"output_bytes":{"type":"integer","minimum":1},"provider_attempts":{"type":"integer","minimum":0}}}
                    }},
                    "command_id":{"type":"string"}
                }),
            ),
        ),
    ]
}
