use crate::params::{ProjectParams, RunDeliveryParams};
use narrative_courier_contracts::{validate_delivery_request, DeliveryRequest};
use narrative_courier_delivery::DeliveryEngine;
use rmcp::{model::CallToolResult, ErrorData as McpError};
use sts_workflow::Engine;

use super::support::blocking_json;

pub(super) async fn context(
    engine: Engine,
    params: ProjectParams,
) -> Result<CallToolResult, McpError> {
    blocking_json(move || {
        let package = engine
            .narrative_package(&params.project)
            .map_err(|e| e.to_string())?;
        let dir = engine
            .project_dir(&params.project)
            .map_err(|e| e.to_string())?;
        DeliveryEngine::context(&dir, &package).map_err(|e| e.to_string())
    })
    .await
}

pub(super) async fn list(
    engine: Engine,
    params: ProjectParams,
) -> Result<CallToolResult, McpError> {
    blocking_json(move || {
        let package = engine
            .narrative_package(&params.project)
            .map_err(|e| e.to_string())?;
        let dir = engine
            .project_dir(&params.project)
            .map_err(|e| e.to_string())?;
        DeliveryEngine::list(&dir, &package).map_err(|e| e.to_string())
    })
    .await
}

pub(super) async fn run(
    engine: Engine,
    params: RunDeliveryParams,
) -> Result<CallToolResult, McpError> {
    let request: DeliveryRequest = serde_json::from_value(params.request).map_err(|e| {
        McpError::invalid_params(format!("request is not a valid DeliveryRequest: {e}"), None)
    })?;
    validate_delivery_request(&request)
        .map_err(|error| McpError::invalid_params(error.to_string(), None))?;
    blocking_json(move || {
        let package = engine
            .narrative_package(&params.project)
            .map_err(|e| e.to_string())?;
        let dir = engine
            .project_dir(&params.project)
            .map_err(|e| e.to_string())?;
        DeliveryEngine::run(&dir, &package, request).map_err(|e| e.to_string())
    })
    .await
}

pub(super) async fn cancel(
    engine: Engine,
    params: ProjectParams,
) -> Result<CallToolResult, McpError> {
    blocking_json(move || {
        let dir = engine
            .project_dir(&params.project)
            .map_err(|e| e.to_string())?;
        DeliveryEngine::cancel(&dir).map_err(|e| e.to_string())
    })
    .await
}
