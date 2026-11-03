from __future__ import annotations

from pathlib import Path
import base64
import binascii
import json
from typing import Any

from .png_inspection import inspect_png
from .util import unique_json_object

# The package boundary has one neutral executable identity. The upstream Cargo
# target may keep its source name; packaging copies its bytes to the canonical
# scene-provider-v2 path, which runtime identity checks require.
PROVIDER_PROTOCOL_SCHEMA = "misegen.provider/2"
PROVIDER_PROTOCOL_VERSION = 2
PROVIDER_BINARY_NAME = "scene-provider-v2"
PROVIDER_TOOL = "scene-provider-v2"
PERFECTPIXEL_INSPECTION_SCHEMA = "perfectpixel.asset-inspection/1"
PERFECTPIXEL_INSPECTION_SCHEMA_VERSION = 1
PERFECTPIXEL_CLI_VERSION = "0.3.1"
PERFECTPIXEL_ROLE = "deterministic-asset-compiler"
DEFAULT_PROVIDER_SHA256 = "c64929d27b21458de5a1b2806a7d5863641d35e4a3f47e2f2922f100ec27d7ae"
DEFAULT_PERFECTPIXEL_SHA256 = "c9521a2a6165ef34efc02048be3de19c25d8d338ebc807053c831781d432aad3"
PRODUCT_ROOT = Path(__file__).resolve().parents[5]
DEFAULT_COMPONENT_LOCK = PRODUCT_ROOT / "contracts" / "runtime-component-lock.json"
INSPECTION_TIMEOUT_SECONDS = 30.0
MAX_IMAGE_BYTES = 8 * 1024 * 1024
MAX_PROVIDER_STDOUT_BYTES = 16 * 1024 * 1024
SUPPORTED_SIZES = {"1024x1024", "1536x1024", "1024x1536"}
SUPPORTED_QUALITIES = {"auto", "low", "medium", "high"}
REQUIRED_ENVIRONMENT = (
    "VERGERAIL_CODEX_PACKAGE",
    "VERGERAIL_MODEL",
    "VERGERAIL_WORKSPACE",
)
UNKNOWN_PROVIDER_CODES = {"resolution_required", "timeout"}
PROCESS_TEARDOWN_GRACE_SECONDS = 2.0
PROCESS_TEARDOWN_FORCE_SECONDS = 2.0
PROVIDER_PROBE_TIMEOUT_SECONDS = 10.0
DISPATCH_EVIDENCE_NAME = "scene-provider-dispatch.json"
PROCESS_EVIDENCE_NAME = "scene-provider-process.json"
POSTFLIGHT_ERROR_EVIDENCE_NAME = "scene-provider-postflight-error.json"
def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    return unique_json_object(pairs, "COMPONENT_LOCK_DUPLICATE_KEY")


def _single_json_document(payload: bytes, *, error_code: str, maximum_bytes: int) -> Any:
    """Parse exactly one bounded JSON document, including pretty JSON.

    ``json.loads`` accepts only one document but its error does not make the
    boundary explicit.  Using ``raw_decode`` lets us reject a second document
    or non-whitespace trailing bytes deterministically while the pairs hook
    rejects duplicate keys at every object depth.
    """
    if not payload or len(payload) > maximum_bytes:
        raise ValueError(f"{error_code}_SIZE")
    try:
        text = payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError(f"{error_code}_UTF8") from error
    decoder = json.JSONDecoder(object_pairs_hook=_reject_duplicate_keys)
    start = len(text) - len(text.lstrip())
    try:
        value, end = decoder.raw_decode(text, start)
    except (json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"{error_code}_JSON") from error
    if text[end:].strip():
        raise ValueError(f"{error_code}_TRAILING")
    return value

