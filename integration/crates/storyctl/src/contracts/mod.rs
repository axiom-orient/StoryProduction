use crate::effects::{files, hash, json};
use crate::error::{Error, Result};
use jsonschema::{Draft, Retrieve, Uri, Validator};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
struct LocalRetriever(Arc<BTreeMap<String, Value>>);

impl Retrieve for LocalRetriever {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> std::result::Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.0.get(uri.as_str()).cloned().ok_or_else(|| {
            format!("schema reference is outside the registered local contracts: {uri}").into()
        })
    }
}

/// A closed Draft 2020-12 registry. No HTTP or filesystem reference retriever.
pub struct SchemaRegistry {
    validators: BTreeMap<String, Validator>,
}

impl SchemaRegistry {
    pub fn new(contract_dir: &Path) -> Result<Self> {
        let contract_dir = contract_dir.canonicalize()?;
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&contract_dir)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<_>>()?;
        paths.retain(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".schema.json"))
        });
        paths.sort();
        let mut resources = BTreeMap::new();
        let mut schemas = Vec::new();
        for path in paths {
            let schema = json::parse_object(&files::read_bytes(&path)?)?;
            let file_name = path
                .file_name()
                .unwrap()
                .to_str()
                .ok_or_else(|| Error::not_applied("schema file name is not UTF-8"))?
                .to_string();
            let key = match schema.get("$id") {
                Some(Value::String(id)) if !id.is_empty() => {
                    if resources.insert(id.clone(), schema.clone()).is_some() {
                        return Err(Error::not_applied(format!("duplicate schema $id: {id}")));
                    }
                    id.clone()
                }
                None => file_name.clone(), // Producer schemas without an $id still compile explicitly.
                _ => {
                    return Err(Error::not_applied(format!(
                        "schema has an invalid $id: {}",
                        path.display()
                    )));
                }
            };
            schemas.push((key, file_name, schema));
        }
        let retriever = LocalRetriever(Arc::new(resources));
        let mut validators = BTreeMap::new();
        for (key, file_name, schema) in schemas {
            let validator = jsonschema::options()
                .with_draft(Draft::Draft202012)
                .should_validate_formats(true)
                .with_retriever(retriever.clone())
                .build(&schema)
                .map_err(|error| {
                    Error::not_applied(format!("invalid schema {file_name}: {error}"))
                })?;
            validators.insert(key, validator);
        }
        Ok(Self { validators })
    }

    pub fn validate(&self, schema_id: &str, value: &Value) -> Result<()> {
        let validator = self
            .validators
            .get(schema_id)
            .ok_or_else(|| Error::not_applied(format!("unknown schema id: {schema_id}")))?;
        let mut errors: Vec<_> = validator.iter_errors(value).collect();
        errors.sort_by_key(|error| error.instance_path().to_string());
        if let Some(first) = errors.first() {
            return Err(Error::not_applied(format!(
                "{schema_id} invalid at {}: {first}",
                first.instance_path()
            )));
        }
        Ok(())
    }
}

fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            Error::not_applied(format!(
                "contract registry {field} must be a non-empty string"
            ))
        })
}

fn pinned_path(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty()
        || relative.contains([':', '\0', '\\'])
        || Path::new(relative).is_absolute()
        || Path::new(relative)
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(Error::not_applied(
            "contract registry path must be a safe relative path",
        ));
    }
    files::non_symlink_path(&root.join(relative))
}

pub fn verify_contract_registry(story_root: &Path, registry_path: &Path) -> Result<Value> {
    let root = story_root.canonicalize()?;
    let registry = json::parse_object(&files::read_bytes(registry_path)?)?;
    if registry.get("schemaVersion").and_then(Value::as_str) != Some("story.contract-registry/1") {
        return Err(Error::not_applied("unsupported contract registry version"));
    }
    let entries = registry
        .get("contracts")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::not_applied("contract registry requires a contracts array"))?;
    let mut ids = BTreeSet::new();
    let mut results = Vec::new();
    for entry in entries {
        let id = string(entry, "id")?;
        if !ids.insert(id) {
            return Err(Error::not_applied(format!(
                "duplicate contract registry id: {id}"
            )));
        }
        let owner = string(entry, "owner")?;
        let producer_relative = string(entry, "producer")?;
        let producer = pinned_path(&root, producer_relative)?;
        let producer_bytes = files::read_bytes(&producer)?;
        let mut consumer_hashes = Vec::new();
        match string(entry, "verification")? {
            "byte_equal" => {
                let consumers = entry
                    .get("consumers")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        Error::not_applied("byte_equal pin requires a consumers array")
                    })?;
                for consumer in consumers {
                    let relative = consumer
                        .as_str()
                        .ok_or_else(|| Error::not_applied("consumer path must be a string"))?;
                    let bytes = files::read_bytes(&pinned_path(&root, relative)?)?;
                    if bytes != producer_bytes {
                        return Err(Error::not_applied(format!(
                            "contract drift: {id} producer {producer_relative} != consumer {relative}"
                        )));
                    }
                    consumer_hashes
                        .push(json!({"path": relative, "sha256": hash::sha256_bytes(&bytes)}));
                }
            }
            "schema" => {} // Content pin registry does not replace owner semantic validation.
            _ => {
                return Err(Error::not_applied(
                    "unsupported contract registry verification",
                ));
            }
        }
        results.push(json!({"id": id, "owner": owner, "producer": producer_relative, "sha256": hash::sha256_bytes(&producer_bytes), "consumers": consumer_hashes, "status": "PASS"}));
    }
    Ok(json!({"status": "PASS", "contracts": results}))
}
