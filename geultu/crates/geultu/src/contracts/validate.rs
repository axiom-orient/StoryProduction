//! Pattern predicates and bound-check helpers backing persisted contract
//! validation. Shared character-class predicates may also serve domain-owned
//! values while field meaning and length rules stay with their owner.

use crate::error::{Error, Result};
use serde::{Deserialize, Deserializer};
use std::collections::BTreeSet;

pub(crate) fn all_lowercase_hex(value: &str) -> bool {
    all_hex(value) && value.bytes().all(|byte| !byte.is_ascii_uppercase())
}

/// Validates a contract-owned prefix followed by exactly 64 lowercase hex digits.
/// The caller retains ownership of the prefix and the field-specific meaning.
pub(crate) fn valid_prefixed_sha256(value: &str, prefix: &str) -> bool {
    !prefix.is_empty() && value.strip_prefix(prefix).is_some_and(valid_sha256_hex)
}

/// Returns whether all bytes are ASCII hexadecimal digits; length is caller-owned.
pub(crate) fn all_hex(value: &str) -> bool {
    value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn valid_sha256_hex(value: &str) -> bool {
    value.len() == 64 && all_lowercase_hex(value)
}

pub(crate) fn valid_package_id(value: &str) -> bool {
    valid_prefixed_sha256(value, "np_")
}

pub(crate) fn valid_sha256_digest(value: &str) -> bool {
    valid_prefixed_sha256(value, "sha256:")
}

pub(crate) fn valid_scene_id(value: &str) -> bool {
    match value.strip_prefix("scene_") {
        Some(rest) => {
            !rest.is_empty()
                && rest.len() <= 120
                && rest
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        }
        None => false,
    }
}

/// ASCII language-tag grammar; the producer owns the minimum subtag width.
pub(crate) fn valid_language_tag(value: &str, min_subtag_chars: usize) -> bool {
    let mut segments = value.split('-');
    let Some(primary) = segments.next() else {
        return false;
    };
    let primary_bytes = primary.as_bytes();
    if !(2..=3).contains(&primary_bytes.len()) || !primary_bytes.iter().all(u8::is_ascii_alphabetic)
    {
        return false;
    }
    segments.all(|segment| {
        let bytes = segment.as_bytes();
        (min_subtag_chars..=8).contains(&bytes.len()) && bytes.iter().all(u8::is_ascii_alphanumeric)
    })
}

/// `^[a-z][a-z0-9_.-]{0,max_len}$`
pub(crate) fn valid_lowercase_token(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'.' | b'-')
        })
        && value
            .bytes()
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
}

/// `^[A-Za-z0-9][A-Za-z0-9_.:/-]{0,191}$`
pub(crate) fn valid_artifact_local(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 192
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'/' | b'-')
        })
        && value
            .bytes()
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric())
}

/// `^[A-Z][A-Z0-9_]{2,127}$` — finding/reason-code pattern shared by
/// validation-receipt findings and domain-decision reason codes.
pub(crate) fn valid_finding_code(value: &str) -> bool {
    let bytes = value.as_bytes();
    (3..=128).contains(&bytes.len())
        && bytes.first().is_some_and(|byte| byte.is_ascii_uppercase())
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || *byte == b'_')
}

pub(crate) fn check_length(label: &str, value: &str, min: usize, max: usize) -> Result<()> {
    let length = value.chars().take(max.saturating_add(1)).count();
    if (min..=max).contains(&length) {
        Ok(())
    } else {
        Err(Error::InvalidInput(format!(
            "{label} must contain {min}..={max} characters"
        )))
    }
}

pub(crate) fn check_items(
    label: &str,
    items: &[String],
    max_count: usize,
    max_item_chars: usize,
) -> Result<()> {
    if items.len() > max_count {
        return Err(Error::InvalidInput(format!(
            "{label} must contain at most {max_count} items"
        )));
    }
    for item in items {
        check_length(label, item, 1, max_item_chars)?;
    }
    Ok(())
}

pub(crate) fn check_unique(label: &str, items: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for item in items {
        if !seen.insert(item.as_str()) {
            return Err(Error::InvalidInput(format!(
                "{label} must contain unique items; '{item}' appears more than once"
            )));
        }
    }
    Ok(())
}

/// Exact three-component numeric version used by both producer version patterns.
pub(super) fn valid_numeric_version(value: &str) -> bool {
    let mut parts = value.split('.');
    for _ in 0..3 {
        if !parts
            .next()
            .is_some_and(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return false;
        }
    }
    parts.next().is_none()
}

/// Missing optional fields use `default`; present fields must decode as T, not null.
pub(super) fn deserialize_present<'de, D, T>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
