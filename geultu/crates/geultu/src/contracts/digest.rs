//! Raw-byte and JCS (RFC 8785) digest helpers for Geultu contract identities.

use crate::error::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Returns the lowercase hexadecimal SHA-256 digest of the exact byte sequence.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Hashes Serde's compact JSON bytes without claiming RFC 8785 canonicalization.
pub(crate) fn serde_json_sha256<T: Serialize>(
    value: &T,
) -> std::result::Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    Ok(sha256_hex(&bytes))
}

/// Returns the lowercase hex SHA-256 of the RFC 8785 (JCS) canonical JSON
/// serialization of `value`, without the `sha256:` prefix so it can be
/// embedded directly into contract identifiers such as `np_<hex>`.
///
/// Canonicalization (UTF-16 key ordering, ES6 number formatting) is delegated
/// to `serde_json_canonicalizer`; this module never hand-rolls either rule.
///
/// # Errors
/// Returns [`crate::error::Error::Json`] when the value cannot be
/// canonicalized (serialization failure, e.g. non-string map keys from a
/// custom serializer).
pub fn jcs_sha256(value: &serde_json::Value) -> Result<String> {
    let canonical = serde_json_canonicalizer::to_string(value)?;
    Ok(sha256_hex(canonical.as_bytes()))
}
