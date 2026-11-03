use std::env;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use promptgen_core::image::ImagePromptRequest;

use super::files::{create_private_file, read_bounded_bytes, read_bounded_utf8};
use super::limits::{MAX_LOG_BYTES, MAX_RESPONSE_BYTES};
use super::reference::ImageMime;
use super::{API_ENDPOINT, API_MODEL, OpenAiExecutionError, OpenAiRunConfig};

const POLL_INTERVAL: Duration = Duration::from_millis(10);

pub(super) struct CurlRequest<'a> {
    pub(super) api_key: &'a str,
    pub(super) prompt_path: &'a Path,
    pub(super) reference_path: &'a Path,
    pub(super) mime: ImageMime,
    pub(super) response_path: &'a Path,
    pub(super) headers_path: &'a Path,
    pub(super) request: &'a ImagePromptRequest,
    pub(super) timeout: Duration,
}

pub(super) fn read_api_key() -> Result<String, OpenAiExecutionError> {
    let key = env::var("OPENAI_API_KEY").map_err(|_| {
        OpenAiExecutionError::new(
            "OPENAI_API_KEY_MISSING",
            "OPENAI_API_KEY must be present in the environment",
        )
    })?;
    validate_api_key(&key)
}

pub(super) fn validate_api_key(key: &str) -> Result<String, OpenAiExecutionError> {
    if key.trim().is_empty() || key.chars().any(char::is_control) {
        return Err(OpenAiExecutionError::new(
            "OPENAI_API_KEY_INVALID",
            "OPENAI_API_KEY must be non-blank and contain no control characters",
        ));
    }
    Ok(key.to_owned())
}

pub(super) fn build_curl_config(input: &CurlRequest<'_>) -> String {
    let mut output = String::new();
    output.push_str("silent = true\nshow-error = true\nrequest = \"POST\"\n");
    output.push_str(&format!("url = {}\n", curl_quote(API_ENDPOINT)));
    output.push_str(&format!(
        "header = {}\n",
        curl_quote(&format!("Authorization: Bearer {}", input.api_key))
    ));
    output.push_str("header = \"Accept: application/json\"\n");
    output.push_str(&format!(
        "form = {}\n",
        curl_quote(&format!("model={API_MODEL}"))
    ));
    output.push_str(&format!(
        "form = {}\n",
        curl_quote(&format!("prompt=<{}", input.prompt_path.display()))
    ));
    output.push_str(&format!(
        "form = {}\n",
        curl_quote(&format!(
            "image[]=@{};type={}",
            input.reference_path.display(),
            input.mime.as_str()
        ))
    ));
    output.push_str(&format!(
        "form = {}\n",
        curl_quote(&format!(
            "size={}x{}",
            input.request.output.width, input.request.output.height
        ))
    ));
    output.push_str(&format!(
        "form = {}\n",
        curl_quote(&format!(
            "quality={}",
            input.request.output.quality.as_str()
        ))
    ));
    output.push_str(&format!(
        "form = {}\n",
        curl_quote(&format!(
            "background={}",
            input.request.output.background.as_str()
        ))
    ));
    output.push_str("form = \"output_format=png\"\n");
    output.push_str("form = \"n=1\"\n");
    output.push_str(&format!(
        "output = {}\n",
        curl_quote(&path_string(input.response_path))
    ));
    output.push_str(&format!(
        "dump-header = {}\n",
        curl_quote(&path_string(input.headers_path))
    ));
    output.push_str(&format!("write-out = {}\n", curl_quote("%{http_code}")));
    output.push_str(&format!("max-time = {}\n", input.timeout.as_secs().max(1)));
    output.push_str("connect-timeout = 20\n");
    output.push_str(&format!("max-filesize = {MAX_RESPONSE_BYTES}\n"));
    output
}

fn curl_quote(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str("\\x");
                output.push_str(&format!("{:02x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

pub(super) fn run_curl(
    config: &OpenAiRunConfig,
    working: &Path,
    curl_config: &str,
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<u16, OpenAiExecutionError> {
    let stdout_file = create_private_file(stdout_path)?;
    let stderr_file = create_private_file(stderr_path)?;
    let mut child = Command::new(&config.curl_binary)
        .arg("--disable")
        .arg("--config")
        .arg("-")
        .env_remove("OPENAI_API_KEY")
        .current_dir(working)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .map_err(|_| {
            OpenAiExecutionError::new("OPENAI_CURL_SPAWN", "curl process could not be started")
        })?;
    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(OpenAiExecutionError::new(
            "OPENAI_CURL_STDIN",
            "curl stdin could not be opened",
        ));
    };
    if let Err(error) = stdin.write_all(curl_config.as_bytes()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(OpenAiExecutionError::new(
            "OPENAI_CURL_STDIN",
            format!("curl configuration could not be written: {error}"),
        ));
    }
    drop(stdin);
    let deadline = Instant::now() + config.timeout.max(Duration::from_secs(1));
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(OpenAiExecutionError::new(
                    "OPENAI_TIMEOUT",
                    "curl process exceeded the configured timeout",
                ));
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(OpenAiExecutionError::new(
                    "OPENAI_CURL_WAIT",
                    format!("curl process status could not be observed: {error}"),
                ));
            }
        }
    };
    let _stderr = read_bounded_bytes(stderr_path, MAX_LOG_BYTES, "curl stderr")?;
    if !status.success() {
        return Err(OpenAiExecutionError::new(
            "OPENAI_CURL_FAILED",
            format!("curl exited with code {}", exit_code(status)),
        ));
    }
    let status_text = read_bounded_utf8(stdout_path, 64, "curl status")?;
    let status_text = status_text.trim();
    if status_text.len() != 3 || !status_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(OpenAiExecutionError::new(
            "OPENAI_HTTP_STATUS",
            "curl did not return one HTTP status code",
        ));
    }
    status_text.parse::<u16>().map_err(|_| {
        OpenAiExecutionError::new("OPENAI_HTTP_STATUS", "curl returned an invalid HTTP status")
    })
}

fn exit_code(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(-1)
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
