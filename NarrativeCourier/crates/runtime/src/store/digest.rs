//! Raw byte digests; file digesting holds one admitted regular source.

use crate::{Error, Result};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

pub fn sha256_file(path: &Path) -> Result<String> {
    let entry = fs::symlink_metadata(path)?;
    if entry.file_type().is_symlink() || !entry.is_file() {
        return Err(Error::Contract(
            "digest source must be a regular non-symlink file".into(),
        ));
    }
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(Error::Contract("digest source changed type".into()));
    }
    #[cfg(unix)]
    if !super::same_file(&entry, &metadata) {
        return Err(Error::Contract(
            "digest source identity changed during open".into(),
        ));
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut count = 0_u64;
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        count = count
            .checked_add(length as u64)
            .ok_or_else(|| Error::Contract("digest source exceeds u64 capacity".into()))?;
        if count > metadata.len() {
            return Err(Error::Contract("digest source grew during read".into()));
        }
        hasher.update(&buffer[..length]);
    }
    if count != metadata.len() || file.metadata()?.len() != count {
        return Err(Error::Contract(
            "digest source size changed during read".into(),
        ));
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
