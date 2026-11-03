//! Canonical artifact reference wire contract.

use super::validate::{valid_artifact_local, valid_lowercase_token, valid_sha256_digest};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

/// Reference to a canonical artifact (`artifact-ref.schema.json`, v1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub id: String,
    pub kind: String,
    pub revision: u64,
    pub digest: String,
}

impl ArtifactRef {
    /// Validates the schema patterns: `id`
    /// (`^[a-z][a-z0-9_.-]{0,63}:` + `[A-Za-z0-9][A-Za-z0-9_.:/-]{0,191}$`),
    /// `kind` (`^[a-z][a-z0-9_.-]{0,63}$`), `revision >= 1`, and `digest`
    /// (`^sha256:[0-9a-f]{64}$`).
    ///
    /// # Errors
    /// Returns [`Error::InvalidInput`] naming the first violated field.
    pub fn validate(&self) -> Result<()> {
        let Some((namespace, local)) = self.id.split_once(':') else {
            return Err(Error::InvalidInput(format!(
                "artifact ref id '{}' must be '<namespace>:<local>'",
                self.id
            )));
        };
        if !valid_lowercase_token(namespace, 64) {
            return Err(Error::InvalidInput(format!(
                "artifact ref id namespace '{namespace}' must match ^[a-z][a-z0-9_.-]{{0,63}}$"
            )));
        }
        if !valid_artifact_local(local) {
            return Err(Error::InvalidInput(format!(
                "artifact ref id local part '{local}' must match ^[A-Za-z0-9][A-Za-z0-9_.:/-]{{0,191}}$"
            )));
        }
        if !valid_lowercase_token(self.kind.as_str(), 64) {
            return Err(Error::InvalidInput(format!(
                "artifact ref kind '{}' must match ^[a-z][a-z0-9_.-]{{0,63}}$",
                self.kind
            )));
        }
        if self.revision < 1 {
            return Err(Error::InvalidInput(
                "artifact ref revision must be at least 1".into(),
            ));
        }
        if !valid_sha256_digest(self.digest.as_str()) {
            return Err(Error::InvalidInput(format!(
                "artifact ref digest '{}' must match ^sha256:[0-9a-f]{{64}}$",
                self.digest
            )));
        }
        Ok(())
    }
}
