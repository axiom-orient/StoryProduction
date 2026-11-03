//! The explicit geultu → NarrativeCourier handoff.
//!
//! This is deliberately not a GeultuStoryPackage.  Geultu owns the reviewed
//! synopsis only; NarrativeCourier owns screenplay and release artifacts.

use super::validate::{valid_language_tag, valid_prefixed_sha256, valid_sha256_hex};
use crate::contracts::digest::jcs_sha256;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

pub const REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_ID: &str = "geultu.dev/reviewed-synopsis-handoff/v1";
pub const REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_VERSION: u32 = 1;
// Character counts mirror the producer JSON Schema, not UTF-8 byte limits.
const MAX_PROJECT_ID_CHARS: usize = 128;
const MAX_TITLE_CHARS: usize = 300;
const MAX_ACTOR_ID_CHARS: usize = 256;
const MAX_REASON_CHARS: usize = 4000;

/// Human approval bound to the exact synopsis bytes and source revision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SynopsisApproval {
    pub actor_id: String,
    pub decision: String,
    pub synopsis_sha256: String,
    pub source_revision: u64,
    pub approved_at_ms: u64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::validate::deserialize_present"
    )]
    pub reason: Option<String>,
}

/// Versioned, raw-byte-bound handoff consumed by NarrativeCourier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeultuReviewedSynopsisHandoff {
    pub schema_id: String,
    pub schema_version: u32,
    pub handoff_id: String,
    pub project_id: String,
    pub source_revision: u64,
    pub synopsis_path: String,
    /// Lowercase raw SHA-256 of the emitted UTF-8 `synopsis.md` bytes.
    pub synopsis_sha256: String,
    pub title: String,
    pub language: String,
    pub approval: SynopsisApproval,
}

impl GeultuReviewedSynopsisHandoff {
    pub fn validate(&self) -> Result<()> {
        if self.schema_id != REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_ID {
            return Err(Error::InvalidInput(format!(
                "schema_id must be {REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_ID}"
            )));
        }
        if self.schema_version != REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_VERSION {
            return Err(Error::InvalidInput(
                "reviewed synopsis handoff schema_version must be 1".into(),
            ));
        }
        if !valid_handoff_id(&self.handoff_id) {
            return Err(Error::InvalidInput(
                "handoff_id must match ^gsh_[0-9a-f]{64}$".into(),
            ));
        }
        bounded_nonempty(&self.project_id, "project_id", MAX_PROJECT_ID_CHARS)?;
        if self.source_revision == 0 {
            return Err(Error::InvalidInput(
                "source_revision must be at least 1".into(),
            ));
        }
        if self.synopsis_path.is_empty()
            || std::path::Path::new(&self.synopsis_path).is_absolute()
            || self.synopsis_path.contains(':')
            || self.synopsis_path.contains('\0')
            || self.synopsis_path.starts_with('\\')
            || self
                .synopsis_path
                .split(['/', '\\'])
                .any(|part| part == "..")
            || std::path::Path::new(&self.synopsis_path)
                .components()
                .any(|part| {
                    matches!(
                        part,
                        std::path::Component::ParentDir | std::path::Component::RootDir
                    )
                })
        {
            return Err(Error::InvalidInput(
                "synopsis_path must be a safe relative path".into(),
            ));
        }
        if !valid_sha256_hex(&self.synopsis_sha256) {
            return Err(Error::InvalidInput(
                "synopsis_sha256 must be lowercase SHA-256 hex".into(),
            ));
        }
        bounded_nonempty(&self.title, "title", MAX_TITLE_CHARS)?;
        if !valid_language_tag(&self.language, 1) {
            return Err(Error::InvalidInput(
                "language must be a BCP-47-like language tag".into(),
            ));
        }
        bounded_nonempty(
            &self.approval.actor_id,
            "approval.actor_id",
            MAX_ACTOR_ID_CHARS,
        )?;
        if self.approval.decision != "APPROVE" {
            return Err(Error::InvalidInput(
                "approval.decision must be APPROVE".into(),
            ));
        }
        if self.approval.synopsis_sha256 != self.synopsis_sha256 {
            return Err(Error::InvalidInput(
                "approval.synopsis_sha256 must match synopsis_sha256".into(),
            ));
        }
        if self.approval.source_revision != self.source_revision {
            return Err(Error::InvalidInput(
                "approval.source_revision must match source_revision".into(),
            ));
        }
        if self.approval.approved_at_ms == 0 {
            return Err(Error::InvalidInput(
                "approval.approved_at_ms must be at least 1".into(),
            ));
        }
        if let Some(reason) = &self.approval.reason {
            if reason.chars().count() > MAX_REASON_CHARS {
                return Err(Error::InvalidInput(
                    "approval.reason must be at most 4000 characters".into(),
                ));
            }
        }
        Ok(())
    }

    /// Computes the producer-owned content identity, excluding the identity field.
    /// Consumers should validate with `validate_identity` instead of rebuilding it.
    pub(crate) fn expected_handoff_id(&self) -> Result<String> {
        let mut value = serde_json::to_value(self)?;
        value
            .as_object_mut()
            .ok_or_else(|| Error::Corrupt("handoff did not serialize as an object".into()))?
            .remove("handoff_id");
        Ok(format!("gsh_{}", jcs_sha256(&value)?))
    }

    pub fn validate_identity(&self) -> Result<()> {
        self.validate()?;
        if self.expected_handoff_id()? != self.handoff_id {
            return Err(Error::InvalidInput(
                "handoff_id does not match the canonical handoff body".into(),
            ));
        }
        Ok(())
    }
}

fn valid_handoff_id(value: &str) -> bool {
    valid_prefixed_sha256(value, "gsh_")
}

fn bounded_nonempty(value: &str, field: &str, max: usize) -> Result<()> {
    if value.trim().is_empty() {
        Err(Error::InvalidInput(format!("{field} must not be empty")))
    } else if value.chars().count() > max {
        Err(Error::InvalidInput(format!(
            "{field} must be at most {max} characters"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handoff() -> GeultuReviewedSynopsisHandoff {
        GeultuReviewedSynopsisHandoff {
            schema_id: REVIEWED_SYNOPSIS_HANDOFF_SCHEMA_ID.into(),
            schema_version: 1,
            handoff_id: format!("gsh_{}", "0".repeat(64)),
            project_id: "book".into(),
            source_revision: 1,
            synopsis_path: "synopsis.md".into(),
            synopsis_sha256: "0".repeat(64),
            title: "책".into(),
            language: "ko".into(),
            approval: SynopsisApproval {
                actor_id: "human".into(),
                decision: "APPROVE".into(),
                synopsis_sha256: "0".repeat(64),
                source_revision: 1,
                approved_at_ms: 1,
                reason: None,
            },
        }
    }

    #[test]
    fn unicode_limits_match_schema_characters() {
        let mut value = handoff();
        value.title = "글".repeat(MAX_TITLE_CHARS);
        value.approval.actor_id = "사".repeat(MAX_ACTOR_ID_CHARS);
        assert!(value.validate().is_ok());
        value.title.push('글');
        assert!(value.validate().is_err());
        value.title = "책".into();
        value.approval.actor_id.push('사');
        assert!(value.validate().is_err());
    }

    #[test]
    fn paths_are_portable_and_cannot_traverse() {
        for path in [
            "../secret",
            "dir/../secret",
            "dir\\..\\secret",
            "\\root",
            "a\0b",
            "/secret",
            "C:secret",
        ] {
            let mut value = handoff();
            value.synopsis_path = path.into();
            assert!(value.validate().is_err(), "{path:?}");
        }
    }
}
