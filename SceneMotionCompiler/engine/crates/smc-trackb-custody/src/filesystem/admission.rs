//! Project-root, request, and fixed artifact-namespace admission before descriptor-backed effects.

use super::descriptors::{directory_identity, metadata_identity, open_directory};
use crate::error::{CustodyError, Result};

const MAX_PATH_BYTES: usize = 1024;
const MAX_ID_BYTES: usize = 128;
pub(crate) const SCENES_DIRECTORY: &str = "scenes";
pub(crate) const TRACKB_DIRECTORY: &str = "trackb";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactDirectory {
    Authority,
    Candidates,
    DeliverySets,
    Approvals,
}

impl ArtifactDirectory {
    const AUTHORITY: &'static str = "authority";
    const CANDIDATES: &'static str = "candidates";
    const DELIVERY_SETS: &'static str = "delivery-sets";
    const APPROVALS: &'static str = "approvals";

    pub(crate) fn from_segment(segment: &str) -> Option<Self> {
        if segment == Self::AUTHORITY {
            Some(Self::Authority)
        } else if segment == Self::CANDIDATES {
            Some(Self::Candidates)
        } else if segment == Self::DELIVERY_SETS {
            Some(Self::DeliverySets)
        } else if segment == Self::APPROVALS {
            Some(Self::Approvals)
        } else {
            None
        }
    }

    pub(crate) fn as_segment(self) -> &'static str {
        match self {
            Self::Authority => Self::AUTHORITY,
            Self::Candidates => Self::CANDIDATES,
            Self::DeliverySets => Self::DELIVERY_SETS,
            Self::Approvals => Self::APPROVALS,
        }
    }

    fn immutable_file_name(self) -> Option<&'static str> {
        match self {
            Self::Authority => Some("trackb-narrative-authority-v2.json"),
            Self::Candidates => Some("trackb-release-candidate-v4.json"),
            Self::DeliverySets => Some("trackb-delivery-set-v2.json"),
            Self::Approvals => None,
        }
    }
}
use serde_json::{Map, Value};
use std::fs::{self, File};
use std::path::{Component, Path};

pub(crate) fn validate_project_root(path: &Path) -> Result<File> {
    if !path.is_absolute() {
        return Err(CustodyError::input("project root must be an absolute path"));
    }
    let encoded_path = path.as_os_str().as_encoded_bytes();
    if encoded_path.len() > MAX_PATH_BYTES || encoded_path.contains(&0) {
        return Err(CustodyError::input(
            "project root path exceeds the bounded path limit",
        ));
    }
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                let bytes = name.as_encoded_bytes();
                if bytes.is_empty() || bytes.len() > 255 || bytes.contains(&0) {
                    return Err(CustodyError::input(
                        "project root contains an unsafe or oversized path component",
                    ));
                }
            }
            _ => {
                return Err(CustodyError::input(
                    "project root must use normalized absolute path components",
                ))
            }
        }
    }
    // Open each component from a descriptor for `/`, refusing symlinks at
    // every boundary.  This descriptor is retained by the session and is the
    // authority for all later narrative traversal.
    let opened = open_directory(path)
        .map_err(|error| CustodyError::io("project root cannot be opened safely", &error))?;
    let canonical = fs::canonicalize(path)
        .map_err(|error| CustodyError::io("project root cannot be resolved", &error))?;
    if canonical != path {
        return Err(CustodyError::input(
            "project root canonical path differs from the requested path",
        ));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| CustodyError::io("project root cannot be inspected", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(CustodyError::input(format!(
            "project root must be a non-symlink directory: {}",
            path.display()
        )));
    }
    let path_identity = metadata_identity(&metadata);
    let opened_identity = directory_identity(&opened)
        .map_err(|error| CustodyError::io("project root identity cannot be verified", &error))?;
    if opened_identity != path_identity {
        return Err(CustodyError::storage(
            "project root changed while it was being validated",
        ));
    }
    Ok(opened)
}

