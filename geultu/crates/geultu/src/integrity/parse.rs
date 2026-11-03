//! Pure SHA256SUMS line parsing and pack-relative path rules.

use crate::error::{Error, Result};
use std::collections::BTreeSet;
use std::path::Path;

/// Parsed sums data consumed only by the integrity verifier.
#[derive(Debug)]
pub(super) struct ChecksumEntry {
    pub(super) digest: String,
    pub(super) path: String,
}

/// Parses SHA256SUMS content into entries, rejecting malformed lines and
/// unsafe paths before any file is touched.
///
/// # Errors
/// Returns [`Error::InvalidInput`] naming the first offending line number.
pub(super) fn parse_sums(content: &str) -> Result<Vec<ChecksumEntry>> {
    let mut entries = Vec::new();
    let mut paths = BTreeSet::new();
    for (index, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let bytes = line.as_bytes();
        if bytes.len() < 66 || bytes[64] != b' ' {
            return Err(Error::InvalidInput(format!(
                "SHA256SUMS line {} is not '<64-hex> <type-char> <path>': {line:?}",
                index + 1
            )));
        }
        let raw_digest = &line[..64];
        if !raw_digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::InvalidInput(format!(
                "SHA256SUMS line {} carries a non-hex digest: {raw_digest}",
                index + 1
            )));
        }
        let type_char = bytes[65];
        if type_char != b' ' && type_char != b'*' {
            return Err(Error::InvalidInput(format!(
                "SHA256SUMS line {} uses unknown type char '{}'; expected ' ' or '*'",
                index + 1,
                char::from(type_char)
            )));
        }
        let path = &line[66..];
        ensure_safe_pack_path(path).map_err(|error| match error {
            Error::InvalidInput(message) => {
                Error::InvalidInput(format!("SHA256SUMS line {}: {message}", index + 1))
            }
            other => other,
        })?;
        if !paths.insert(path) {
            return Err(Error::InvalidInput(format!(
                "SHA256SUMS line {} repeats path '{path}'",
                index + 1
            )));
        }
        entries.push(ChecksumEntry {
            digest: raw_digest.to_ascii_lowercase(),
            path: path.to_string(),
        });
    }
    if entries.is_empty() {
        return Err(Error::InvalidInput(
            "SHA256SUMS carries no checksum lines".into(),
        ));
    }
    Ok(entries)
}

/// Rejects sum entries that could escape the pack directory: absolute
/// paths, `..` components, backslashes, and empty paths. Verified before
/// any filesystem access so a hostile sums file cannot make the loader read
/// outside the pack.
pub(super) fn ensure_safe_pack_path(path: &str) -> Result<()> {
    if path.is_empty() {
        return Err(Error::InvalidInput(
            "sums entry path must not be empty".into(),
        ));
    }
    if path.contains('\0') {
        return Err(Error::InvalidInput("sums entry path contains NUL".into()));
    }
    if path.contains('\\') {
        return Err(Error::InvalidInput(format!(
            "sums entry path '{path}' contains a backslash"
        )));
    }
    if Path::new(path).is_absolute()
        || Path::new(path)
            .components()
            .any(|component| matches!(component, std::path::Component::Prefix(_)))
    {
        return Err(Error::InvalidInput(format!(
            "sums entry path '{path}' is absolute"
        )));
    }
    if path.split('/').any(|part| matches!(part, "." | "..")) {
        return Err(Error::InvalidInput(format!(
            "sums entry path '{path}' must not contain '.' or '..' components"
        )));
    }
    if path.split('/').any(str::is_empty) {
        return Err(Error::InvalidInput(format!(
            "sums entry path '{path}' has an empty component"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_sums;
    use crate::error::Error;

    #[test]
    fn parses_text_and_binary_mode_lines_case_insensitively() {
        let sums = "ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef0123456789  a.md\n\
                    0000000000000000000000000000000000000000000000000000000000000000 *b.bin\n";
        let entries = parse_sums(sums).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "a.md");
        assert!(entries[0]
            .digest
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit()));
        assert_eq!(entries[1].path, "b.bin");
    }

    #[test]
    fn rejects_malformed_lines_and_unsafe_paths() {
        assert!(parse_sums("short  a.md\n").is_err());
        assert!(parse_sums(
            "zzzz000000000000000000000000000000000000000000000000000000000000  a.md\n"
        )
        .is_err());
        assert!(parse_sums(
            "abcdef000000000000000000000000000000000000000000000000000000000000\ta.md\n"
        )
        .is_err());
        let valid_digest = format!("abcdef{}", "0".repeat(58));
        let traversal = parse_sums(&format!("{valid_digest}  ../escape.md\n")).unwrap_err();
        assert!(matches!(traversal, Error::InvalidInput(message) if message.contains("..")));
        let absolute = parse_sums(&format!("{valid_digest}  /etc/passwd\n")).unwrap_err();
        assert!(matches!(absolute, Error::InvalidInput(message) if message.contains("absolute")));
        let backslash = parse_sums(&format!("{valid_digest}  a\\b.md\n")).unwrap_err();
        assert!(matches!(backslash, Error::InvalidInput(message) if message.contains("backslash")));
        assert!(parse_sums("\n \n").is_err(), "blank-only sums must fail");
    }
}
