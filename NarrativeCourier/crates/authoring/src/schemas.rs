//! Embedded producer schemas. No HTTP/file retrieval and no new schema authority.

use crate::codec::{json, Value};
use crate::{codec, require, Error, Result};

fn schema(name: &str) -> Result<Value> {
    let source = match name {
        "text-storyboard-v1" => include_str!("../../../contracts/text-storyboard-v1.schema.json"),
        "text-storyboard-task-v1" => {
            include_str!("../../../contracts/text-storyboard-task-v1.schema.json")
        }
        "text-storyboard-review-v1" => {
            include_str!("../../../contracts/text-storyboard-review-v1.schema.json")
        }
        "animatic-plan-v1" => include_str!("../../../contracts/animatic-plan-v1.schema.json"),
        "animatic-review-v1" => include_str!("../../../contracts/animatic-review-v1.schema.json"),
        "animatic-packet-v1" => include_str!("../../../contracts/animatic-packet-v1.schema.json"),
        _ => return Err(Error::refusal("UNKNOWN_PRODUCER_SCHEMA")),
    };
    let mut value: Value = serde_json::from_str(source)?;
    if name == "animatic-packet-v1" {
        // Resolve only the two producer-declared immutable refs, in memory.
        value["properties"]["plan"] = schema("animatic-plan-v1")?;
        value["properties"]["review"] = schema("animatic-review-v1")?;
    }
    Ok(value)
}
pub(crate) fn draft_schema() -> Result<Value> {
    let board = schema("text-storyboard-v1")?;
    let mut draft = board["$defs"]["draft"].clone();
    draft["$schema"] = board["$schema"].clone();
    draft["$defs"] = json!({"scene": board["$defs"]["scene"]});
    Ok(draft)
}
pub(crate) fn validate_draft(value: &Value) -> Result<()> {
    validate("text-storyboard-draft", &draft_schema()?, value)
}
fn validate(name: &str, schema: &Value, value: &Value) -> Result<()> {
    // Existing NVC Python admission did not enable a FormatChecker. Preserve that policy.
    let semantic_schema = schema.to_semantic();
    let semantic_value = value.to_semantic();
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(false)
        .build(&semantic_schema)
        .map_err(|error| Error::refusal(format!("SCHEMA_CONFIGURATION:{name}:{error}")))?;
    validator
        .validate(&semantic_value)
        .map_err(|error| Error::refusal(format!("SCHEMA:{name}:{error}")))
}
pub(crate) fn sealed(name: &str, value: &Value) -> Result<()> {
    codec::canonical_bytes(value)?;
    validate(name, &schema(name)?, value)?;
    require(
        value["contentHash"] == codec::content_hash(value)?,
        format!("CONTENT_HASH:{name}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_python_sealed_artifacts_keep_their_exact_bytes_and_hashes() {
        let fixtures: [(&str, &[u8]); 4] = [
            (
                "text-storyboard-task-v1",
                include_bytes!("../tests/fixtures/legacy-task.json"),
            ),
            (
                "text-storyboard-v1",
                include_bytes!("../tests/fixtures/legacy-board.json"),
            ),
            (
                "text-storyboard-review-v1",
                include_bytes!("../tests/fixtures/legacy-review.json"),
            ),
            (
                "animatic-packet-v1",
                include_bytes!("../tests/fixtures/legacy-animatic-packet.json"),
            ),
        ];
        for (name, bytes) in fixtures {
            let value = codec::decode(bytes).unwrap();
            sealed(name, &value).unwrap();
            let emitted = if name == "animatic-packet-v1" {
                sealed("animatic-plan-v1", &value["plan"]).unwrap();
                sealed("animatic-review-v1", &value["review"]).unwrap();
                let mut canonical = codec::canonical_bytes(&value).unwrap();
                canonical.push(b'\n');
                canonical
            } else {
                codec::artifact_bytes(&value).unwrap()
            };
            assert_eq!(emitted, bytes, "{name}");
        }
    }
}
