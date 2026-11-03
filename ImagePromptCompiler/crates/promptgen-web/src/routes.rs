mod catalog;
pub(crate) mod generation;

use promptgen_core::image::available_lut_presets;
use promptgen_core::json::JsonValue;
use std::net::TcpStream;

use self::catalog::{catalog_response_body, serve_catalog_asset};
use self::generation::{handle_generate, handle_interview};
use super::http::{HttpRequest, Response};
use super::security::validate_request_authority;
use super::server::ServerState;
use super::{APP_CSS, APP_JS, INDEX_HTML, READ_TIMEOUT, WebError};

pub(super) fn handle_connection(
    mut stream: TcpStream,
    state: &ServerState,
) -> Result<(), WebError> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(READ_TIMEOUT))?;
    let request = match HttpRequest::read_from(&mut stream, state.config.max_body_bytes) {
        Ok(request) => request,
        Err(error) => {
            let response = Response::json_error(error.status, error.code, error.message);
            response.write_to(&mut stream)?;
            return Ok(());
        }
    };
    let response = route(request, state);
    response.write_to(&mut stream)?;
    Ok(())
}

pub(super) fn route(request: HttpRequest, state: &ServerState) -> Response {
    if let Err(response) = validate_request_authority(&request, state) {
        return response;
    }
    let path = request.path.split('?').next().unwrap_or(&request.path);
    match (request.method.as_str(), path) {
        ("GET", "/") | ("GET", "/index.html") => {
            Response::text(200, "text/html; charset=utf-8", INDEX_HTML)
        }
        ("GET", "/app.css") => Response::text(200, "text/css; charset=utf-8", APP_CSS),
        ("GET", "/app.js") => Response::text(200, "text/javascript; charset=utf-8", APP_JS),
        ("GET", "/api/v1/health") => Response::json(
            200,
            JsonValue::object([
                ("ok", JsonValue::from(true)),
                ("service", JsonValue::from("promptgen-web")),
                ("version", JsonValue::from(env!("CARGO_PKG_VERSION"))),
            ]),
        ),
        ("GET", "/api/v1/catalog") => match catalog_response_body() {
            Ok(body) => Response::bytes(
                200,
                "application/json; charset=utf-8",
                body.as_bytes().to_vec(),
            ),
            Err(error) => Response::json_error(500, "CATALOG_LOAD", error.clone()),
        },
        ("GET", asset_path) if asset_path.starts_with("/api/v1/catalog/assets/") => {
            serve_catalog_asset(&request, asset_path, state)
        }
        ("GET", "/api/v1/lut-presets") => Response::json(
            200,
            JsonValue::object([
                ("ok", JsonValue::from(true)),
                ("presets", available_lut_presets()),
            ]),
        ),
        ("POST", "/api/v1/interview") => handle_interview(&request, state),
        ("POST", "/api/v1/generate") => handle_generate(&request, state),
        ("GET", artifact_path) if artifact_path.starts_with("/artifacts/") => {
            self::generation::serve_artifact(artifact_path, state)
        }
        _ => Response::json_error(404, "HTTP_NOT_FOUND", "route not found"),
    }
}
