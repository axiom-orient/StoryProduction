use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use promptgen_core::json::JsonValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexRunConfig {
    pub codex_binary: PathBuf,
    pub codex_home: PathBuf,
    pub output_path: PathBuf,
    pub timeout: Duration,
    pub artifact_wait_timeout: Duration,
    /// Maximum generated candidates, including the first attempt.
    pub max_fidelity_attempts: u8,
    pub overwrite: bool,
}

impl CodexRunConfig {
    pub fn new(
        codex_binary: impl Into<PathBuf>,
        codex_home: impl Into<PathBuf>,
        output_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            codex_binary: codex_binary.into(),
            codex_home: codex_home.into(),
            output_path: output_path.into(),
            timeout: Duration::from_secs(240),
            artifact_wait_timeout: Duration::from_secs(30),
            max_fidelity_attempts: 4,
            overwrite: false,
        }
    }

    pub(crate) fn generated_root(&self) -> PathBuf {
        self.codex_home.join("generated_images")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexExecutionReceipt {
    pub backend: String,
    pub codex_version: String,
    pub event_contract: String,
    pub thread_id: String,
    pub image_call_id: String,
    pub source_artifact: PathBuf,
    pub output_artifact: PathBuf,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub dimensions_normalized: bool,
    pub started_unix_ms: u64,
    pub finished_unix_ms: u64,
    pub process_exit_code: i32,
    pub compiled_prompt_sha256: String,
    pub compiled_prompt_chars: u64,
    pub fidelity_checks: Vec<FidelityCheck>,
    pub control_notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectCountCheck {
    pub id: String,
    pub expected: u16,
    pub observed: u16,
    pub evidence: String,
    pub instances: Vec<VisibleInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleInstance {
    pub x_percent: u8,
    pub y_percent: u8,
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextFidelityCheck {
    pub id: String,
    pub exact: bool,
    pub observed_text: String,
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FidelityCheck {
    pub candidate_sha256: String,
    pub pass: bool,
    pub summary: String,
    pub violations: Vec<String>,
    pub repair_instruction: String,
    pub subject_counts: Vec<SubjectCountCheck>,
    pub text_checks: Vec<TextFidelityCheck>,
}

impl FidelityCheck {
    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "candidate_sha256",
                JsonValue::from(self.candidate_sha256.clone()),
            ),
            ("pass", JsonValue::from(self.pass)),
            (
                "repair_instruction",
                JsonValue::from(self.repair_instruction.clone()),
            ),
            (
                "subject_counts",
                JsonValue::array(self.subject_counts.iter().map(|check| {
                    JsonValue::object([
                        ("evidence", JsonValue::from(check.evidence.clone())),
                        ("expected", JsonValue::from(u64::from(check.expected))),
                        ("id", JsonValue::from(check.id.clone())),
                        (
                            "instances",
                            JsonValue::array(check.instances.iter().map(|instance| {
                                JsonValue::object([
                                    ("evidence", JsonValue::from(instance.evidence.clone())),
                                    ("x_percent", JsonValue::from(u64::from(instance.x_percent))),
                                    ("y_percent", JsonValue::from(u64::from(instance.y_percent))),
                                ])
                            })),
                        ),
                        ("observed", JsonValue::from(u64::from(check.observed))),
                    ])
                })),
            ),
            ("summary", JsonValue::from(self.summary.clone())),
            (
                "text_checks",
                JsonValue::array(self.text_checks.iter().map(|check| {
                    JsonValue::object([
                        ("evidence", JsonValue::from(check.evidence.clone())),
                        ("exact", JsonValue::from(check.exact)),
                        ("id", JsonValue::from(check.id.clone())),
                        (
                            "observed_text",
                            JsonValue::from(check.observed_text.clone()),
                        ),
                    ])
                })),
            ),
            ("violations", JsonValue::strings(&self.violations)),
        ])
    }
}

impl CodexExecutionReceipt {
    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("backend", JsonValue::from(self.backend.clone())),
            ("codex_version", JsonValue::from(self.codex_version.clone())),
            (
                "compiled_prompt_chars",
                JsonValue::from(self.compiled_prompt_chars),
            ),
            (
                "compiled_prompt_sha256",
                JsonValue::from(self.compiled_prompt_sha256.clone()),
            ),
            ("control_notes", JsonValue::strings(&self.control_notes)),
            (
                "event_contract",
                JsonValue::from(self.event_contract.clone()),
            ),
            (
                "dimensions_normalized",
                JsonValue::from(self.dimensions_normalized),
            ),
            (
                "fidelity_checks",
                JsonValue::array(self.fidelity_checks.iter().map(FidelityCheck::to_json)),
            ),
            ("finished_unix_ms", JsonValue::from(self.finished_unix_ms)),
            ("height", JsonValue::from(u64::from(self.height))),
            ("image_call_id", JsonValue::from(self.image_call_id.clone())),
            (
                "output_artifact",
                JsonValue::from(self.output_artifact.display().to_string()),
            ),
            (
                "process_exit_code",
                JsonValue::from(i64::from(self.process_exit_code)),
            ),
            ("sha256", JsonValue::from(self.sha256.clone())),
            (
                "source_artifact",
                JsonValue::from(self.source_artifact.display().to_string()),
            ),
            (
                "source_height",
                JsonValue::from(u64::from(self.source_height)),
            ),
            (
                "source_width",
                JsonValue::from(u64::from(self.source_width)),
            ),
            ("started_unix_ms", JsonValue::from(self.started_unix_ms)),
            ("thread_id", JsonValue::from(self.thread_id.clone())),
            ("width", JsonValue::from(u64::from(self.width))),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexExecutionError {
    pub code: &'static str,
    pub message: String,
}

impl CodexExecutionError {
    pub(super) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for CodexExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CodexExecutionError {}
