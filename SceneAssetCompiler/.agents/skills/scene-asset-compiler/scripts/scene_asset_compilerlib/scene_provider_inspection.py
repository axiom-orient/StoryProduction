from __future__ import annotations

from pathlib import Path
from typing import Any

from .png_inspection import inspect_png
from .scene_provider_process import _run_bounded_process
from .scene_provider_protocol import (
    INSPECTION_TIMEOUT_SECONDS,
    MAX_PROVIDER_STDOUT_BYTES,
    PERFECTPIXEL_INSPECTION_SCHEMA,
    PERFECTPIXEL_INSPECTION_SCHEMA_VERSION,
    _single_json_document,
)

def _inspect_with_perfectpixel(
    perfectpixel: Path,
    output_path: Path,
    *,
    output_sha256: str,
    identity: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, Any]]:
    stdout, stderr, custody = _run_bounded_process(
        [str(perfectpixel), "inspect", str(output_path)],
        input_bytes=b"",
        timeout_seconds=INSPECTION_TIMEOUT_SECONDS,
        cwd=output_path.parent,
    )
    if custody.get("returncode") != 0 or stderr:
        raise ValueError("PERFECTPIXEL_INSPECT_TRANSPORT")
    inspection = _single_json_document(
        stdout,
        error_code="PERFECTPIXEL_INSPECT",
        maximum_bytes=MAX_PROVIDER_STDOUT_BYTES,
    )
    if not isinstance(inspection, dict) or inspection.get("ok") is not True:
        raise ValueError("PERFECTPIXEL_INSPECT_FAILED")
    if (
        inspection.get("schema") != PERFECTPIXEL_INSPECTION_SCHEMA
        or inspection.get("schemaVersion") != PERFECTPIXEL_INSPECTION_SCHEMA_VERSION
    ):
        raise ValueError("PERFECTPIXEL_INSPECT_SCHEMA")
    if not isinstance(inspection.get("input"), str) or not inspection["input"].strip():
        raise ValueError("PERFECTPIXEL_INSPECT_INPUT")
    if inspection.get("inputSha256") != output_sha256:
        raise ValueError(
            f"PERFECTPIXEL_INSPECTION_STALE:expected={output_sha256}:observed={inspection.get('inputSha256')}"
        )
    output_size_bytes = output_path.stat().st_size
    input_byte_count = inspection.get("inputByteCount")
    if type(input_byte_count) is not int or input_byte_count != output_size_bytes:
        raise ValueError("PERFECTPIXEL_INSPECT_BYTE_LENGTH")
    artifact = inspection.get("artifact")
    if (
        not isinstance(artifact, dict)
        or artifact.get("sha256") != output_sha256
        or artifact.get("mediaType") != "image/png"
        or type(artifact.get("bytes")) is not int
        or artifact.get("bytes") != output_size_bytes
    ):
        raise ValueError("PERFECTPIXEL_INSPECT_ARTIFACT")
    width = inspection.get("width")
    height = inspection.get("height")
    if type(width) is not int or type(height) is not int or width < 1 or height < 1:
        raise ValueError("PERFECTPIXEL_INSPECT_DIMENSIONS")
    try:
        actual_png = inspect_png(output_path.read_bytes())
    except Exception as error:
        raise ValueError(f"PERFECTPIXEL_INSPECT_OUTPUT_INVALID:{error}") from error
    if (width, height) != (actual_png["width"], actual_png["height"]):
        raise ValueError(
            f"PERFECTPIXEL_INSPECT_DIMENSIONS:{width}x{height}!={actual_png['width']}x{actual_png['height']}"
        )
    has_alpha = inspection.get("hasAlpha")
    if type(has_alpha) is not bool or has_alpha is not actual_png["has_transparent_pixels"]:
        raise ValueError("PERFECTPIXEL_INSPECT_ALPHA")
    pixel_spec = inspection.get("pixelSpec")
    expected_alpha = "straight" if actual_png["has_transparent_pixels"] else "opaque"
    if not isinstance(pixel_spec, dict) or pixel_spec.get("alpha") != expected_alpha:
        raise ValueError("PERFECTPIXEL_INSPECT_ALPHA_SEMANTICS")
    # Do not retain an absolute temporary/workspace path in durable attempt
    # evidence.  The hash is the stable input identity.
    normalized = dict(inspection)
    normalized["input"] = "outputs/generated.png"
    evidence = {
        "schema_version": 1,
        "schema": PERFECTPIXEL_INSPECTION_SCHEMA,
        "tool": "perfectpixel",
        "executable_sha256": identity["perfectpixel"]["sha256"],
        "inspection_schema": identity["perfectpixel"]["inspectionSchema"],
        "inspection_schema_version": identity["perfectpixel"]["inspectionSchemaVersion"],
        "cli_version": identity["perfectpixel"]["cliVersion"],
        "role": identity["perfectpixel"]["role"],
        "source_manifest_sha256": identity["perfectpixel"]["sourceManifestSha256"],
        "input": {
            "path": "outputs/generated.png",
            "sha256": output_sha256,
            "size_bytes": output_size_bytes,
        },
        "inspection": normalized,
        "process": {
            "leader_pid": custody["leader_pid"],
            "process_group": custody["process_group"],
            "leader_reaped": custody["leader_reaped"],
            "no_survivors": custody["no_survivors"],
            "pipes_closed": custody["pipes_closed"],
        },
    }
    return evidence, identity
