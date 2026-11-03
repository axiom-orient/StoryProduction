use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::atomic::Ordering;

use base64::Engine as _;
use promptgen_core::image_artifact::png;
use promptgen_core::json::{JsonValue, parse_with_limits};

use super::OpenAiExecutionError;
use super::files::NEXT_NONCE;
use super::files::{preflight_output, read_bounded_utf8, write_private_file};
use super::limits::{MAX_LOG_BYTES, MAX_RESPONSE_BYTES};
use super::support::io_error;
use promptgen_core::image_artifact::sha256::digest_hex;

const MAX_DECODED_IMAGE_BYTES: usize = 50 * 1024 * 1024;
const MAX_REQUEST_ID_CHARS: usize = 256;

pub(super) fn decode_response(
    response: &str,
    expected_dimensions: (u32, u32),
) -> Result<Vec<u8>, OpenAiExecutionError> {
    let value = parse_with_limits(response, MAX_RESPONSE_BYTES, 64).map_err(|_| {
        OpenAiExecutionError::new("OPENAI_RESPONSE_JSON", "OpenAI response is not valid JSON")
    })?;
    let root = value.as_object().ok_or_else(|| {
        OpenAiExecutionError::new("OPENAI_RESPONSE_JSON", "OpenAI response must be an object")
    })?;
    let data = root
        .get("data")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| {
            OpenAiExecutionError::new("OPENAI_RESPONSE_DATA", "OpenAI response data is missing")
        })?;
    if data.len() != 1 {
        return Err(OpenAiExecutionError::new(
            "OPENAI_RESPONSE_DATA",
            "OpenAI response must contain exactly one image",
        ));
    }
    let item = data[0].as_object().ok_or_else(|| {
        OpenAiExecutionError::new("OPENAI_RESPONSE_DATA", "OpenAI response image is invalid")
    })?;
    let encoded = item
        .get("b64_json")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            OpenAiExecutionError::new(
                "OPENAI_RESPONSE_BASE64",
                "OpenAI response b64_json is missing",
            )
        })?;
    let max_encoded = (MAX_DECODED_IMAGE_BYTES.div_ceil(3) * 4) + 4;
    if encoded.is_empty()
        || encoded.len() > max_encoded
        || !encoded.len().is_multiple_of(4)
        || !encoded
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
    {
        return Err(OpenAiExecutionError::new(
            "OPENAI_RESPONSE_BASE64",
            "OpenAI response b64_json is not bounded canonical base64",
        ));
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| {
            OpenAiExecutionError::new(
                "OPENAI_RESPONSE_BASE64",
                "OpenAI response b64_json could not be decoded",
            )
        })?;
    if decoded.len() > MAX_DECODED_IMAGE_BYTES {
        return Err(OpenAiExecutionError::new(
            "OPENAI_RESPONSE_SIZE",
            "decoded image exceeds the bounded output size",
        ));
    }
    png::validate_bytes(&decoded, expected_dimensions).map_err(|error| {
        OpenAiExecutionError::new(
            "OPENAI_INVALID_PNG",
            format!("decoded OpenAI image failed strict PNG validation: {error}"),
        )
    })?;
    Ok(decoded)
}

pub(super) fn publish_png(
    bytes: &[u8],
    destination: &Path,
    overwrite: bool,
    expected_dimensions: (u32, u32),
) -> Result<(String, png::PngInfo), OpenAiExecutionError> {
    preflight_output(destination, overwrite)?;
    let parent = destination
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| io_error("create output directory", parent, error))?;
    let name = destination
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("image.png");
    let temporary = parent.join(format!(
        ".{name}.promptgen-openai-partial-{}-{}",
        std::process::id(),
        NEXT_NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    let operation = (|| {
        write_private_file(&temporary, bytes)?;
        let info = png::validate_file(&temporary, expected_dimensions).map_err(|error| {
            OpenAiExecutionError::new(
                "OPENAI_INVALID_PNG",
                format!("published candidate failed strict PNG validation: {error}"),
            )
        })?;
        preflight_output(destination, overwrite)?;
        if overwrite {
            fs::rename(&temporary, destination)
                .map_err(|error| io_error("publish replacement output", destination, error))?;
        } else {
            fs::hard_link(&temporary, destination)
                .map_err(|error| io_error("publish new output", destination, error))?;
            // The destination link is the publication point. Cleanup of the
            // private temporary name is best effort so a cleanup-only failure
            // cannot report an error after a valid output has been published.
            let _ = fs::remove_file(&temporary);
        }
        Ok((digest_hex(bytes), info))
    })();
    if operation.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    operation
}

pub(super) fn read_request_id(path: &Path) -> Result<Option<String>, OpenAiExecutionError> {
    let headers = read_bounded_utf8(path, MAX_LOG_BYTES, "response headers")?;
    Ok(headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("x-request-id") {
            return None;
        }
        let value = value.trim();
        if value.is_empty()
            || value.chars().count() > MAX_REQUEST_ID_CHARS
            || value.chars().any(char::is_control)
        {
            return None;
        }
        Some(value.to_owned())
    }))
}
