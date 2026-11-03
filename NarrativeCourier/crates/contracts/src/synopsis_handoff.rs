//! Human-approved geultu synopsis handoff consumed by NVC.

use narrative_courier_runtime::paths::validate_relative_path;
use narrative_courier_runtime::{Error, Result};
use serde::{Deserialize, Serialize};

use super::common::{nonempty, sha, valid_prefixed_sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewedSynopsisApproval {
    pub actor_id: String,
    pub decision: String,
    pub synopsis_sha256: String,
    pub source_revision: u64,
    pub approved_at_ms: u64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::common::deserialize_present"
    )]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewedSynopsisHandoff {
    pub schema_id: String,
    pub schema_version: u32,
    pub handoff_id: String,
    pub project_id: String,
    pub source_revision: u64,
    pub synopsis_path: String,
    pub synopsis_sha256: String,
    pub title: String,
    pub language: String,
    pub approval: ReviewedSynopsisApproval,
}

pub fn validate_reviewed_synopsis_handoff(v: &ReviewedSynopsisHandoff) -> Result<()> {
    if v.schema_id != "geultu.dev/reviewed-synopsis-handoff/v1" {
        return Err(Error::Contract(
            "reviewed synopsis handoff schema_id is unsupported".into(),
        ));
    }
    if v.schema_version != 1 {
        return Err(Error::Contract(
            "reviewed synopsis handoff schema_version must be 1".into(),
        ));
    }
    if !valid_prefixed_sha256(&v.handoff_id, "gsh_") {
        return Err(Error::Contract(
            "reviewed synopsis handoff handoff_id is invalid".into(),
        ));
    }
    nonempty(&v.project_id, "project_id")?;
    if v.source_revision == 0 {
        return Err(Error::Contract(
            "reviewed synopsis handoff source_revision must be at least 1".into(),
        ));
    }
    validate_relative_path(&v.synopsis_path)?;
    if !v.synopsis_path.ends_with(".md") {
        return Err(Error::Contract(
            "reviewed synopsis handoff synopsis_path must be Markdown".into(),
        ));
    }
    sha(&v.synopsis_sha256, "synopsis_sha256")?;
    nonempty(&v.title, "title")?;
    if v.language.len() < 2
        || v.language.len() > 32
        || !v
            .language
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(Error::Contract(
            "reviewed synopsis handoff language is invalid".into(),
        ));
    }
    nonempty(&v.approval.actor_id, "approval.actor_id")?;
    if v.approval.decision != "APPROVE" {
        return Err(Error::Contract(
            "reviewed synopsis handoff approval.decision must be APPROVE".into(),
        ));
    }
    if v.approval.synopsis_sha256 != v.synopsis_sha256
        || v.approval.source_revision != v.source_revision
    {
        return Err(Error::Contract(
            "reviewed synopsis handoff approval is not bound to source".into(),
        ));
    }
    if v.approval.approved_at_ms == 0 {
        return Err(Error::Contract(
            "reviewed synopsis handoff approval.approved_at_ms must be at least 1".into(),
        ));
    }
    Ok(())
}

pub fn validate_reviewed_synopsis_handoff_identity(v: &ReviewedSynopsisHandoff) -> Result<()> {
    validate_reviewed_synopsis_handoff(v)?;
    let mut value = serde_json::to_value(v)?;
    value
        .as_object_mut()
        .ok_or_else(|| Error::Contract("reviewed synopsis handoff is not an object".into()))?
        .remove("handoff_id");
    let expected = format!(
        "gsh_{}",
        narrative_courier_runtime::token::hash_jcs(&value)?
    );
    if expected != v.handoff_id {
        return Err(Error::Contract(
            "reviewed synopsis handoff handoff_id is not bound to its body".into(),
        ));
    }
    Ok(())
}
