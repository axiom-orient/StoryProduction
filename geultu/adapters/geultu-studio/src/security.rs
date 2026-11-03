use super::http::Request;
#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::io::Read;
use std::net::{IpAddr, SocketAddr};

#[cfg(unix)]
pub(super) fn generate_session_token() -> Result<String, String> {
    const TOKEN_BYTES: usize = 32;
    let mut bytes = [0_u8; TOKEN_BYTES];
    let mut source = File::open("/dev/urandom")
        .map_err(|_| "Studio could not open the operating-system CSPRNG".to_string())?;
    source
        .read_exact(&mut bytes)
        .map_err(|_| "Studio could not read the operating-system CSPRNG".to_string())?;
    Ok(hex_encode(&bytes))
}

#[cfg(not(unix))]
pub(super) fn generate_session_token() -> Result<String, String> {
    Err("Studio session tokens require a Unix operating-system CSPRNG".into())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = vec![0_u8; bytes.len() * 2];
    for (index, byte) in bytes.iter().copied().enumerate() {
        encoded[index * 2] = HEX[(byte >> 4) as usize];
        encoded[index * 2 + 1] = HEX[(byte & 0x0f) as usize];
    }
    String::from_utf8(encoded).expect("hex encoding is ASCII")
}

pub(super) fn authorize(request: &Request, token: &str) -> Result<(), String> {
    if request.headers.get("x-geultu-token").map(String::as_str) != Some(token) {
        return Err("missing or invalid X-Geultu-Token".into());
    }
    if let Some(origin) = request.headers.get("origin") {
        let host = request
            .headers
            .get("host")
            .ok_or("Host header is required")?;
        let allowed_http = format!("http://{host}");
        if origin != &allowed_http {
            return Err("cross-origin API request rejected".into());
        }
    }
    Ok(())
}

pub(super) fn validate_host(request: &Request, listen: SocketAddr) -> Result<(), String> {
    let host = request
        .headers
        .get("host")
        .ok_or("Host header is required")?;
    let (host_without_port, port) = parse_host_authority(host).ok_or("Host header rejected")?;
    if port != listen.port() {
        return Err("Host header rejected".into());
    }
    let allowed = host_without_port == "localhost"
        || host_without_port
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback() || ip == listen.ip())
            .unwrap_or(false);
    if allowed {
        Ok(())
    } else {
        Err("Host header rejected".into())
    }
}

fn parse_host_authority(host: &str) -> Option<(&str, u16)> {
    let host = host.trim();
    if let Some(rest) = host.strip_prefix('[') {
        let (address, suffix) = rest.split_once(']')?;
        if suffix.is_empty() {
            return Some((address, 80));
        }
        let port = parse_port(suffix.strip_prefix(':')?)?;
        return Some((address, port));
    }
    if host.contains('[') || host.contains(']') {
        return None;
    }
    match host.matches(':').count() {
        0 => Some((host, 80)),
        1 => {
            let (address, port) = host.split_once(':')?;
            Some((address, parse_port(port)?))
        }
        _ => None,
    }
}

fn parse_port(port: &str) -> Option<u16> {
    if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    port.parse().ok()
}
