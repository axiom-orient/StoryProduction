//! Image-local typed decoding and prompt-field projections.
use std::collections::BTreeMap;

use crate::json::{DecodeError, JsonValue, expect_string, take_optional, take_required};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptLanguage {
    Korean,
    English,
}

impl PromptLanguage {
    pub(crate) fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "ko" | "korean" => Ok(Self::Korean),
            "en" | "english" => Ok(Self::English),
            _ => Err(DecodeError::new(path, "expected one of: ko, en")),
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::Korean => "ko",
            Self::English => "en",
        }
    }
}

pub(crate) fn take_language(
    fields: &mut BTreeMap<String, JsonValue>,
    path: &str,
) -> Result<PromptLanguage, DecodeError> {
    match take_optional(fields, "language") {
        Some(value) => {
            let value = expect_string(value, &format!("{path}.language"))?;
            PromptLanguage::parse(&value, &format!("{path}.language"))
        }
        None => Ok(PromptLanguage::Korean),
    }
}

pub(crate) fn take_object_array<T>(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
    mut decode: impl FnMut(JsonValue, &str) -> Result<T, DecodeError>,
) -> Result<Vec<T>, DecodeError> {
    let value = take_required(fields, key, path)?;
    let values = match value {
        JsonValue::Array(values) => values,
        _ => return Err(DecodeError::new(format!("{path}.{key}"), "expected array")),
    };
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| decode(value, &format!("{path}.{key}[{index}]")))
        .collect()
}

pub(crate) fn take_optional_object_array<T>(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
    mut decode: impl FnMut(JsonValue, &str) -> Result<T, DecodeError>,
) -> Result<Vec<T>, DecodeError> {
    let Some(value) = take_optional(fields, key) else {
        return Ok(Vec::new());
    };
    let values = match value {
        JsonValue::Array(values) => values,
        _ => return Err(DecodeError::new(format!("{path}.{key}"), "expected array")),
    };
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| decode(value, &format!("{path}.{key}[{index}]")))
        .collect()
}

/// Normalizes one user-owned scalar before inserting it into a compiler-owned
/// Markdown structure.
///
/// Typed fields carry instructions, but they must not be able to create new
/// top-level sections merely by containing a newline followed by `##`. Keeping
/// the complete token sequence while folding whitespace preserves semantics and
/// keeps section ownership with the compiler.
pub(crate) fn prompt_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for token in value.split_whitespace() {
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(token);
    }
    output
}

pub(crate) fn quote(value: &str) -> String {
    JsonValue::from(prompt_text(value)).to_compact_string()
}

pub(crate) fn heading(language: PromptLanguage, ko: &str, en: &str) -> String {
    match language {
        PromptLanguage::Korean => format!("## {ko}"),
        PromptLanguage::English => format!("## {en}"),
    }
}

pub(crate) fn label<'a>(language: PromptLanguage, ko: &'a str, en: &'a str) -> &'a str {
    match language {
        PromptLanguage::Korean => ko,
        PromptLanguage::English => en,
    }
}
