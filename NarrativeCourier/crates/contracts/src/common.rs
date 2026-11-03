//! Shared scalar, identity, and uniqueness invariants for NVC contracts.

use std::collections::BTreeSet;

use narrative_courier_runtime::{Error, Result};

pub(super) fn nonempty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(Error::Contract(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

pub(super) fn sha(value: &str, field: &str) -> Result<()> {
    if is_lowercase_sha256(value) {
        Ok(())
    } else {
        Err(Error::Contract(format!(
            "{field} must be lowercase SHA-256"
        )))
    }
}

pub fn is_narrative_ref(value: &str) -> bool {
    ["SRC-", "REL-", "GAP-", "CON-", "SCN-"]
        .iter()
        .any(|p| value.starts_with(p))
        && value
            .split_once('-')
            .map(|(_, tail)| {
                !tail.is_empty()
                    && tail
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            })
            .unwrap_or(false)
}

pub(super) fn unique<'a>(values: impl IntoIterator<Item = &'a String>, field: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(Error::Contract(format!("duplicate {field}: {value}")));
        }
    }
    Ok(())
}

pub(super) fn nonempty_items(values: &[String], field: &str) -> Result<()> {
    for value in values {
        nonempty(value, field)?;
    }
    Ok(())
}

pub(super) fn valid_aspect_ratio(value: &str) -> bool {
    let Some((a, b)) = value.split_once(':') else {
        return false;
    };
    !a.is_empty()
        && !b.is_empty()
        && !a.starts_with('0')
        && !b.starts_with('0')
        && a.bytes().all(|c| c.is_ascii_digit())
        && b.bytes().all(|c| c.is_ascii_digit())
}

pub(super) fn valid_asset_id(value: &str) -> bool {
    let Some((head, tail)) = value.split_once('-') else {
        return false;
    };
    !head.is_empty()
        && head.bytes().all(|c| c.is_ascii_uppercase())
        && !tail.is_empty()
        && tail
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}

pub(super) fn valid_prefixed_sha256(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(is_lowercase_sha256)
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// Missing optional fields use serde defaults; a present value must have the schema's actual type.
pub(super) fn deserialize_present<'de, D, T>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Producer identity for a serialized object with its designated identity field omitted.
pub(super) fn body_identity<T: serde::Serialize>(
    value: &T,
    field: &str,
    prefix: &str,
) -> Result<String> {
    let mut body = serde_json::to_value(value)?;
    let object = body
        .as_object_mut()
        .ok_or_else(|| Error::Contract("identity body must be an object".into()))?;
    if object.remove(field).is_none() {
        return Err(Error::Contract(format!(
            "identity body has no {field} field"
        )));
    }
    Ok(format!(
        "{prefix}{}",
        narrative_courier_runtime::token::hash_jcs(&body)?
    ))
}
