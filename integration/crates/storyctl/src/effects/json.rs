use crate::error::{Error, Result};
use serde_json::Value;
use std::collections::BTreeSet;

/// Decode one JSON value without silently overwriting duplicate keys.
pub fn parse_value(source: &[u8]) -> Result<Value> {
    if source.len() > super::MAX_BYTES {
        return Err(Error::not_applied("JSON input exceeds 16 MiB"));
    }
    std::str::from_utf8(source).map_err(|_| Error::not_applied("JSON is not UTF-8"))?;
    let mut scanner = Scanner { source, cursor: 0 };
    scanner.value(0)?;
    let value: Value = serde_json::from_slice(source)
        .map_err(|error| Error::not_applied(format!("invalid JSON: {error}")))?;
    Ok(value)
}

pub fn parse_object(source: &[u8]) -> Result<Value> {
    let value = parse_value(source)?;
    if !value.is_object() {
        return Err(Error::not_applied("input must be a JSON object"));
    }
    Ok(value)
}

/// Persist the established Python sorted, two-space, UTF-8 byte form.
/// This is Story's storage serializer, never an owner's canonical hash codec.
pub fn stable_bytes(value: &Value) -> Result<Vec<u8>> {
    let mut output = String::new();
    encode(value, 0, &mut output)?;
    output.push('\n');
    Ok(output.into_bytes())
}

fn encode(value: &Value, depth: usize, output: &mut String) -> Result<()> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(number) => {
            let raw = number.to_string();
            if raw.contains(['.', 'e', 'E']) {
                let float = raw
                    .parse::<f64>()
                    .map_err(|_| Error::not_applied("invalid JSON number"))?;
                if !float.is_finite() {
                    return Err(Error::not_applied("non-finite JSON value"));
                }
                output.push_str(&python_float(float)?);
            } else {
                output.push_str(if raw == "-0" { "0" } else { &raw });
            }
        }
        Value::String(value) => output.push_str(
            &serde_json::to_string(value).map_err(|error| Error::not_applied(error.to_string()))?,
        ),
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                output.push('\n');
                output.push_str(&"  ".repeat(depth + 1));
                encode(value, depth + 1, output)?;
                if index + 1 < values.len() {
                    output.push(',');
                }
            }
            if !values.is_empty() {
                output.push('\n');
                output.push_str(&"  ".repeat(depth));
            }
            output.push(']');
        }
        Value::Object(values) => {
            output.push('{');
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            for (index, (key, value)) in entries.iter().enumerate() {
                output.push('\n');
                output.push_str(&"  ".repeat(depth + 1));
                output.push_str(
                    &serde_json::to_string(key)
                        .map_err(|error| Error::not_applied(error.to_string()))?,
                );
                output.push_str(": ");
                encode(value, depth + 1, output)?;
                if index + 1 < entries.len() {
                    output.push(',');
                }
            }
            if !entries.is_empty() {
                output.push('\n');
                output.push_str(&"  ".repeat(depth));
            }
            output.push('}');
        }
    }
    Ok(())
}

fn python_float(value: f64) -> Result<String> {
    if value == 0.0 {
        return Ok(if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .into());
    }
    let raw =
        serde_json::to_string(&value).map_err(|error| Error::not_applied(error.to_string()))?;
    let (sign, unsigned) = if let Some(rest) = raw.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", raw.as_str())
    };
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (
            mantissa,
            exponent
                .parse::<i32>()
                .map_err(|_| Error::not_applied("invalid float exponent"))?,
        ),
        None => (unsigned, 0),
    };
    let decimal = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
    let mut digits = mantissa.replace('.', "");
    let leading = digits.bytes().take_while(|byte| *byte == b'0').count();
    digits.drain(..leading);
    let position = decimal + exponent - leading as i32;
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }
    let scientific_exponent = position - 1;
    if !(-4..16).contains(&scientific_exponent) {
        let tail = if digits.len() > 1 {
            format!(".{}", &digits[1..])
        } else {
            String::new()
        };
        Ok(format!(
            "{sign}{}{tail}e{}{:02}",
            &digits[..1],
            if scientific_exponent < 0 { "-" } else { "+" },
            scientific_exponent.abs()
        ))
    } else if position <= 0 {
        Ok(format!(
            "{sign}0.{}{digits}",
            "0".repeat((-position) as usize)
        ))
    } else if position as usize >= digits.len() {
        Ok(format!(
            "{sign}{digits}{}.0",
            "0".repeat(position as usize - digits.len())
        ))
    } else {
        Ok(format!(
            "{sign}{}.{}",
            &digits[..position as usize],
            &digits[position as usize..]
        ))
    }
}

