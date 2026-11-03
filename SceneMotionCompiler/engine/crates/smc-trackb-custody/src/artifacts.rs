//! Stable artifact snapshots, token rules, byte limits, and response projections.

pub(crate) mod path;

use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;

use serde_json::{Map, Value};

use crate::error::{CustodyError, Result};
use crate::filesystem::descriptors::{
    file_identity, fstatat_no_follow, stat_identity, stat_is_regular, FileIdentity,
};
use crate::protocol::{canonical_json, encode_base64url, sha256_prefixed};

pub(super) const MAX_READ_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(super) struct Snapshot {
    pub(super) exists: bool,
    pub(super) identity: Option<FileIdentity>,
    pub(super) bytes: Vec<u8>,
    pub(super) sha256: Option<String>,
}

impl Snapshot {
    pub(super) fn absent() -> Self {
        Self {
            exists: false,
            identity: None,
            bytes: Vec::new(),
            sha256: None,
        }
    }

    pub(super) fn token(&self) -> String {
        if !self.exists {
            return "NONE".to_owned();
        }
        let identity = self.identity.expect("present snapshots have identity");
        let mut object = Map::new();
        object.insert(
            "bytesHash".to_owned(),
            Value::String(self.sha256.clone().unwrap_or_default()),
        );
        object.insert(
            "byteLength".to_owned(),
            Value::from(self.bytes.len() as u64),
        );
        object.insert("dev".to_owned(), Value::from(identity.dev));
        object.insert("ino".to_owned(), Value::from(identity.ino));
        encode_base64url(canonical_json(&Value::Object(object)).as_bytes())
    }
}

pub(super) fn snapshot_entry(parent: &File, name: &str, maximum_bytes: usize) -> Result<Snapshot> {
    let initial_stat = fstatat_no_follow(parent, name);
    let initial_stat = match initial_stat {
        Ok(stat) => stat,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Snapshot::absent()),
        Err(error) => {
            return Err(CustodyError::storage(format!(
                "storage entry cannot be inspected safely: {error}"
            )))
        }
    };
    if !stat_is_regular(&initial_stat) {
        return Err(CustodyError::storage(
            "storage entry must be a regular non-symlink file",
        ));
    }
    let expected = stat_identity(&initial_stat);
    let file = open_read_at(parent, name)
        .map_err(|error| CustodyError::io("storage file cannot be opened", &error))?;
    let opened = file_identity(&file)
        .map_err(|error| CustodyError::io("storage fd identity failed", &error))?;
    if opened != expected {
        return Err(CustodyError::storage("storage file changed before read"));
    }
    let size = file
        .metadata()
        .map_err(|error| CustodyError::io("storage file metadata failed", &error))?
        .size();
    if size > maximum_bytes as u64 {
        return Err(CustodyError::storage(format!(
            "storage file exceeds bounded read size: {size}"
        )));
    }
    let mut bytes = Vec::with_capacity(size as usize);
    let mut bounded = file.take(maximum_bytes as u64 + 1);
    bounded
        .read_to_end(&mut bytes)
        .map_err(|error| CustodyError::io("storage file read failed", &error))?;
    if bytes.len() > maximum_bytes {
        return Err(CustodyError::storage(
            "storage file exceeded bounded read size",
        ));
    }
    if bytes.len() != size as usize {
        return Err(CustodyError::storage(
            "storage file changed while being read",
        ));
    }
    let final_stat = fstatat_no_follow(parent, name)
        .map_err(|error| CustodyError::io("storage file disappeared after read", &error))?;
    if !stat_is_regular(&final_stat) || stat_identity(&final_stat) != expected {
        return Err(CustodyError::storage(
            "storage file was replaced while being read",
        ));
    }
    let sha256 = sha256_prefixed(&bytes);
    Ok(Snapshot {
        exists: true,
        identity: Some(expected),
        bytes,
        sha256: Some(sha256),
    })
}

pub(super) fn validate_snapshot_token(value: &str) -> Result<()> {
    if value != "NONE" && (value.is_empty() || value.len() > 16 * 1024) {
        return Err(CustodyError::input(
            "snapshot token must be NONE or a bounded opaque token",
        ));
    }
    Ok(())
}

pub(super) fn artifact_read_value(kind: &str, path: &str, snapshot: &Snapshot) -> Value {
    let mut object = Map::new();
    object.insert("kind".to_owned(), Value::String(kind.to_owned()));
    object.insert("path".to_owned(), Value::String(path.to_owned()));
    object.insert("exists".to_owned(), Value::Bool(snapshot.exists));
    if snapshot.exists {
        object.insert(
            "bytesBase64".to_owned(),
            Value::String(encode_base64url(&snapshot.bytes)),
        );
        object.insert(
            "byteLength".to_owned(),
            Value::from(snapshot.bytes.len() as u64),
        );
        object.insert(
            "sha256".to_owned(),
            Value::String(snapshot.sha256.clone().unwrap_or_default()),
        );
    }
    Value::Object(object)
}

fn open_read_at(parent: &File, name: &str) -> io::Result<File> {
    let c_name = CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let fd = unsafe { libc::openat(parent.as_raw_fd(), c_name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub(super) fn snapshot_value(path: &str, snapshot: &Snapshot) -> Value {
    let mut object = Map::new();
    if !path.is_empty() {
        object.insert("path".to_owned(), Value::String(path.to_owned()));
    }
    object.insert("exists".to_owned(), Value::Bool(snapshot.exists));
    object.insert("snapshotToken".to_owned(), Value::String(snapshot.token()));
    if snapshot.exists {
        let identity = snapshot.identity.expect("present snapshot identity");
        object.insert(
            "bytesBase64".to_owned(),
            Value::String(encode_base64url(&snapshot.bytes)),
        );
        object.insert(
            "byteLength".to_owned(),
            Value::from(snapshot.bytes.len() as u64),
        );
        object.insert(
            "sha256".to_owned(),
            Value::String(snapshot.sha256.clone().unwrap_or_default()),
        );
        object.insert("device".to_owned(), Value::from(identity.dev));
        object.insert("inode".to_owned(), Value::from(identity.ino));
    }
    Value::Object(object)
}
