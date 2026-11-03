//! H5 refinement patch flow over the language-v1 packs
//! (`assets/refinement-packs-language-v1`).
//!
//! The pack supplies policy and locale authority (`catalog.json` plus its
//! own `SHA256SUMS`, which verifies every locale archive by digest — zip
//! contents are deliberately not read in this milestone). The diagnosis
//! rules live in [`diagnose`] and are engine-owned and deterministic.
//! Catalog loading, diagnosis, and candidate/changeset proposal are kept in
//! separate submodules; this module preserves the public types and entrypoints.
//!
//! # translation_forbidden is structural, not advisory
//!
//! [`RefinementPolicy`] requires `translation_forbidden == true` at load
//! time (any other value fails closed: this engine only implements the
//! translation-forbidden policy). Beyond the flag, a translated string has
//! **no representable carrier** in this module:
//!
//! - `refinement.diagnose` and `refinement.propose_patch` accept no
//!   free-text parameter at all — their inputs are project/scene ids only.
//! - [`diagnose::RefinementFinding::replacement`] is populated exclusively
//!   by pure collapse/normalize functions applied to characters scanned out
//!   of the diagnosed document itself; there is no code path that can source
//!   replacement text from anywhere else.
//! - The resulting changeset operation reuses the scene's own text with
//!   spans rewritten in place, so the applied content is byte-derived from
//!   the same-language source.
//!
//! # Budget integration
//!
//! `propose_refinement_patch` writes a pending candidate and changeset only;
//! it spends no revision units. Budget consumption happens exclusively at
//! `changeset.approve`, which already carries the H6 `run_id` admission
//! ([`crate::admission::budget::admit_revision_spend`]) — refinement application is
//! therefore gated by the same revision budget as every other text change.

mod catalog;
mod diagnose;
mod proposal;

pub use catalog::load_refinement_pack;
pub use diagnose::{diagnose_text, RefinementFinding};
pub use proposal::{propose_refinement_patch, repair_decision};

use crate::contracts::DomainDecision;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `api_version` this loader understands; other majors fail closed.
pub const REFINEMENT_CATALOG_API_VERSION: &str = "geultu.dev/refinement-pack-catalog/v1";
/// Candidate `kind` stamped onto refinement proposals so approval can
/// require an explicit bound human decision.
pub const REFINEMENT_CANDIDATE_KIND: &str = "refinement_patch";
/// Provider identity on refinement candidate receipts.
pub const REFINEMENT_PROVIDER_ID: &str = "geultu-refinement";

/// Policy block of `catalog.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementPolicy {
    pub operations: Vec<String>,
    pub output_authority: String,
    pub translation_forbidden: bool,
    pub approval_owned_by: String,
}

impl RefinementPolicy {
    /// Fail-closed policy conformance: this engine implements exactly the
    /// diagnose→plan_patch flow under geultu-owned approval with
    /// translation forbidden. A pack declaring anything else is rejected
    /// rather than partially honored.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first unsupported policy.
    pub fn validate(&self) -> Result<()> {
        if !self.translation_forbidden {
            return Err(Error::InvalidInput(
                "refinement policy translation_forbidden must be true; \
                 this engine cannot honor a translation-allowing pack"
                    .into(),
            ));
        }
        if self.approval_owned_by != "geultu" {
            return Err(Error::InvalidInput(format!(
                "refinement policy approval_owned_by '{}' must be 'geultu'",
                self.approval_owned_by
            )));
        }
        if self.output_authority != "geultu_polish_patch" {
            return Err(Error::InvalidInput(
                "refinement policy output_authority must be geultu_polish_patch".into(),
            ));
        }
        let supported = ["diagnose", "plan_patch"];
        for operation in &self.operations {
            if !supported.contains(&operation.as_str()) {
                return Err(Error::InvalidInput(format!(
                    "refinement policy operation '{operation}' is not supported; \
                     supported operations: diagnose, plan_patch"
                )));
            }
        }
        Ok(())
    }

    /// Returns `true` when the catalog grants `operation`.
    #[must_use]
    pub fn allows(&self, operation: &str) -> bool {
        self.operations.iter().any(|allowed| allowed == operation)
    }
}

/// Typed `catalog.json` of a language-v1 refinement pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementCatalog {
    pub api_version: String,
    pub catalog_id: String,
    pub source_release_version: String,
    pub source_release_date: String,
    pub locale_archives: BTreeMap<String, String>,
    pub policy: RefinementPolicy,
}

impl RefinementCatalog {
    /// Validates identity and per-locale archive naming.
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] for a foreign api version, an
    /// invalid locale tag, or an unsafe archive file name.
    pub fn validate(&self) -> Result<()> {
        if self.api_version != REFINEMENT_CATALOG_API_VERSION {
            return Err(Error::InvalidInput(format!(
                "refinement catalog api_version '{}' must be '{REFINEMENT_CATALOG_API_VERSION}'",
                self.api_version
            )));
        }
        crate::domain::validation::validate_id("refinement catalog id", &self.catalog_id)?;
        self.policy.validate()?;
        if self.locale_archives.is_empty() {
            return Err(Error::InvalidInput(
                "refinement catalog must list at least one locale archive".into(),
            ));
        }
        for (locale, archive) in &self.locale_archives {
            if !crate::contracts::validate::valid_language_tag(locale, 1) {
                return Err(Error::InvalidInput(format!(
                    "refinement catalog locale '{locale}' must match ^[A-Za-z]{{2,3}}(?:-[A-Za-z0-9]{{2,8}})*$"
                )));
            }
            if archive.is_empty() || archive.contains(['/', '\\']) || archive.starts_with('.') {
                return Err(Error::InvalidInput(format!(
                    "refinement catalog archive '{archive}' must be a plain file name"
                )));
            }
        }
        Ok(())
    }
}

/// A verified refinement pack: parsed catalog plus the verified
/// path→digest map (locale archives are checked by digest only).
#[derive(Debug, Clone)]
pub struct RefinementPack {
    pub(super) catalog: RefinementCatalog,
    pub(super) files: BTreeMap<String, String>,
}

impl RefinementPack {
    pub fn catalog(&self) -> &RefinementCatalog {
        &self.catalog
    }
    pub fn files(&self) -> &BTreeMap<String, String> {
        &self.files
    }
}

/// Input for [`propose_refinement_patch`]; deliberately carries no text
/// field — see the module-level translation_forbidden documentation.
#[derive(Debug, Clone)]
pub struct PatchProposalInput {
    pub scene_id: String,
    pub candidate_id: Option<String>,
    pub changeset_id: Option<String>,
    pub command_id: String,
}

/// Builds the deterministic REPAIR proposal for one scene: candidate +
/// pending changeset carrying the patched text, plus the domain-decision
/// proposal document. Nothing here writes canonical text; application still
/// requires `changeset.approve` with a bound human decision.
///
/// # Errors
/// Returns [`Error::InvalidInput`] when the policy forbids plan_patch or
/// the scene has no mechanically patchable findings, [`Error::Json`] when
/// serialization fails, and commit errors from the workspace transactions.
/// Result of [`propose_refinement_patch`].
#[derive(Debug, Clone, Serialize)]
pub struct PatchProposal {
    pub proposal: DomainDecision,
    pub candidate_id: String,
    pub changeset_id: String,
    pub scene_id: String,
    pub source_scene_revision: u64,
    pub applied_codes: Vec<String>,
    pub source_chars: usize,
    pub patched_chars: usize,
}
