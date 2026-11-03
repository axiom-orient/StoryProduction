from __future__ import annotations

from pathlib import Path
import os
import subprocess
import time
import uuid

from .util import atomic_bytes, exclusive_json, sha256_bytes, sha256_file
from .scene_provider_protocol import (
    DISPATCH_EVIDENCE_NAME,
    PROCESS_EVIDENCE_NAME,
    PROCESS_TEARDOWN_FORCE_SECONDS,
    PROCESS_TEARDOWN_GRACE_SECONDS,
    PROVIDER_PROTOCOL_SCHEMA,
    PROVIDER_PROTOCOL_VERSION,
    PROVIDER_TOOL,
    UNKNOWN_PROVIDER_CODES,
    _failure,
    _image,
    _request,
    _request_bytes,
    _response,
)
from .scene_provider_process import (
    _custody_notes,
    _process_group_exists,
    _provider_environment,
    _teardown_process_group,
)
from .scene_provider_inspection import _inspect_with_perfectpixel
from .scene_provider_preflight import preflight_components
from .scene_provider_attempt import _attempt_contract, _finish_attempt_safe, _utc_now


def execute_scene_provider(
    run_dir: Path,
    task_id: str,
    attempt_no: int,
    provider_bin: Path,
    *,
    quality: str,
    timeout_ms: int = 10 * 60 * 1000,
    dry_run: bool = False,
    perfectpixel_bin: Path | None = None,
    component_lock: Path | None = None,
) -> dict:
    """Execute one immutable, reference-free PNG attempt through Scene Provider v2."""
    attempt_dir, task, _attempt = _attempt_contract(Path(run_dir), task_id, attempt_no)
    perfectpixel_value = perfectpixel_bin or os.environ.get("SCENE_ASSET_COMPILER_PERFECTPIXEL_BIN")
    if not perfectpixel_value:
        raise ValueError("PERFECTPIXEL_BINARY_REQUIRED")
    identity = preflight_components(Path(provider_bin), Path(perfectpixel_value), component_lock)
    provider = Path(identity["provider"]["path"])
    perfectpixel = Path(identity["perfectpixel"]["path"])
    environment = _provider_environment()
    prompt = (attempt_dir / "generation-capsule.md").read_text(encoding="utf-8")
    if not prompt.strip():
        raise ValueError("SCENE_PROVIDER_CAPSULE_EMPTY")
    request_id = f"scene-asset-compiler-image-{uuid.uuid4()}"
    request = _request(task, prompt, quality, timeout_ms, request_id)
    provider_sha256 = identity["provider"]["sha256"]
    provider_protocol_probe = identity["provider"]["protocolProbe"]
    request_bytes = _request_bytes(request)
    if dry_run:
        return {
            "status": "READY",
            "provider": str(provider),
            "providerSha256": provider_sha256,
            "providerProtocolSchema": identity["provider"]["protocolSchema"],
            "componentLockSha256": identity["lockSha256"],
            "perfectpixel": identity["perfectpixel"],
            "request": {
                "schemaVersion": request["schemaVersion"],
                "operation": request["operation"],
                "reasoning": request["reasoning"],
                "timeoutMs": request["timeoutMs"],
                "maximumResponseBytes": request["maximumResponseBytes"],
                "imageOptions": request["imageOptions"],
                "promptBytes": len(prompt.encode("utf-8")),
                "promptSha256": sha256_bytes(prompt.encode("utf-8")),
            },
        }

    started_at = _utc_now()
    common_notes = [
        f"scene_provider.protocol_schema={PROVIDER_PROTOCOL_SCHEMA}",
        f"scene_provider.protocol_version={PROVIDER_PROTOCOL_VERSION}",
        f"scene_provider.protocol_probe_schema={provider_protocol_probe['protocolSchema']}",
        f"scene_provider.protocol_probe_version={provider_protocol_probe['protocolVersion']}",
        f"scene_provider.protocol_probe_no_survivors={str(provider_protocol_probe['custody']['no_survivors']).lower()}",
        f"scene_provider.provider_sha256={provider_sha256}",
        f"scene_asset_compiler.component_lock_sha256={identity['lockSha256']}",
        f"perfectpixel.sha256={identity['perfectpixel']['sha256']}",
        f"perfectpixel.inspection_schema={identity['perfectpixel']['inspectionSchema']}",
        f"perfectpixel.cli_version={identity['perfectpixel']['cliVersion']}",
        f"perfectpixel.source_manifest_sha256={identity['perfectpixel']['sourceManifestSha256']}",
        f"scene_provider.timeout_ms={timeout_ms}",
        f"scene_provider.quality={quality}",
        f"scene_provider.background={request['imageOptions']['background']}",
        f"scene_provider.size={request['imageOptions']['size']}",
    ]
    try:
        exclusive_json(
            attempt_dir / DISPATCH_EVIDENCE_NAME,
            {
                "schema_version": 1,
                "task_id": task_id,
                "attempt_no": attempt_no,
                "request_id": request_id,
                "created_at": started_at,
                "provider": PROVIDER_TOOL,
                "provider_sha256": provider_sha256,
                "provider_protocol_schema": identity["provider"]["protocolSchema"],
                "provider_protocol_version": identity["provider"]["protocolVersion"],
                "provider_schema_version": identity["provider"]["schemaVersion"],
                "provider_protocol_probe": provider_protocol_probe,
                "component_lock_sha256": identity["lockSha256"],
                "perfectpixel_sha256": identity["perfectpixel"]["sha256"],
                "perfectpixel_inspection_schema": identity["perfectpixel"]["inspectionSchema"],
                "perfectpixel_inspection_schema_version": identity["perfectpixel"]["inspectionSchemaVersion"],
                "perfectpixel_cli_version": identity["perfectpixel"]["cliVersion"],
                "perfectpixel_source_manifest_sha256": identity["perfectpixel"]["sourceManifestSha256"],
                "request_frame_sha256": sha256_bytes(request_bytes),
                "capsule_sha256": sha256_bytes(prompt.encode("utf-8")),
                "operation": request["operation"],
                "timeout_ms": timeout_ms,
                "maximum_response_bytes": request["maximumResponseBytes"],
                "image_options": request["imageOptions"],
            },
        )
    except FileExistsError as error:
        raise ValueError(f"SCENE_PROVIDER_ATTEMPT_ALREADY_EXECUTED:{DISPATCH_EVIDENCE_NAME}") from error
    process_started_at = time.monotonic_ns()
    try:
        process = subprocess.Popen(
            [str(provider)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=environment["VERGERAIL_WORKSPACE"],
            env=environment,
            start_new_session=True,
        )
        process_group = process.pid
        exclusive_json(
            attempt_dir / PROCESS_EVIDENCE_NAME,
            {
                "schema_version": 1,
                "task_id": task_id,
                "attempt_no": attempt_no,
                "request_id": request_id,
                "leader_pid": process.pid,
                "process_group": process_group,
                "monotonic_started_ns": process_started_at,
                "operation_deadline_ns": process_started_at + (timeout_ms + 10000) * 1_000_000,
                "teardown_grace_ms": int(PROCESS_TEARDOWN_GRACE_SECONDS * 1000),
                "teardown_force_ms": int(PROCESS_TEARDOWN_FORCE_SECONDS * 1000),
            },
        )
        stdout, stderr = process.communicate(input=request_bytes, timeout=(timeout_ms / 1000) + 10)
    except subprocess.TimeoutExpired:
        finished_at = _utc_now()
        _stdout, _stderr, custody = _teardown_process_group(process, process_group)
        finished, incomplete = _finish_attempt_safe(
            attempt_dir, run_dir, task_id, attempt_no,
            request_id=request_id,
            started_at=started_at,
            finished_at=finished_at,
            outcome="UNKNOWN",
            notes=common_notes + _custody_notes(custody) + ["SCENE_PROVIDER_EXECUTOR_TIMEOUT; billed outcome may be unknown; do not retry this attempt."],
        )
        if incomplete is not None:
            return incomplete
        receipt, manifest, review = finished
        return {"status": "OUTCOME_UNKNOWN", "receipt": receipt, "result": manifest, "mechanicalReview": review}
    except OSError as error:
        finished_at = _utc_now()
        custody_notes = []
        custody_incomplete = False
        if "process" in locals() and "process_group" in locals():
            _stdout, _stderr, custody = _teardown_process_group(process, process_group)
            custody_notes = _custody_notes(custody)
            custody_incomplete = not custody["leader_reaped"] or not custody["no_survivors"]
        outcome = "UNKNOWN" if custody_incomplete else "FAILED"
        finished, incomplete = _finish_attempt_safe(
            attempt_dir, run_dir, task_id, attempt_no,
            request_id=request_id,
            started_at=started_at,
            finished_at=finished_at,
            outcome=outcome,
            notes=common_notes + custody_notes + [f"SCENE_PROVIDER_START_FAILED:{type(error).__name__}:{error}"],
        )
        if incomplete is not None:
            return incomplete
        receipt, manifest, review = finished
        return {
            "status": "OUTCOME_UNKNOWN" if outcome == "UNKNOWN" else "PROVIDER_FAILED",
            "receipt": receipt,
            "result": manifest,
            "mechanicalReview": review,
        }

    finished_at = _utc_now()
    custody = {
        "leader_pid": process.pid,
        "process_group": process_group,
        "term_sent": False,
        "kill_sent": False,
        "pipes_closed": True,
        "leader_reaped": process.poll() is not None,
        "no_survivors": not _process_group_exists(process_group),
    }
    if not custody["no_survivors"]:
        _extra_stdout, _extra_stderr, custody = _teardown_process_group(process, process_group)
    common_notes += _custody_notes(custody)
    if not custody["leader_reaped"] or not custody["no_survivors"]:
        finished, incomplete = _finish_attempt_safe(
            attempt_dir, run_dir, task_id, attempt_no,
            request_id=request_id,
            started_at=started_at,
            finished_at=finished_at,
            outcome="UNKNOWN",
            notes=common_notes + ["SCENE_PROVIDER_PROCESS_CUSTODY_INCOMPLETE; do not retry this attempt."],
        )
        if incomplete is not None:
            return incomplete
        receipt, manifest, review = finished
        return {"status": "OUTCOME_UNKNOWN", "receipt": receipt, "result": manifest, "mechanicalReview": review}
    if process.returncode != 0 or stderr:
        finished, incomplete = _finish_attempt_safe(
            attempt_dir, run_dir, task_id, attempt_no,
            request_id=request_id,
            started_at=started_at,
            finished_at=finished_at,
            outcome="UNKNOWN",
            notes=common_notes + [f"SCENE_PROVIDER_TRANSPORT_FAILURE:returncode={process.returncode}; billed outcome may be unknown; do not retry this attempt."],
        )
        if incomplete is not None:
            return incomplete
        receipt, manifest, review = finished
        return {"status": "OUTCOME_UNKNOWN", "receipt": receipt, "result": manifest, "mechanicalReview": review}

    try:
        response = _response(stdout, request_id)
        error = _failure(response)
        if error is not None:
            outcome = "UNKNOWN" if error["code"] in UNKNOWN_PROVIDER_CODES or "OutcomeUnknown" in error["message"] else "FAILED"
            finished, incomplete = _finish_attempt_safe(
                attempt_dir, run_dir, task_id, attempt_no,
                request_id=request_id,
                started_at=started_at,
                finished_at=finished_at,
                outcome=outcome,
                notes=common_notes + [f"SCENE_PROVIDER_FAILURE:{error['code']}:{error['message']}"],
            )
            if incomplete is not None:
                return incomplete
            receipt, manifest, review = finished
            return {
                "status": "OUTCOME_UNKNOWN" if outcome == "UNKNOWN" else "PROVIDER_FAILED",
                "error": error,
                "receipt": receipt,
                "result": manifest,
                "mechanicalReview": review,
            }
        data, info = _image(response)
    except Exception as error:
        finished, incomplete = _finish_attempt_safe(
            attempt_dir, run_dir, task_id, attempt_no,
            request_id=request_id,
            started_at=started_at,
            finished_at=finished_at,
            outcome="UNKNOWN",
            notes=common_notes + [f"SCENE_PROVIDER_RESPONSE_INVALID:{error}; billed outcome may be unknown; do not retry this attempt."],
        )
        if incomplete is not None:
            return incomplete
        receipt, manifest, review = finished
        return {"status": "OUTCOME_UNKNOWN", "receipt": receipt, "result": manifest, "mechanicalReview": review}

    output_path = attempt_dir / "outputs" / "generated.png"
    atomic_bytes(output_path, data)
    output_sha256 = sha256_bytes(data)
    try:
        inspection, _inspection_identity = _inspect_with_perfectpixel(
            perfectpixel,
            output_path,
            output_sha256=output_sha256,
            identity=identity,
        )
        inspection_path = attempt_dir / "perfectpixel-inspection.json"
        exclusive_json(inspection_path, inspection)
        inspection_ref = {
            "path": inspection_path.relative_to(attempt_dir).as_posix(),
            "sha256": sha256_file(inspection_path),
        }
    except Exception as error:
        detail = {
            "schema_version": 1,
            "task_id": task_id,
            "attempt_no": attempt_no,
            "created_at": _utc_now(),
            "input_path": "outputs/generated.png",
            "input_sha256": output_sha256,
            "provider_sha256": provider_sha256,
            "perfectpixel_sha256": identity["perfectpixel"]["sha256"],
            "error_type": type(error).__name__,
            "error": str(error),
        }
        try:
            exclusive_json(attempt_dir / "perfectpixel-inspection-error.json", detail)
        except FileExistsError:
            pass
        finished, incomplete = _finish_attempt_safe(
            attempt_dir, run_dir, task_id, attempt_no,
            request_id=request_id,
            started_at=started_at,
            finished_at=_utc_now(),
            outcome="UNKNOWN",
            notes=common_notes + [f"PERFECTPIXEL_INSPECTION_FAILED:{type(error).__name__}:{error}; do not retry this attempt."],
        )
        if incomplete is not None:
            return incomplete
        receipt, manifest, review = finished
        return {
            "status": "OUTCOME_UNKNOWN",
            "error": {"code": "PERFECTPIXEL_INSPECTION_FAILED", "type": type(error).__name__, "message": str(error)},
            "receipt": receipt,
            "result": manifest,
            "mechanicalReview": review,
        }
    observed = response["image"].get("transparentBackground")
    finished, incomplete = _finish_attempt_safe(
        attempt_dir, run_dir, task_id, attempt_no,
        request_id=request_id,
        started_at=started_at,
        finished_at=finished_at,
        outcome="SUCCEEDED",
        notes=common_notes + [
            f"scene_provider.transparentBackground={observed if observed is not None else 'UNKNOWN'}",
            f"png.dimensions={info['width']}x{info['height']}",
        ],
        perfectpixel_inspection=inspection_ref,
    )
    if incomplete is not None:
        incomplete["output"] = {"path": str(output_path), "sha256": sha256_bytes(data)}
        return incomplete
    receipt, manifest, review = finished
    status = "READY_FOR_VISUAL_REVIEW" if review["status"] == "PASS" else "MECHANICAL_REVIEW_FAILED"
    return {
        "status": status,
        "output": {
            "path": str(output_path),
            "sha256": output_sha256,
            "width": info["width"],
            "height": info["height"],
            "hasTransparentPixels": info["has_transparent_pixels"],
        },
        "receipt": receipt,
        "result": manifest,
        "mechanicalReview": review,
    }
