//! Request dispatcher and state owner for a single custody process.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::{CustodyError, Result};
use crate::filesystem::admission::{validate_opaque_id, validate_project_root};
use crate::lock_release::LockHandle;
use crate::startup::{Config, SessionPurpose};

mod lifecycle;
mod measurement;
mod pointer;
mod project_ingress;
mod storage;
#[cfg(test)]
mod tests;

pub(super) const PROJECT_ARTIFACT_SEQUENCE: [&str; 7] = [
    "narrativeAuthority",
    "projectionReceipt",
    "assetHandoff",
    "html",
    "provenance",
    "evidenceReceipt",
    "sceneGraph",
];

#[derive(Debug, Clone)]
struct ProjectArtifactRead {
    exists: bool,
    sha256: Option<String>,
}

#[derive(Debug, Clone)]
struct MeasurementArtifactRead {
    exists: bool,
}

#[derive(Debug)]
struct ArtifactDirectories {
    authority: File,
    candidates: File,
    delivery_sets: File,
    approvals: File,
}

#[derive(Debug)]
pub(super) struct Session {
    config: Config,
    purpose: SessionPurpose,
    /// Retain the admitted root descriptor so path replacement cannot redirect
    /// later storage effects through the startup pathname.
    project_fd: Option<File>,
    read_only: bool,
    narrative_id: Option<String>,
    operation_id: Option<String>,
    narrative_dir: Option<PathBuf>,
    narrative_fd: Option<File>,
    artifact_directories: Option<ArtifactDirectories>,
    project_artifact_reads: BTreeMap<String, ProjectArtifactRead>,
    project_artifact_sequence_index: usize,
    measurement_artifact_reads: BTreeMap<String, MeasurementArtifactRead>,
    measurement_artifact_sequence_index: usize,
    lock: Option<LockHandle>,
    temporary_counter: u64,
}

impl Session {
    pub(super) fn new(config: Config) -> Result<Self> {
        let project_fd = validate_project_root(&config.project_root)?;
        let purpose = config.purpose;
        Ok(Self {
            config,
            purpose,
            project_fd: Some(project_fd),
            read_only: false,
            narrative_id: None,
            operation_id: None,
            narrative_dir: None,
            narrative_fd: None,
            artifact_directories: None,
            project_artifact_reads: BTreeMap::new(),
            project_artifact_sequence_index: 0,
            measurement_artifact_reads: BTreeMap::new(),
            measurement_artifact_sequence_index: 0,
            lock: None,
            temporary_counter: 0,
        })
    }

    pub(super) fn handle_request(&mut self, request: &Value) -> Result<Value> {
        let object = request
            .as_object()
            .ok_or_else(|| CustodyError::input("request must be an object"))?;
        let id = object
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| CustodyError::input("request id must be a string"))?;
        validate_opaque_id(id, "request id")?;
        let operation = object
            .get("op")
            .and_then(Value::as_str)
            .ok_or_else(|| CustodyError::input("request op must be a string"))?;
        match operation {
            "readProjectArtifact" => self.read_project_artifact(object),
            "readMeasurementArtifact" => self.read_measurement_artifact(object),
            "initialize" => self.initialize(object),
            "acquire" => self.acquire(object),
            "snapshot" => self.snapshot(object),
            "read" => self.read(object),
            "writeImmutable" => self.write_immutable(object),
            "publishPointer" => self.publish_pointer(object),
            "publishMeasurementReceipt" => self.publish_measurement_receipt(object),
            "close" => self.close(object),
            unknown => Err(CustodyError::input(format!(
                "unsupported custody operation: {unknown}"
            ))),
        }
    }
}
