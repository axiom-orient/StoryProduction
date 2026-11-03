//! Refinement catalog integrity, policy, and archive coverage loading.

use super::{RefinementCatalog, RefinementPack};
use crate::error::{Error, Result};
use crate::integrity::{read_verified_text, verify_sums};
use std::path::Path;

/// Loads and verifies the refinement pack under `dir`, failing closed
/// before any parse when integrity does not hold.
///
/// Every archive named by the catalog must be covered by the pack's own
/// SHA256SUMS; archives present on disk but unlisted follow `shasum -c`
/// semantics and are ignored.
///
/// # Errors
/// Returns integrity errors from [`verify_sums`], validation errors from
/// [`RefinementCatalog::validate`], and [`Error::Corrupt`] /
/// [`Error::NotFound`] when a listed archive disagrees or is missing.
pub fn load_refinement_pack(dir: &Path) -> Result<RefinementPack> {
    let directory = dir.canonicalize()?;
    let dir = directory.as_path();
    let files = verify_sums(dir, &dir.join("SHA256SUMS"))?;
    let raw = read_verified_text(dir, &files, "catalog.json")?;
    let catalog: RefinementCatalog = serde_json::from_str(&raw)?;
    catalog.validate()?;
    for (locale, archive) in &catalog.locale_archives {
        let listed = format!("archives/{archive}");
        if !files.contains_key(&listed) {
            return Err(Error::Corrupt(format!(
                "refinement catalog locale '{locale}' names archive '{archive}' \
                 which is not covered by SHA256SUMS"
            )));
        }
    }
    Ok(RefinementPack { catalog, files })
}
