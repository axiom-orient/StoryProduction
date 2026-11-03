#[cfg(unix)]
mod artifacts;
#[cfg(unix)]
mod error;
#[cfg(unix)]
mod filesystem;
#[cfg(unix)]
mod lock_release;
#[cfg(unix)]
mod process_identity;
#[cfg(unix)]
mod protocol;
#[cfg(unix)]
mod record_fields;
#[cfg(unix)]
mod release_records;
#[cfg(unix)]
mod session;
#[cfg(unix)]
mod startup;

#[cfg(unix)]
use error::{CustodyError, Result};
#[cfg(unix)]
use protocol::{read_bounded_line, write_response};
#[cfg(unix)]
use session::Session;

#[cfg(unix)]
use std::io::{self, BufReader, Write};

#[cfg(unix)]
fn main() {
    if let Err(error) = run() {
        let _ = writeln!(io::stderr(), "{}: {}", error.code, error.message);
        std::process::exit(1);
    }
}

#[cfg(unix)]
fn run() -> Result<()> {
    let mut session = Session::new(startup::parse_config()?)?;
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    loop {
        let Some(line) = read_bounded_line(&mut reader)? else {
            // An interrupted parent may leave a lock for the next owner to
            // reclaim. Do not unlink it during implicit EOF: ownership cannot
            // be proven after the session descriptor is gone.
            return Ok(());
        };
        if line.is_empty() {
            return Err(CustodyError::input("empty request line"));
        }
        let request: serde_json::Value = serde_json::from_slice(&line)
            .map_err(|error| CustodyError::input(format!("request is not valid JSON: {error}")))?;
        let request_id = request
            .as_object()
            .and_then(|object| object.get("id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        match session.handle_request(&request) {
            Ok(result) => write_response(&mut writer, &request_id, Ok(result))?,
            Err(error) => write_response(&mut writer, &request_id, Err(error))?,
        }
        if request.get("op").and_then(serde_json::Value::as_str) == Some("close") {
            return Ok(());
        }
    }
}

#[cfg(not(unix))]
fn main() {
    eprintln!("smc-trackb-custody requires a POSIX target");
    std::process::exit(1);
}
