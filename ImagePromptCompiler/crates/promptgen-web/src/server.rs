use std::fs;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use super::http::Response;
use super::routes;
use super::{MAX_ACTIVE_CONNECTIONS, READ_TIMEOUT, WebConfig, WebError};

/// Resolves the directory that holds the runtime catalog assets.
///
/// The published binary is relocatable, so the compile-time source layout cannot be
/// used here. Candidates are tried in declared order and the first directory holding
/// `image_catalog.json` wins; when none matches the server still starts and the asset
/// route reports the miss explicitly instead of serving from an unknown path.
pub fn resolve_catalog_dir(explicit: Option<&Path>) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(path) = explicit {
        candidates.push(path.to_path_buf());
    }
    if let Some(value) = std::env::var_os("PROMPTGEN_CATALOG_DIR") {
        candidates.push(PathBuf::from(value));
    }
    // An installed binary sits next to its catalog; a source checkout runs it from
    // `target/<profile>/` or `target/debug/deps/`. Walking up covers both without
    // encoding either layout.
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        candidates.extend(directory.ancestors().map(|base| base.join("catalog")));
    }
    if let Ok(current) = std::env::current_dir() {
        candidates.extend(current.ancestors().map(|base| base.join("catalog")));
    }
    candidates
        .into_iter()
        .find(|candidate| candidate.join("image_catalog.json").is_file())
}

pub(super) struct ServerState {
    pub(super) config: WebConfig,
    pub(super) catalog_dir: Option<PathBuf>,
    pub(super) local_addr: SocketAddr,
    pub(super) generation_busy: AtomicBool,
    pub(super) active_connections: AtomicUsize,
}

/// Stop signal for a running [`serve_listener`].
///
/// The accept loop blocks instead of polling, so the flag alone cannot end it: an
/// idle server is parked inside `accept`. Signalling therefore sets the flag and
/// opens one throwaway loopback connection, which is the wake-up event. The
/// alternative — a non-blocking listener with a sleep — burns wake-ups forever and
/// adds that sleep to the latency of every request.
#[derive(Clone, Debug)]
pub struct ShutdownSignal {
    flag: Arc<AtomicBool>,
    address: SocketAddr,
}

impl ShutdownSignal {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            address,
        }
    }

    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }

    pub fn signal(&self) {
        self.flag.store(true, Ordering::Release);
        if let Ok(stream) = TcpStream::connect_timeout(&self.address, READ_TIMEOUT) {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}

pub fn serve(config: WebConfig) -> Result<(), WebError> {
    validate_config(&config)?;
    let listener = TcpListener::bind(config.bind).map_err(|error| {
        WebError::new(
            "WEB_BIND",
            format!("failed to bind {}: {error}", config.bind),
        )
    })?;
    let local_addr = listener.local_addr()?;
    eprintln!("promptGen UI: http://{local_addr}");
    serve_listener(listener, config, Arc::new(AtomicBool::new(false)))
}

pub fn serve_listener(
    listener: TcpListener,
    mut config: WebConfig,
    shutdown: Arc<AtomicBool>,
) -> Result<(), WebError> {
    validate_config(&config)?;
    let local_addr = listener.local_addr()?;
    config.bind = local_addr;
    fs::create_dir_all(&config.output_dir).map_err(|error| {
        WebError::new(
            "WEB_OUTPUT_DIR",
            format!("failed to create {}: {error}", config.output_dir.display()),
        )
    })?;
    let catalog_dir = resolve_catalog_dir(config.catalog_dir.as_deref());
    if catalog_dir.is_none() {
        eprintln!(
            "promptGen: no catalog directory found; card images are unavailable. \
             Set PROMPTGEN_CATALOG_DIR or run from a directory containing catalog/image_catalog.json."
        );
    }
    let state = Arc::new(ServerState {
        config,
        catalog_dir,
        local_addr,
        generation_busy: AtomicBool::new(false),
        active_connections: AtomicUsize::new(0),
    });
    loop {
        // Checked before dispatching rather than around a poll: the wake-up
        // connection from `ShutdownSignal` is what returns control here.
        if shutdown.load(Ordering::Acquire) {
            return Ok(());
        }
        match listener.accept() {
            Ok((mut stream, _peer)) => {
                if shutdown.load(Ordering::Acquire) {
                    return Ok(());
                }
                let Some(connection_guard) = ConnectionGuard::acquire(&state) else {
                    let rejection = (|| -> io::Result<()> {
                        stream.set_write_timeout(Some(READ_TIMEOUT))?;
                        Response::json_error(
                            503,
                            "WEB_CONNECTION_LIMIT",
                            "too many concurrent local connections",
                        )
                        .write_to(&mut stream)
                    })();
                    if let Err(error) = rejection {
                        eprintln!("promptGen connection-limit response error: {error}");
                    }
                    continue;
                };
                let worker_state = Arc::clone(&state);
                if let Err(error) = thread::Builder::new()
                    .name("promptgen-http".to_owned())
                    .spawn(move || {
                        let _connection_guard = connection_guard;
                        if let Err(error) = routes::handle_connection(stream, &worker_state) {
                            eprintln!("promptGen web request error: {error}");
                        }
                    })
                {
                    eprintln!("promptGen failed to spawn request worker: {error}");
                }
            }
            // A connection can die between the kernel queueing it and `accept`
            // returning; that is the peer's failure, not the server's.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionAborted | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

pub(super) fn validate_config(config: &WebConfig) -> Result<(), WebError> {
    if !config.bind.ip().is_loopback() {
        return Err(WebError::new(
            "WEB_BIND_NOT_LOOPBACK",
            format!("refusing non-loopback bind address {}", config.bind),
        ));
    }
    if config.max_body_bytes == 0 || config.max_body_bytes > 16 * 1024 * 1024 {
        return Err(WebError::new(
            "WEB_BODY_LIMIT",
            "max_body_bytes must be within 1..=16777216",
        ));
    }
    Ok(())
}

pub(super) struct ConnectionGuard {
    state: Arc<ServerState>,
}

impl ConnectionGuard {
    pub(super) fn acquire(state: &Arc<ServerState>) -> Option<Self> {
        state
            .active_connections
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < MAX_ACTIVE_CONNECTIONS).then_some(current + 1)
            })
            .ok()?;
        Some(Self {
            state: Arc::clone(state),
        })
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let previous = self.state.active_connections.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "connection counter underflow");
    }
}
