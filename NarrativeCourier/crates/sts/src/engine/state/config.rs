//! Project config loading and source-input path resolution.

use super::super::{
    read_reference_bytes, resolve_regular_file_within_root, sanitize_filename,
    validate_reference_type, Engine, SOURCE_FORMATS,
};
use crate::model::project::{ProjectConfig, ReviewedSynopsisBinding};
use narrative_courier_contracts::{
    validate_reviewed_synopsis_handoff_identity, ReviewedSynopsisHandoff,
};
use narrative_courier_runtime::store::sha256_bytes;
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

impl Engine {
    pub fn load_config(&self, project: &Path) -> Result<ProjectConfig> {
        let config: ProjectConfig = serde_yaml::from_slice(&fs::read(
            resolve_regular_file_within_root(project, "config.yaml")?,
        )?)?;
        if config.title.trim().is_empty()
            || !SOURCE_FORMATS.contains(&config.source_format.as_str())
            || config.target_duration == 0
            || config.synopsis_file.is_empty()
        {
            return Err(Error::Contract("config.yaml fails contract".into()));
        }
        if let Some(binding) = &config.reviewed_synopsis {
            let synopsis = fs::read(self.resolve_synopsis(project, &config)?)?;
            if sha256_bytes(&synopsis) != binding.synopsis_sha256
                || config.title != binding.title.trim()
            {
                return Err(Error::Contract(
                    "reviewed synopsis/config differs from its approval binding".into(),
                ));
            }
            let mut receipt_found = false;
            for reference in self.resolve_references(project, &config)? {
                let bytes = read_reference_bytes(&reference)?;
                let digest = sha256_bytes(&bytes);
                if digest == binding.handoff_sha256 {
                    let handoff: ReviewedSynopsisHandoff = serde_json::from_slice(&bytes)?;
                    validate_reviewed_synopsis_handoff_identity(&handoff)?;
                    if ReviewedSynopsisBinding::from_verified_handoff(&handoff, digest) != *binding
                    {
                        return Err(Error::Contract(
                            "reviewed synopsis receipt does not match its stored binding".into(),
                        ));
                    }
                    receipt_found = true;
                    break;
                }
            }
            if !receipt_found {
                return Err(Error::Contract(
                    "reviewed synopsis approval reference is missing or changed".into(),
                ));
            }
        }
        Ok(config)
    }

    pub(in crate::engine) fn resolve_synopsis(
        &self,
        project: &Path,
        config: &ProjectConfig,
    ) -> Result<PathBuf> {
        resolve_regular_file_within_root(project, &format!("input/{}", config.synopsis_file))
    }

    pub(in crate::engine) fn resolve_references(
        &self,
        project: &Path,
        config: &ProjectConfig,
    ) -> Result<Vec<PathBuf>> {
        let mut resolved = Vec::new();
        let mut names = BTreeSet::new();
        for name in &config.reference_files {
            let clean = sanitize_filename(name)?;
            if clean != *name {
                return Err(Error::Contract(format!(
                    "unsafe reference filename: {name}"
                )));
            }
            validate_reference_type(name)?;
            if !names.insert(name) {
                return Err(Error::Contract(format!(
                    "duplicate reference filename: {name}"
                )));
            }
            let path =
                resolve_regular_file_within_root(project, &format!("input/references/{name}"))?;
            resolved.push(path);
        }
        Ok(resolved)
    }
}
