use promptgen_mcp::{serve_http as serve_mcp_http, serve_stdio as serve_mcp_stdio};
use promptgen_web::serve as serve_web;

use super::super::error::{CliError, EXIT_OK};
use super::super::options::{McpOptions, McpTransport, ServeOptions};

pub(crate) fn serve_ui(options: ServeOptions) -> Result<u8, CliError> {
    serve_web(options.config).map_err(|error| CliError::execution(error.to_string()))?;
    Ok(EXIT_OK)
}

pub(crate) fn serve_mcp(options: McpOptions) -> Result<u8, CliError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| CliError::execution(format!("create MCP runtime: {error}")))?;
    match options.transport {
        McpTransport::Stdio => runtime
            .block_on(serve_mcp_stdio())
            .map_err(CliError::execution)?,
        McpTransport::Http => runtime
            .block_on(serve_mcp_http(options.config))
            .map_err(|error| CliError::execution(error.to_string()))?,
    }
    Ok(EXIT_OK)
}
