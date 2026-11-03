//! Serde contract types for `text-run-state.schema.json` (H6) and
//! `visual-run-state.schema.json` (H7).
//!
//! Field sets mirror the canonical schemas 1:1;
//! `#[serde(deny_unknown_fields)]` enforces `additionalProperties: false`,
//! and pattern/enum constraints serde cannot express are enforced by the
//! `validate` methods, which fail closed with field-naming
//! [`Error::InvalidInput`] messages.

use super::validate::{check_length, valid_lowercase_token};
use crate::contracts::{ArtifactRef, EffectReceipt};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `schema_id` const of `text-run-state.schema.json`.
pub const TEXT_RUN_STATE_SCHEMA_ID: &str = "geultu.dev/text-run-state/v1";

/// `schema_id` const of `visual-run-state.schema.json`.
pub const VISUAL_RUN_STATE_SCHEMA_ID: &str = "geultu.dev/visual-run-state/v1";

/// Upper bound of `accepted_outputs` entries in a visual run state,
/// mirroring `maxProperties: 512`.
pub const VISUAL_ACCEPTED_OUTPUTS_LIMIT: usize = 512;

/// Upper bound of `accepted_outputs` entries, mirroring `maxProperties`.
pub const ACCEPTED_OUTPUTS_LIMIT: usize = 256;

/// `status` enum of `text-run-state.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TextRunStatus {
    Ready,
    Running,
    WaitingHuman,
    RepairRequired,
    Completed,
    Failed,
}

impl TextRunStatus {
    /// Returns `true` while the run may still spend budget units.
    #[must_use]
    pub(crate) fn can_spend(&self) -> bool {
        !matches!(self, Self::Completed | Self::Failed)
    }
}

/// `budgets` object of `text-run-state.schema.json` — the authority for H6
/// budget admission (`revision_remaining` / `pivot_remaining`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunBudgets {
    pub revision_remaining: u64,
    pub pivot_remaining: u64,
}

/// `frozen_inputs` object shared by the run-state family of schemas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunFrozenInputs {
    pub synopsis: ArtifactRef,
    pub style: ArtifactRef,
    pub canon: ArtifactRef,
    pub genre: ArtifactRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<ArtifactRef>,
}

impl RunFrozenInputs {
    /// Validates all four required artifact refs and the `sources` bound
    /// (`maxItems: 10000`).
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated field.
    pub fn validate(&self) -> Result<()> {
        self.synopsis.validate()?;
        self.style.validate()?;
        self.canon.validate()?;
        self.genre.validate()?;
        if self.sources.len() > 10_000 {
            return Err(Error::InvalidInput(
                "frozen_inputs sources must contain at most 10000 items".into(),
            ));
        }
        for source in &self.sources {
            source.validate()?;
        }
        Ok(())
    }
}

/// TextRunState v1 (`text-run-state.schema.json`).
///
/// The engine persists this document as opened by the operator; it never
/// invents frozen inputs or budgets. `pending_decision` is intentionally
/// [`serde_json::Value`]: the schema constrains it to `object|null` only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextRunState {
    pub schema_id: String,
    pub project_id: String,
    pub run_id: String,
    pub revision: u64,
    pub stage: String,
    pub status: TextRunStatus,
    pub attempt: u64,
    pub budgets: RunBudgets,
    pub frozen_inputs: RunFrozenInputs,
    pub accepted_outputs: BTreeMap<String, ArtifactRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_decision: Option<serde_json::Value>,
}

