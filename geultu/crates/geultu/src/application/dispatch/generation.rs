//! Generation routes from request preparation through an untrusted draft.

use super::super::requests::generation::GenerationGenerate;
use super::super::{parse, Application};
use crate::error::Result;
use crate::generation::GenerationRequest;
use serde_json::{json, Value};

pub(in crate::application) fn call(
    app: &Application,
    name: &str,
    arguments: Value,
) -> Result<Value> {
    match name {
        "writing.types.list" => Ok(json!({
            "writing_types": app.generation.writing_types(),
            "providers": app.generation.provider_ids()
        })),
        "generation.prepare" => {
            let request: GenerationRequest = parse(arguments)?;
            Ok(serde_json::to_value(app.prepare_generation(request)?)?)
        }
        "generation.generate" => {
            let request: GenerationGenerate = parse(arguments)?;
            let provider_id = request.provider_id;
            let plan = app.prepare_generation(request.request)?;
            let draft = app.generation.generate(&plan, &provider_id)?;
            Ok(json!({
                "plan": &plan,
                "provider_id": &provider_id,
                "draft": &draft,
                "candidate": {
                    "kind": format!("{}:{}", plan.writing_type.as_str(), plan.stage.as_str()),
                    "writing_type": plan.writing_type.as_str(),
                    "stage": plan.stage.as_str(),
                    "source_revision": plan.source_content_revision,
                    "content": &draft.content,
                    "provider": &provider_id,
                    "model": &draft.model,
                    "request_id": &draft.request_id,
                    "input_hash": plan.input_hash
                }
            }))
        }
        _ => Err(super::unknown_tool(name)),
    }
}
