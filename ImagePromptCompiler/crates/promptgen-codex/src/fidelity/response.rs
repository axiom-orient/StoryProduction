use std::collections::BTreeMap;

use promptgen_core::json::JsonValue;

use super::{CodexExecutionError, SubjectCountCheck, TextFidelityCheck, VisibleInstance};

pub(super) fn subject_counts(
    object: &BTreeMap<String, JsonValue>,
) -> Result<Vec<SubjectCountCheck>, CodexExecutionError> {
    required_array(object, "subject_counts")?
        .iter()
        .map(|value| {
            let value = value
                .as_object()
                .ok_or_else(|| invalid("subject_counts item is not an object"))?;
            let expected = required_u16(value, "expected")?;
            let observed = required_u16(value, "observed")?;
            let instances = visible_instances(value)?;
            Ok(SubjectCountCheck {
                id: required_string(value, "id")?.to_owned(),
                expected,
                observed,
                evidence: required_string(value, "evidence")?.to_owned(),
                instances,
            })
        })
        .collect()
}

fn visible_instances(
    object: &BTreeMap<String, JsonValue>,
) -> Result<Vec<VisibleInstance>, CodexExecutionError> {
    required_array(object, "instances")?
        .iter()
        .map(|value| {
            let value = value
                .as_object()
                .ok_or_else(|| invalid("instances item is not an object"))?;
            Ok(VisibleInstance {
                x_percent: required_u8(value, "x_percent")?,
                y_percent: required_u8(value, "y_percent")?,
                evidence: required_string(value, "evidence")?.to_owned(),
            })
        })
        .collect()
}

pub(super) fn text_checks(
    object: &BTreeMap<String, JsonValue>,
) -> Result<Vec<TextFidelityCheck>, CodexExecutionError> {
    required_array(object, "text_checks")?
        .iter()
        .map(|value| {
            let value = value
                .as_object()
                .ok_or_else(|| invalid("text_checks item is not an object"))?;
            Ok(TextFidelityCheck {
                id: required_string(value, "id")?.to_owned(),
                exact: required_bool(value, "exact")?,
                observed_text: required_string(value, "observed_text")?.to_owned(),
                evidence: required_string(value, "evidence")?.to_owned(),
            })
        })
        .collect()
}

pub(super) fn string_array(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<String>, CodexExecutionError> {
    required_array(object, key)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| invalid(format!("{key} contains a non-string")))
        })
        .collect()
}

fn required_array<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a [JsonValue], CodexExecutionError> {
    object
        .get(key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| invalid(format!("{key} is missing or not an array")))
}

pub(super) fn required_string<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, CodexExecutionError> {
    object
        .get(key)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| invalid(format!("{key} is missing or not a string")))
}

pub(super) fn required_bool(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<bool, CodexExecutionError> {
    object
        .get(key)
        .and_then(JsonValue::as_bool)
        .ok_or_else(|| invalid(format!("{key} is missing or not a boolean")))
}

fn required_u16(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<u16, CodexExecutionError> {
    object
        .get(key)
        .and_then(JsonValue::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(|| invalid(format!("{key} is missing or outside u16")))
}

fn required_u8(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<u8, CodexExecutionError> {
    object
        .get(key)
        .and_then(JsonValue::as_u64)
        .filter(|value| *value <= 100)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or_else(|| invalid(format!("{key} is missing or outside 0..=100")))
}

pub(super) fn invalid(message: impl Into<String>) -> CodexExecutionError {
    CodexExecutionError::new("CODEX_FIDELITY_RESULT", message)
}
