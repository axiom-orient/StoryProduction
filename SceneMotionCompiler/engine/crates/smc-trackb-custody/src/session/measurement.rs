use std::io::Write;

use serde_json::{Map, Value};

use super::{MeasurementArtifactRead, Session};
use crate::artifacts::path::{
    measurement_storage_parent, read_project_artifact_at, revalidate_project_directory_edges,
};
use crate::artifacts::{artifact_read_value, snapshot_entry, snapshot_value};
use crate::error::{CustodyError, Result};
use crate::filesystem::admission::{require_keys, required_safe_relative_path, required_string};
use crate::filesystem::descriptors::{file_identity, link_at, open_exclusive_at, sync_directory};
use crate::filesystem::publication::cleanup_owned_temp;
#[cfg(test)]
use crate::filesystem::publication::MeasurementPublishTestFault;
use crate::protocol::{decode_base64url_bounded, json_object};
use crate::startup::SessionPurpose;

const MEASUREMENT_ARTIFACT_SEQUENCE: [&str; 3] = ["narrativeAuthority", "html", "provenance"];
const MAX_MEASUREMENT_AUTHORITY_BYTES: usize = 8 * 1024 * 1024;
const MAX_MEASUREMENT_HTML_BYTES: usize = 16 * 1024 * 1024;
const MAX_MEASUREMENT_PROVENANCE_BYTES: usize = 1024 * 1024;
const MAX_MEASUREMENT_RECEIPT_BYTES: usize = 64 * 1024;

impl Session {
    pub(crate) fn read_measurement_artifact(
        &mut self,
        object: &Map<String, Value>,
    ) -> Result<Value> {
        if self.purpose != SessionPurpose::Measurement {
            return Err(CustodyError::input(
                "readMeasurementArtifact is only available in a measurement session",
            ));
        }
        require_keys(object, &["id", "op", "kind", "path"])?;
        let kind = required_string(object, "kind")?;
        let maximum_bytes = match kind.as_str() {
            "narrativeAuthority" => MAX_MEASUREMENT_AUTHORITY_BYTES,
            "html" => MAX_MEASUREMENT_HTML_BYTES,
            "provenance" => MAX_MEASUREMENT_PROVENANCE_BYTES,
            _ => {
                return Err(CustodyError::input(format!(
                    "unsupported measurement artifact kind: {kind}"
                )))
            }
        };
        if self.measurement_artifact_reads.contains_key(&kind) {
            return Err(CustodyError::input(format!(
                "measurement artifact kind may be read only once per session: {kind}"
            )));
        }
        let expected_kind = MEASUREMENT_ARTIFACT_SEQUENCE
            .get(self.measurement_artifact_sequence_index)
            .copied()
            .ok_or_else(|| {
                CustodyError::input("measurement artifact sequence is already complete")
            })?;
        if kind != expected_kind {
            return Err(CustodyError::input(format!(
                "measurement artifact kind must be read in fixed order; expected {expected_kind}, got {kind}"
            )));
        }
        let path = required_safe_relative_path(object, "path")?;
        let project_fd = self
            .project_fd
            .as_ref()
            .ok_or_else(|| CustodyError::input("project root is not initialized"))?;
        let snapshot = read_project_artifact_at(project_fd, &path, maximum_bytes)?;
        let result = artifact_read_value(&kind, &path, &snapshot);
        self.measurement_artifact_reads.insert(
            kind,
            MeasurementArtifactRead {
                exists: snapshot.exists,
            },
        );
        self.measurement_artifact_sequence_index += 1;
        Ok(result)
    }

    pub(super) fn publish_measurement_receipt(
        &mut self,
        object: &Map<String, Value>,
    ) -> Result<Value> {
        #[cfg(test)]
        {
            self.publish_measurement_receipt_inner(object, MeasurementPublishTestFault::None)
        }
        #[cfg(not(test))]
        self.publish_measurement_receipt_inner(object)
    }

    #[cfg(test)]
    pub(crate) fn publish_measurement_receipt_with_test_fault(
        &mut self,
        object: &Map<String, Value>,
        fault: MeasurementPublishTestFault,
    ) -> Result<Value> {
        self.publish_measurement_receipt_inner(object, fault)
    }

    #[cfg(test)]
    fn publish_measurement_receipt_inner(
        &mut self,
        object: &Map<String, Value>,
        fault: MeasurementPublishTestFault,
    ) -> Result<Value> {
        self.publish_measurement_receipt_body(object, Some(fault))
    }

    #[cfg(not(test))]
    fn publish_measurement_receipt_inner(&mut self, object: &Map<String, Value>) -> Result<Value> {
        self.publish_measurement_receipt_body(object)
    }

