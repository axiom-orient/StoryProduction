"""Validate immutable generation attempts and record provider outcomes."""
from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path

from .execution import record_execution
from .output_manifest import manifest_outputs
from .mechanical_review import mechanical_review
from .scene_provider_protocol import (
    DISPATCH_EVIDENCE_NAME,
    POSTFLIGHT_ERROR_EVIDENCE_NAME,
    PROCESS_EVIDENCE_NAME,
    PROVIDER_TOOL,
    SUPPORTED_SIZES,
)
from .util import atomic_json, load_json


def _utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _attempt_contract(run_dir: Path, task_id: str, attempt_no: int) -> tuple[Path, dict, dict]:
    run_dir = Path(run_dir).resolve()
    if not task_id or task_id in {".", ".."} or Path(task_id).parts != (task_id,):
        raise ValueError("SCENE_PROVIDER_TASK_ID_INVALID")
    task_dir = run_dir / "tasks" / task_id
    attempt_dir = task_dir / "attempts" / f"{attempt_no:04d}"
    if task_dir.is_symlink() or attempt_dir.is_symlink() or not task_dir.is_dir() or not attempt_dir.is_dir():
        raise ValueError("SCENE_PROVIDER_ATTEMPT_MISSING")
    task_path = task_dir / "task.json"
    attempt_path = attempt_dir / "attempt.json"
    request_path = run_dir / "request.json"
    for path in (task_path, attempt_path, request_path):
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"SCENE_PROVIDER_PREFLIGHT_FILE:{path.name}")
    task = load_json(task_path)
    attempt = load_json(attempt_path)
    run_request = load_json(request_path)
    review_policy = run_request.get("review_policy") if isinstance(run_request, dict) else None
    if not isinstance(review_policy, dict) or not isinstance(review_policy.get("allow_warnings", True), bool):
        raise ValueError("SCENE_PROVIDER_REVIEW_POLICY_INVALID")
    if task.get("task_id") != task_id or attempt.get("task_id") != task_id or attempt.get("attempt_no") != attempt_no:
        raise ValueError("SCENE_PROVIDER_ATTEMPT_LINEAGE")
    if attempt.get("mode") != "GENERATE":
        raise ValueError("SCENE_PROVIDER_TEXT_TO_IMAGE_ONLY")
    if attempt.get("resolved_references") or (attempt.get("repair") or {}).get("edit_target"):
        raise ValueError("SCENE_PROVIDER_REFERENCE_INPUT_UNSUPPORTED")
    output = task.get("output") or {}
    if output.get("preferred") != "PNG":
        raise ValueError("SCENE_PROVIDER_PNG_ONLY")
    if output.get("size") not in SUPPORTED_SIZES:
        raise ValueError("SCENE_PROVIDER_SIZE_UNSUPPORTED")
    if not isinstance(output.get("transparent_background"), bool):
        raise ValueError("SCENE_PROVIDER_BACKGROUND_CONTRACT")
    canonical_provider_evidence = {
        DISPATCH_EVIDENCE_NAME,
        PROCESS_EVIDENCE_NAME,
        POSTFLIGHT_ERROR_EVIDENCE_NAME,
    }
    unsupported_provider_evidence = [
        entry.name
        for entry in attempt_dir.iterdir()
        if (entry.is_file() or entry.is_symlink())
        and entry.name.endswith(("-dispatch.json", "-process.json", "-postflight-error.json"))
        and entry.name not in canonical_provider_evidence
    ]
    if unsupported_provider_evidence:
        raise ValueError("SCENE_PROVIDER_ATTEMPT_UNSUPPORTED_EVIDENCE")
    for immutable in (
        DISPATCH_EVIDENCE_NAME,
        PROCESS_EVIDENCE_NAME,
        "execution-receipt.json",
        "result.json",
        "mechanical-review.json",
        POSTFLIGHT_ERROR_EVIDENCE_NAME,
        "perfectpixel-inspection.json",
        "perfectpixel-inspection-error.json",
    ):
        if (attempt_dir / immutable).exists():
            raise ValueError(f"SCENE_PROVIDER_ATTEMPT_ALREADY_EXECUTED:{immutable}")
    outputs = attempt_dir / "outputs"
    if outputs.is_symlink() or not outputs.is_dir():
        raise ValueError("SCENE_PROVIDER_OUTPUT_DIRECTORY_INVALID")
    if any(outputs.iterdir()):
        raise ValueError("SCENE_PROVIDER_OUTPUT_DIRECTORY_NOT_EMPTY")
    capsule = attempt_dir / "generation-capsule.md"
    prompt = attempt_dir / "prompt.md"
    if capsule.is_symlink() or not capsule.is_file() or prompt.is_symlink() or not prompt.is_file():
        raise ValueError("SCENE_PROVIDER_PROMPT_EVIDENCE_INVALID")
    return attempt_dir, task, attempt





