use rmcp::{model::CallToolResult, ErrorData as McpError};

pub(super) async fn blocking_json<T, F>(f: F) -> Result<CallToolResult, McpError>
where
    T: serde::Serialize + Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    let outcome = tokio::task::spawn_blocking(f).await.map_err(|error| {
        McpError::internal_error(
            format!("worker join failed; operation outcome may be unknown: {error}"),
            None,
        )
    })?;
    let value = match outcome {
        Ok(value) => value,
        Err(error) => {
            return Ok(CallToolResult::error(vec![
                rmcp::model::ContentBlock::text(error),
            ]))
        }
    };
    let text = serde_json::to_string_pretty(&value).map_err(|e| {
        McpError::internal_error(
            format!("result serialization failed; effects may already have been applied: {e}"),
            None,
        )
    })?;
    Ok(CallToolResult::success(vec![
        rmcp::model::ContentBlock::text(text),
    ]))
}
