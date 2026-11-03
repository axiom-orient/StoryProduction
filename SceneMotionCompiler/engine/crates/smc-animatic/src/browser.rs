use crate::{
    io::{self, Result},
    process,
};
use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tungstenite::{stream::MaybeTlsStream, Message, WebSocket};

fn discover(configured: &str, candidates: &[&str], label: &str) -> Result<PathBuf> {
    if let Ok(value) = std::env::var(configured) {
        return process::executable(&value, label);
    }
    for candidate in candidates {
        if let Ok(path) = process::executable(candidate, label) {
            return Ok(path);
        }
    }
    Err(format!("{label}_UNAVAILABLE"))
}
pub fn renderer_lock(reduced: bool) -> Result<(Value, Vec<u8>)> {
    let browser = discover(
        "SMC_CHROMIUM_PATH",
        &[
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/usr/bin/chromium",
            "chromium",
            "google-chrome",
        ],
        "CHROMIUM",
    )?;
    let font = if let Ok(value) = std::env::var("SMC_FONT_PATH") {
        PathBuf::from(value)
    } else {
        [
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
            "/System/Library/Fonts/Helvetica.ttc",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
        .ok_or("FONT_UNAVAILABLE")?
    };
    let browser_bytes = io::read(&browser, 512 * 1024 * 1024)?;
    let font_bytes = io::read(&font, 96 * 1024 * 1024)?;
    io::check(
        font_bytes.len() >= 4
            && (font_bytes[..4] == [0, 1, 0, 0]
                || [b"OTTO", b"ttcf", b"wOFF", b"wOF2"]
                    .iter()
                    .any(|magic| font_bytes[..4] == magic[..])),
        "UNSUPPORTED_FONT",
    )?;
    let version = String::from_utf8(process::plain(&browser, &["--version"])?)
        .map_err(|e| e.to_string())?
        .trim()
        .to_string();
    io::check(!version.is_empty(), "CHROMIUM_VERSION_EMPTY")?;
    let platform = if cfg!(target_os = "macos") {
        "darwin"
    } else {
        std::env::consts::OS
    };
    let architecture = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        std::env::consts::ARCH
    };
    Ok((
        json!({"schemaVersion":1,"rendererLockId":format!("renderer.chromium.{platform}.{architecture}"),"browser":{"name":"chromium","version":version,"binaryPath":browser,"binarySha256":io::sha(&browser_bytes),"binaryBytes":browser_bytes.len()},"font":{"path":font,"sha256":io::sha(&font_bytes)},"os":platform,"architecture":architecture,"locale":"ko-KR","timezone":"UTC","colorScheme":"light","reducedMotion":if reduced{"reduce"}else{"no-preference"},"networkPolicy":"deny-external","screenshotFormat":"png","fontPolicy":"force-locked-font","implementationVersion":"0.1.0"}),
        font_bytes,
    ))
}

