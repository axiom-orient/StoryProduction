"""Probe sealed Scene Provider/PerfectPixel binaries before any asset dispatch."""
from __future__ import annotations

from pathlib import Path
import json
import uuid
from typing import Any

from .scene_provider_identity import _perfectpixel_path, _provider_lock_identity
from .scene_provider_process import _run_bounded_process
from .scene_provider_protocol import (
    INSPECTION_TIMEOUT_SECONDS,
    MAX_PROVIDER_STDOUT_BYTES,
    PROVIDER_PROBE_TIMEOUT_SECONDS,
    PROVIDER_PROTOCOL_SCHEMA,
    PROVIDER_PROTOCOL_VERSION,
    _failure,
    _single_json_document,
)
from .util import sha256_file


def _probe_provider_protocol_binary(
    provider: Path,
    provider_sha256: str,
    *,
    timeout_seconds: float = PROVIDER_PROBE_TIMEOUT_SECONDS,
) -> dict[str, Any]:
    """Observe the v2 envelope from the exact sealed provider executable.

    ``protocol_probe`` is intentionally not a provider operation.  A v2
    provider must reject the unknown operation before authentication or image
    generation, while still emitting its v2 typed error envelope.  The real
    provider does not echo a protocol-schema label on the wire, so the
    ``misegen.provider/2`` label is attached only after observing the v2
    response envelope and is explicitly recorded as probe-contract evidence,
    never copied from the component lock.
    """
    request_id = f"scene-asset-compiler-probe-{uuid.uuid4()}"
    request = {
        "schemaVersion": PROVIDER_PROTOCOL_VERSION,
        "requestId": request_id,
        "operation": "protocol_probe",
        "messages": [],
        "tools": [],
        "reasoning": "off",
        "timeoutMs": 1000,
        "maximumResponseBytes": 4096,
        "prompt": None,
    }
    stdout, stderr, custody = _run_bounded_process(
        [str(provider)],
        input_bytes=json.dumps(request, separators=(",", ":")).encode("utf-8"),
        timeout_seconds=timeout_seconds,
    )
    if custody.get("returncode") != 0 or stderr:
        raise ValueError("SCENE_PROVIDER_PROTOCOL_PROBE_TRANSPORT")
    response = _single_json_document(
        stdout,
        error_code="SCENE_PROVIDER_PROTOCOL_PROBE",
        maximum_bytes=MAX_PROVIDER_STDOUT_BYTES,
    )
    if not isinstance(response, dict):
        raise ValueError("SCENE_PROVIDER_PROTOCOL_PROBE_TYPE")
    response_schema_version = response.get("schemaVersion")
    if response_schema_version != PROVIDER_PROTOCOL_VERSION:
        raise ValueError(
            f"SCENE_PROVIDER_PROTOCOL_PROBE_SCHEMA_VERSION:expected={PROVIDER_PROTOCOL_VERSION}:observed={response_schema_version}"
        )
    if response.get("requestId") != request_id:
        raise ValueError("SCENE_PROVIDER_PROTOCOL_PROBE_CORRELATION")
    # The current provider proves its wire family through the v2 envelope and
    # typed rejection.  A newer provider may also echo the wrapper label and
    # explicit protocolVersion; validate those optional observations without
    # weakening the strict failure-envelope parser used for image responses.
    probe_failure = dict(response)
    has_echoed_schema = "protocolSchema" in probe_failure
    echoed_schema = probe_failure.pop("protocolSchema", None)
    has_explicit_protocol_version = "protocolVersion" in probe_failure
    explicit_protocol_version = probe_failure.pop("protocolVersion", None)
    if has_echoed_schema and echoed_schema != PROVIDER_PROTOCOL_SCHEMA:
        raise ValueError("SCENE_PROVIDER_PROTOCOL_PROBE_PROTOCOL_SCHEMA")
    if has_explicit_protocol_version and explicit_protocol_version != PROVIDER_PROTOCOL_VERSION:
        raise ValueError("SCENE_PROVIDER_PROTOCOL_PROBE_PROTOCOL_VERSION")
    error = _failure(probe_failure)
    if error is None or error.get("code") != "invalid_request" or error.get("retryable") is not False:
        raise ValueError("SCENE_PROVIDER_PROTOCOL_PROBE_NOT_REJECTED")
    return {
        "status": "PASS",
        "protocolSchema": PROVIDER_PROTOCOL_SCHEMA,
        "protocolSchemaEvidence": "response.protocolSchema" if has_echoed_schema else "v2_response_envelope_and_invalid_request",
        "protocolVersion": explicit_protocol_version if has_explicit_protocol_version else response_schema_version,
        "protocolVersionEvidence": "response.protocolVersion" if has_explicit_protocol_version else "response.schemaVersion",
        "schemaVersion": response_schema_version,
        "providerSha256": provider_sha256,
        "request": request,
        "response": response,
        "custody": custody,
    }


