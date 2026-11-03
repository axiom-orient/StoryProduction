use crate::store::sha256_bytes;
use crate::{Error, Result};
use serde::Serialize;
use serde_json::Value;

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Canonical JSON for the schema-adapted NarrativeCourier safe-integer profile of RFC 8785.
/// Payloads must already be valid contract data; this is not a wire parser or schema validator.
/// Serialized floating representations and integers outside ±(2^53-1) fail instead of being rounded.
/// Input adaptation (including duplicate fields and non-finite custom serializers) is caller-owned.
pub fn jcs_bytes<T: Serialize>(payload: &T) -> Result<Vec<u8>> {
    let value = serde_json::to_value(payload)?;
    let mut out = Vec::new();
    write_jcs(&value, &mut out)?;
    Ok(out)
}

pub fn hash_jcs<T: Serialize>(payload: &T) -> Result<String> {
    Ok(sha256_bytes(&jcs_bytes(payload)?))
}

fn write_jcs(value: &Value, out: &mut Vec<u8>) -> Result<()> {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Number(n) => {
            if let Some(v) = n.as_i64() {
                if v.unsigned_abs() > MAX_SAFE_INTEGER {
                    return Err(Error::Contract(
                        "integer is outside the canonical safe-integer profile".into(),
                    ));
                }
                out.extend_from_slice(v.to_string().as_bytes());
            } else if let Some(v) = n.as_u64() {
                if v > MAX_SAFE_INTEGER {
                    return Err(Error::Contract(
                        "integer is outside the canonical safe-integer profile".into(),
                    ));
                }
                out.extend_from_slice(v.to_string().as_bytes());
            } else {
                return Err(Error::Contract(
                    "non-integer JSON number is outside NarrativeCourier canonical contract domain"
                        .into(),
                ));
            }
        }
        Value::String(s) => out.extend_from_slice(serde_json::to_string(s)?.as_bytes()),
        Value::Array(values) => {
            out.push(b'[');
            for (i, item) in values.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_jcs(item, out)?;
            }
            out.push(b']');
        }
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            out.push(b'{');
            for (i, (key, item)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                out.extend_from_slice(serde_json::to_string(key)?.as_bytes());
                out.push(b':');
                write_jcs(item, out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}
