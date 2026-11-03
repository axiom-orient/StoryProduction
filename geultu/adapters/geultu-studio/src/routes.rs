use geultu::{Application, Error};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::{SocketAddr, TcpStream};

use super::assets::{APP_JS, INDEX_HTML, STYLE_CSS};
use super::http::{read_request, respond, respond_json, SECURITY_HEADERS};
use super::security::{authorize, validate_host};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCall {
    name: String,
    #[serde(default = "empty_object")]
    arguments: Value,
}
fn empty_object() -> Value {
    json!({})
}

pub(super) fn handle(
    stream: &mut TcpStream,
    app: &Application,
    token: &str,
    listen: SocketAddr,
) -> Result<(), String> {
    let request = match read_request(stream) {
        Ok(request) => request,
        Err(error) => return reject_request(stream, 400, error),
    };
    if let Err(error) = validate_host(&request, listen) {
        return reject_request(stream, 403, error);
    }
    let path = request.path.split('?').next().unwrap_or("/");
    match (request.method.as_str(), path) {
        ("GET", "/") => {
            let html = INDEX_HTML.replace("{{TOKEN}}", token);
            respond(
                stream,
                200,
                "text/html; charset=utf-8",
                html.as_bytes(),
                SECURITY_HEADERS,
            )
        }
        ("GET", "/app.js") => respond(
            stream,
            200,
            "text/javascript; charset=utf-8",
            APP_JS.as_bytes(),
            SECURITY_HEADERS,
        ),
        ("GET", "/style.css") => respond(
            stream,
            200,
            "text/css; charset=utf-8",
            STYLE_CSS.as_bytes(),
            SECURITY_HEADERS,
        ),
        ("GET", "/healthz") => respond_json(
            stream,
            200,
            &json!({"ok":true,"adapter":"GeultuStudio","engine":"geultu"}),
        ),
        ("GET", "/api/tools") => {
            if let Err(error) = authorize(&request, token) {
                return reject_request(stream, 403, error);
            }
            respond_json(
                stream,
                200,
                &json!({"ok":true,"result":app.tool_definitions()}),
            )
        }
        ("POST", "/api/tools/call") => {
            if let Err(error) = authorize(&request, token) {
                return reject_request(stream, 403, error);
            }
            let call: ToolCall = match serde_json::from_slice(&request.body) {
                Ok(call) => call,
                Err(error) => {
                    return reject_request(stream, 400, format!("invalid request JSON: {error}"))
                }
            };
            match app.call(&call.name, call.arguments) {
                Ok(result) => respond_json(stream, 200, &json!({"ok":true,"result":result})),
                Err(error) => {
                    let status = error_status(&error);
                    respond_json(stream, status, &json!({"ok":false,"error":error.body()}))
                }
            }
        }
        _ => respond_json(
            stream,
            404,
            &json!({"ok":false,"error":{"code":"not_found","message":"route was not found"}}),
        ),
    }
}

fn reject_request(stream: &mut TcpStream, status: u16, message: String) -> Result<(), String> {
    respond_json(
        stream,
        status,
        &json!({"ok":false,"error":{"code":"adapter","message":message}}),
    )
}

fn error_status(error: &Error) -> u16 {
    match error {
        Error::InvalidInput(_) | Error::BudgetExhausted(_) => 400,
        Error::NotFound(_) => 404,
        Error::Conflict(_) | Error::Busy(_) => 409,
        Error::AdapterUnavailable(_) => 503,
        Error::Corrupt(_)
        | Error::Io(_)
        | Error::Json(_)
        | Error::CommittedButSnapshotFailed { .. } => 500,
    }
}
