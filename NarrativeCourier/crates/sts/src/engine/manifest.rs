//! Stable STS manifest entrypoints for loading, validation, and identity.

mod identity;
mod validation;

use super::{resolve_regular_file_within_root, Engine};
use crate::model::workflow::{DomainIdentity, Manifest};
use narrative_courier_runtime::Result;
use std::fs;

impl Engine {
    pub fn load_manifest(&self) -> Result<Manifest> {
        let path = resolve_regular_file_within_root(&self.skill_root(), "workflow.json")?;
        let bytes = fs::read(path)?;
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        self.validate_manifest(&manifest)?;
        Ok(manifest)
    }

    pub fn validate_manifest(&self, manifest: &Manifest) -> Result<()> {
        validation::validate(self, manifest)
    }

    pub(super) fn domain_identity(&self, manifest: &Manifest) -> Result<DomainIdentity> {
        identity::domain_identity(self, manifest)
    }
}