pub(crate) fn validate_storage_segment(segment: &str) -> Result<()> {
    if segment.is_empty()
        || segment == "."
        || segment == ".."
        || segment.len() > MAX_PATH_BYTES
        || segment.contains('\\')
        || segment.contains('\0')
        || !segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(CustodyError::input(format!(
            "unsafe storage path segment: {segment:?}"
        )));
    }
    Ok(())
}

pub(crate) fn required_safe_relative_path(
    object: &Map<String, Value>,
    key: &str,
) -> Result<String> {
    let path = required_string(object, key)?;
    validate_safe_relative_path(&path)?;
    Ok(path)
}

pub(crate) fn validate_safe_relative_path(path: &str) -> Result<()> {
    if path.is_empty()
        || path.len() > MAX_PATH_BYTES
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.contains('\0')
    {
        return Err(CustodyError::input(format!(
            "unsafe relative storage path: {path:?}"
        )));
    }
    for component in path.split('/') {
        validate_storage_segment(component)?;
    }
    // `Path::components` is a second guard against platform-specific lexical
    // surprises on the host running the helper.
    for component in Path::new(path).components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(CustodyError::input(format!(
                "unsafe relative storage path: {path:?}"
            )));
        }
    }
    Ok(())
}

/// Immutable writes are deliberately narrower than the generic safe-path
/// guard.  Pointer publication has its own CAS operation and may never be
/// smuggled through writeImmutable.  Hash directory names are the on-disk
/// form of a `sha256:` value (the prefix is omitted in the storage namespace).
pub(crate) fn validate_immutable_path(path: &str) -> Result<()> {
    validate_safe_relative_path(path)?;
    let parts: Vec<&str> = path.split('/').collect();
    let Some(directory) = parts
        .first()
        .and_then(|segment| ArtifactDirectory::from_segment(segment))
    else {
        return Err(CustodyError::input(
            "immutable path is outside the fixed Track B artifact namespace",
        ));
    };
    let hash_segment = |value: &str| {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    };
    let artifact = parts.len() == 3
        && directory
            .immutable_file_name()
            .is_some_and(|filename| parts[2] == filename)
        && hash_segment(parts[1]);
    let approval = directory == ArtifactDirectory::Approvals
        && parts.len() == 4
        && hash_segment(parts[1])
        && (parts[2] == "NONE" || hash_segment(parts[2]))
        && parts[3] == "trackb-release-approval-receipt-v2.json";
    if artifact || approval {
        Ok(())
    } else {
        Err(CustodyError::input(
            "immutable path is outside the fixed Track B artifact namespace",
        ))
    }
}

pub(crate) fn validate_narrative_id(value: &str) -> Result<()> {
    if !value.starts_with("narrative.")
        || value.len() < "narrative.".len() + 1
        || value.len() > MAX_ID_BYTES
        || !value["narrative.".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(CustodyError::input(format!(
            "invalid narrativeId: {value:?}"
        )));
    }
    Ok(())
}

pub(crate) fn validate_opaque_id(value: &str, label: &str) -> Result<()> {
    if !is_valid_opaque_id(value) {
        return Err(CustodyError::input(format!("invalid {label}: {value:?}")));
    }
    Ok(())
}

pub(crate) fn is_valid_opaque_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

pub(crate) fn required_string(object: &Map<String, Value>, key: &str) -> Result<String> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| CustodyError::input(format!("{key} must be a string")))?;
    if value.trim().is_empty() {
        return Err(CustodyError::input(format!("{key} must not be empty")));
    }
    Ok(value.to_owned())
}

pub(crate) fn require_keys(object: &Map<String, Value>, expected: &[&str]) -> Result<()> {
    let expected_set: std::collections::BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: std::collections::BTreeSet<&str> = object.keys().map(String::as_str).collect();
    if actual_set != expected_set {
        return Err(CustodyError::input(format!(
            "request fields must be exactly: {}",
            expected.join(", ")
        )));
    }
    Ok(())
}
