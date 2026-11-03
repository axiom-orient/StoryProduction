use std::io::Write;

use serde_json::{Map, Value};

use super::{Session, PROJECT_ARTIFACT_SEQUENCE};
use crate::artifacts::{snapshot_entry, snapshot_value, Snapshot, MAX_READ_BYTES};
use crate::error::{CustodyError, Result};
use crate::filesystem::admission::{
    require_keys, required_safe_relative_path, required_string, validate_immutable_path,
    validate_narrative_id, validate_opaque_id,
};
use crate::filesystem::descriptors::{file_identity, link_at, open_exclusive_at, sync_directory};
use crate::filesystem::publication::cleanup_owned_temp;
use crate::lock_release::{acquire_lock, release_lock};
use crate::protocol::{decode_base64url_bounded, json_object, PROTOCOL_VERSION};
use crate::startup::SessionPurpose;

const MAX_WRITE_BYTES: usize = 1024 * 1024;

impl Session {
    pub(crate) fn initialize(&mut self, object: &Map<String, Value>) -> Result<Value> {
        if self.purpose == SessionPurpose::Measurement {
            return Err(CustodyError::input(
                "initialize is not available in a measurement session",
            ));
        }
        require_keys(object, &["id", "op", "mode", "narrativeId", "operationId"])?;
        if self.lock.is_some() || self.narrative_id.is_some() {
            return Err(CustodyError::input("session is already initialized"));
        }
        let mode = required_string(object, "mode")?;
        let read_only = match mode.as_str() {
            "mutable" => false,
            "readOnly" => true,
            _ => {
                return Err(CustodyError::input(
                    "mode must be exactly mutable or readOnly",
                ))
            }
        };
        let narrative_id = required_string(object, "narrativeId")?;
        validate_narrative_id(&narrative_id)?;
        let operation_id = required_string(object, "operationId")?;
        validate_opaque_id(&operation_id, "operationId")?;
        if self.purpose == SessionPurpose::ProjectIngress
            && (self.project_artifact_sequence_index != PROJECT_ARTIFACT_SEQUENCE.len()
                || self.project_artifact_reads.len() != PROJECT_ARTIFACT_SEQUENCE.len()
                || PROJECT_ARTIFACT_SEQUENCE.iter().any(|kind| {
                    self.project_artifact_reads
                        .get(*kind)
                        .is_none_or(|read| !read.exists)
                }))
        {
            return Err(CustodyError::input(
                "project-ingress requires all seven artifacts present in the fixed read order before initialize",
            ));
        }
        if self.project_fd.is_none() {
            return Err(CustodyError::input(
                "project root descriptor is not retained from startup",
            ));
        }
        self.read_only = read_only;
        self.narrative_id = Some(narrative_id.clone());
        self.operation_id = Some(operation_id);
        if read_only {
            // A read-only status/reconcile session only needs the narrative
            // directory itself.  Artifact directories are optional and are
            // opened lazily from this retained descriptor for the requested
            // path, so an absent unrelated artifact tree cannot hide a
            // present narrative-root pointer.
            if let Some((narrative_dir, narrative_fd)) =
                self.open_narrative_directory_read_only(&narrative_id)?
            {
                self.narrative_dir = Some(narrative_dir);
                self.narrative_fd = Some(narrative_fd);
            }
        } else {
            let (narrative_dir, narrative_fd, artifact_directories) =
                self.open_narrative_directory(&narrative_id)?;
            self.narrative_dir = Some(narrative_dir);
            self.narrative_fd = Some(narrative_fd);
            self.artifact_directories = Some(artifact_directories);
        }
        Ok(json_object([
            ("protocolVersion", Value::from(PROTOCOL_VERSION)),
            ("narrativeId", Value::String(narrative_id)),
            ("mode", Value::String(mode)),
            ("namespacePresent", Value::Bool(self.narrative_fd.is_some())),
        ]))
    }