pub struct Browser {
    child: process::Group,
    _profile: tempfile::TempDir,
    pub port: u16,
}
impl Browser {
    pub fn launch(lock: &Value) -> Result<Self> {
        let path = Path::new(
            lock["browser"]["binaryPath"]
                .as_str()
                .ok_or("BROWSER_PATH")?,
        );
        io::check(
            io::sha(&io::read(path, 512 * 1024 * 1024)?) == lock["browser"]["binarySha256"],
            "BROWSER_BYTES_CHANGED",
        )?;
        let version =
            String::from_utf8(process::plain(path, &["--version"])?).map_err(|e| e.to_string())?;
        io::check(
            version.trim() == lock["browser"]["version"],
            "BROWSER_VERSION_CHANGED",
        )?;
        let font = Path::new(lock["font"]["path"].as_str().ok_or("FONT_PATH")?);
        io::check(
            io::sha(&io::read(font, 96 * 1024 * 1024)?) == lock["font"]["sha256"],
            "FONT_BYTES_CHANGED",
        )?;
        let profile = io::io(tempfile::Builder::new().prefix("smc-chromium-").tempdir())?;
        let mut command = Command::new(path);
        command
            .args([
                "--headless=new",
                "--no-sandbox",
                "--disable-gpu",
                "--disable-dev-shm-usage",
                "--disable-background-networking",
                "--disable-component-update",
                "--disable-default-apps",
                "--disable-extensions",
                "--disable-sync",
                "--metrics-recording-only",
                "--no-first-run",
                "--remote-debugging-port=0",
            ])
            .arg(format!("--user-data-dir={}", profile.path().display()))
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = process::spawn(&mut command)?;
        let deadline = Instant::now() + Duration::from_secs(12);
        while Instant::now() < deadline {
            io::check(!io::interrupted(), "WORKER_INTERRUPTED")?;
            io::check(
                io::io(child.0.try_wait())?.is_none(),
                "CHROMIUM_EXITED_BEFORE_DEVTOOLS",
            )?;
            if let Ok(text) = fs::read_to_string(profile.path().join("DevToolsActivePort")) {
                if let Some(port) = text
                    .lines()
                    .next()
                    .and_then(|line| line.parse::<u16>().ok())
                    .filter(|v| *v > 0)
                {
                    return Ok(Self {
                        child,
                        _profile: profile,
                        port,
                    });
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err("CHROMIUM_DEVTOOLS_TIMEOUT".into())
    }
    pub fn close(&mut self) -> Result<()> {
        self.child
            .stop()
            .map_err(|e| format!("OUTCOME_UNKNOWN:CLEANUP_NOT_CONFIRMED:{e}"))
    }
}

fn endpoint(port: u16) -> Result<String> {
    let mut socket = io::io(TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().map_err(|_| "PORT")?,
        Duration::from_secs(10),
    ))?;
    io::io(socket.set_read_timeout(Some(Duration::from_millis(100))))?;
    io::io(socket.set_write_timeout(Some(Duration::from_secs(10))))?;
    io::io(socket.write_all(format!("PUT /json/new?about%3Ablank HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").as_bytes()))?;
    let mut response = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        io::check(Instant::now() < deadline, "CDP_ENDPOINT_TIMEOUT")?;
        let mut chunk = [0u8; 8192];
        match socket.read(&mut chunk) {
            Ok(0) => break,
            Ok(length) => response.extend_from_slice(&chunk[..length]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue
            }
            Err(error) => return Err(format!("CDP_ENDPOINT_READ:{error}")),
        }
        io::check(response.len() <= 65536, "CDP_ENDPOINT_RESPONSE_LIMIT")?;
        if let Some(header_end) = response.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
            let headers =
                std::str::from_utf8(&response[..header_end]).map_err(|_| "CDP_ENDPOINT_HEADERS")?;
            let length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .ok_or("CDP_ENDPOINT_CONTENT_LENGTH_REQUIRED")?;
            io::check(
                header_end + 4 + length <= 65536,
                "CDP_ENDPOINT_RESPONSE_LIMIT",
            )?;
            if response.len() >= header_end + 4 + length {
                response.truncate(header_end + 4 + length);
                break;
            }
        }
    }
    io::check(response.len() <= 65536, "CDP_ENDPOINT_RESPONSE_LIMIT")?;
    let response = String::from_utf8(response).map_err(|e| e.to_string())?;
    io::check(response.starts_with("HTTP/1.1 200"), "CDP_ENDPOINT_HTTP")?;
    let body = response
        .split_once("\r\n\r\n")
        .ok_or("CDP_ENDPOINT_PROTOCOL")?
        .1;
    let value: Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    value["webSocketDebuggerUrl"]
        .as_str()
        .filter(|url| url.starts_with(&format!("ws://127.0.0.1:{port}/")))
        .map(str::to_string)
        .ok_or("CDP_ENDPOINT_IDENTITY".into())
}

