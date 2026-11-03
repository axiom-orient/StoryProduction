//! Read-only context required to author a deterministic DeliveryRequest.

use super::{project_owned_path, verify_released_package, DeliveryContext, DeliveryEngine};
use crate::backend::describe_backends;
use narrative_courier_contracts::NarrativePackage;
use narrative_courier_runtime::store::sha256_bytes;
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::Path;

impl DeliveryEngine {
    pub fn context(project: &Path, package: &NarrativePackage) -> Result<DeliveryContext> {
        let package_sha = verify_released_package(project, package)?;
        let screenplay_path =
            project_owned_path(project, &package.canonical_narrative.artifact.path)?;
        let screenplay = fs::read_to_string(screenplay_path)?;
        if sha256_bytes(screenplay.as_bytes()) != package.canonical_narrative.artifact.sha256 {
            return Err(Error::Contract(
                "HASH_MISMATCH: Delivery context screenplay changed after release admission".into(),
            ));
        }
        let scenes = crate::planner::scene::extract_scenes(&screenplay)?;
        Ok(DeliveryContext {
            narrative_package_sha256: package_sha,
            backends: describe_backends()?,
            scenes,
        })
    }
}