    pub(crate) fn acquire(&mut self, object: &Map<String, Value>) -> Result<Value> {
        if self.purpose == SessionPurpose::Measurement {
            return Err(CustodyError::input(
                "acquire is not available in a measurement session",
            ));
        }
        require_keys(object, &["id", "op", "narrativeId", "operationId"])?;
        if self.read_only {
            return Err(CustodyError::input(
                "acquire is forbidden in a read-only custody session",
            ));
        }
        if self.lock.is_some() {
            return Err(CustodyError::input("session already owns a narrative lock"));
        }
        let narrative_id = required_string(object, "narrativeId")?;
        validate_narrative_id(&narrative_id)?;
        let operation_id = required_string(object, "operationId")?;
        validate_opaque_id(&operation_id, "operationId")?;
        if let Some(existing) = &self.narrative_id {
            if existing != &narrative_id || self.operation_id.as_deref() != Some(&operation_id) {
                return Err(CustodyError::input(
                    "acquire must use the exact initialized narrative and operation",
                ));
            }
        } else {
            let (narrative_dir, narrative_fd, artifact_directories) =
                self.open_narrative_directory(&narrative_id)?;
            self.narrative_id = Some(narrative_id.clone());
            self.operation_id = Some(operation_id.clone());
            self.narrative_dir = Some(narrative_dir);
            self.narrative_fd = Some(narrative_fd);
            self.artifact_directories = Some(artifact_directories);
        }
        let parent = self
            .narrative_fd
            .as_ref()
            .ok_or_else(|| CustodyError::input("narrative directory is not initialized"))?
            .try_clone()
            .map_err(|error| CustodyError::io("narrative descriptor cannot be cloned", &error))?;
        let lock = acquire_lock(parent, &narrative_id, &operation_id)?;
        self.lock = Some(lock);
        Ok(json_object([
            ("protocolVersion", Value::from(PROTOCOL_VERSION)),
            ("narrativeId", Value::String(narrative_id)),
            ("operationId", Value::String(operation_id)),
            ("lockHeld", Value::Bool(true)),
        ]))
    }

    pub(crate) fn snapshot(&mut self, object: &Map<String, Value>) -> Result<Value> {
        if self.purpose == SessionPurpose::Measurement {
            return Err(CustodyError::input(
                "snapshot is not available in a measurement session",
            ));
        }
        require_keys(object, &["id", "op", "path"])?;
        let path = required_safe_relative_path(object, "path")?;
        if !self.read_only {
            self.require_lock()?;
        }
        let location = if self.read_only {
            self.storage_parent_for_read(&path)?
        } else {
            Some(self.storage_parent(&path, false)?)
        };
        let Some((parent, name)) = location else {
            return Ok(snapshot_value(&path, &Snapshot::absent()));
        };
        let snapshot = snapshot_entry(&parent, &name, MAX_READ_BYTES)?;
        Ok(snapshot_value(&path, &snapshot))
    }

    pub(super) fn read(&mut self, object: &Map<String, Value>) -> Result<Value> {
        if self.purpose == SessionPurpose::Measurement {
            return Err(CustodyError::input(
                "read is not available in a measurement session",
            ));
        }
        require_keys(object, &["id", "op", "path"])?;
        let path = required_safe_relative_path(object, "path")?;
        if !self.read_only {
            self.require_lock()?;
        }
        let location = if self.read_only {
            self.storage_parent_for_read(&path)?
        } else {
            Some(self.storage_parent(&path, false)?)
        };
        let Some((parent, name)) = location else {
            return Ok(snapshot_value(&path, &Snapshot::absent()));
        };
        let snapshot = snapshot_entry(&parent, &name, MAX_READ_BYTES)?;
        if !snapshot.exists && !self.read_only {
            return Err(CustodyError::storage(format!(
                "bounded read target is absent: {path}"
            )));
        }
        Ok(snapshot_value(&path, &snapshot))
    }

