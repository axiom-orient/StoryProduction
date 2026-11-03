//! Producer-owned EffectReceipt v1, including failed and unknown external outcomes.

use super::validate::{
    check_length, valid_finding_code, valid_prefixed_sha256, valid_sha256_digest,
};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EffectStatus {
    Committed,
    Failed,
    UnknownOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectReceipt {
    pub receipt_id: String,
    pub effect_id: String,
    pub idempotency_key: String,
    pub status: EffectStatus,
    pub request_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
}

impl EffectReceipt {
    /// Validates the existing schema shape; status alone never proves adoption or success.
    pub fn validate(&self) -> Result<()> {
        if !valid_prefixed_sha256(&self.receipt_id, "effect_") {
            return Err(Error::InvalidInput(
                "effect receipt_id must match ^effect_[0-9a-f]{64}$".into(),
            ));
        }
        check_length("effect_id", &self.effect_id, 1, 256)?;
        if !valid_prefixed_sha256(&self.idempotency_key, "idem_") {
            return Err(Error::InvalidInput(
                "effect idempotency_key must match ^idem_[0-9a-f]{64}$".into(),
            ));
        }
        if !valid_sha256_digest(&self.request_digest) {
            return Err(Error::InvalidInput(
                "effect request_digest must be lowercase sha256:<hex>".into(),
            ));
        }
        if let Some(digest) = &self.response_digest {
            if !valid_sha256_digest(digest) {
                return Err(Error::InvalidInput(
                    "effect response_digest must be lowercase sha256:<hex>".into(),
                ));
            }
        }
        if let Some(reference) = &self.external_reference {
            check_length("effect external_reference", reference, 0, 2000)?;
        }
        if let Some(code) = &self.failure_code {
            if !valid_finding_code(code) {
                return Err(Error::InvalidInput(
                    "effect failure_code must match ^[A-Z][A-Z0-9_]{2,127}$".into(),
                ));
            }
        }
        Ok(())
    }
}
