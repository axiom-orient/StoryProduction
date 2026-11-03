//! MCP server metadata and stdio lifecycle.

use crate::roots::resolve_roots;
use crate::tools::StsMcp;
use rmcp::{
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool_handler,
    transport::stdio,
    ServerHandler, ServiceExt,
};
use sts_workflow::Engine;

const HUMAN_AUTHORITY_NOTICE: &str = "\
NarrativeCourier MCP: project ingress, read, export and mechanical-delivery surface. \
STS semantic gates (`decide`) and delivery review (`approve|reject`) are human-only; \
they are intentionally not exposed as tools and must be performed in the Tauri UI or \
via the NarrativeCourier CLI with the exact gate token shown for the reviewed artifact.";

#[tool_handler(router = self.tool_router)]
impl ServerHandler for StsMcp {
    fn get_info(&self) -> ServerInfo {
        let mut info = Implementation::new("narrative-courier-mcp", env!("CARGO_PKG_VERSION"));
        info.title = Some("NarrativeCourier STS + Loss-Controlled Delivery".into());
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(info)
            .with_instructions(HUMAN_AUTHORITY_NOTICE)
    }
}

/// Serve MCP over stdin/stdout until the peer disconnects.
pub async fn serve_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let roots = resolve_roots(&std::env::args().skip(1).collect::<Vec<_>>())?;
    let service = StsMcp::new(Engine::with_roots(roots.assets, roots.projects)?)
        .serve(stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}
