//! Bounded JSON-lines input/output, canonical JSON, protocol hashes, and timestamps.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::error::{CustodyError, Result};

pub(super) const PROTOCOL_VERSION: u64 = 1;
const MAX_REQUEST_LINE_BYTES: usize = 24 * 1024 * 1024;

pub(super) fn encode_base64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(super) fn decode_base64url_bounded(encoded: &str, maximum_bytes: usize) -> Result<Vec<u8>> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|error| CustodyError::input(format!("bytesBase64 is invalid: {error}")))?;
    if bytes.len() > maximum_bytes {
        return Err(CustodyError::input(format!(
            "payload exceeds bounded size {maximum_bytes}"
        )));
    }
    Ok(bytes)
}

pub(super) fn read_bounded_line<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = reader
            .read(&mut byte)
            .map_err(|error| CustodyError::io("protocol read failed", &error))?;
        if read == 0 {
            if line.is_empty() {
                return Ok(None);
            }
            break;
        }
        if byte[0] == b'\n' {
            break;
        }
        line.push(byte[0]);
        if line.len() > MAX_REQUEST_LINE_BYTES {
            return Err(CustodyError::input(format!(
                "request line exceeds {MAX_REQUEST_LINE_BYTES} bytes"
            )));
        }
    }
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    Ok(Some(line))
}

pub(super) fn json_object<const N: usize>(items: [(&str, Value); N]) -> Value {
    let mut object = Map::new();
    for (key, value) in items {
        object.insert(key.to_owned(), value);
    }
    Value::Object(object)
}

pub(super) fn write_response<W: Write>(
    writer: &mut W,
    id: &str,
    result: Result<Value>,
) -> Result<()> {
    let mut response = Map::new();
    response.insert("id".to_owned(), Value::String(id.to_owned()));
    match result {
        Ok(value) => {
            response.insert("ok".to_owned(), Value::Bool(true));
            response.insert("result".to_owned(), value);
        }
        Err(error) => {
            response.insert("ok".to_owned(), Value::Bool(false));
            let mut details = Map::new();
            details.insert("code".to_owned(), Value::String(error.code));
            details.insert("message".to_owned(), Value::String(error.message));
            details.insert("retryable".to_owned(), Value::Bool(error.retryable));
            response.insert("error".to_owned(), Value::Object(details));
        }
    }
    let line = serde_json::to_string(&Value::Object(response)).map_err(|error| {
        CustodyError::storage(format!("protocol response serialization failed: {error}"))
    })?;
    if line.len() + 1 > MAX_REQUEST_LINE_BYTES {
        return Err(CustodyError::storage(
            "protocol response exceeds bounded size",
        ));
    }
    writer
        .write_all(line.as_bytes())
        .and_then(|_| writer.write_all(b"\n"))
        .and_then(|_| writer.flush())
        .map_err(|error| CustodyError::io("protocol response write failed", &error))
}

pub(super) fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => {
            serde_json::to_string(value).expect("string serialization cannot fail")
        }
        Value::Array(values) => {
            let items = values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{items}]")
        }
        Value::Object(values) => {
            let sorted: BTreeMap<&String, &Value> = values.iter().collect();
            let items = sorted
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("key serialization cannot fail"),
                        canonical_json(value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{items}}}")
        }
    }
}

pub(super) fn sha256_prefixed(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

pub(super) fn is_sha256_hash(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn utc_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default() as libc::time_t;
    let mut tm = unsafe { std::mem::zeroed::<libc::tm>() };
    unsafe {
        libc::gmtime_r(&seconds, &mut tm);
    }
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    )
}

pub(super) fn is_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
}
