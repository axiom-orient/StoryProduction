use serde_json::{Map, Value};

use super::{ProjectArtifactRead, Session, PROJECT_ARTIFACT_SEQUENCE};
use crate::artifacts::artifact_read_value;
use crate::artifacts::path::read_project_artifact_at;
use crate::artifacts::MAX_READ_BYTES;
use crate::error::{CustodyError, Result};
use crate::filesystem::admission::{require_keys, required_safe_relative_path, required_string};
use crate::protocol::is_sha256_hash;
use crate::startup::SessionPurpose;

impl Session {
    pub(super) fn read_project_artifact(&mut self, object: &Map<String, Value>) -> Result<Value> {
        if self.purpose != SessionPurpose::ProjectIngress {
            return Err(CustodyError::input(
                "readProjectArtifact is only available in a project-ingress session",
            ));
        }
        let base_keys = ["id", "op", "kind", "path"];
        let kind = required_string(object, "kind")?;
        let scene_graph = match kind.as_str() {
            "narrativeAuthority" | "projectionReceipt" | "assetHandoff" | "html" | "provenance"
            | "evidenceReceipt" => {
                require_keys(object, &base_keys)?;
                false
            }
            "sceneGraph" => {
                require_keys(
                    object,
                    &["id", "op", "kind", "path", "projectionReceiptHash"],
                )?;
                true
            }
            _ => {
                return Err(CustodyError::input(format!(
                    "unsupported project artifact kind: {kind}"
                )))
            }
        };
        if self.narrative_id.is_some() {
            return Err(CustodyError::input(
                "readProjectArtifact is only available before narrative initialize",
            ));
        }
        if self.project_artifact_reads.contains_key(&kind) {
            return Err(CustodyError::input(format!(
                "project artifact kind may be read only once per session: {kind}"
            )));
        }
        let expected_kind = PROJECT_ARTIFACT_SEQUENCE
            .get(self.project_artifact_sequence_index)
            .copied()
            .ok_or_else(|| CustodyError::input("project artifact sequence is already complete"))?;
        if kind != expected_kind {
            return Err(CustodyError::input(format!(
                "project artifact kind must be read in fixed order; expected {expected_kind}, got {kind}"
            )));
        }
        let path = required_safe_relative_path(object, "path")?;
        if scene_graph {
            let expected_projection_hash = required_string(object, "projectionReceiptHash")?;
            if !is_sha256_hash(&expected_projection_hash) {
                return Err(CustodyError::input(
                    "projectionReceiptHash must be a sha256:<hex> hash",
                ));
            }
            let Some(cached) = self.project_artifact_reads.get("projectionReceipt") else {
                return Err(CustodyError::input(
                    "sceneGraph requires a prior projectionReceipt read",
                ));
            };
            if cached.sha256.as_deref() != Some(expected_projection_hash.as_str()) {
                return Err(CustodyError::input(
                    "projectionReceiptHash does not match the cached raw receipt hash",
                ));
            }
        }
        let project_fd = self
            .project_fd
            .as_ref()
            .ok_or_else(|| CustodyError::input("project root is not initialized"))?;
        let snapshot = read_project_artifact_at(project_fd, &path, MAX_READ_BYTES)?;
        let result = artifact_read_value(&kind, &path, &snapshot);
        self.project_artifact_reads.insert(
            kind,
            ProjectArtifactRead {
                exists: snapshot.exists,
                sha256: snapshot.sha256.clone(),
            },
        );
        self.project_artifact_sequence_index += 1;
        Ok(result)
    }
}
