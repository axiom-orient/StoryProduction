use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub fn sha256_hex_bytes(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
pub fn sha256_bytes(data: &[u8]) -> String {
    format!("sha256:{}", sha256_hex_bytes(data))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = super::files::open_verified(path)?;
    let before = file.metadata()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65_536];
    let mut length = 0u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        length += count as u64;
    }
    let after = file.metadata()?;
    if length != before.len()
        || after.len() != before.len()
        || after.dev() != before.dev()
        || after.ino() != before.ino()
        || after.mtime() != before.mtime()
        || after.mtime_nsec() != before.mtime_nsec()
        || after.ctime() != before.ctime()
        || after.ctime_nsec() != before.ctime_nsec()
    {
        return Err(Error::not_applied("input changed while hashing"));
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub fn is_sha256(value: &str) -> bool {
    story_integration_core::Sha256::new(value).is_ok()
}
