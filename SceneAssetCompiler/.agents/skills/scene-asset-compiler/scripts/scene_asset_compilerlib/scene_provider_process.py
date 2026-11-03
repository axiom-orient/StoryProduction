from __future__ import annotations

from pathlib import Path
import os
import signal
import subprocess
import time
from typing import Any

from .scene_provider_protocol import (
    PROCESS_TEARDOWN_FORCE_SECONDS,
    PROCESS_TEARDOWN_GRACE_SECONDS,
    REQUIRED_ENVIRONMENT,
)

def _provider_environment() -> dict[str, str]:
    environment = os.environ.copy()
    missing = [name for name in REQUIRED_ENVIRONMENT if not environment.get(name)]
    if missing:
        raise ValueError(f"SCENE_PROVIDER_ENVIRONMENT_MISSING:{','.join(missing)}")
    for name in ("VERGERAIL_CODEX_PACKAGE", "VERGERAIL_WORKSPACE"):
        path = Path(environment[name]).expanduser()
        if not path.is_absolute() or not path.is_dir():
            raise ValueError(f"SCENE_PROVIDER_ENVIRONMENT_PATH:{name}")
    return environment


def _process_group_exists(process_group: int) -> bool:
    try:
        os.killpg(process_group, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def _leader_owns_process_group(process: subprocess.Popen, process_group: int) -> bool:
    """Require the live child leader to still own the recorded group before signalling it."""
    if process.poll() is not None:
        return False
    try:
        return os.getpgid(process.pid) == process_group
    except (ProcessLookupError, PermissionError):
        return False


def _teardown_process_group(process: subprocess.Popen, process_group: int) -> tuple[bytes, bytes, dict]:
    evidence = {
        "leader_pid": process.pid,
        "process_group": process_group,
        "returncode": None,
        "term_sent": False,
        "kill_sent": False,
        "pre_signal_identity_check_passed": False,
        "teardown_error": None,
        "pipes_closed": False,
        "leader_reaped": False,
        "no_survivors": False,
    }
    if _process_group_exists(process_group):
        if _leader_owns_process_group(process, process_group):
            evidence["pre_signal_identity_check_passed"] = True
            try:
                os.killpg(process_group, signal.SIGTERM)
                evidence["term_sent"] = True
            except ProcessLookupError:
                pass
            except PermissionError as error:
                evidence["teardown_error"] = f"SIGTERM_PERMISSION:{error}"
        else:
            evidence["teardown_error"] = "PROCESS_GROUP_IDENTITY_UNPROVEN"
    try:
        stdout, stderr = process.communicate(timeout=PROCESS_TEARDOWN_GRACE_SECONDS)
        evidence["pipes_closed"] = True
    except subprocess.TimeoutExpired:
        stdout, stderr = b"", b""
    if process.poll() is None or _process_group_exists(process_group):
        if _leader_owns_process_group(process, process_group):
            evidence["pre_signal_identity_check_passed"] = True
            try:
                os.killpg(process_group, signal.SIGKILL)
                evidence["kill_sent"] = True
            except ProcessLookupError:
                pass
            except PermissionError as error:
                evidence["teardown_error"] = f"SIGKILL_PERMISSION:{error}"
        elif _process_group_exists(process_group):
            evidence["teardown_error"] = "PROCESS_GROUP_IDENTITY_UNPROVEN"
        try:
            stdout, stderr = process.communicate(timeout=PROCESS_TEARDOWN_FORCE_SECONDS)
            evidence["pipes_closed"] = True
        except subprocess.TimeoutExpired:
            stdout, stderr = b"", b""
    survivor_deadline = time.monotonic() + PROCESS_TEARDOWN_FORCE_SECONDS
    while _process_group_exists(process_group) and time.monotonic() < survivor_deadline:
        time.sleep(0.05)
    evidence["leader_reaped"] = process.poll() is not None
    evidence["returncode"] = process.returncode
    evidence["no_survivors"] = not _process_group_exists(process_group)
    return stdout, stderr, evidence


def _custody_notes(evidence: dict) -> list[str]:
    notes = [
        f"scene_provider.process_pid={evidence['leader_pid']}",
        f"scene_provider.process_group={evidence['process_group']}",
        f"scene_provider.leader_reaped={str(evidence['leader_reaped']).lower()}",
        f"scene_provider.no_survivors={str(evidence['no_survivors']).lower()}",
        f"scene_provider.pipes_closed={str(evidence['pipes_closed']).lower()}",
        f"scene_provider.term_sent={str(evidence.get('term_sent', False)).lower()}",
        f"scene_provider.kill_sent={str(evidence.get('kill_sent', False)).lower()}",
        f"scene_provider.pre_signal_identity_check_passed={str(evidence.get('pre_signal_identity_check_passed', False)).lower()}",
    ]
    if evidence.get("teardown_error"):
        notes.append(f"scene_provider.teardown_error={evidence['teardown_error']}")
    return notes


def _run_bounded_process(
    command: list[str],
    *,
    input_bytes: bytes,
    timeout_seconds: float,
    cwd: Path | None = None,
) -> tuple[bytes, bytes, dict[str, Any]]:
    """Run one owned helper process and prove its group was reaped."""
    started_ns = time.monotonic_ns()
    try:
        process = subprocess.Popen(
            command,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=str(cwd) if cwd is not None else None,
            start_new_session=True,
        )
    except OSError as error:
        raise ValueError(f"HELPER_START_FAILED:{type(error).__name__}:{error}") from error
    process_group = process.pid
    try:
        stdout, stderr = process.communicate(input=input_bytes, timeout=timeout_seconds)
    except subprocess.TimeoutExpired as error:
        _stdout, _stderr, custody = _teardown_process_group(process, process_group)
        if not custody["leader_reaped"] or not custody["no_survivors"]:
            raise ValueError(
                f"HELPER_TIMEOUT_CUSTODY_UNPROVEN:pid={process.pid}:pgid={process_group}"
            ) from error
        raise ValueError(f"HELPER_TIMEOUT:pid={process.pid}:pgid={process_group}") from error
    custody = {
        "leader_pid": process.pid,
        "process_group": process_group,
        "monotonic_started_ns": started_ns,
        "term_sent": False,
        "kill_sent": False,
        "pipes_closed": True,
        "leader_reaped": process.poll() is not None,
        "no_survivors": not _process_group_exists(process_group),
        "returncode": process.returncode,
    }
    if not custody["no_survivors"]:
        stdout, stderr, custody = _teardown_process_group(process, process_group)
    if not custody["leader_reaped"] or not custody["no_survivors"]:
        raise ValueError(
            f"HELPER_PROCESS_CUSTODY_UNPROVEN:pid={process.pid}:pgid={process_group}"
        )
    return stdout, stderr, custody
