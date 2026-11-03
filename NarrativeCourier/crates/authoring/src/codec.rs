//! Strict wire decoder and owner canonical/immutable byte profiles.

pub(crate) mod value;
pub(crate) use value::{json, Map, Value};

use crate::{require, Error, Result, MAX_JSON_BYTES};
use narrative_courier_runtime::{store::sha256_bytes, token::jcs_bytes};

pub(crate) fn decode(bytes: &[u8]) -> Result<Value> {
    require(bytes.len() <= MAX_JSON_BYTES, "JSON_TOO_LARGE")?;
    let value: Value = serde_json::from_slice(bytes)?;
    require(value.is_object(), "OBJECT_REQUIRED")?;
    jcs_bytes(&value)?;
    Ok(value)
}
pub(crate) fn raw_hash(bytes: &[u8]) -> String {
    format!("sha256:{}", sha256_bytes(bytes))
}
pub(crate) fn canonical_bytes(value: &Value) -> Result<Vec<u8>> {
    Ok(jcs_bytes(value)?)
}
pub(crate) fn content_hash(value: &Value) -> Result<String> {
    let mut body = value.clone();
    body.as_object_mut()
        .ok_or_else(|| Error::refusal("OBJECT_REQUIRED"))?
        .remove("contentHash");
    Ok(raw_hash(&canonical_bytes(&body)?))
}
pub(crate) fn seal(mut value: Value) -> Result<Value> {
    let hash = content_hash(&value)?;
    value
        .as_object_mut()
        .ok_or_else(|| Error::refusal("OBJECT_REQUIRED"))?
        .insert("contentHash".into(), hash.into());
    Ok(value)
}
pub(crate) fn artifact_bytes(value: &Value) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicates_floats_and_unsafe_numbers() {
        for text in [
            r#"{"a":1,"a":2}"#,
            r#"{"a":{"b":1,"b":2}}"#,
            r#"{"a":1.0}"#,
            r#"{"a":9007199254740992}"#,
            r#"{"a":NaN}"#,
            r#"{"a":"\ud800"}"#,
            "[]",
        ] {
            assert!(decode(text.as_bytes()).is_err(), "{text}");
        }
    }
    #[test]
    fn python_pretty_bytes_and_utf16_canonical_profile_are_distinct() {
        let value = decode("{\"z\":\"한글\",\"𐀀\":1,\"\":2}".as_bytes()).unwrap();
        assert_eq!(
            artifact_bytes(&value).unwrap(),
            "{\n  \"z\": \"한글\",\n  \"𐀀\": 1,\n  \"\": 2\n}\n".as_bytes()
        );
        assert_eq!(
            canonical_bytes(&value).unwrap(),
            "{\"z\":\"한글\",\"𐀀\":1,\"\":2}".as_bytes()
        );
    }

    #[test]
    fn private_ordered_profile_does_not_change_the_existing_serde_json_profile() {
        let input = br#"{"z":1,"a":2}"#;
        let ordered = decode(input).unwrap();
        assert_eq!(
            artifact_bytes(&ordered).unwrap(),
            b"{\n  \"z\": 1,\n  \"a\": 2\n}\n"
        );
        let semantic: serde_json::Value = serde_json::from_slice(input).unwrap();
        assert_eq!(
            serde_json::to_string(&semantic).unwrap(),
            r#"{"a":2,"z":1}"#
        );
        assert_eq!(ordered, decode(br#"{"a":2,"z":1}"#).unwrap());
    }
}
