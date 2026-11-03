//! Verify-then-parse loading of a complete craft pack.

use super::{CraftPack, CraftPackManifest, WorkflowManifest, CRAFT_PACK_API_VERSION};
use crate::error::{Error, Result};
use crate::integrity::{read_verified_text, verify_sums};
use std::path::Path;

/// Loads and verifies the craft pack under `dir`, failing closed before any
/// parse when integrity does not hold.
///
/// # Errors
/// Returns integrity errors from [`verify_sums`], [`Error::InvalidInput`]
/// for cross-check violations (api version, manifest digest agreement,
/// prompt asset count, duplicate node ids), and [`Error::Json`] /
/// [`Error::Io`] for unreadable or malformed pack files.
pub fn load_craft_pack(dir: &Path) -> Result<CraftPack> {
    let directory = dir.canonicalize()?;
    let dir = directory.as_path();
    let files = verify_sums(dir, &dir.join("SHA256SUMS"))?;
    let manifest_raw = read_verified_text(dir, &files, "craft-pack.json")?;
    let manifest: CraftPackManifest = serde_json::from_str(&manifest_raw)?;
    if manifest.api_version != CRAFT_PACK_API_VERSION {
        return Err(Error::InvalidInput(format!(
            "craft-pack api_version '{}' must be '{CRAFT_PACK_API_VERSION}'",
            manifest.api_version
        )));
    }

    let recorded = files.get(&manifest.workflow_manifest).ok_or_else(|| {
        Error::InvalidInput(format!(
            "craft-pack workflow manifest '{}' is not covered by SHA256SUMS",
            manifest.workflow_manifest
        ))
    })?;
    if recorded.as_str() != manifest.workflow_manifest_sha256 {
        return Err(Error::Corrupt(format!(
            "craft-pack.json workflow_manifest_sha256 ({}) disagrees with SHA256SUMS ({recorded}) for '{}'",
            manifest.workflow_manifest_sha256, manifest.workflow_manifest
        )));
    }

    let prompt_assets = files
        .keys()
        .filter(|path| path.starts_with("prompts/") && path.as_str() != manifest.workflow_manifest)
        .count() as u64;
    if prompt_assets != manifest.prompt_asset_count {
        return Err(Error::Corrupt(format!(
            "craft-pack prompt_asset_count {} disagrees with {} prompt files verified under prompts/",
            manifest.prompt_asset_count, prompt_assets
        )));
    }

    let workflow_raw = read_verified_text(dir, &files, &manifest.workflow_manifest)?;
    let workflow: WorkflowManifest = serde_json::from_str(&workflow_raw)?;
    if workflow.schema_version != 1 {
        return Err(Error::InvalidInput(
            "unsupported craft workflow schema_version".into(),
        ));
    }
    if workflow.nodes.is_empty() {
        return Err(Error::InvalidInput(
            "workflow manifest must contain at least one node".into(),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for node in &workflow.nodes {
        crate::domain::validation::validate_id("craft workflow node id", node.id())?;
        if !seen.insert(node.id()) {
            return Err(Error::InvalidInput(format!(
                "workflow manifest node id '{}' appears more than once",
                node.id()
            )));
        }
    }

    for node in &workflow.nodes {
        if let super::WorkflowNode::Agent(agent) = node {
            for prompt in [&agent.system_prompt, &agent.task_prompt] {
                if !files.contains_key(prompt) {
                    return Err(Error::Corrupt(format!(
                        "agent '{}' cites unverified prompt '{prompt}'",
                        agent.id
                    )));
                }
            }
        }
    }
    Ok(CraftPack {
        directory,
        manifest,
        workflow,
        files,
    })
}
