//! Verified craft-pack citations attached to generation plans.

use super::CraftPack;

impl CraftPack {
    /// Planning reference attached additively to generation plans: real
    /// pack identity plus the verified agent nodes with their task prompt
    /// paths, so a plan cites verifiable craft-pack content instead of
    /// invented instructions.
    pub fn plan_reference(&self) -> crate::generation::CraftpackPlanRef {
        crate::generation::CraftpackPlanRef {
            pack_id: self.manifest.pack_id.clone(),
            version: self.manifest.version.clone(),
            workflow_node_count: self.workflow.nodes.len(),
            workflow_manifest_sha256: self.manifest.workflow_manifest_sha256.clone(),
            nodes: self.workflow.agent_nodes(),
        }
    }
}
