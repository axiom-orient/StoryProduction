use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;

pub(super) const MAX_HEADER_BYTES: usize = 64 * 1024;
// The domain keeps the real 글투 corpus cap at 20 MiB. HTTP JSON may expand
// control characters up to six bytes each, so leave a bounded envelope while
// accepting the complete domain-sized request.
pub(super) const MAX_BODY_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct Request {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) headers: BTreeMap<String, String>,
    pub(super) body: Vec<u8>,
}

pub(super) fn read_request(stream: &mut TcpStream) -> Result<Request, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end;
    loop {
        let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            return Err("connection closed before request headers".into());
        }
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(index) = find_bytes(&bytes, b"\r\n\r\n") {
            header_end = index + 4;
            if header_end > MAX_HEADER_BYTES {
                return Err("request headers exceed limit".into());
            }
            break;
        }
        if bytes.len() > MAX_HEADER_BYTES {
            return Err("request headers exceed limit".into());
        }
    }
    let header =
        std::str::from_utf8(&bytes[..header_end]).map_err(|_| "request headers are not UTF-8")?;
    let mut lines = header.split("\r\n");
    let request_line = lines.next().ok_or("missing request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("missing method")?.to_string();
    let path = parts.next().ok_or("missing path")?.to_string();
    let version = parts.next().ok_or("missing HTTP version")?;
    if parts.next().is_some() {
        return Err("malformed request line".into());
    }
    if version != "HTTP/1.1" {
        return Err("only HTTP/1.1 is supported".into());
    }
    let mut headers = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line.split_once(':').ok_or("malformed header")?;
        if name.is_empty()
            || name != name.trim()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
        {
            return Err("invalid header name".into());
        }
        if headers
            .insert(name.to_ascii_lowercase(), value.trim().to_string())
            .is_some()
        {
            return Err("duplicate request header".into());
        }
    }
    if headers.contains_key("transfer-encoding") {
        return Err("transfer-encoding is not supported".into());
    }
    let content_length = headers
        .get("content-length")
        .map(|value| {
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err("invalid content-length");
            }
            value.parse::<usize>().map_err(|_| "invalid content-length")
        })
        .transpose()?
        .unwrap_or(0);
    if content_length > MAX_BODY_BYTES {
        return Err("request body exceeds limit".into());
    }
    drop(bytes.drain(..header_end));
    let mut body = bytes;
    if body.len() > content_length {
        return Err("request body exceeds content-length".into());
    }
    while body.len() < content_length {
        let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            return Err("connection closed before request body".into());
        }
        body.extend_from_slice(&chunk[..count]);
        if body.len() > content_length {
            return Err("request body exceeds content-length".into());
        }
    }
    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}

pub(super) fn respond_json(
    stream: &mut TcpStream,
    status: u16,
    value: &Value,
) -> Result<(), String> {
    let body = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    respond(
        stream,
        status,
        "application/json; charset=utf-8",
        &body,
        SECURITY_HEADERS,
    )
}

pub(super) const SECURITY_HEADERS: &[(&str, &str)] = &[
    ("Cache-Control", "no-store"),
    ("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"),
    ("Referrer-Policy", "no-referrer"),
    ("X-Content-Type-Options", "nosniff"),
    ("X-Frame-Options", "DENY"),
];

pub(super) fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    headers: &[(&str, &str)],
) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    write!(stream, "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n", body.len()).map_err(|error| error.to_string())?;
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n").map_err(|error| error.to_string())?;
    }
    stream
        .write_all(b"\r\n")
        .map_err(|error| error.to_string())?;
    stream.write_all(body).map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
