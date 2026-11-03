//! Filesystem digest verification for parsed checksum manifests.

use super::parse::{ensure_safe_pack_path, parse_sums};
use crate::contracts::digest::sha256_hex;
use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

/// Verifies every sums entry against the files under `pack_dir` and returns
/// the verified entries as an ordered path→digest map.
///
/// Mirrors `shasum -a 256 -c` outcomes: a modified file or a listed-but-
/// missing file is a fail-closed error naming the offending path. Files
/// present on disk but absent from the sums file are ignored, exactly as
/// `shasum -c` ignores them.
///
/// # Errors
/// Returns [`Error::Corrupt`] on a digest mismatch and [`Error::Io`] /
/// [`Error::NotFound`] when a listed file cannot be read.
pub fn verify_sums(pack_dir: &Path, sums_path: &Path) -> Result<BTreeMap<String, String>> {
    let content = read_sums_file(pack_dir, sums_path)?;
    let entries = parse_sums(&content)?;
    let mut verified = BTreeMap::new();
    for entry in entries {
        let actual = hash_pack_file(pack_dir, &entry.path).map_err(|error| match error {
            invalid @ Error::InvalidInput(_) => invalid,
            Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Error::NotFound(format!(
                    "pack file '{}' listed in {} cannot be read: {error}",
                    entry.path,
                    sums_path.display()
                ))
            }
            other => other,
        })?;
        if actual != entry.digest {
            return Err(Error::Corrupt(format!(
                "integrity mismatch for '{}': SHA256SUMS expects {}, actual {actual}",
                entry.path, entry.digest
            )));
        }
        verified.insert(entry.path, actual);
    }
    Ok(verified)
}

fn read_sums_file(pack_dir: &Path, sums_path: &Path) -> Result<String> {
    let Ok(relative) = sums_path.strip_prefix(pack_dir) else {
        // The public verifier permits an explicitly supplied external sums
        // file; pack-local manifests use the same no-symlink path admission
        // as every listed entry.
        return Ok(fs::read_to_string(sums_path)?);
    };
    let relative = relative
        .to_str()
        .ok_or_else(|| Error::InvalidInput("pack checksum path must be valid UTF-8".into()))?;
    let mut file = open_pack_file(pack_dir, relative).map_err(|error| match error {
        Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Error::NotFound(format!(
                "checksum file '{}' cannot be read: {error}",
                sums_path.display()
            ))
        }
        other => other,
    })?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

// Pack lookup admission is shared by checksum reads and the bytes parsed by loaders.
fn open_pack_file(root: &Path, relative: &str) -> Result<File> {
    ensure_safe_pack_path(relative)?;
    let parts: Vec<_> = relative.split('/').collect();
    let mut current = root.to_path_buf();
    for (index, part) in parts.iter().enumerate() {
        current.push(part);
        let metadata = fs::symlink_metadata(&current)?;
        if metadata.file_type().is_symlink()
            || (index + 1 < parts.len() && !metadata.is_dir())
            || (index + 1 == parts.len() && !metadata.is_file())
        {
            return Err(Error::InvalidInput(format!(
                "unsafe pack entry '{relative}'"
            )));
        }
    }
    let file = File::open(current)?;
    if !file.metadata()?.is_file() {
        return Err(Error::InvalidInput(format!(
            "pack entry '{relative}' is not a regular file"
        )));
    }
    Ok(file)
}

fn hash_pack_file(root: &Path, relative: &str) -> Result<String> {
    let mut file = open_pack_file(root, relative)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// Read the exact text bytes consumed by a loader under the previously verified digest.
pub(crate) fn read_verified_text(
    root: &Path,
    files: &BTreeMap<String, String>,
    relative: &str,
) -> Result<String> {
    let expected = files.get(relative).ok_or_else(|| {
        Error::Corrupt(format!(
            "pack document '{relative}' is not covered by SHA256SUMS"
        ))
    })?;
    let mut text = String::new();
    let mut file = open_pack_file(root, relative).map_err(|error| match error {
        Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound => Error::NotFound(
            format!("pack document '{relative}' cannot be read: {error}"),
        ),
        other => other,
    })?;
    file.read_to_string(&mut text)?;
    if sha256_hex(text.as_bytes()) != *expected {
        return Err(Error::Corrupt(format!(
            "pack document '{relative}' changed after checksum verification"
        )));
    }
    Ok(text)
}
