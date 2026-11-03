//! Raw-byte fingerprints for the project config, synopsis, and references.

use super::super::{resolve_regular_file_within_root, Engine};
use crate::model::workflow::{FileFingerprint, NodeStatus, SourceSnapshot, State};
use narrative_courier_runtime::store::sha256_file;
use narrative_courier_runtime::{Error, Result};
use std::fs;
use std::path::Path;

impl Engine {
    pub(in crate::engine) fn build_source_snapshot(
        &self,
        project: &Path,
    ) -> Result<SourceSnapshot> {
        let project_root = if project.is_relative() {
            project.to_path_buf()
        } else {
            fs::canonicalize(project)?
        };
        let project = project_root.as_path();
        let config = self.load_config(project)?;
        let config_path = resolve_regular_file_within_root(project, "config.yaml")?;
        let synopsis = self.resolve_synopsis(project, &config)?;
        let references = self.resolve_references(project, &config)?;
        let fingerprint = |path: &Path| -> Result<FileFingerprint> {
            let relative = path.strip_prefix(project).map_err(|_| {
                Error::Contract("source input escapes the canonical project root".into())
            })?;
            Ok(FileFingerprint {
                path: relative.to_string_lossy().replace('\\', "/"),
                sha256: sha256_file(path)?,
            })
        };
        Ok(SourceSnapshot {
            config: fingerprint(&config_path)?,
            synopsis: fingerprint(&synopsis)?,
            references: references
                .iter()
                .map(|path| fingerprint(path))
                .collect::<Result<Vec<_>>>()?,
        })
    }

    pub(in crate::engine) fn verify_source_snapshot(
        &self,
        project: &Path,
        state: &State,
    ) -> Result<()> {
        if state.source_snapshot.is_none()
            && state
                .nodes
                .get("step_00")
                .is_some_and(|record| record.status == NodeStatus::Completed || record.version > 0)
        {
            return Err(Error::Contract(
                "source snapshot is missing after STEP 0".into(),
            ));
        }
        if let Some(expected) = &state.source_snapshot {
            if &self.build_source_snapshot(project)? != expected {
                return Err(Error::Contract("source/config/references changed after STEP 0; mixed-generation execution is blocked".into()));
            }
        }
        Ok(())
    }
}
