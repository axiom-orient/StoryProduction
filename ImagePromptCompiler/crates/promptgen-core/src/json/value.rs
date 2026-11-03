use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    pub fn object(entries: impl IntoIterator<Item = (impl Into<String>, JsonValue)>) -> Self {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    pub fn array(values: impl IntoIterator<Item = JsonValue>) -> Self {
        Self::Array(values.into_iter().collect())
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    /// Builds a JSON array of strings from any borrowed string sequence.
    ///
    /// Every request model serializes string collections, so the conversion
    /// lives on the value type instead of being re-spelled at each call site.
    pub fn strings<'a>(
        values: impl IntoIterator<Item = &'a (impl AsRef<str> + 'a + ?Sized)>,
    ) -> Self {
        Self::Array(
            values
                .into_iter()
                .map(|value| Self::String(value.as_ref().to_owned()))
                .collect(),
        )
    }

    pub fn number(value: impl ToString) -> Self {
        Self::Number(value.to_string())
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn to_compact_string(&self) -> String {
        let mut output = String::new();
        write_value(self, &mut output, 0, false).expect("writing to String cannot fail");
        output
    }

    pub fn to_pretty_string(&self) -> String {
        let mut output = String::new();
        write_value(self, &mut output, 0, true).expect("writing to String cannot fail");
        output.push('\n');
        output
    }
}

impl From<&str> for JsonValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for JsonValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<bool> for JsonValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<u64> for JsonValue {
    fn from(value: u64) -> Self {
        Self::Number(value.to_string())
    }
}

impl From<usize> for JsonValue {
    fn from(value: usize) -> Self {
        Self::Number(value.to_string())
    }
}

impl From<i64> for JsonValue {
    fn from(value: i64) -> Self {
        Self::Number(value.to_string())
    }
}

fn write_value(value: &JsonValue, output: &mut String, depth: usize, pretty: bool) -> fmt::Result {
    match value {
        JsonValue::Null => output.push_str("null"),
        JsonValue::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        JsonValue::Number(value) => output.push_str(value),
        JsonValue::String(value) => write_string(value, output)?,
        JsonValue::Array(values) => {
            output.push('[');
            if !values.is_empty() {
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    if pretty {
                        output.push('\n');
                        indent(output, depth + 1);
                    }
                    write_value(value, output, depth + 1, pretty)?;
                }
                if pretty {
                    output.push('\n');
                    indent(output, depth);
                }
            }
            output.push(']');
        }
        JsonValue::Object(fields) => {
            output.push('{');
            if !fields.is_empty() {
                for (index, (key, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    if pretty {
                        output.push('\n');
                        indent(output, depth + 1);
                    }
                    write_string(key, output)?;
                    output.push(':');
                    if pretty {
                        output.push(' ');
                    }
                    write_value(value, output, depth + 1, pretty)?;
                }
                if pretty {
                    output.push('\n');
                    indent(output, depth);
                }
            }
            output.push('}');
        }
    }
    Ok(())
}

fn write_string(value: &str, output: &mut String) -> fmt::Result {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000C}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{0000}'..='\u{001F}' => write!(output, "\\u{:04X}", character as u32)?,
            _ => output.push(character),
        }
    }
    output.push('"');
    Ok(())
}

fn indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}
