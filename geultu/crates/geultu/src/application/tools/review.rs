//! Review tool metadata and input schemas.

use super::super::ToolDefinition;
use super::common::{object_schema, project_id_schema, tool};
use serde_json::{json, Value};

fn human_decision_schema(base_required: &[&str], base_properties: Value) -> Value {
    let mut required = base_required.to_vec();
    required.extend([
        "actor_id",
        "expected_artifact_revision",
        "expected_artifact_digest",
    ]);
    let Value::Object(mut properties) = base_properties else {
        unreachable!("human decision schema properties must be an object");
    };
    properties.insert(
        "actor_id".into(),
        json!({"type":"string", "minLength":1, "maxLength":256}),
    );
    properties.insert("reason".into(), json!({"type":"string", "maxLength":4000}));
    properties.insert(
        "expected_artifact_revision".into(),
        json!({"type":"integer", "minimum":1}),
    );
    properties.insert(
        "expected_artifact_digest".into(),
        json!({"type":"string", "pattern":"^sha256:[0-9a-f]{64}$"}),
    );
    object_schema(&required, Value::Object(properties))
}

pub(super) fn tools() -> Vec<ToolDefinition> {
    let project = project_id_schema();
    vec![
        tool(
            "review.run",
            "Review project",
            "Run deterministic style and continuity checks",
            object_schema(&["project_id"], project.clone()),
        ),
        tool(
            "proof.verify",
            "Verify candidate",
            "Measure fidelity, grounding, naturalness, and protected-term preservation; issues a validation receipt, recorded when project_id is supplied",
            object_schema(
                &["source", "candidate"],
                json!({"source":{"type":"string"},"candidate":{"type":"string"},"evidence":{"type":"array","items":{"type":"string"}},"protected_terms":{"type":"array","items":{"type":"string"}},"project_id":{"type":"string"},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "candidate.create",
            "Create candidate",
            "Store external AI or human output as non-canonical candidate with receipt",
            object_schema(
                &["project_id", "kind", "content", "provider", "model"],
                json!({"project_id":{"type":"string"},"candidate_id":{"type":"string"},"kind":{"type":"string"},"writing_type":{"type":"string"},"stage":{"type":"string"},"source_revision":{"type":"integer"},"content":{"type":"string"},"provider":{"type":"string"},"model":{"type":"string"},"request_id":{"type":"string"},"input_hash":{"type":"string"},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "scene.prepare_candidate",
            "Prepare scene candidate",
            "Atomically record a non-canonical candidate and one pending scene replacement against the current content and scene revisions. This does not approve or apply the candidate.",
            object_schema(
                &["project_id", "scene_id", "expected_content_revision", "expected_scene_revision", "candidate_id", "changeset_id", "kind", "content", "provider", "model"],
                json!({"project_id":{"type":"string"},"scene_id":{"type":"string"},"expected_content_revision":{"type":"integer","minimum":0},"expected_scene_revision":{"type":"integer","minimum":0},"candidate_id":{"type":"string"},"changeset_id":{"type":"string"},"kind":{"type":"string"},"content":{"type":"string"},"provider":{"type":"string"},"model":{"type":"string"},"request_id":{"type":"string"},"input_hash":{"type":"string"},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "candidate.list",
            "List candidates",
            "List pending and decided candidates",
            object_schema(&["project_id"], project.clone()),
        ),
        tool(
            "candidate.subject",
            "Read candidate subject",
            "Read the exact subject identity of a pending candidate. Use its revision and digest for a separate human decision.",
            object_schema(
                &["project_id", "candidate_id"],
                json!({"project_id":{"type":"string"},"candidate_id":{"type":"string"}}),
            ),
        ),
        tool(
            "candidate.reject",
            "Reject candidate",
            "Reject a pending candidate without changing canonical text; records a human decision bound to its exact subject.",
            human_decision_schema(
                &["project_id", "candidate_id"],
                json!({"project_id":{"type":"string"},"candidate_id":{"type":"string"},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "changeset.create",
            "Create changeset",
            "Structure candidate output as explicit canonical operations",
            object_schema(
                &["project_id", "candidate_id", "operations"],
                json!({"project_id":{"type":"string"},"candidate_id":{"type":"string"},"changeset_id":{"type":"string"},"operations":{"type":"array","items":{"type":"object"}},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "changeset.approve",
            "Approve changeset",
            "Atomically apply a pending changeset with project and scene revision checks and a human decision bound to the exact candidate subject.",
            human_decision_schema(
                &["project_id", "changeset_id", "expected_project_revision"],
                json!({"project_id":{"type":"string"},"changeset_id":{"type":"string"},"expected_project_revision":{"type":"integer","minimum":0},"run_id":{"type":"string"},"command_id":{"type":"string"}}),
            ),
        ),
        tool(
            "changeset.reject",
            "Reject changeset",
            "Reject a pending changeset with a human decision bound to the exact candidate subject.",
            human_decision_schema(
                &["project_id", "changeset_id"],
                json!({"project_id":{"type":"string"},"changeset_id":{"type":"string"},"command_id":{"type":"string"}}),
            ),
        ),
    ]
}
