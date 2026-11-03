//! Tool-schema projection of producer constraints, with locally scoped refs expanded for embedding.

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub(super) fn nonempty_text(generator: &mut SchemaGenerator) -> Schema {
    let mut schema = String::json_schema(generator);
    schema.insert("minLength".into(), 1.into());
    schema.insert("pattern".into(), "\\S".into());
    schema
}

pub(super) fn positive_duration(generator: &mut SchemaGenerator) -> Schema {
    let mut schema = u32::json_schema(generator);
    schema.insert("minimum".into(), 1.into());
    schema.insert("maximum".into(), u32::MAX.into());
    schema
}

fn expand(value: &Value, root: &Value, active: &mut BTreeSet<String>) -> Value {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                let pointer = reference
                    .strip_prefix('#')
                    .expect("DeliveryRequest refs are producer-local");
                assert!(
                    active.insert(reference.to_owned()),
                    "recursive producer request schema cannot be embedded"
                );
                let definition = root
                    .pointer(pointer)
                    .expect("producer schema reference must resolve");
                let expanded = expand(definition, root, active);
                active.remove(reference);
                let mut siblings = object.clone();
                siblings.remove("$ref");
                if siblings.is_empty() {
                    expanded
                } else {
                    serde_json::json!({"allOf": [expanded, expand(&Value::Object(siblings), root, active)]})
                }
            } else {
                Value::Object(
                    object
                        .iter()
                        .map(|(key, value)| (key.clone(), expand(value, root, active)))
                        .collect(),
                )
            }
        }
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| expand(value, root, active))
                .collect(),
        ),
        other => other.clone(),
    }
}

pub(super) fn delivery_request(_: &mut SchemaGenerator) -> Schema {
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    SCHEMA
        .get_or_init(|| {
            let source: Value = serde_json::from_str(include_str!(
                "../../../../contracts/delivery-request.schema.json"
            ))
            .expect("embedded producer DeliveryRequest schema must be JSON");
            let mut embedded = source.clone();
            for key in ["$defs", "$id", "$schema"] {
                embedded
                    .as_object_mut()
                    .expect("producer schema object")
                    .remove(key);
            }
            Schema::try_from(expand(&embedded, &source, &mut BTreeSet::new()))
                .expect("producer schema projection must be an object")
        })
        .clone()
}