    fn publish_measurement_receipt_body(
        &mut self,
        object: &Map<String, Value>,
        #[cfg(test)] fault: Option<MeasurementPublishTestFault>,
    ) -> Result<Value> {
        if self.purpose != SessionPurpose::Measurement {
            return Err(CustodyError::input(
                "publishMeasurementReceipt is only available in a measurement session",
            ));
        }
        require_keys(object, &["id", "op", "path", "bytesBase64"])?;
        if self.measurement_artifact_sequence_index != MEASUREMENT_ARTIFACT_SEQUENCE.len()
            || self.measurement_artifact_reads.len() != MEASUREMENT_ARTIFACT_SEQUENCE.len()
            || MEASUREMENT_ARTIFACT_SEQUENCE.iter().any(|kind| {
                self.measurement_artifact_reads
                    .get(*kind)
                    .is_none_or(|read| !read.exists)
            })
        {
            return Err(CustodyError::input(
                "publishMeasurementReceipt requires all three measurement artifacts to be read and present first",
            ));
        }
        let path = required_safe_relative_path(object, "path")?;
        let encoded = required_string(object, "bytesBase64")?;
        let bytes = decode_base64url_bounded(&encoded, MAX_MEASUREMENT_RECEIPT_BYTES)?;
        let project_fd = self
            .project_fd
            .as_ref()
            .ok_or_else(|| CustodyError::input("project root is not initialized"))?;
        let (parent, target_name, edges) = measurement_storage_parent(project_fd, &path, true)?;
        let existing = snapshot_entry(&parent, &target_name, MAX_MEASUREMENT_RECEIPT_BYTES)?;
        if existing.exists {
            revalidate_project_directory_edges(&edges)?;
            if existing.bytes == bytes {
                return Ok(json_object([
                    ("path", Value::String(path)),
                    ("reused", Value::Bool(true)),
                    ("snapshot", snapshot_value("", &existing)),
                ]));
            }
            return Err(CustodyError::storage(format!(
                "measurement receipt already contains different bytes: {path}"
            )));
        }

        self.temporary_counter = self.temporary_counter.saturating_add(1);
        let temporary_name = format!(
            ".smc-trackb-measurement-{}-{}.tmp",
            std::process::id(),
            self.temporary_counter
        );
        let mut temporary =
            open_exclusive_at(&parent, &temporary_name, 0o600).map_err(|error| {
                CustodyError::io("measurement receipt temporary cannot be created", &error)
            })?;
        let mut temporary_identity = file_identity(&temporary).map_err(|error| {
            CustodyError::io("measurement receipt temporary identity failed", &error)
        })?;
        let operation_result = (|| -> Result<Value> {
            temporary.write_all(&bytes).map_err(|error| {
                CustodyError::io("measurement receipt temporary write failed", &error)
            })?;
            temporary.sync_all().map_err(|error| {
                CustodyError::io("measurement receipt temporary fsync failed", &error)
            })?;
            temporary_identity = file_identity(&temporary).map_err(|error| {
                CustodyError::io(
                    "measurement receipt temporary identity failed after write",
                    &error,
                )
            })?;
            match link_at(&parent, &temporary_name, &target_name) {
                Ok(()) => {}
                Err(error) if error.raw_os_error() == Some(libc::EEXIST) => {
                    let raced =
                        snapshot_entry(&parent, &target_name, MAX_MEASUREMENT_RECEIPT_BYTES)?;
                    revalidate_project_directory_edges(&edges)?;
                    if raced.exists && raced.bytes == bytes {
                        return Ok(json_object([
                            ("path", Value::String(path.clone())),
                            ("reused", Value::Bool(true)),
                            ("snapshot", snapshot_value("", &raced)),
                        ]));
                    }
                    return Err(CustodyError::storage(format!(
                        "measurement receipt already contains different bytes: {path}"
                    )));
                }
                Err(error) => {
                    return Err(CustodyError::io(
                        "measurement receipt publish failed",
                        &error,
                    ));
                }
            }
            #[cfg(test)]
            if matches!(fault, Some(MeasurementPublishTestFault::FailParentSync)) {
                return Err(CustodyError::storage(
                    "test seam injected measurement parent fsync failure",
                ));
            }
            sync_directory(&parent).map_err(|error| {
                CustodyError::io("measurement receipt parent fsync failed", &error)
            })?;
            revalidate_project_directory_edges(&edges)?;
            #[cfg(test)]
            if matches!(fault, Some(MeasurementPublishTestFault::FailBeforeReopen)) {
                return Err(CustodyError::storage(
                    "test seam injected measurement descriptor reopen failure",
                ));
            }
            let published = snapshot_entry(&parent, &target_name, MAX_MEASUREMENT_RECEIPT_BYTES)?;
            if !published.exists || published.bytes != bytes {
                return Err(CustodyError::storage(
                    "measurement receipt changed while publishing",
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
                    "measurement receipt temporary cleanup failed after {}: {}",
                    error.code, cleanup_error.message
                )));
            }
            return Err(error.clone());
        }
        cleanup_owned_temp(&parent, &temporary_name, temporary_identity)?;
        operation_result
    }
}
