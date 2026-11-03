//! ValidationReceipt serde contract.
//!
//! Receipt field sets mirror the canonical schema 1:1. HumanDecision and
//! DecisionKind are owned and exported by the sibling human_decision module.
//! `#[serde(deny_unknown_fields)]` enforces `additionalProperties: false`,
//! and pattern/length constraints serde cannot express are enforced by the
//! `validate` methods, which fail closed with field-naming
//! [`Error::InvalidInput`] messages.

use super::validate::{
    check_length, valid_lowercase_token, valid_numeric_version, valid_prefixed_sha256,
    valid_sha256_digest,
};
use crate::contracts::ArtifactRef;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Upper bound on receipt findings, mirroring `maxItems: 10000`.
pub const RECEIPT_FINDINGS_LIMIT: usize = 10_000;

/// `outcome` enum of `validation-receipt.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReceiptOutcome {
    Pass,
    Fail,
}

/// Finding `severity` enum of `validation-receipt.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReceiptSeverity {
    Error,
    Warning,
    Info,
}

/// `validator` object of `validation-receipt.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptValidator {
    pub id: String,
    pub version: String,
}

impl ReceiptValidator {
    /// Validates `id` (`^[a-z][a-z0-9_.-]{0,127}$`) and `version`
    /// (`^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.-]+)?$`).
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated field.
    pub fn validate(&self) -> Result<()> {
        if !valid_lowercase_token(self.id.as_str(), 128) {
            return Err(Error::InvalidInput(format!(
                "validator id '{}' must match ^[a-z][a-z0-9_.-]{{0,127}}$",
                self.id
            )));
        }
        if !valid_validator_version(self.version.as_str()) {
            return Err(Error::InvalidInput(format!(
                "validator version '{}' must match ^[0-9]+\\.[0-9]+\\.[0-9]+(?:[-+][A-Za-z0-9.-]+)?$",
                self.version
            )));
        }
        Ok(())
    }
}

/// One finding of a validation receipt (`validation-receipt.schema.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptFinding {
    pub code: String,
    pub severity: ReceiptSeverity,
    pub message: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::validate::deserialize_present"
    )]
    pub path: Option<String>,
}

impl ReceiptFinding {
    /// Validates `code` (`^[A-Z][A-Z0-9_]{2,127}$`), `message`
    /// (1..=4000 characters), and optional `path` (at most 2000 characters).
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated field.
    pub fn validate(&self) -> Result<()> {
        if !super::validate::valid_finding_code(&self.code) {
            return Err(Error::InvalidInput(format!(
                "finding code '{}' must match ^[A-Z][A-Z0-9_]{{2,127}}$",
                self.code
            )));
        }
        check_length("finding message", &self.message, 1, 4000)?;
        if let Some(path) = &self.path {
            check_length("finding path", path, 0, 2000)?;
        }
        Ok(())
    }
}

/// ValidationReceipt v1 (`validation-receipt.schema.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReceipt {
    pub receipt_id: String,
    pub validator: ReceiptValidator,
    pub subject: ArtifactRef,
    pub binding_digest: String,
    pub outcome: ReceiptOutcome,
    pub findings: Vec<ReceiptFinding>,
}

impl ValidationReceipt {
    /// Validates every schema constraint: `receipt_id`
    /// (`^vr_[0-9a-f]{64}$`), validator identity, subject ref, binding
    /// digest (`^sha256:[0-9a-f]{64}$`), and finding count/content.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated constraint.
    pub fn validate(&self) -> Result<()> {
        if !valid_receipt_id(self.receipt_id.as_str()) {
            return Err(Error::InvalidInput(format!(
                "receipt_id '{}' must match ^vr_[0-9a-f]{{64}}$",
                self.receipt_id
            )));
        }
        self.validator.validate()?;
        self.subject.validate()?;
        if !valid_sha256_digest(self.binding_digest.as_str()) {
            return Err(Error::InvalidInput(format!(
                "binding_digest '{}' must match ^sha256:[0-9a-f]{{64}}$",
                self.binding_digest
            )));
        }
        if self.findings.len() > RECEIPT_FINDINGS_LIMIT {
            return Err(Error::InvalidInput(format!(
                "findings must contain at most {RECEIPT_FINDINGS_LIMIT} items"
            )));
        }
        for finding in &self.findings {
            finding.validate()?;
        }
        Ok(())
    }
}

/// `^vr_[0-9a-f]{64}$`
pub(crate) fn valid_receipt_id(value: &str) -> bool {
    valid_prefixed_sha256(value, "vr_")
}

/// `^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.-]+)?$`
fn valid_validator_version(value: &str) -> bool {
    let (core, suffix) = match value.split_once(['-', '+']) {
        Some((core, suffix)) => (core, Some(suffix)),
        None => (value, None),
    };
    if !valid_numeric_version(core) {
        return false;
    }
    match suffix {
        None => true,
        Some(suffix) => {
            !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        }
    }
}