def _preflight_provider(
    provider_bin: Path,
    component_lock: Path | None = None,
    *,
    timeout_seconds: float = PROVIDER_PROBE_TIMEOUT_SECONDS,
) -> dict[str, Any]:
    provider, lock, lock_path, lock_sha256, provider_sha256 = _provider_lock_identity(provider_bin, component_lock)
    provider_probe = _probe_provider_protocol_binary(provider, provider_sha256, timeout_seconds=timeout_seconds)
    # Re-hash after the child exits.  A replacement during the probe cannot be
    # allowed to turn observed protocol evidence into dispatch authorization.
    post_probe_sha256 = sha256_file(provider)
    if post_probe_sha256 != provider_sha256:
        raise ValueError(
            f"PROVIDER_IDENTITY_CHANGED_DURING_PROBE:before={provider_sha256}:after={post_probe_sha256}"
        )
    if provider_probe["providerSha256"] != provider_sha256:
        raise ValueError("SCENE_PROVIDER_PROTOCOL_PROBE_HASH")
    return {
        "lock": lock,
        "lockPath": lock_path,
        "lockSha256": lock_sha256,
        "provider": {
            "path": str(provider),
            "sha256": provider_sha256,
            # These are observed from the probe response.  The lock remains
            # the expected contract, not the proof of the running binary.
            "protocolSchema": provider_probe["protocolSchema"],
            "protocolVersion": provider_probe["protocolVersion"],
            "schemaVersion": provider_probe["schemaVersion"],
            "protocolProbe": provider_probe,
        },
    }


def preflight_provider(provider_bin: Path, component_lock: Path | None = None) -> dict[str, Any]:
    """Seal provider bytes and require an observed v2 protocol probe."""
    identity = _preflight_provider(provider_bin, component_lock)
    return {
        "lockPath": str(identity["lockPath"]),
        "lockSha256": identity["lockSha256"],
        "provider": dict(identity["provider"]),
    }