    pub(crate) fn write_immutable(&mut self, object: &Map<String, Value>) -> Result<Value> {
        if self.purpose == SessionPurpose::Measurement {
            return Err(CustodyError::input(
                "writeImmutable is not available in a measurement session",
            ));
        }
        require_keys(object, &["id", "op", "path", "bytesBase64"])?;
        if self.read_only {
            return Err(CustodyError::input(
                "writeImmutable is forbidden in a read-only custody session",
            ));
        }
        self.require_lock()?;
        let path = required_safe_relative_path(object, "path")?;
        validate_immutable_path(&path)?;
        let encoded = required_string(object, "bytesBase64")?;
        let bytes = decode_base64url_bounded(&encoded, MAX_WRITE_BYTES)?;
        let (parent, target_name) = self.storage_parent(&path, true)?;
        let existing = snapshot_entry(&parent, &target_name, MAX_WRITE_BYTES)?;
        if existing.exists {
            if existing.bytes == bytes {
                return Ok(json_object([
                    ("path", Value::String(path)),
                    ("reused", Value::Bool(true)),
                    ("snapshot", snapshot_value("", &existing)),
                ]));
            }
            return Err(CustodyError::storage(format!(
                "immutable target already contains different bytes: {path}"
            )));
        }

        self.temporary_counter = self.temporary_counter.saturating_add(1);
        let operation_id = self
            .operation_id
            .as_deref()
            .ok_or_else(|| CustodyError::input("operation is not initialized"))?;
        let temporary_name = format!(
            ".smc-trackb-immutable-{operation_id}-{}.tmp",
            self.temporary_counter
        );
        let mut temporary = open_exclusive_at(&parent, &temporary_name, 0o600)
            .map_err(|error| CustodyError::io("immutable temporary cannot be created", &error))?;
        let mut temporary_identity = file_identity(&temporary)
            .map_err(|error| CustodyError::io("immutable temporary identity failed", &error))?;
        let operation_result = (|| -> Result<Value> {
            temporary
                .write_all(&bytes)
                .map_err(|error| CustodyError::io("immutable temporary write failed", &error))?;
            temporary
                .sync_all()
                .map_err(|error| CustodyError::io("immutable temporary fsync failed", &error))?;
            temporary_identity = file_identity(&temporary).map_err(|error| {
                CustodyError::io("immutable temporary identity failed after write", &error)
            })?;
            if let Err(error) = link_at(&parent, &temporary_name, &target_name) {
                if error.raw_os_error() == Some(libc::EEXIST) {
                    let raced = snapshot_entry(&parent, &target_name, MAX_WRITE_BYTES)?;
                    if raced.exists && raced.bytes == bytes {
                        return Ok(json_object([
                            ("path", Value::String(path.clone())),
                            ("reused", Value::Bool(true)),
                            ("snapshot", snapshot_value("", &raced)),
                        ]));
                    }
                }
                return Err(CustodyError::io("immutable target publish failed", &error));
            }
            sync_directory(&parent)
                .map_err(|error| CustodyError::io("immutable parent fsync failed", &error))?;
            let published = snapshot_entry(&parent, &target_name, MAX_WRITE_BYTES)?;
            if !published.exists || published.bytes != bytes {
                return Err(CustodyError::storage(
                    "immutable target changed while publishing",
                ));
            }
            Ok(json_object([
                ("path", Value::String(path.clone())),
                ("reused", Value::Bool(false)),
                ("snapshot", snapshot_value("", &published)),
            ]))
        })();
        if let Err(error) = &operation_result {
            if let Err(cleanup_error) =
                cleanup_owned_temp(&parent, &temporary_name, temporary_identity)
            {
                return Err(CustodyError::storage(format!(
                    "immutable temporary cleanup failed after {}: {}",
                    error.code, cleanup_error.message
                )));
            }
            return Err(error.clone());
        }
        // A successful link leaves a private temporary directory entry.  It
        // is safe to unlink only after proving that it is still our inode.
        cleanup_owned_temp(&parent, &temporary_name, temporary_identity)?;
        operation_result
    }

    pub(super) fn close(&mut self, object: &Map<String, Value>) -> Result<Value> {
        require_keys(object, &["id", "op"])?;
        if self.read_only {
            if self.lock.is_some() {
                return Err(CustodyError::storage(
                    "read-only custody session cannot release a lock",
                ));
            }
            self.narrative_id = None;
            self.operation_id = None;
            self.narrative_dir = None;
            self.narrative_fd = None;
            self.artifact_directories = None;
            return Ok(json_object([("closed", Value::Bool(true))]));
        }
        if self.lock.is_none() {
            return Ok(json_object([("closed", Value::Bool(true))]));
        }
        let lock = self.lock.take().expect("checked above");
        release_lock(lock)?;
        self.narrative_id = None;
        self.operation_id = None;
        self.narrative_dir = None;
        self.narrative_fd = None;
        self.artifact_directories = None;
        Ok(json_object([("closed", Value::Bool(true))]))
    }
}
