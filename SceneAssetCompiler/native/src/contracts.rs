//! SAC-owned validation of pinned NVC input and SAC producer contracts.
use crate::{check, files, Error, Result};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("strict JSON without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Strict, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Strict(Value::Number(n)))
                    .ok_or_else(|| E::custom("NON_FINITE_NUMBER"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut out = Vec::new();
                while let Some(v) = a.next_element::<Strict>()? {
                    out.push(v.0);
                }
                Ok(Strict(out.into()))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Strict, A::Error> {
                let mut out = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if out.contains_key(&k) {
                        return Err(de::Error::custom(format!("DUPLICATE_JSON_KEY:{k}")));
                    }
                    out.insert(k, a.next_value::<Strict>()?.0);
                }
                Ok(Strict(out.into()))
            }
        }
        d.deserialize_any(V)
    }
}

pub fn parse(raw: &[u8]) -> Result<Value> {
    let mut decoder = serde_json::Deserializer::from_slice(raw);
    let value = Strict::deserialize(&mut decoder)
        .map_err(|e| Error::refused(format!("JSON_INVALID:{e}")))?
        .0;
    decoder
        .end()
        .map_err(|e| Error::refused(format!("JSON_INVALID:{e}")))?;
    check(value.is_object(), "JSON_OBJECT_REQUIRED")?;
    Ok(value)
}

fn integer_profile(value: &Value) -> Result<()> {
    match value {
        Value::Number(v) => check(
            v.as_i64()
                .is_some_and(|n| n.unsigned_abs() <= 9_007_199_254_740_991)
                || v.as_u64().is_some_and(|n| n <= 9_007_199_254_740_991),
            "CANONICAL_SAFE_INTEGER_REQUIRED",
        ),
        Value::Array(v) => v.iter().try_for_each(integer_profile),
        Value::Object(v) => v.values().try_for_each(integer_profile),
        _ => Ok(()),
    }
}

pub fn canonical(value: &Value) -> Result<Vec<u8>> {
    // Both pinned NVC and SAC schemas have closed ASCII keys. serde_json's sorted map
    // therefore preserves the existing compact, UTF-8 producer canonical bytes.
    serde_json::to_vec(value).map_err(|e| Error::refused(e.to_string()))
}

pub fn seal(mut value: Value) -> Result<Value> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| Error::refused("JSON_OBJECT_REQUIRED"))?;
    object.remove("contentHash");
    let digest = format!("sha256:{}", sha256(&canonical(&value)?));
    value["contentHash"] = Value::String(digest);
    Ok(value)
}

#[derive(Clone)]
struct LocalRefs(HashMap<String, Value>);
impl jsonschema::Retrieve for LocalRefs {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> std::result::Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.0
            .get(uri.as_str())
            .cloned()
            .ok_or_else(|| format!("UNREGISTERED_SCHEMA_REF:{uri}").into())
    }
}

pub struct Contracts {
    schemas: HashMap<String, Value>,
    refs: LocalRefs,
}
impl Contracts {
    pub fn new(owner_root: &Path) -> Result<Self> {
        let mut schemas = HashMap::new();
        let mut refs = HashMap::new();
        for (producer, folder, names) in [
            (
                "sac",
                "docs/contracts",
                vec!["animatic-assets-v1", "animatic-assets-review-v1"],
            ),
            (
                "nvc",
                "contracts/narrative-courier",
                vec![
                    "animatic-packet-v1",
                    "animatic-plan-v1",
                    "animatic-review-v1",
                ],
            ),
        ] {
            for name in names {
                let path = owner_root.join(folder).join(format!("{name}.schema.json"));
                let schema = parse(&files::read_absolute(&path, 16 * 1024 * 1024)?)?;
                let id = schema["$id"]
                    .as_str()
                    .ok_or_else(|| Error::refused("SCHEMA_ID_REQUIRED"))?
                    .to_owned();
                check(
                    refs.insert(id, schema.clone()).is_none(),
                    "DUPLICATE_SCHEMA_ID",
                )?;
                schemas.insert(format!("{producer}:{name}"), schema);
            }
        }
        Ok(Self {
            schemas,
            refs: LocalRefs(refs),
        })
    }
    pub fn validate(&self, producer: &str, name: &str, value: &Value) -> Result<()> {
        if producer == "nvc" {
            integer_profile(value)?;
        }
        let schema = self
            .schemas
            .get(&format!("{producer}:{name}"))
            .ok_or_else(|| Error::refused("SCHEMA_NOT_FOUND"))?;
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .should_validate_formats(true)
            .with_retriever(self.refs.clone())
            .build(schema)
            .map_err(|e| Error::refused(format!("SCHEMA_INVALID:{e}")))?;
        validator
            .validate(value)
            .map_err(|e| Error::refused(format!("CONTRACT_INVALID:{e}")))?;
        check(
            value["contentHash"] == seal(value.clone())?["contentHash"],
            "CONTENT_HASH_CHANGED",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_json_rejects_ambiguous_keys() {
        assert!(parse(br#"{"a":1,"a":2}"#).is_err());
        assert!(parse(b"[]").is_err());
    }
    #[test]
    fn canonical_matches_existing_unicode_and_integer_profile() {
        assert_eq!(
            canonical(&serde_json::json!({"z":1,"a":"한글"})).unwrap(),
            "{\"a\":\"한글\",\"z\":1}".as_bytes()
        );
        assert_eq!(
            canonical(&serde_json::json!({"a":1.0})).unwrap(),
            b"{\"a\":1.0}"
        );
        assert!(integer_profile(&serde_json::json!({"a":1.0})).is_err());
    }
}
