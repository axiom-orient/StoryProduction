//! Serde contract type for `domain-decision.schema.json` (H5).
//!
//! The field set mirrors the canonical schema 1:1;
//! `#[serde(deny_unknown_fields)]` enforces `additionalProperties: false`,
//! and pattern/enum constraints serde cannot express are enforced by
//! [`DomainDecision::validate`], which fails closed with field-naming
//! [`Error::InvalidInput`] messages.

use super::receipts::valid_receipt_id;
use super::validate::{check_length, valid_finding_code, valid_prefixed_sha256};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// `schema_id`-style identity prefix of a domain decision id.
pub const DOMAIN_DECISION_ID_PREFIX: &str = "decision_";

/// `decision` enum of `domain-decision.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DomainDecisionKind {
    Accept,
    Reject,
    Repair,
    HumanRequired,
    Fail,
}

/// DomainDecision v1 (`domain-decision.schema.json`).
///
/// This is the typed form of an engine-proposed (or recorded) routing
/// decision over one command. H5 uses it to publish refinement patch
/// proposals (`REPAIR`) whose application still requires a bound
/// [`super::HumanDecision`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainDecision {
    pub decision_id: String,
    pub command_id: String,
    pub state_revision: u64,
    pub decision: DomainDecisionKind,
    pub selected_receipts: Vec<String>,
    pub reason_codes: Vec<String>,
}

impl DomainDecision {
    /// Validates every schema constraint: `decision_id`
    /// (`^decision_[0-9a-f]{64}$`), `command_id` (1..=256 characters),
    /// `state_revision >= 0`, receipt ids (`^vr_[0-9a-f]{64}$`, at most 256,
    /// unique), and reason codes (`^[A-Z][A-Z0-9_]{2,127}$`, 1..=256 items,
    /// unique).
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated constraint.
    pub fn validate(&self) -> Result<()> {
        if !valid_domain_decision_id(self.decision_id.as_str()) {
            return Err(Error::InvalidInput(format!(
                "decision_id '{}' must match ^decision_[0-9a-f]{{64}}$",
                self.decision_id
            )));
        }
        check_length("command_id", &self.command_id, 1, 256)?;
        if self.selected_receipts.len() > 256 {
            return Err(Error::InvalidInput(
                "selected_receipts must contain at most 256 items".into(),
            ));
        }
        let mut receipts = BTreeSet::new();
        for receipt in &self.selected_receipts {
            if !valid_receipt_id(receipt) {
                return Err(Error::InvalidInput(format!(
                    "selected_receipts entry '{receipt}' must match ^vr_[0-9a-f]{{64}}$"
                )));
            }
            if !receipts.insert(receipt.as_str()) {
                return Err(Error::InvalidInput(format!(
                    "selected_receipts must be unique; '{receipt}' appears more than once"
                )));
            }
        }
        if self.reason_codes.is_empty() || self.reason_codes.len() > 256 {
            return Err(Error::InvalidInput(
                "reason_codes must contain between 1 and 256 items".into(),
            ));
        }
        let mut codes = BTreeSet::new();
        for code in &self.reason_codes {
            if !valid_finding_code(code) {
                return Err(Error::InvalidInput(format!(
                    "reason_codes entry '{code}' must match ^[A-Z][A-Z0-9_]{{2,127}}$"
                )));
            }
            if !codes.insert(code.as_str()) {
                return Err(Error::InvalidInput(format!(
                    "reason_codes must be unique; '{code}' appears more than once"
                )));
            }
        }
        Ok(())
    }
}

/// `^decision_[0-9a-f]{64}$`
pub(crate) fn valid_domain_decision_id(value: &str) -> bool {
    valid_prefixed_sha256(value, DOMAIN_DECISION_ID_PREFIX)
}
