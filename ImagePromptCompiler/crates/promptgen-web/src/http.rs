use std::collections::BTreeMap;
use std::io::{self, Read, Write};

use promptgen_core::json::JsonValue;

const MAX_HEADER_BYTES: usize = 32 * 1024;

#[derive(Debug)]
pub(super) struct HttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) version: String,
    pub(super) headers: BTreeMap<String, String>,
    pub(super) body: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct RequestError {
    pub(super) status: u16,
    pub(super) code: &'static str,
    pub(super) message: String,
}

impl RequestError {
    fn new(status: u16, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
}

impl HttpRequest {
    pub(super) fn read_from(
        reader: &mut impl Read,
        max_body_bytes: usize,
    ) -> Result<Self, RequestError> {
        const HEADER_TERMINATOR: &[u8] = b"\r\n\r\n";
        let mut buffer = Vec::with_capacity(4096);
        let mut chunk = [0_u8; 2048];
        let mut search_start = 0usize;
        let header_end = loop {
            if buffer.len() > MAX_HEADER_BYTES {
                return Err(RequestError::new(
                    431,
                    "HTTP_HEADERS_TOO_LARGE",
                    "request headers exceed 32768 bytes",
                ));
            }
            if let Some(index) = find_subsequence(&buffer[search_start..], HEADER_TERMINATOR) {
                break search_start + index + HEADER_TERMINATOR.len();
            }
            let previous_len = buffer.len();
            let read = reader.read(&mut chunk).map_err(|error| {
                RequestError::new(400, "HTTP_READ", format!("failed to read request: {error}"))
            })?;
            if read == 0 {
                return Err(RequestError::new(
                    400,
                    "HTTP_INCOMPLETE_HEADERS",
                    "connection ended before complete headers",
                ));
            }
            buffer.extend_from_slice(&chunk[..read]);
            search_start = previous_len.saturating_sub(HEADER_TERMINATOR.len() - 1);
        };

        let head = std::str::from_utf8(&buffer[..header_end - 4]).map_err(|_| {
            RequestError::new(400, "HTTP_HEADER_ENCODING", "headers must be valid UTF-8")
        })?;
        let mut lines = head.split("\r\n");
        let request_line = lines.next().unwrap_or_default();
        let parts = request_line.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(RequestError::new(
                400,
                "HTTP_REQUEST_LINE",
                "request line must contain method, path, and HTTP version",
            ));
        }
        let method = parts[0].to_owned();
        let path = parts[1].to_owned();
        let version = parts[2].to_owned();
        if version != "HTTP/1.1" && version != "HTTP/1.0" {
            return Err(RequestError::new(
                505,
                "HTTP_VERSION",
                "only HTTP/1.0 and HTTP/1.1 are supported",
            ));
        }
        if !path.starts_with('/') || path.contains('\0') {
            return Err(RequestError::new(400, "HTTP_PATH", "invalid request path"));
        }

