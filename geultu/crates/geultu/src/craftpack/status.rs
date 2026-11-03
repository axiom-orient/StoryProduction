//! Read-only craft-pack identity and integrity status projection.

use super::CraftPack;
use serde_json::json;

impl CraftPack {
    /// Read-only status report surfacing pack identity, integrity state,
    /// and the drift-visible counts (node totals, prompt file digests).
    ///
    /// This describes the immutable loaded snapshot, not a new filesystem verification.
    pub fn status(&self) -> serde_json::Value {
        let files: Vec<_> = self
            .files
            .iter()
            .map(|(path, digest)| json!({"path": path, "sha256": digest}))
            .collect();
        json!({
            "pack_id": self.manifest.pack_id,
            "version": self.manifest.version,
            "api_version": self.manifest.api_version,
            "license": self.manifest.license,
            "redistribution_policy": self.manifest.redistribution_policy,
            "pack_dir": self.directory.display().to_string(),
            "integrity": {
                "verified": true,
                "checked_files": self.files.len(),
            },
            "prompt_asset_count": self.manifest.prompt_asset_count,
            "workflow": {
                "manifest_path": self.manifest.workflow_manifest,
                "manifest_sha256": self.manifest.workflow_manifest_sha256,
                "schema_version": self.workflow.schema_version,
                "node_count": self.workflow.nodes.len(),
                "agent_nodes": self.workflow.agent_count(),
                "human_gates": self.workflow.human_count(),
            },
            "files": files,
        })
    }
}
