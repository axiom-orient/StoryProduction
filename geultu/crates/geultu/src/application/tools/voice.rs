//! Voice tool metadata and input schemas.

use super::super::ToolDefinition;
use super::common::{object_schema, project_id_schema, tool};
use crate::voice::MAX_VOICE_SAMPLES;
use serde_json::json;

pub(super) fn tools() -> Vec<ToolDefinition> {
    let project = project_id_schema();
    vec![
        tool(
            "voice.profile.create",
            "Create writing voice",
            "Measure request-local samples and activate one bounded local writing-voice profile without storing sample prose",
            object_schema(
                &["project_id", "expected_project_revision", "samples"],
                json!({
                    "project_id":{"type":"string"},
                    "profile_id":{"type":"string"},
                    "expected_project_revision":{"type":"integer","minimum":0},
                    "samples":{"type":"array","minItems":1,"maxItems":MAX_VOICE_SAMPLES,"items":{"oneOf":[
                        {"type":"string"},
                        {"type":"object","required":["text"],"additionalProperties":false,"properties":{"id":{"type":"string"},"text":{"type":"string"}}}
                    ]}},
                    "command_id":{"type":"string"}
                }),
            ),
        ),
        tool(
            "voice.profile.get",
            "Get writing voice",
            "Read the active bounded writing-voice measurements and Korean guidance",
            object_schema(&["project_id"], project.clone()),
        ),
        tool(
            "voice.compare",
            "Compare writing voice",
            "Compare candidate text to the active writing voice with a readable score and tips",
            object_schema(
                &["project_id", "candidate_text"],
                json!({"project_id":{"type":"string"},"candidate_text":{"type":"string"}}),
            ),
        ),
        tool(
            "voice.guide",
            "Build rewrite instruction",
            "Create a copyable instruction for an external AI or editor while preserving scene facts, names, and meaning",
            object_schema(
                &["project_id", "scene_id"],
                json!({"project_id":{"type":"string"},"scene_id":{"type":"string"}}),
            ),
        ),
    ]
}