def _probe_perfectpixel_schema(
    perfectpixel: Path,
    expected: dict[str, Any],
    *,
    timeout_seconds: float = INSPECTION_TIMEOUT_SECONDS,
) -> dict[str, Any]:
    """Probe the actual PerfectPixel executable before any asset dispatch."""
    stdout, stderr, custody = _run_bounded_process(
        [str(perfectpixel), "schema"],
        input_bytes=b"",
        timeout_seconds=timeout_seconds,
        cwd=perfectpixel.parent,
    )
    if custody.get("returncode") != 0 or stderr:
        raise ValueError("PERFECTPIXEL_SCHEMA_PROBE_TRANSPORT")
    schema = _single_json_document(
        stdout,
        error_code="PERFECTPIXEL_SCHEMA_PROBE",
        maximum_bytes=MAX_PROVIDER_STDOUT_BYTES,
    )
    if not isinstance(schema, dict):
        raise ValueError("PERFECTPIXEL_SCHEMA_PROBE_TYPE")
    if schema.get("cliVersion") != expected["cliVersion"]:
        raise ValueError(
            f"PERFECTPIXEL_SCHEMA_CLI_VERSION:expected={expected['cliVersion']}:observed={schema.get('cliVersion')}"
        )
    if schema.get("inspectSchema") != expected["inspectionSchema"]:
        raise ValueError("PERFECTPIXEL_SCHEMA_INSPECTION_SCHEMA")
    if schema.get("role") != expected["role"]:
        raise ValueError("PERFECTPIXEL_SCHEMA_ROLE")
    commands = schema.get("commands")
    if not isinstance(commands, list) or any(not isinstance(command, str) for command in commands):
        raise ValueError("PERFECTPIXEL_SCHEMA_COMMANDS")
    if not {"schema", "inspect"}.issubset(commands):
        raise ValueError("PERFECTPIXEL_SCHEMA_COMMANDS_REQUIRED")
    adapter = schema.get("assetAdapter")
    if not isinstance(adapter, dict) or adapter.get("inspectionSchema") != expected["inspectionSchema"]:
        raise ValueError("PERFECTPIXEL_SCHEMA_ASSET_ADAPTER")
    return {
        "cliVersion": schema["cliVersion"],
        "inspectionSchema": schema["inspectSchema"],
        "role": schema["role"],
        "commands": commands,
        "custody": custody,
    }


def preflight_components(
    provider_bin: Path,
    perfectpixel_bin: Path,
    component_lock: Path | None = None,
) -> dict[str, Any]:
    """Seal provider v2 and standalone PerfectPixel inspection before dispatch."""
    provider_identity = _preflight_provider(Path(provider_bin), component_lock)
    provider = Path(provider_identity["provider"]["path"])
    perfectpixel = _perfectpixel_path(Path(perfectpixel_bin))
    lock = provider_identity["lock"]
    lock_path = provider_identity["lockPath"]
    lock_sha256 = provider_identity["lockSha256"]
    provider_sha256 = provider_identity["provider"]["sha256"]
    perfectpixel_sha256 = sha256_file(perfectpixel)
    expected_perfectpixel = lock["perfectpixel"]["sha256"]
    if perfectpixel_sha256 != expected_perfectpixel:
        raise ValueError(f"PERFECTPIXEL_IDENTITY_MISMATCH:expected={expected_perfectpixel}:observed={perfectpixel_sha256}")
    perfectpixel_lock = lock["perfectpixel"]
    schema_probe = _probe_perfectpixel_schema(perfectpixel, perfectpixel_lock)
    return {
        "lockPath": str(lock_path),
        "lockSha256": lock_sha256,
        "provider": dict(provider_identity["provider"]),
        "perfectpixel": {
            "path": str(perfectpixel),
            "sha256": perfectpixel_sha256,
            "role": perfectpixel_lock["role"],
            "inspectionSchema": perfectpixel_lock["inspectionSchema"],
            "inspectionSchemaVersion": perfectpixel_lock["inspectionSchemaVersion"],
            "cliVersion": perfectpixel_lock["cliVersion"],
            "commands": list(perfectpixel_lock["commands"]),
            "sourceManifestSha256": perfectpixel_lock["sourceManifestSha256"],
            "schemaProbe": schema_probe,
        },
    }





def probe_scene_provider_protocol(
    provider_bin: Path,
    *,
    component_lock: Path | None = None,
    timeout_seconds: float = 10.0,
) -> dict[str, Any]:
    """Probe the Scene Provider with a deliberately unsupported v2 operation.

    The probe must return one typed v2 ``invalid_request`` record and never reaches
    a model or image-generation operation.
    """
    # This probe is deliberately independent of PerfectPixel.  It proves the
    # provider envelope without dispatching an image-generation operation.
    identity = _preflight_provider(
        provider_bin,
        component_lock,
        timeout_seconds=timeout_seconds,
    )
    return dict(identity["provider"]["protocolProbe"])
