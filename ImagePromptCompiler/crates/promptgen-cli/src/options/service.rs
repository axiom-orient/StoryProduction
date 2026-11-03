use std::collections::BTreeSet;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use promptgen_mcp::McpHttpConfig;
use promptgen_web::WebConfig;

use crate::error::CliError;
use crate::help::{print_mcp_help, print_serve_help};

use super::{ArgCursor, claim_option, parse_body_bytes, parse_seconds};

#[derive(Debug)]
pub(crate) struct ServeOptions {
    pub(crate) config: WebConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum McpTransport {
    Stdio,
    Http,
}

impl McpTransport {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "stdio" => Ok(Self::Stdio),
            "http" => Ok(Self::Http),
            _ => Err(CliError::usage(format!(
                "invalid --transport {value:?}; expected stdio or http"
            ))),
        }
    }
}

#[derive(Debug)]
pub(crate) struct McpOptions {
    pub(crate) transport: McpTransport,
    pub(crate) config: McpHttpConfig,
}

pub(crate) fn parse_serve_options(mut cursor: ArgCursor) -> Result<ServeOptions, CliError> {
    let mut config = WebConfig::default();
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        match argument.as_str() {
            "--bind" => {
                claim_option(&mut seen, "--bind")?;
                let value = cursor.required_value("--bind")?;
                config.bind = value.parse::<SocketAddr>().map_err(|error| {
                    CliError::usage(format!("invalid --bind {value:?}: {error}"))
                })?;
            }
            "--output-dir" => {
                claim_option(&mut seen, "--output-dir")?;
                config.output_dir = PathBuf::from(cursor.required_value("--output-dir")?);
            }
            "--catalog-dir" => {
                claim_option(&mut seen, "--catalog-dir")?;
                config.catalog_dir = Some(PathBuf::from(cursor.required_value("--catalog-dir")?));
            }
            "--codex-binary" => {
                claim_option(&mut seen, "--codex-binary")?;
                config.codex_binary = PathBuf::from(cursor.required_value("--codex-binary")?);
            }
            "--codex-home" => {
                claim_option(&mut seen, "--codex-home")?;
                config.codex_home = PathBuf::from(cursor.required_value("--codex-home")?);
            }
            "--timeout-seconds" => {
                claim_option(&mut seen, "--timeout-seconds")?;
                config.request_timeout = Duration::from_secs(parse_seconds(
                    &cursor.required_value("--timeout-seconds")?,
                    "--timeout-seconds",
                    3_600,
                )?);
            }
            "--artifact-wait-seconds" => {
                claim_option(&mut seen, "--artifact-wait-seconds")?;
                config.artifact_wait_timeout = Duration::from_secs(parse_seconds(
                    &cursor.required_value("--artifact-wait-seconds")?,
                    "--artifact-wait-seconds",
                    600,
                )?);
            }
            "--help" | "-h" => {
                print_serve_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(ServeOptions { config })
}

pub(crate) fn parse_mcp_options(mut cursor: ArgCursor) -> Result<McpOptions, CliError> {
    let mut transport = McpTransport::Stdio;
    let mut config = McpHttpConfig::default();
    let mut bearer_token = None;
    let mut http_option_seen = false;
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        match argument.as_str() {
            "--transport" => {
                claim_option(&mut seen, "--transport")?;
                transport = McpTransport::parse(&cursor.required_value("--transport")?)?;
            }
            "--bind" => {
                claim_option(&mut seen, "--bind")?;
                http_option_seen = true;
                let value = cursor.required_value("--bind")?;
                config.bind = value.parse::<SocketAddr>().map_err(|error| {
                    CliError::usage(format!("invalid --bind {value:?}: {error}"))
                })?;
            }
            "--bearer-token" => {
                claim_option(&mut seen, "--bearer-token")?;
                http_option_seen = true;
                let value = cursor.required_value("--bearer-token")?;
                if value.is_empty() {
                    return Err(CliError::usage("--bearer-token must not be empty"));
                }
                bearer_token = Some(value);
            }
            "--allowed-host" => {
                claim_option(&mut seen, "--allowed-host")?;
                http_option_seen = true;
                let value = cursor.required_value("--allowed-host")?;
                config.allowed_hosts = value
                    .split(',')
                    .map(str::trim)
                    .filter(|host| !host.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
                if config.allowed_hosts.is_empty() {
                    return Err(CliError::usage("--allowed-host must contain a hostname"));
                }
            }
            "--max-body-bytes" => {
                claim_option(&mut seen, "--max-body-bytes")?;
                http_option_seen = true;
                config.max_body_bytes =
                    parse_body_bytes(&cursor.required_value("--max-body-bytes")?)?;
            }
            "--help" | "-h" => {
                print_mcp_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    if transport == McpTransport::Stdio && http_option_seen {
        return Err(CliError::usage(
            "HTTP-only MCP options require --transport http",
        ));
    }
    if bearer_token.is_none() {
        bearer_token =
            match env::var_os("PROMPTGEN_MCP_BEARER_TOKEN") {
                Some(value) => Some(value.into_string().map_err(|_| {
                    CliError::usage("PROMPTGEN_MCP_BEARER_TOKEN must be valid UTF-8")
                })?),
                None => None,
            };
    }
    config.bearer_token = bearer_token;
    Ok(McpOptions { transport, config })
}
