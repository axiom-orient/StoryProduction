//! Hash the executable workflow manifest's ordered file set.

use super::super::{resolve_regular_file_within_root, Engine};
use crate::model::workflow::{DomainIdentity, Manifest};
use narrative_courier_runtime::store::{sha256_bytes, sha256_file};
use narrative_courier_runtime::{Error, Result};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;

pub(super) fn domain_identity(engine: &Engine, manifest: &Manifest) -> Result<DomainIdentity> {
    let workflow = resolve_regular_file_within_root(&engine.skill_root(), "workflow.json")?;
    let workflow_bytes = fs::read(workflow)?;
    let current: Manifest = serde_json::from_slice(&workflow_bytes)?;
    if current != *manifest {
        return Err(Error::Contract(
            "workflow changed after manifest admission".into(),
        ));
    }
    let mut paths = BTreeSet::new();
    paths.insert("workflow.json");
    for node in &manifest.nodes {
        if let Some(path) = &node.system_prompt {
            paths.insert(path.as_str());
        }
        if let Some(path) = &node.task_prompt {
            paths.insert(path.as_str());
        }
        for path in &node.principles {
            paths.insert(path.as_str());
        }
        for token in &node.inputs {
            let raw = token.strip_prefix('?').unwrap_or(token);
            if raw == "config.yaml"
                || raw.starts_with('@')
                || raw.starts_with("input/")
                || raw.starts_with("output/")
            {
                continue;
            }
            match engine.resolve_asset_file(raw) {
                Ok(_) => {
                    paths.insert(raw);
                }
                Err(Error::Io(error))
                    if token.starts_with('?') && error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
    }
    let mut files = Vec::new();
    for relative in paths {
        let digest = if relative == "workflow.json" {
            sha256_bytes(&workflow_bytes)
        } else {
            sha256_file(&engine.resolve_asset_file(relative)?)?
        };
        files.push(json!({"path": relative, "sha256": digest}));
    }
    Ok(DomainIdentity {
        id: manifest.domain_id.clone(),
        version: manifest.domain_version.clone(),
        sha256: sha256_bytes(&serde_json::to_vec(&files)?),
    })
}