def _finish_attempt(
    run_dir: Path,
    task_id: str,
    attempt_no: int,
    *,
    request_id: str,
    started_at: str,
    finished_at: str,
    outcome: str,
    notes: list[str],
    perfectpixel_inspection: dict[str, str] | None = None,
) -> tuple[dict, dict, dict]:
    draft = {
        "provider": "OPENAI_CHATGPT",
        "tool": PROVIDER_TOOL,
        "model": "gpt-image-2",
        "model_snapshot": "UNKNOWN",
        "request_id": request_id,
        "started_at": started_at,
        "finished_at": finished_at,
        "outcome": outcome,
        "evidence_level": "EXECUTOR_RECORDED",
        "submitted_prompt_path": "generation-capsule.md",
        "capsule_verbatim": True,
        "notes": notes,
    }
    if perfectpixel_inspection is not None:
        draft["perfectpixel_inspection"] = dict(perfectpixel_inspection)
    receipt = record_execution(run_dir, task_id, attempt_no, draft)
    manifest = manifest_outputs(run_dir, task_id, attempt_no)
    review = mechanical_review(run_dir, task_id, attempt_no)
    return receipt, manifest, review


def _finish_attempt_safe(
    attempt_dir: Path,
    run_dir: Path,
    task_id: str,
    attempt_no: int,
    *,
    request_id: str,
    started_at: str,
    finished_at: str,
    outcome: str,
    notes: list[str],
    perfectpixel_inspection: dict[str, str] | None = None,
) -> tuple[tuple[dict, dict, dict] | None, dict | None]:
    try:
        return _finish_attempt(
            run_dir,
            task_id,
            attempt_no,
            request_id=request_id,
            started_at=started_at,
            finished_at=finished_at,
            outcome=outcome,
            notes=notes,
            perfectpixel_inspection=perfectpixel_inspection,
        ), None
    except Exception as error:
        detail = {
            "schema_version": 1,
            "task_id": task_id,
            "attempt_no": attempt_no,
            "request_id": request_id,
            "created_at": _utc_now(),
            "provider_outcome": outcome,
            "error_type": type(error).__name__,
            "error": str(error),
        }
        persistence_error = None
        try:
            atomic_json(attempt_dir / POSTFLIGHT_ERROR_EVIDENCE_NAME, detail)
        except Exception as record_error:
            persistence_error = f"{type(record_error).__name__}:{record_error}"
        result = {
            "status": "POSTFLIGHT_INCOMPLETE",
            "providerOutcome": outcome,
            "error": {"code": "SCENE_PROVIDER_POSTFLIGHT_INCOMPLETE", "type": type(error).__name__, "message": str(error)},
        }
        if persistence_error is not None:
            result["errorRecord"] = f"NOT_WRITTEN:{persistence_error}"
        else:
            result["errorRecord"] = str(attempt_dir / POSTFLIGHT_ERROR_EVIDENCE_NAME)
        return None, result
