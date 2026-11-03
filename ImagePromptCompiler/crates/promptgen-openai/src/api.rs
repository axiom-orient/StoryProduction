//! Stable public configuration, result, and error contracts for the OpenAI adapter.

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use promptgen_core::json::JsonValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenAiRunConfig {
    pub curl_binary: PathBuf,
    pub output_path: PathBuf,
    pub timeout: Duration,
    pub overwrite: bool,
}

impl OpenAiRunConfig {
    pub fn new(curl_binary: impl Into<PathBuf>, output_path: impl Into<PathBuf>) -> Self {
        Self {
            curl_binary: curl_binary.into(),
            output_path: output_path.into(),
            timeout: Duration::from_secs(240),
            overwrite: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenAiEditReceipt {
    pub backend: String,
    pub provider: String,
    pub model: String,
    pub endpoint_contract: String,
    pub request_id: Option<String>,
    pub http_status: u16,
    pub compiled_prompt_sha256: String,
    pub compiled_prompt_chars: u64,
    pub reference_index: u8,
    pub reference_role: String,
    pub reference_sha256: String,
    pub reference_bytes: u64,
    pub output_sha256: String,
    pub output_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub started_unix_ms: u64,
    pub finished_unix_ms: u64,
}

impl OpenAiEditReceipt {
    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("backend", JsonValue::from(self.backend.clone())),
            (
                "compiled_prompt_chars",
                JsonValue::from(self.compiled_prompt_chars),
            ),
            (
                "compiled_prompt_sha256",
                JsonValue::from(self.compiled_prompt_sha256.clone()),
            ),
            (
                "endpoint_contract",
                JsonValue::from(self.endpoint_contract.clone()),
            ),
            ("finished_unix_ms", JsonValue::from(self.finished_unix_ms)),
            ("height", JsonValue::from(u64::from(self.height))),
            ("http_status", JsonValue::from(u64::from(self.http_status))),
            ("model", JsonValue::from(self.model.clone())),
            (
                "output_path",
                JsonValue::from(self.output_path.display().to_string()),
            ),
            ("output_sha256", JsonValue::from(self.output_sha256.clone())),
            ("provider", JsonValue::from(self.provider.clone())),
            ("reference_bytes", JsonValue::from(self.reference_bytes)),
            (
                "reference_index",
                JsonValue::from(u64::from(self.reference_index)),
            ),
            (
                "reference_role",
                JsonValue::from(self.reference_role.clone()),
            ),
            (
                "reference_sha256",
                JsonValue::from(self.reference_sha256.clone()),
            ),
            (
                "request_id",
                self.request_id
                    .as_ref()
                    .map_or(JsonValue::Null, |value| JsonValue::from(value.clone())),
            ),
            ("started_unix_ms", JsonValue::from(self.started_unix_ms)),
            ("width", JsonValue::from(u64::from(self.width))),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenAiExecutionError {
    pub code: &'static str,
    pub message: String,
}

impl OpenAiExecutionError {
    pub(super) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for OpenAiExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for OpenAiExecutionError {}
