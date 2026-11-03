mod assets;
mod config;
mod http;
mod routes;
mod security;

use geultu::Application;
use geultu_writer_process::ProcessWriter;
use geultu_writing::builtin_adapters;
use std::net::TcpListener;
use std::time::Duration;

use config::Config;
use routes::handle;
use security::generate_session_token;

fn main() {
    if let Err(error) = run() {
        eprintln!("GeultuStudio: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::parse()?;
    config.validate()?;
    let token = generate_session_token()?;
    let mut generation = geultu::GenerationRegistry::default();
    for adapter in builtin_adapters() {
        generation.add_writing_adapter(adapter);
    }
    if let Some(program) = &config.writer_program {
        generation.add_provider(
            ProcessWriter::new("process", program.clone(), config.writer_args.clone())
                .map_err(|error| error.to_string())?,
        );
    }
    let app = Application::open(&config.workspace)
        .map_err(|error| error.to_string())?
        .with_generation_registry(generation);
    let listener = TcpListener::bind(config.listen)
        .map_err(|error| format!("cannot bind {}: {error}", config.listen))?;
    let listen = listener.local_addr().map_err(|error| error.to_string())?;
    eprintln!("GeultuStudio {}", env!("CARGO_PKG_VERSION"));
    eprintln!("workspace: {}", config.workspace.display());
    eprintln!("open: http://{listen}/?token={token}");
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .and_then(|()| stream.set_write_timeout(Some(Duration::from_secs(10))))
                {
                    eprintln!("connection configuration failed: {error}");
                    continue;
                }
                if let Err(error) = handle(&mut stream, &app, &token, listen) {
                    // A failed response write cannot be repaired by appending another response.
                    // The Application operation may already have committed.
                    eprintln!("connection response failed: {error}");
                }
            }
            Err(error) => eprintln!("accept error: {error}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::config::Config;
    use super::http::{read_request, Request, MAX_HEADER_BYTES};
    use super::security::{generate_session_token, validate_host};
    use std::collections::BTreeMap;
    use std::io::Write;
    use std::net::TcpStream;

    const JSON_ESCAPE_EXPANSION: usize = 6;
    const VOICE_REQUEST_ENVELOPE_BYTES: usize = 7 * 1024 * 1024;
    const VOICE_REQUEST_LINE_END_BYTES: usize = 1;

    fn request(host: &str) -> Request {
        let mut headers = BTreeMap::new();
        headers.insert("host".into(), host.into());
        Request {
            method: "GET".into(),
            path: "/healthz".into(),
            headers,
            body: Vec::new(),
        }
    }

    fn config(args: &[&str]) -> Config {
        let args = args
            .iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>();
        Config::from_args(&args).expect("test arguments should parse")
    }

    #[test]
    fn rejects_non_loopback_listener_before_bind() {
        let config = config(&["--listen", "0.0.0.0:8765"]);
        let error = config
            .validate()
            .expect_err("non-loopback listener must be rejected");
        assert!(error.contains("non-loopback"));
    }

    #[test]
    fn accepts_loopback_listener() {
        let config = config(&["--listen", "127.0.0.1:8765"]);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn transport_body_covers_worst_case_voice_json_envelope() {
        let worst_case = geultu::voice::MAX_VOICE_CORPUS_BYTES * JSON_ESCAPE_EXPANSION
            + VOICE_REQUEST_ENVELOPE_BYTES
            + VOICE_REQUEST_LINE_END_BYTES;
        assert!(super::http::MAX_BODY_BYTES >= worst_case);
    }

    #[test]
    fn accepts_ipv6_loopback_listener() {
        let config = config(&["--listen", "[::1]:8765"]);
        assert!(config.validate().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn generates_independent_hex_session_tokens() {
        let first = generate_session_token().expect("Unix CSPRNG should be available");
        let second = generate_session_token().expect("Unix CSPRNG should be available");

        assert_eq!(first.len(), 64);
        assert_eq!(second.len(), 64);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(second.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[cfg(not(unix))]
    #[test]
    fn rejects_session_token_generation_without_os_csprng() {
        let error = generate_session_token().expect_err("non-Unix token generation must fail");
        assert!(error.contains("Unix"));
    }

    #[test]
    fn accepts_bracketed_ipv6_loopback_host_authority() {
        let request = request("[::1]:8765");
        let listen = "[::1]:8765".parse().unwrap();
        assert!(validate_host(&request, listen).is_ok());
    }

    #[test]
    fn rejects_unbracketed_ipv6_host_authority() {
        let request = request("::1:8765");
        let listen = "[::1]:8765".parse().unwrap();
        assert!(validate_host(&request, listen).is_err());
    }

    #[test]
    fn rejects_malformed_bracketed_ipv6_host_authority() {
        let request = request("[::1:8765");
        let listen = "[::1]:8765".parse().unwrap();
        assert!(validate_host(&request, listen).is_err());
    }

    #[test]
    fn rejects_ipv6_unspecified_host_authority() {
        let request = request("[::]:8765");
        let listen = "[::1]:8765".parse().unwrap();
        assert!(validate_host(&request, listen).is_err());
    }

    #[test]
    fn rejects_ipv6_non_loopback_host_authority() {
        let request = request("[2001:db8::1]:8765");
        let listen = "[::1]:8765".parse().unwrap();
        assert!(validate_host(&request, listen).is_err());
    }
    fn read_fixture(bytes: Vec<u8>) -> Result<Request, String> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let writer = std::thread::spawn(move || {
            let _ = client.write_all(&bytes);
            let _ = client.shutdown(std::net::Shutdown::Write);
        });
        let (mut server, _) = listener.accept().unwrap();
        server
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let result = read_request(&mut server);
        drop(server);
        writer.join().unwrap();
        result
    }

    #[test]
    fn ambiguous_http_framing_is_rejected() {
        for request in [
            "POST /api/tools/call HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nContent-Length: 2\r\n\r\n{}",
            "POST /api/tools/call HTTP/1.1\r\nHost: localhost\r\nContent-Length: +2\r\n\r\n{}",
            "POST /api/tools/call HTTP/1.1 extra\r\nHost: localhost\r\n\r\n",
            "POST /api/tools/call HTTP/1.1\r\nHost: localhost\r\n\r\n{}",
        ] {
            assert!(read_fixture(request.as_bytes().to_vec()).is_err());
        }
    }

    #[test]
    fn body_read_ahead_does_not_count_towards_header_limit() {
        let mut request = format!(
            "POST /api/tools/call HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            MAX_HEADER_BYTES
        );
        request.push_str(&"x".repeat(MAX_HEADER_BYTES));
        assert_eq!(
            read_fixture(request.into_bytes()).unwrap().body.len(),
            MAX_HEADER_BYTES
        );
    }
}
