use std::fs;
use std::sync::OnceLock;
use std::time::UNIX_EPOCH;

use promptgen_core::catalog::{catalog_json, find_entry};
use promptgen_core::json::JsonValue;

use super::super::http::{HttpRequest, Response};
use super::super::server::ServerState;

const MAX_CATALOG_ASSET_BYTES: u64 = 12 * 1024 * 1024;

fn asset_validator(metadata: &fs::Metadata) -> String {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |value| value.as_nanos());
    format!("\"{:x}-{modified:x}\"", metadata.len())
}

pub(super) fn catalog_response_body() -> Result<&'static String, &'static String> {
    static BODY: OnceLock<Result<String, String>> = OnceLock::new();
    BODY.get_or_init(|| {
        let value = catalog_json(None).map_err(|error| error.to_string())?;
        Ok(
            JsonValue::object([("ok", JsonValue::from(true)), ("catalog", value)])
                .to_compact_string(),
        )
    })
    .as_ref()
}

pub(super) fn serve_catalog_asset(
    request: &HttpRequest,
    path: &str,
    state: &ServerState,
) -> Response {
    let id = path.trim_start_matches("/api/v1/catalog/assets/");
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Response::json_error(404, "CATALOG_ASSET_NOT_FOUND", "catalog asset not found");
    }
    let entry = match find_entry(id) {
        Ok(Some(entry)) => entry,
        Ok(None) | Err(_) => {
            return Response::json_error(404, "CATALOG_ASSET_NOT_FOUND", "catalog asset not found");
        }
    };
    let Some(root) = state.catalog_dir.as_ref() else {
        return Response::json_error(
            503,
            "CATALOG_ASSET_ROOT_UNAVAILABLE",
            "no catalog directory is configured; set PROMPTGEN_CATALOG_DIR",
        );
    };
    let asset = root.join(&entry.asset_path);
    let metadata = match fs::metadata(&asset) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_CATALOG_ASSET_BYTES => metadata,
        _ => {
            return Response::json_error(404, "CATALOG_ASSET_NOT_FOUND", "catalog asset not found");
        }
    };
    if metadata.len() == 0 {
        return Response::json_error(404, "CATALOG_ASSET_NOT_FOUND", "catalog asset not found");
    }
    let validator = asset_validator(&metadata);
    if request.headers.get("if-none-match").is_some_and(|value| {
        value
            .split(',')
            .any(|candidate| candidate.trim() == validator)
    }) {
        return Response::not_modified(validator);
    }
    match fs::read(asset) {
        Ok(bytes) => Response::cached_bytes("image/png", bytes, validator),
        Err(_) => Response::json_error(404, "CATALOG_ASSET_NOT_FOUND", "catalog asset not found"),
    }
}