// The grammar remains serde_json's responsibility. This prepass only detects
// duplicate decoded keys and non-finite numeric values before Value loses them.
struct Scanner<'a> {
    source: &'a [u8],
    cursor: usize,
}

impl Scanner<'_> {
    fn whitespace(&mut self) {
        while self
            .source
            .get(self.cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.cursor += 1;
        }
    }

    fn string(&mut self) -> Result<String> {
        let start = self.cursor;
        self.cursor += 1;
        let mut escaped = false;
        while let Some(byte) = self.source.get(self.cursor) {
            self.cursor += 1;
            if escaped {
                escaped = false;
                continue;
            }
            if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                return serde_json::from_slice(&self.source[start..self.cursor])
                    .map_err(|error| Error::not_applied(format!("invalid JSON string: {error}")));
            }
        }
        Err(Error::not_applied("unterminated JSON string"))
    }

    fn value(&mut self, depth: usize) -> Result<()> {
        if depth > 128 {
            return Err(Error::not_applied("JSON nesting limit exceeded"));
        }
        self.whitespace();
        match self.source.get(self.cursor).copied() {
            Some(b'{') => {
                self.cursor += 1;
                let mut keys = BTreeSet::new();
                loop {
                    self.whitespace();
                    if self.source.get(self.cursor) == Some(&b'}') {
                        self.cursor += 1;
                        break;
                    }
                    if self.source.get(self.cursor) != Some(&b'"') {
                        return Err(Error::not_applied("JSON object key must be a string"));
                    }
                    let key = self.string()?;
                    if !keys.insert(key.clone()) {
                        return Err(Error::not_applied(format!("duplicate JSON key: {key}")));
                    }
                    self.whitespace();
                    if self.source.get(self.cursor) != Some(&b':') {
                        return Err(Error::not_applied("JSON object is missing a colon"));
                    }
                    self.cursor += 1;
                    self.value(depth + 1)?;
                    self.whitespace();
                    match self.source.get(self.cursor) {
                        Some(b',') => self.cursor += 1,
                        Some(b'}') => {
                            self.cursor += 1;
                            break;
                        }
                        _ => return Err(Error::not_applied("unterminated JSON object")),
                    }
                }
            }
            Some(b'[') => {
                self.cursor += 1;
                loop {
                    self.whitespace();
                    if self.source.get(self.cursor) == Some(&b']') {
                        self.cursor += 1;
                        break;
                    }
                    self.value(depth + 1)?;
                    self.whitespace();
                    match self.source.get(self.cursor) {
                        Some(b',') => self.cursor += 1,
                        Some(b']') => {
                            self.cursor += 1;
                            break;
                        }
                        _ => return Err(Error::not_applied("unterminated JSON array")),
                    }
                }
            }
            Some(b'"') => {
                self.string()?;
            }
            Some(_) => {
                let start = self.cursor;
                while self
                    .source
                    .get(self.cursor)
                    .is_some_and(|byte| !byte.is_ascii_whitespace() && !b",]}".contains(byte))
                {
                    self.cursor += 1;
                }
                if start == self.cursor {
                    return Err(Error::not_applied("missing JSON value"));
                }
                let raw = std::str::from_utf8(&self.source[start..self.cursor])
                    .map_err(|_| Error::not_applied("JSON is not UTF-8"))?;
                if matches!(raw, "NaN" | "Infinity" | "-Infinity")
                    || (raw.contains(['.', 'e', 'E'])
                        && raw.parse::<f64>().is_ok_and(|number| !number.is_finite()))
                {
                    return Err(Error::not_applied("non-finite JSON value"));
                }
            }
            None => return Err(Error::not_applied("missing JSON value")),
        }
        Ok(())
    }
}
