//! Serde contract types for `route-admission.schema.json` and
//! `receipt-selector.schema.json` (H6).
//!
//! Field sets mirror the canonical schemas 1:1;
//! `#[serde(deny_unknown_fields)]` enforces `additionalProperties: false`,
//! and pattern/enum constraints serde cannot express are enforced by the
//! `validate` methods, which fail closed with field-naming
//! [`Error::InvalidInput`] messages.

use super::validate::{check_length, valid_lowercase_token, valid_numeric_version};
use crate::contracts::ArtifactRef;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `route_id` enum of `route-admission.schema.json`.
// UPPERCASE would drop the underscore in TEXT_TO_NVC and desync from the schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RouteId {
    Text,
    TextToNvc,
    Nvc,
}

impl RouteId {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::TextToNvc => "TEXT_TO_NVC",
            Self::Nvc => "NVC",
        }
    }
}

/// `budget` object of `route-admission.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionBudget {
    pub time_ms: u64,
    pub output_bytes: u64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::validate::deserialize_present"
    )]
    pub provider_attempts: Option<u64>,
}

impl AdmissionBudget {
    /// Validates `time_ms >= 1`, `output_bytes >= 1`, and
    /// `provider_attempts >= 0` when present.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the violated field.
    pub fn validate(&self) -> Result<()> {
        if self.time_ms < 1 {
            return Err(Error::InvalidInput(
                "budget time_ms must be at least 1".into(),
            ));
        }
        if self.output_bytes < 1 {
            return Err(Error::InvalidInput(
                "budget output_bytes must be at least 1".into(),
            ));
        }
        Ok(())
    }
}

/// ReceiptSelector v1 (`receipt-selector.schema.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptSelector {
    pub validator_id: String,
    pub subject_name: String,
    pub required_outcome: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::validate::deserialize_present"
    )]
    pub min_version: Option<String>,
}

impl ReceiptSelector {
    /// Validates identity patterns and the `required_outcome: "PASS"` const.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated field.
    pub fn validate(&self) -> Result<()> {
        if !valid_lowercase_token(self.validator_id.as_str(), 128) {
            return Err(Error::InvalidInput(format!(
                "validator_id '{}' must match ^[a-z][a-z0-9_.-]{{0,127}}$",
                self.validator_id
            )));
        }
        if !valid_lowercase_token(self.subject_name.as_str(), 64) {
            return Err(Error::InvalidInput(format!(
                "subject_name '{}' must match ^[a-z][a-z0-9_.-]{{0,63}}$",
                self.subject_name
            )));
        }
        if self.required_outcome != "PASS" {
            return Err(Error::InvalidInput(
                "required_outcome must be \"PASS\"".into(),
            ));
        }
        if let Some(min_version) = &self.min_version {
            if !valid_numeric_version(min_version) {
                return Err(Error::InvalidInput(format!(
                    "min_version '{min_version}' must match ^[0-9]+\\.[0-9]+\\.[0-9]+$"
                )));
            }
        }
        Ok(())
    }
}

/// RouteAdmission v1 (`route-admission.schema.json`).
///
/// The typed contract for admitted routes; document-level admission flows
/// consume this type from H4/H7 onward. M2 ships the type and its fail-closed
/// validation alongside the budget gate below.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteAdmission {
    pub route_id: RouteId,
    pub command_id: String,
    pub input_refs: BTreeMap<String, ArtifactRef>,
    pub required_receipts: Vec<ReceiptSelector>,
    pub capabilities: Vec<String>,
    pub budget: AdmissionBudget,
}

impl RouteAdmission {
    /// Validates every schema constraint: ref-map bounds and key patterns,
    /// selector count/patterns, capability uniqueness/patterns, and budget.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated constraint.
    pub fn validate(&self) -> Result<()> {
        check_length("command_id", &self.command_id, 1, 256)?;
        if self.input_refs.is_empty() || self.input_refs.len() > 64 {
            return Err(Error::InvalidInput(
                "input_refs must contain between 1 and 64 entries".into(),
            ));
        }
        for (name, reference) in &self.input_refs {
            if !valid_lowercase_token(name.as_str(), 64) {
                return Err(Error::InvalidInput(format!(
                    "input_refs key '{name}' must match ^[a-z][a-z0-9_.-]{{0,63}}$"
                )));
            }
            reference.validate()?;
        }
        if self.required_receipts.len() > 256 {
            return Err(Error::InvalidInput(
                "required_receipts must contain at most 256 items".into(),
            ));
        }
        for selector in &self.required_receipts {
            selector.validate()?;
        }
        if self.capabilities.len() > 128 {
            return Err(Error::InvalidInput(
                "capabilities must contain at most 128 items".into(),
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for capability in &self.capabilities {
            let bytes = capability.as_bytes();
            let pattern_ok = (1..=128).contains(&bytes.len())
                && bytes.first().is_some_and(|byte| byte.is_ascii_lowercase())
                && bytes.iter().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'.' | b':' | b'-')
                });
            if !pattern_ok {
                return Err(Error::InvalidInput(format!(
                    "capability '{capability}' must match ^[a-z][a-z0-9_.:-]{{0,127}}$"
                )));
            }
            if !seen.insert(capability.as_str()) {
                return Err(Error::InvalidInput(format!(
                    "capabilities must be unique; '{capability}' appears more than once"
                )));
            }
        }
        self.budget.validate()
    }
}
