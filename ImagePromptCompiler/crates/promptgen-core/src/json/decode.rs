use std::collections::BTreeMap;
use std::fmt;

use super::JsonValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError {
    pub path: String,
    pub message: String,
}

impl DecodeError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for DecodeError {}

pub fn into_object(
    value: JsonValue,
    path: &str,
) -> Result<BTreeMap<String, JsonValue>, DecodeError> {
    match value {
        JsonValue::Object(fields) => Ok(fields),
        _ => Err(DecodeError::new(path, "expected object")),
    }
}

pub fn into_array(value: JsonValue, path: &str) -> Result<Vec<JsonValue>, DecodeError> {
    match value {
        JsonValue::Array(values) => Ok(values),
        _ => Err(DecodeError::new(path, "expected array")),
    }
}

pub fn take_required(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<JsonValue, DecodeError> {
    fields
        .remove(key)
        .ok_or_else(|| DecodeError::new(format!("{path}.{key}"), "required field is missing"))
}

pub fn take_optional(fields: &mut BTreeMap<String, JsonValue>, key: &str) -> Option<JsonValue> {
    match fields.remove(key) {
        Some(JsonValue::Null) | None => None,
        value => value,
    }
}

pub fn take_string(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<String, DecodeError> {
    let value = take_required(fields, key, path)?;
    expect_string(value, &format!("{path}.{key}"))
}

pub fn take_optional_string(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<String>, DecodeError> {
    take_optional(fields, key)
        .map(|value| expect_string(value, &format!("{path}.{key}")))
        .transpose()
}

pub fn take_bool(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<bool, DecodeError> {
    let value = take_required(fields, key, path)?;
    expect_bool(value, &format!("{path}.{key}"))
}

pub fn take_optional_bool(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<bool>, DecodeError> {
    take_optional(fields, key)
        .map(|value| expect_bool(value, &format!("{path}.{key}")))
        .transpose()
}

pub fn take_u64(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<u64, DecodeError> {
    let value = take_required(fields, key, path)?;
    expect_u64(value, &format!("{path}.{key}"))
}

pub fn take_optional_u64(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<u64>, DecodeError> {
    take_optional(fields, key)
        .map(|value| expect_u64(value, &format!("{path}.{key}")))
        .transpose()
}

pub fn take_f64(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<f64, DecodeError> {
    let value = take_required(fields, key, path)?;
    expect_f64(value, &format!("{path}.{key}"))
}

pub fn take_optional_f64(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<f64>, DecodeError> {
    take_optional(fields, key)
        .map(|value| expect_f64(value, &format!("{path}.{key}")))
        .transpose()
}

pub fn take_string_array(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Vec<String>, DecodeError> {
    let value = take_required(fields, key, path)?;
    expect_string_array(value, &format!("{path}.{key}"))
}

pub fn take_optional_string_array(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<Vec<String>>, DecodeError> {
    take_optional(fields, key)
        .map(|value| expect_string_array(value, &format!("{path}.{key}")))
        .transpose()
}

pub fn expect_string(value: JsonValue, path: &str) -> Result<String, DecodeError> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(DecodeError::new(path, "expected string")),
    }
}

pub fn expect_bool(value: JsonValue, path: &str) -> Result<bool, DecodeError> {
    match value {
        JsonValue::Bool(value) => Ok(value),
        _ => Err(DecodeError::new(path, "expected boolean")),
    }
}

pub fn expect_u64(value: JsonValue, path: &str) -> Result<u64, DecodeError> {
    match value {
        JsonValue::Number(value) => value
            .parse::<u64>()
            .map_err(|_| DecodeError::new(path, "expected non-negative integer")),
        _ => Err(DecodeError::new(path, "expected non-negative integer")),
    }
}

pub fn expect_f64(value: JsonValue, path: &str) -> Result<f64, DecodeError> {
    match value {
        JsonValue::Number(value) => value
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .ok_or_else(|| DecodeError::new(path, "expected finite number")),
        _ => Err(DecodeError::new(path, "expected finite number")),
    }
}

pub fn expect_string_array(value: JsonValue, path: &str) -> Result<Vec<String>, DecodeError> {
    let values = into_array(value, path)?;
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| expect_string(value, &format!("{path}[{index}]")))
        .collect()
}

pub fn reject_unknown(fields: BTreeMap<String, JsonValue>, path: &str) -> Result<(), DecodeError> {
    if let Some(key) = fields.keys().next() {
        Err(DecodeError::new(format!("{path}.{key}"), "unknown field"))
    } else {
        Ok(())
    }
}
