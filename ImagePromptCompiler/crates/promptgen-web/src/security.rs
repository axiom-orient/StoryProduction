use std::net::SocketAddr;

use super::http::{HttpRequest, Response};
use super::server::ServerState;

pub(super) fn validate_output_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 128 {
        return Err("output_name length must be within 1..=128".to_owned());
    }
    if !name.ends_with(".png") {
        return Err("output_name must end with .png".to_owned());
    }
    if name.starts_with('.') || name.contains("..") {
        return Err("output_name may not be hidden or contain parent traversal".to_owned());
    }
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(
            "output_name may contain only ASCII letters, digits, '-', '_', and '.'".to_owned(),
        );
    }
    Ok(())
}

pub(super) fn allowed_host(value: &str, local_addr: SocketAddr) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    let host = if lower.starts_with('[') {
        lower.split(']').next().map(|value| format!("{value}]"))
    } else {
        lower.split(':').next().map(ToOwned::to_owned)
    };
    matches!(host.as_deref(), Some("localhost" | "127.0.0.1" | "[::1]"))
        || host.as_deref() == Some(local_addr.ip().to_string().as_str())
}

pub(super) fn allowed_origin(value: &str, local_addr: SocketAddr) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    [
        format!("http://localhost:{}", local_addr.port()),
        format!("http://127.0.0.1:{}", local_addr.port()),
        format!("http://[::1]:{}", local_addr.port()),
        format!("http://{}", local_addr),
    ]
    .contains(&lower)
}

pub(super) fn validate_request_authority(
    request: &HttpRequest,
    state: &ServerState,
) -> Result<(), Response> {
    let host = request
        .headers
        .get("host")
        .map(String::as_str)
        .unwrap_or_default();
    if request.version == "HTTP/1.1" && host.is_empty() {
        return Err(Response::json_error(
            400,
            "HTTP_HOST_REQUIRED",
            "Host header is required",
        ));
    }
    if !host.is_empty() && !allowed_host(host, state.local_addr) {
        return Err(Response::json_error(
            403,
            "HTTP_HOST_REJECTED",
            "Host header is not a loopback promptGen origin",
        ));
    }
    if request.method == "POST" {
        if request
            .headers
            .get("x-promptgen-client")
            .map(String::as_str)
            != Some("web-v1")
        {
            return Err(Response::json_error(
                403,
                "HTTP_CLIENT_HEADER",
                "POST requires X-PromptGen-Client: web-v1",
            ));
        }
        if !request
            .headers
            .get("content-type")
            .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"))
        {
            return Err(Response::json_error(
                415,
                "HTTP_CONTENT_TYPE",
                "POST requires application/json",
            ));
        }
        if let Some(origin) = request.headers.get("origin")
            && !allowed_origin(origin, state.local_addr)
        {
            return Err(Response::json_error(
                403,
                "HTTP_ORIGIN_REJECTED",
                "Origin is not the local promptGen UI",
            ));
        }
    }
    Ok(())
}
