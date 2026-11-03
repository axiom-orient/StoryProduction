//! Delivery-owned asset ingress and materialization owners.

pub(super) mod imported_image;
pub(super) mod materialize;

use narrative_courier_runtime::store::{atomic_write, sha256_file};
use narrative_courier_runtime::{Error, Result};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

pub(in crate::engine) const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

/// Read one held source handle; metadata and actual image bytes share the same bound.
pub(in crate::engine) fn read_source_bytes(path: &Path, maximum: Option<u64>) -> Result<Vec<u8>> {
    let entry = fs::symlink_metadata(path)?;
    if entry.file_type().is_symlink() || !entry.is_file() {
        return Err(Error::Contract(
            "INVALID_ASSET_REQUEST: source is not a regular file".into(),
        ));
    }
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || maximum.is_some_and(|limit| metadata.len() > limit) {
        return Err(Error::Contract(
            "INVALID_ASSET_REQUEST: source is not regular or exceeds its byte limit".into(),
        ));
    }
    let mut bytes = Vec::new();
    if let Some(limit) = maximum {
        file.take(limit + 1).read_to_end(&mut bytes)?;
    } else {
        file.take(metadata.len().saturating_add(1))
            .read_to_end(&mut bytes)?;
    }
    if bytes.len() as u64 != metadata.len()
        || maximum.is_some_and(|limit| bytes.len() as u64 > limit)
    {
        return Err(Error::Contract(
            "INVALID_ASSET_REQUEST: source size changed or exceeded its byte limit".into(),
        ));
    }
    Ok(bytes)
}

/// Content-addressed cache admission; an existing different body is never repaired implicitly.
fn publish_blob(project: &Path, relative: &str, bytes: &[u8], digest: &str) -> Result<()> {
    let target = super::project_owned_path(project, relative)?;
    let parent = target
        .parent()
        .ok_or_else(|| Error::Contract("asset blob has no parent".into()))?;
    fs::create_dir_all(parent)?;
    let target = super::project_owned_path(project, relative)?;
    match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            if metadata.len() != bytes.len() as u64 || sha256_file(&target)? != digest {
                return Err(Error::Contract("ASSET_HASH_MISMATCH: existing immutable blob differs; explicit recovery is required".into()));
            }
        }
        Ok(_) => {
            return Err(Error::Contract(
                "INVALID_PATH: asset blob is not a regular file".into(),
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => atomic_write(&target, bytes)?,
        Err(error) => return Err(error.into()),
    }
    if fs::metadata(&target)?.len() != bytes.len() as u64 || sha256_file(&target)? != digest {
        return Err(Error::Contract(
            "ASSET_HASH_MISMATCH: asset blob publication changed".into(),
        ));
    }
    Ok(())
}

/// Capture one materialized image for persisted admission and document representation.
pub(crate) fn read_materialized_image(
    project: &Path,
    item: &narrative_courier_contracts::AssetItem,
) -> Result<Vec<u8>> {
    let path = super::project_owned_path(project, &item.path)?;
    let bytes = read_source_bytes(&path, Some(MAX_IMAGE_BYTES))?;
    let (format, _, _) = imported_image::inspect_imported_image(&bytes)?;
    let (extension, mime) = imported_image::imported_format_details(format);
    if item.kind != narrative_courier_contracts::AssetKind::Image
        || item.path != format!("visual/assets/blobs/{}.{}", item.sha256, extension)
        || item.size_bytes != bytes.len() as u64
        || item.mime != mime
        || narrative_courier_runtime::store::sha256_bytes(&bytes) != item.sha256
    {
        return Err(Error::Contract(format!(
            "ASSET_HASH_MISMATCH: materialized image {} differs from its index",
            item.asset_id
        )));
    }
    Ok(bytes)
}
