use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use rmcp::ServiceExt;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService, stdio};
use serde_json::json;

use super::PromptGenServer;

pub const MCP_PROTOCOL_VERSION: &str = "2026-07-28";
pub const DEFAULT_MCP_PORT: u16 = 4174;
pub const DEFAULT_MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpHttpConfig {
    pub bind: SocketAddr,
    pub bearer_token: Option<String>,
    pub allowed_hosts: Vec<String>,
    pub max_body_bytes: usize,
}

impl Default for McpHttpConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_MCP_PORT),
            bearer_token: None,
            allowed_hosts: Vec::new(),
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
        }
    }
}

impl McpHttpConfig {
    pub(super) fn validate(&self) -> io::Result<()> {
        if self.max_body_bytes == 0 || self.max_body_bytes > MAX_BODY_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("MCP body limit must be within 1..={MAX_BODY_BYTES} bytes"),
            ));
        }
        if self.bearer_token.as_deref().is_some_and(str::is_empty) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "MCP bearer token must not be empty",
            ));
        }
        if self.allowed_hosts.iter().any(|host| host.trim().is_empty()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "MCP allowed hosts must not contain empty values",
            ));
        }
        if !self.bind.ip().is_loopback() {
            if self.bearer_token.is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "non-loopback MCP binds require a bearer token",
                ));
            }
            if self.allowed_hosts.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "non-loopback MCP binds require at least one --allowed-host",
                ));
            }
        }
        Ok(())
    }
}

/// Build the stateless MCP HTTP router without binding a socket.
///
/// Every POST is handled as a self-contained request. No MCP session store is
/// installed, and the server advertises only protocol 2026-07-28.
pub fn build_router(config: McpHttpConfig) -> io::Result<Router> {
    config.validate()?;

    let mut server_config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_stateless_protocol_metadata_required(true)
        .with_max_request_body_bytes(config.max_body_bytes);
    if !config.allowed_hosts.is_empty() {
        server_config = server_config.with_allowed_hosts(config.allowed_hosts.clone());
    }

    let service = StreamableHttpService::new(
        || Ok(PromptGenServer),
        Arc::new(LocalSessionManager::default()),
        server_config,
    );
    Ok(Router::new()
        .route("/health", get(health))
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(
            config.bearer_token,
            require_bearer,
        )))
}

/// Run the stateless Streamable HTTP MCP endpoint.
pub async fn serve_http(config: McpHttpConfig) -> io::Result<()> {
    config.validate()?;
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let router = build_router(config)?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(io::Error::other)
}

/// Run the standard MCP stdio transport.
pub async fn serve_stdio() -> Result<(), String> {
    let running = PromptGenServer
        .serve(stdio())
        .await
        .map_err(|error| error.to_string())?;
    running.waiting().await.map_err(|error| error.to_string())?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        else {
            let _ = tokio::signal::ctrl_c().await;
            return;
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = signal.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "service": "promptgen-mcp",
        "protocol_version": MCP_PROTOCOL_VERSION,
        "stateless": true,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn require_bearer(
    State(expected): State<Option<String>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let Some(expected) = expected else {
        return next.run(request).await;
    };
    let authorized = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == format!("Bearer {expected}"));
    if authorized {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            [(axum::http::header::WWW_AUTHENTICATE, "Bearer")],
            "MCP bearer authentication required",
        )
            .into_response()
    }
}
