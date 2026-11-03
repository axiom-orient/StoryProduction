#![forbid(unsafe_code)]

#[cfg(test)]
mod asset_contract;

mod http;
mod request;
mod routes;
mod security;
mod server;

#[cfg(test)]
use http::{HttpRequest, RequestError};
#[cfg(test)]
use security::{allowed_host, validate_output_name};
#[cfg(test)]
use server::{ConnectionGuard, ServerState, validate_config};
pub use server::{ShutdownSignal, resolve_catalog_dir, serve, serve_listener};
#[cfg(test)]
use std::net::{TcpListener, TcpStream};

use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
#[cfg(test)]
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(test)]
use http::find_subsequence;

const INDEX_HTML: &str = include_str!("assets/index.html");
const APP_CSS: &str = include_str!("assets/app.css");
const APP_JS: &str = include_str!("assets/app.js");
const DEFAULT_MAX_BODY_BYTES: usize = 12 * 1024 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_ACTIVE_CONNECTIONS: usize = 32;

#[derive(Clone, Debug)]
pub struct WebConfig {
    pub bind: SocketAddr,
    pub codex_binary: PathBuf,
    pub codex_home: PathBuf,
    pub output_dir: PathBuf,
    /// Directory holding `image_catalog.json` and `assets/`. Card PNGs are read from
    /// here at runtime; `None` resolves it once at startup with [`resolve_catalog_dir`].
    pub catalog_dir: Option<PathBuf>,
    pub max_body_bytes: usize,
    pub request_timeout: Duration,
    pub artifact_wait_timeout: Duration,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:4173".parse().expect("literal socket address"),
            codex_binary: PathBuf::from("codex"),
            codex_home: default_codex_home(),
            output_dir: PathBuf::from("var/output"),
            catalog_dir: None,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            request_timeout: Duration::from_secs(240),
            artifact_wait_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug)]
pub struct WebError {
    pub code: &'static str,
    pub message: String,
}

impl WebError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for WebError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for WebError {}

impl From<io::Error> for WebError {
    fn from(error: io::Error) -> Self {
        Self::new("WEB_IO", error.to_string())
    }
}

/// The catalog is immutable for the life of the process, so its response body is
/// built once. Rebuilding and re-serializing the catalog on every
/// Studio load is pure waste: the bytes are identical every time.
fn default_output_name() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("promptgen-{millis}.png")
}

fn default_codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
