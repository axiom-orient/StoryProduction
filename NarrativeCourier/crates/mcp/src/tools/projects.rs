use crate::params::ProjectParams;
use rmcp::{model::CallToolResult, ErrorData as McpError};
use sts_workflow::Engine;

use super::support::blocking_json;

pub(super) async fn list(engine: Engine) -> Result<CallToolResult, McpError> {
    blocking_json(move || engine.list_projects().map_err(|e| e.to_string())).await
}

pub(super) async fn status(
    engine: Engine,
    params: ProjectParams,
) -> Result<CallToolResult, McpError> {
    blocking_json(move || {
        engine
            .project_view(&params.project)
            .map_err(|e| e.to_string())
    })
    .await
}

pub(super) async fn doctor(
    engine: Engine,
    params: ProjectParams,
) -> Result<CallToolResult, McpError> {
    blocking_json(move || engine.doctor(&params.project).map_err(|e| e.to_string())).await
}
