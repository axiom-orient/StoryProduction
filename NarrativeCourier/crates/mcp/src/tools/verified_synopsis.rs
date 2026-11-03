use crate::params::ReviewedSynopsisParams;
use rmcp::{model::CallToolResult, ErrorData as McpError};
use sts_workflow::Engine;

use super::support::blocking_json;

pub(super) async fn import(
    engine: Engine,
    params: ReviewedSynopsisParams,
) -> Result<CallToolResult, McpError> {
    blocking_json(move || {
        engine
            .create_project_from_reviewed_synopsis(
                &sts_workflow::CreateReviewedSynopsisProjectRequest {
                    project_name: params.project.unwrap_or_default(),
                    title: params.title,
                    handoff_root: params.handoff_root,
                    handoff_path: params.handoff_path,
                    target_duration: params.target_duration,
                    genre: vec![],
                    tone: String::new(),
                    theme_direction: String::new(),
                },
            )
            .map_err(|e| e.to_string())
    })
    .await
}