        let mut headers = BTreeMap::new();
        for line in lines {
            let Some((name, value)) = line.split_once(':') else {
                return Err(RequestError::new(
                    400,
                    "HTTP_HEADER",
                    format!("malformed header line {line:?}"),
                ));
            };
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_owned();
            if name.is_empty() || headers.insert(name.clone(), value).is_some() {
                return Err(RequestError::new(
                    400,
                    "HTTP_DUPLICATE_HEADER",
                    format!("duplicate or empty header {name:?}"),
                ));
            }
        }
        if headers.contains_key("transfer-encoding") {
            return Err(RequestError::new(
                400,
                "HTTP_TRANSFER_ENCODING",
                "chunked or transformed request bodies are not supported",
            ));
        }
        let content_length = match headers.get("content-length") {
            Some(value) => value.parse::<usize>().map_err(|_| {
                RequestError::new(400, "HTTP_CONTENT_LENGTH", "invalid Content-Length")
            })?,
            None if method == "POST" => {
                return Err(RequestError::new(
                    411,
                    "HTTP_LENGTH_REQUIRED",
                    "POST requires Content-Length",
                ));
            }
            None => 0,
        };
        if content_length > max_body_bytes {
            return Err(RequestError::new(
                413,
                "HTTP_BODY_TOO_LARGE",
                format!("request body exceeds {max_body_bytes} bytes"),
            ));
        }
        if method != "POST" && content_length != 0 {
            return Err(RequestError::new(
                400,
                "HTTP_UNEXPECTED_BODY",
                "only POST requests may contain a body",
            ));
        }
        let already = buffer.len().saturating_sub(header_end);
        if already > content_length {
            buffer.truncate(header_end + content_length);
        }
        while buffer.len() - header_end < content_length {
            let remaining = content_length - (buffer.len() - header_end);
            let read_size = remaining.min(chunk.len());
            let read = reader.read(&mut chunk[..read_size]).map_err(|error| {
                RequestError::new(
                    400,
                    "HTTP_BODY_READ",
                    format!("failed to read body: {error}"),
                )
            })?;
            if read == 0 {
                return Err(RequestError::new(
                    400,
                    "HTTP_INCOMPLETE_BODY",
                    "connection ended before Content-Length bytes were read",
                ));
            }
            buffer.extend_from_slice(&chunk[..read]);
        }
        Ok(Self {
            method,
            path,
            version,
            headers,
            body: buffer[header_end..header_end + content_length].to_vec(),
        })
    }
}

#[derive(Debug)]
pub(super) struct Response {
    pub(super) status: u16,
    content_type: &'static str,
    pub(super) body: Vec<u8>,
    cache_control: &'static str,
    etag: Option<String>,
}

impl Response {
    pub(super) fn text(status: u16, content_type: &'static str, body: &str) -> Self {
        Self {
            status,
            content_type,
            body: body.as_bytes().to_vec(),
            cache_control: "no-store",
            etag: None,
        }
    }

    pub(super) fn bytes(status: u16, content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status,
            content_type,
            body,
            cache_control: "private, max-age=60",
            etag: None,
        }
    }

    /// Catalog art is immutable for a given build, and a single grid pulls tens of megabytes.
    /// A validator plus a long private max-age keeps tab switches and reloads off the wire.
    pub(super) fn cached_bytes(content_type: &'static str, body: Vec<u8>, etag: String) -> Self {
        Self {
            status: 200,
            content_type,
            body,
            cache_control: "private, max-age=86400",
            etag: Some(etag),
        }
    }

    pub(super) fn not_modified(etag: String) -> Self {
        Self {
            status: 304,
            content_type: "application/octet-stream",
            body: Vec::new(),
            cache_control: "private, max-age=86400",
            etag: Some(etag),
        }
    }

    pub(super) fn json(status: u16, value: JsonValue) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body: value.to_compact_string().into_bytes(),
            cache_control: "no-store",
            etag: None,
        }
    }

    pub(super) fn json_error(status: u16, code: &'static str, message: impl Into<String>) -> Self {
        Self::json(
            status,
            JsonValue::object([
                (
                    "error",
                    JsonValue::object([
                        ("code", JsonValue::from(code)),
                        ("message", JsonValue::from(message.into())),
                    ]),
                ),
                ("ok", JsonValue::from(false)),
            ]),
        )
    }

    pub(super) fn write_to(&self, writer: &mut impl Write) -> io::Result<()> {
        let etag = match &self.etag {
            Some(value) => format!("ETag: {value}\r\n"),
            None => String::new(),
        };
        write!(
            writer,
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: {}\r\n{}Connection: close\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nX-Frame-Options: DENY\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: blob:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'\r\n\r\n",
            self.status,
            reason_phrase(self.status),
            self.content_type,
            self.body.len(),
            self.cache_control,
            etag,
        )?;
        writer.write_all(&self.body)?;
        writer.flush()
    }
}

pub(super) fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

const fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        304 => "Not Modified",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ => "Error",
    }
}