def _request(task: dict, prompt: str, quality: str, timeout_ms: int, request_id: str) -> dict:
    if quality not in SUPPORTED_QUALITIES:
        raise ValueError("SCENE_PROVIDER_QUALITY_UNSUPPORTED")
    if not isinstance(timeout_ms, int) or isinstance(timeout_ms, bool) or not 1 <= timeout_ms <= 30 * 60 * 1000:
        raise ValueError("SCENE_PROVIDER_TIMEOUT_INVALID")
    output = task["output"]
    return {
        "schemaVersion": PROVIDER_PROTOCOL_VERSION,
        "requestId": request_id,
        "operation": "image_generate",
        "messages": [],
        "tools": [],
        "reasoning": "off",
        "timeoutMs": timeout_ms,
        "maximumResponseBytes": MAX_IMAGE_BYTES,
        "prompt": prompt,
        "imageOptions": {
            "background": "transparent" if output["transparent_background"] else "opaque",
            "size": output["size"],
            "quality": quality,
        },
    }


def _request_bytes(request: dict) -> bytes:
    """Serialize one provider request using the v2 wire representation."""
    return json.dumps(request, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def _response(stdout: bytes, request_id: str) -> dict:
    if not stdout or len(stdout) > MAX_PROVIDER_STDOUT_BYTES:
        raise ValueError("SCENE_PROVIDER_RESPONSE_SIZE")
    value = _single_json_document(
        stdout,
        error_code="SCENE_PROVIDER_RESPONSE",
        maximum_bytes=MAX_PROVIDER_STDOUT_BYTES,
    )
    if not isinstance(value, dict):
        raise ValueError("SCENE_PROVIDER_RESPONSE_TYPE")
    if type(value.get("schemaVersion")) is not int or value.get("schemaVersion") != PROVIDER_PROTOCOL_VERSION or value.get("requestId") != request_id:
        raise ValueError("SCENE_PROVIDER_RESPONSE_CORRELATION")
    return value


def _failure(value: dict) -> dict | None:
    if value.get("ok") is not False:
        return None
    if set(value) != {"schemaVersion", "requestId", "ok", "error"}:
        raise ValueError("SCENE_PROVIDER_FAILURE_FIELDS")
    error = value.get("error")
    if not isinstance(error, dict) or set(error) != {"code", "message", "retryable"}:
        raise ValueError("SCENE_PROVIDER_FAILURE_BODY")
    if not isinstance(error.get("code"), str) or not error["code"]:
        raise ValueError("SCENE_PROVIDER_FAILURE_CODE")
    if not isinstance(error.get("message"), str) or not isinstance(error.get("retryable"), bool):
        raise ValueError("SCENE_PROVIDER_FAILURE_BODY")
    return error


def _image(value: dict) -> tuple[bytes, dict]:
    if set(value) != {"schemaVersion", "requestId", "operation", "image"} or value.get("operation") != "image_generate":
        raise ValueError("SCENE_PROVIDER_SUCCESS_FIELDS")
    image = value.get("image")
    allowed = {"mediaType", "base64", "byteLength", "width", "height", "transparentBackground"}
    required = allowed - {"transparentBackground"}
    if not isinstance(image, dict) or set(image) - allowed or not required <= set(image):
        raise ValueError("SCENE_PROVIDER_IMAGE_FIELDS")
    if image.get("mediaType") != "image/png":
        raise ValueError("SCENE_PROVIDER_IMAGE_MEDIA_TYPE")
    encoded = image.get("base64")
    if not isinstance(encoded, str) or not encoded:
        raise ValueError("SCENE_PROVIDER_IMAGE_BASE64")
    try:
        data = base64.b64decode(encoded, validate=True)
    except (ValueError, binascii.Error) as error:
        raise ValueError("SCENE_PROVIDER_IMAGE_BASE64") from error
    if not 0 < len(data) <= MAX_IMAGE_BYTES or type(image.get("byteLength")) is not int or image.get("byteLength") != len(data):
        raise ValueError("SCENE_PROVIDER_IMAGE_BYTE_LENGTH")
    if type(image.get("width")) is not int or type(image.get("height")) is not int or image["width"] < 1 or image["height"] < 1:
        raise ValueError("SCENE_PROVIDER_IMAGE_DIMENSIONS")
    info = inspect_png(data)
    if image.get("width") != info["width"] or image.get("height") != info["height"]:
        raise ValueError("SCENE_PROVIDER_IMAGE_DIMENSIONS")
    observed = image.get("transparentBackground")
    if observed is not None and not isinstance(observed, bool):
        raise ValueError("SCENE_PROVIDER_IMAGE_TRANSPARENCY")
    return data, info