pub struct Cdp {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    next: u64,
    pub violations: Vec<String>,
    pub context: Option<u64>,
    guard_binding: String,
    guard_nonce: String,
    pub guard_ready: bool,
}
impl Cdp {
    pub fn connect(port: u16) -> Result<Self> {
        let (socket, _) = tungstenite::client::connect_with_config(
            endpoint(port)?,
            Some(
                tungstenite::protocol::WebSocketConfig::default()
                    .max_message_size(Some(512 * 1024 * 1024))
                    .max_frame_size(Some(512 * 1024 * 1024)),
            ),
            0,
        )
        .map_err(|e| e.to_string())?;
        let stream = match socket.get_ref() {
            MaybeTlsStream::Plain(stream) => stream,
            _ => return Err("CDP_REQUIRES_LOCAL_PLAIN_SOCKET".into()),
        };
        io::io(stream.set_read_timeout(Some(Duration::from_millis(100))))?;
        io::io(stream.set_write_timeout(Some(Duration::from_secs(10))))?;
        Ok(Self {
            socket,
            next: 1,
            violations: Vec::new(),
            context: None,
            guard_binding: String::new(),
            guard_nonce: String::new(),
            guard_ready: false,
        })
    }
    fn event(&mut self, value: &Value) -> Result<()> {
        let method = value["method"].as_str().unwrap_or("");
        let params = &value["params"];
        match method {
            "Runtime.executionContextCreated" => self.context = params["context"]["id"].as_u64(),
            "Runtime.exceptionThrown" => self.violations.push(format!(
                "RUNTIME_EXCEPTION:{}",
                params["exceptionDetails"]["exception"]["description"]
            )),
            "Runtime.consoleAPICalled" if params["type"] == "error" => {
                self.violations.push("CONSOLE_ERROR".into())
            }
            "Log.entryAdded" if params["entry"]["level"] == "error" => {
                self.violations.push("LOG_ERROR".into())
            }
            "Network.requestWillBeSent" => {
                let url = params["request"]["url"].as_str().unwrap_or("");
                if !url.starts_with("data:") && !url.starts_with("about:") {
                    self.violations.push(format!("EXTERNAL_REQUEST:{url}"));
                }
            }
            "Runtime.bindingCalled" if params["name"] == self.guard_binding => {
                let message: Value = serde_json::from_str(
                    params["payload"].as_str().ok_or("GUARD_BINDING_PAYLOAD")?,
                )
                .map_err(|e| e.to_string())?;
                io::check(message["nonce"] == self.guard_nonce, "GUARD_NONCE_MISMATCH")?;
                io::check(
                    params["executionContextId"].as_u64() == self.context,
                    "GUARD_CONTEXT_MISMATCH",
                )?;
                if message["type"] == "ready" {
                    io::check(message["url"] == "about:blank", "GUARD_URL_CHANGED")?;
                    self.guard_ready = true;
                } else {
                    self.violations
                        .push(format!("NAVIGATION_GUARD:{}", message));
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub fn send(
        &mut self,
        method: &str,
        params: Value,
        mut cancel: impl FnMut() -> Result<()>,
    ) -> Result<Value> {
        let id = self.next;
        self.next += 1;
        let payload = json!({"id":id,"method":method,"params":params}).to_string();
        self.socket
            .send(Message::Text(payload.into()))
            .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            cancel()?;
            io::check(Instant::now() < deadline, "CDP_COMMAND_TIMEOUT")?;
            match self.socket.read() {
                Ok(Message::Text(raw)) => {
                    let value: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                    if value["id"] == id {
                        if !value["error"].is_null() {
                            return Err(format!("CDP_{method}:{}", value["error"]));
                        }
                        return Ok(value["result"].clone());
                    }
                    self.event(&value)?;
                }
                Ok(Message::Ping(bytes)) => {
                    self.socket
                        .send(Message::Pong(bytes))
                        .map_err(|e| e.to_string())?;
                }
                Ok(Message::Close(_)) => return Err("CDP_CLOSED".into()),
                Ok(_) => {}
                Err(tungstenite::Error::Io(e))
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(e) => return Err(e.to_string()),
            }
        }
    }
    pub fn configure(
        &mut self,
        lock: &Value,
        plan: &Value,
        guard_source: &str,
        mut cancel: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        for method in [
            "Page.enable",
            "Runtime.enable",
            "Network.enable",
            "Log.enable",
            "Accessibility.enable",
        ] {
            self.send(method, json!({}), &mut cancel)?;
        }
        let supported=self.send("Runtime.evaluate",json!({"expression":"typeof globalThis.navigation === 'object' && typeof globalThis.navigation.addEventListener === 'function'","returnByValue":true}),&mut cancel)?;
        io::check(
            supported["result"]["value"] == true,
            "NAVIGATION_API_UNAVAILABLE",
        )?;
        let mut random = io::io(std::fs::File::open("/dev/urandom"))?;
        let mut bytes = [0u8; 32];
        io::io(random.read_exact(&mut bytes))?;
        self.guard_nonce = bytes.iter().map(|b| format!("{b:02x}")).collect();
        self.guard_binding = format!("__smcNavigationGuard_{}", &self.guard_nonce[..32]);
        self.send(
            "Runtime.addBinding",
            json!({"name":self.guard_binding}),
            &mut cancel,
        )?;
        let source = guard_source
            .split_once("String.raw`")
            .ok_or("GUARD_ASSET")?
            .1
            .split_once("`;\n")
            .ok_or("GUARD_ASSET")?
            .0
            .replace(
                "\"__SMC_NAVIGATION_GUARD_BINDING__\"",
                &json!(self.guard_binding).to_string(),
            )
            .replace(
                "\"__SMC_NAVIGATION_GUARD_NONCE__\"",
                &json!(self.guard_nonce).to_string(),
            );
        let registration = self.send(
            "Page.addScriptToEvaluateOnNewDocument",
            json!({"source":source,"runImmediately":true}),
            &mut cancel,
        )?;
        io::check(
            registration["identifier"].is_string(),
            "GUARD_INSTALLATION_FAILED",
        )?;
        self.send(
            "Runtime.evaluate",
            json!({"expression":"true","returnByValue":true}),
            &mut cancel,
        )?;
        io::check(self.guard_ready, "GUARD_READY_NOT_ATTESTED")?;
        self.send(
            "Network.setBlockedURLs",
            json!({"urls":["http://*","https://*","ws://*","wss://*","file:*","ftp:*"]}),
            &mut cancel,
        )?;
        self.send("Emulation.setDeviceMetricsOverride",json!({"width":plan["canvas"]["width"],"height":plan["canvas"]["height"],"deviceScaleFactor":1,"mobile":false,"screenWidth":plan["canvas"]["width"],"screenHeight":plan["canvas"]["height"]}),&mut cancel)?;
        self.send(
            "Emulation.setTimezoneOverride",
            json!({"timezoneId":lock["timezone"]}),
            &mut cancel,
        )?;
        self.send(
            "Emulation.setLocaleOverride",
            json!({"locale":lock["locale"]}),
            &mut cancel,
        )?;
        self.send("Emulation.setEmulatedMedia",json!({"features":[{"name":"prefers-color-scheme","value":lock["colorScheme"]},{"name":"prefers-reduced-motion","value":lock["reducedMotion"]}]}),&mut cancel)?;
        Ok(())
    }
}
