use std::collections::BTreeMap;
use std::fmt;

use super::JsonValue;

const DEFAULT_MAX_BYTES: usize = 16 * 1024 * 1024;
const DEFAULT_MAX_DEPTH: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub message: String,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at line {}, column {} (byte {})",
            self.message, self.line, self.column, self.offset
        )
    }
}

impl std::error::Error for JsonError {}

pub fn parse(input: &str) -> Result<JsonValue, JsonError> {
    parse_with_limits(input, DEFAULT_MAX_BYTES, DEFAULT_MAX_DEPTH)
}

pub fn parse_with_limits(
    input: &str,
    max_bytes: usize,
    max_depth: usize,
) -> Result<JsonValue, JsonError> {
    if input.len() > max_bytes {
        return Err(JsonError {
            message: format!("JSON exceeds maximum size of {max_bytes} bytes"),
            offset: 0,
            line: 1,
            column: 1,
        });
    }
    let mut parser = Parser {
        input,
        position: 0,
        max_depth,
    };
    parser.skip_whitespace();
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if parser.position != input.len() {
        return Err(parser.error("trailing content after JSON value"));
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a str,
    position: usize,
    max_depth: usize,
}

impl Parser<'_> {
    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        if depth > self.max_depth {
            return Err(self.error("JSON nesting depth exceeded"));
        }
        self.skip_whitespace();
        match self.peek_byte() {
            Some(b'n') => self.parse_literal("null", JsonValue::Null),
            Some(b't') => self.parse_literal("true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(depth + 1),
            Some(b'{') => self.parse_object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            Some(_) => Err(self.error("unexpected character while parsing JSON value")),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn parse_literal(&mut self, literal: &str, value: JsonValue) -> Result<JsonValue, JsonError> {
        if self.input[self.position..].starts_with(literal) {
            self.position += literal.len();
            Ok(value)
        } else {
            Err(self.error(&format!("expected {literal}")))
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.expect_byte(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.consume_byte(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value(depth)?);
            self.skip_whitespace();
            if self.consume_byte(b']') {
                break;
            }
            self.expect_byte(b',')?;
            self.skip_whitespace();
            if self.peek_byte() == Some(b']') {
                return Err(self.error("trailing comma in array"));
            }
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.expect_byte(b'{')?;
        self.skip_whitespace();
        let mut fields = BTreeMap::new();
        if self.consume_byte(b'}') {
            return Ok(JsonValue::Object(fields));
        }
        loop {
            if self.peek_byte() != Some(b'"') {
                return Err(self.error("object key must be a JSON string"));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            self.skip_whitespace();
            let value = self.parse_value(depth)?;
            if fields.insert(key.clone(), value).is_some() {
                return Err(self.error(&format!("duplicate object key {key:?}")));
            }
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                break;
            }
            self.expect_byte(b',')?;
            self.skip_whitespace();
            if self.peek_byte() == Some(b'}') {
                return Err(self.error("trailing comma in object"));
            }
        }
        Ok(JsonValue::Object(fields))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect_byte(b'"')?;
        let mut output = String::new();
        loop {
            let Some(byte) = self.peek_byte() else {
                return Err(self.error("unterminated JSON string"));
            };
            match byte {
                b'"' => {
                    self.position += 1;
                    return Ok(output);
                }
                b'\\' => {
                    self.position += 1;
                    let escaped = self
                        .next_byte()
                        .ok_or_else(|| self.error("unterminated escape sequence"))?;
                    match escaped {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'b' => output.push('\u{0008}'),
                        b'f' => output.push('\u{000C}'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'u' => {
                            let first = self.parse_hex_quad()?;
                            let scalar = if (0xD800..=0xDBFF).contains(&first) {
                                if self.next_byte() != Some(b'\\') || self.next_byte() != Some(b'u')
                                {
                                    return Err(self.error(
                                        "high surrogate must be followed by a low surrogate",
                                    ));
                                }
                                let second = self.parse_hex_quad()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    return Err(self.error("invalid low surrogate"));
                                }
                                0x10000
                                    + (((first as u32 - 0xD800) << 10) | (second as u32 - 0xDC00))
                            } else if (0xDC00..=0xDFFF).contains(&first) {
                                return Err(self.error("unexpected low surrogate"));
                            } else {
                                first as u32
                            };
                            let character = char::from_u32(scalar)
                                .ok_or_else(|| self.error("invalid Unicode scalar value"))?;
                            output.push(character);
                        }
                        _ => return Err(self.error("invalid JSON escape sequence")),
                    }
                }
                0x00..=0x1F => {
                    return Err(self.error("unescaped control character in JSON string"));
                }
                0x20..=0x7F => {
                    self.position += 1;
                    output.push(byte as char);
                }
                _ => {
                    let character = self.input[self.position..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.error("invalid UTF-8 in JSON string"))?;
                    self.position += character.len_utf8();
                    output.push(character);
                }
            }
        }
    }

    fn parse_hex_quad(&mut self) -> Result<u16, JsonError> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let byte = self
                .next_byte()
                .ok_or_else(|| self.error("truncated Unicode escape"))?;
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(self.error("non-hexadecimal digit in Unicode escape")),
            };
            value = (value << 4) | u16::from(digit);
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<String, JsonError> {
        let start = self.position;
        self.consume_byte(b'-');
        match self.peek_byte() {
            Some(b'0') => {
                self.position += 1;
                if matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    return Err(self.error("leading zero in JSON number"));
                }
            }
            Some(b'1'..=b'9') => {
                self.position += 1;
                while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => return Err(self.error("invalid JSON number")),
        }
        if self.consume_byte(b'.') {
            if !matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(self.error("fraction must contain at least one digit"));
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(self.error("exponent must contain at least one digit"));
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.position += 1;
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), JsonError> {
        if self.consume_byte(expected) {
            Ok(())
        } else {
            Err(self.error(&format!("expected {:?}", expected as char)))
        }
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let value = self.peek_byte()?;
        self.position += 1;
        Some(value)
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).copied()
    }

    fn error(&self, message: &str) -> JsonError {
        let prefix = &self.input[..self.position.min(self.input.len())];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit_once('\n')
            .map_or(prefix.chars().count() + 1, |(_, tail)| {
                tail.chars().count() + 1
            });
        JsonError {
            message: message.to_owned(),
            offset: self.position,
            line,
            column,
        }
    }
}