impl TextRunState {
    /// Validates every schema constraint: `schema_id` const, identity length
    /// bounds, stage pattern `^T(?:0[0-9]|[12][0-9]|3[0-3])$`, accepted-output
    /// key patterns and count, and nested refs/budgets.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated constraint.
    pub fn validate(&self) -> Result<()> {
        if self.schema_id != TEXT_RUN_STATE_SCHEMA_ID {
            return Err(Error::InvalidInput(format!(
                "schema_id '{}' must be '{TEXT_RUN_STATE_SCHEMA_ID}'",
                self.schema_id
            )));
        }
        check_length("run project_id", &self.project_id, 1, 256)?;
        check_length("run_id", &self.run_id, 1, 256)?;
        if !valid_run_stage(self.stage.as_str()) {
            return Err(Error::InvalidInput(format!(
                "stage '{}' must match ^T(?:0[0-9]|[12][0-9]|3[0-3])$",
                self.stage
            )));
        }
        if self
            .pending_decision
            .as_ref()
            .is_some_and(|value| !value.is_object() && !value.is_null())
        {
            return Err(Error::InvalidInput(
                "pending_decision must be an object or null".into(),
            ));
        }
        self.frozen_inputs.validate()?;
        if self.accepted_outputs.len() > ACCEPTED_OUTPUTS_LIMIT {
            return Err(Error::InvalidInput(format!(
                "accepted_outputs must contain at most {ACCEPTED_OUTPUTS_LIMIT} entries"
            )));
        }
        for (name, output) in &self.accepted_outputs {
            if !valid_lowercase_token(name.as_str(), 64) {
                return Err(Error::InvalidInput(format!(
                    "accepted_outputs key '{name}' must match ^[a-z][a-z0-9_.-]{{0,63}}$"
                )));
            }
            output.validate()?;
        }
        Ok(())
    }
}

/// `^T(?:0[0-9]|[12][0-9]|3[0-3])$` — stages T00 through T33.
#[must_use]
fn valid_run_stage(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 3 || bytes[0] != b'T' {
        return false;
    }
    let tens = bytes[1];
    let ones = bytes[2];
    if !ones.is_ascii_digit() {
        return false;
    }
    match tens {
        b'0' | b'1' | b'2' => true,
        b'3' => ones <= b'3',
        _ => false,
    }
}

/// `status` enum of `visual-run-state.schema.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VisualRunStatus {
    Ready,
    Running,
    RepairRequired,
    RecoveryRequired,
    Completed,
    Failed,
}

/// `^S(?:0[0-9]|1[0-2])$` — stages S00 through S12.
#[must_use]
fn valid_visual_stage(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 3 || bytes[0] != b'S' {
        return false;
    }
    let tens = bytes[1];
    let ones = bytes[2];
    if !ones.is_ascii_digit() {
        return false;
    }
    match tens {
        b'0' => true,
        b'1' => ones <= b'2',
        _ => false,
    }
}

/// VisualRunState v1 (`visual-run-state.schema.json`).
///
/// The engine persists this document as admitted at the NVC boundary; it
/// never invents the narrative-package reference or output map. The schema's
/// optional `pending_effect` is a schema-validated [`EffectReceipt`]; failed
/// and unknown external outcomes remain explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualRunState {
    pub schema_id: String,
    pub project_id: String,
    pub run_id: String,
    pub revision: u64,
    pub stage: String,
    pub status: VisualRunStatus,
    pub narrative_package: ArtifactRef,
    pub accepted_outputs: BTreeMap<String, ArtifactRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_effect: Option<EffectReceipt>,
}

impl VisualRunState {
    /// Validates every schema constraint: `schema_id` const, identity length
    /// bounds, stage pattern `^S(?:0[0-9]|1[0-2])$`, accepted-output key
    /// patterns and count, and the nested narrative-package ref.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated constraint.
    pub fn validate(&self) -> Result<()> {
        if self.schema_id != VISUAL_RUN_STATE_SCHEMA_ID {
            return Err(Error::InvalidInput(format!(
                "schema_id '{}' must be '{VISUAL_RUN_STATE_SCHEMA_ID}'",
                self.schema_id
            )));
        }
        check_length("run project_id", &self.project_id, 1, 256)?;
        check_length("run_id", &self.run_id, 1, 256)?;
        if !valid_visual_stage(self.stage.as_str()) {
            return Err(Error::InvalidInput(format!(
                "stage '{}' must match ^S(?:0[0-9]|1[0-2])$",
                self.stage
            )));
        }
        self.narrative_package.validate()?;
        if let Some(effect) = &self.pending_effect {
            effect.validate()?;
        }
        if self.accepted_outputs.len() > VISUAL_ACCEPTED_OUTPUTS_LIMIT {
            return Err(Error::InvalidInput(format!(
                "accepted_outputs must contain at most {VISUAL_ACCEPTED_OUTPUTS_LIMIT} entries"
            )));
        }
        for (name, output) in &self.accepted_outputs {
            if !valid_lowercase_token(name.as_str(), 64) {
                return Err(Error::InvalidInput(format!(
                    "accepted_outputs key '{name}' must match ^[a-z][a-z0-9_.-]{{0,63}}$"
                )));
            }
            output.validate()?;
        }
        Ok(())
    }
}
