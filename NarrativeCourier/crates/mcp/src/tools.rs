//! MCP tools delegating to the canonical STS and delivery engines.

mod delivery;
mod projects;
mod screenplay;
mod support;
mod verified_synopsis;

use crate::params::{
    ExportScreenplayParams, ImportScreenplayParams, ProjectParams, ReviewedSynopsisParams,
    RunDeliveryParams,
};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::CallToolResult,
    tool, tool_router, ErrorData as McpError,
};
use sts_workflow::Engine;

/// MCP tool surface over one NarrativeCourier home (assets root + projects root).
#[derive(Debug, Clone)]
pub struct StsMcp {
    engine: Engine,
    pub(super) tool_router: ToolRouter<Self>,
}

#[tool_router]
impl StsMcp {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine,
            tool_router: Self::tool_router(),
        }
    }

    /// List all projects in the configured projects root.
    #[tool(
        description = "List NarrativeCourier projects with name, title, workflow status and last update time.",
        annotations(
            title = "List projects",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_projects(&self) -> Result<CallToolResult, McpError> {
        projects::list(self.engine.clone()).await
    }

    /// Print the live workflow view of one project (stage/node states).
    #[tool(
        description = "Get the current STS workflow state of one project, including stages, node statuses and whether the project is awaiting a human decision.",
        annotations(
            title = "Get project status",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn project_status(
        &self,
        Parameters(params): Parameters<ProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        projects::status(self.engine.clone(), params).await
    }

    /// Run integrity diagnostics for one project.
    #[tool(
        description = "Run NarrativeCourier doctor diagnostics for one project: state integrity, durable output hashes and released NarrativePackage freshness.",
        annotations(
            title = "Doctor project",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn doctor(
        &self,
        Parameters(params): Parameters<ProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        projects::doctor(self.engine.clone(), params).await
    }

    /// Get the delivery context of a released narrative.
    #[tool(
        description = "Load the released NarrativePackage delivery context (canonical scene refs, source snapshot, package hash) required to author a DeliveryRequest. Requires the project to be Completed by final human release.",
        annotations(
            title = "Get delivery context",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn delivery_context(
        &self,
        Parameters(params): Parameters<ProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        delivery::context(self.engine.clone(), params).await
    }

    /// List committed deliveries for the current released narrative.
    #[tool(
        description = "List delivery records already committed for the current released NarrativePackage of one project.",
        annotations(
            title = "List deliveries",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_deliveries(
        &self,
        Parameters(params): Parameters<ProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        delivery::list(self.engine.clone(), params).await
    }

    /// Execute a deterministic delivery request (HTML | JSON | PNG | WEBP | PDF | MP4).
    ///
    /// Additive and idempotent: writes new artifacts under `visual/`, never
    /// reinterprets the narrative, and rejects unsupported semantics explicitly.
    #[tool(
        description = "Run a deterministic delivery for a Completed project: pass a full DeliveryRequest document and receive the persisted DeliverySet. Outputs land under visual/deliveries; visual results stay NEEDS_REVIEW until Human review. Unsupported options fail explicitly. A failed call does not imply that earlier effects were rolled back.",
        annotations(
            title = "Run delivery",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn run_delivery(
        &self,
        Parameters(params): Parameters<RunDeliveryParams>,
    ) -> Result<CallToolResult, McpError> {
        delivery::run(self.engine.clone(), params).await
    }

    /// Ingest an external screenplay (.fountain/.fdx) as reference material.
    #[tool(
        description = "Create a NarrativeCourier project from an external Fountain or Final Draft .fdx screenplay plus a human-approved synopsis. The screenplay is normalized deterministically into project references (format parsing only; no semantic narrative artifacts are produced). Requires the synopsis to come from a human.",
        annotations(
            title = "Import screenplay",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn import_screenplay(
        &self,
        Parameters(params): Parameters<ImportScreenplayParams>,
    ) -> Result<CallToolResult, McpError> {
        screenplay::import(self.engine.clone(), params).await
    }

    /// Verify and ingest a geultu-reviewed synopsis as the semantic source.
    #[tool(
        description = "Verify a GeultuReviewedSynopsisHandoff v1, including raw synopsis hash and human approval binding, then create a NarrativeCourier STS project. Handoff paths are relative to the declared root and no screenplay is accepted at this boundary.",
        annotations(
            title = "Import verified synopsis",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn import_verified_synopsis(
        &self,
        Parameters(params): Parameters<ReviewedSynopsisParams>,
    ) -> Result<CallToolResult, McpError> {
        verified_synopsis::import(self.engine.clone(), params).await
    }

    /// Export the Completed canonical screenplay within the project output directory.
    #[tool(
        description = "Export the COMPLETED canonical screenplay of a finished project. Formats: fountain | fdx | docx. When output_path is omitted the artifact lands at output/final_screenplay.<ext>; a bare filename with the matching format extension may be given and always stays inside the project output directory.",
        annotations(
            title = "Export screenplay",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn export_screenplay(
        &self,
        Parameters(params): Parameters<ExportScreenplayParams>,
    ) -> Result<CallToolResult, McpError> {
        screenplay::export(self.engine.clone(), params).await
    }

    /// Cancel an in-flight delivery run. Committed DeliverySets are never touched.
    #[tool(
        description = "Cancel the active delivery run of one project. Only uncommitted work is discarded; already committed DeliverySets remain canonical.",
        annotations(
            title = "Cancel delivery",
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn cancel_delivery(
        &self,
        Parameters(params): Parameters<ProjectParams>,
    ) -> Result<CallToolResult, McpError> {
        delivery::cancel(self.engine.clone(), params).await
    }
}
