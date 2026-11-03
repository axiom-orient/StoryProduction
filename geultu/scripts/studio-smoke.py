#!/usr/bin/env python3
"""Bounded local Studio runtime smoke with explicit process custody."""

from __future__ import annotations

import http.client
import json
import os
import re
import signal
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TOKEN_HEX_LENGTH = 64


def authority(host: str, port: int) -> str:
    return f"[{host}]:{port}" if ":" in host else f"{host}:{port}"


def choose_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.bind(("127.0.0.1", 0))
        return int(probe.getsockname()[1])


def build_current_binaries() -> tuple[Path, Path]:
    cargo = os.environ.get("CARGO", "cargo")
    binaries = (("geultu-studio", "GeultuStudio"), ("geultu-cli", "geultu"))
    paths: list[Path] = []
    for package, binary_name in binaries:
        subprocess.run(
            [
                cargo,
                "build",
                "--manifest-path",
                str(ROOT / "Cargo.toml"),
                "-p",
                package,
                "--bin",
                binary_name,
                "--release",
                "--offline",
                "--target-dir",
                str(ROOT / "target"),
            ],
            cwd=ROOT,
            check=True,
        )
        binary = ROOT / "target" / "release" / binary_name
        if not binary.is_file() or not os.access(binary, os.X_OK):
            raise RuntimeError(f"release build did not produce {binary_name}")
        paths.append(binary)
    print("studio-artifact: release binaries built from this checkout")
    return paths[0], paths[1]


def validate_session_token(token: str) -> None:
    if len(token) != TOKEN_HEX_LENGTH or re.fullmatch(r"[0-9a-f]+", token) is None:
        raise AssertionError("Studio announced an invalid session-token format")


def init_workspace(cli_binary: Path, workspace: Path) -> None:
    subprocess.run(
        [str(cli_binary), "--workspace", str(workspace), "init"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )


def read_json(body: bytes) -> dict:
    value = json.loads(body.decode("utf-8"))
    if not isinstance(value, dict):
        raise AssertionError(f"expected JSON object, got {value!r}")
    return value


def request(
    port: int,
    method: str,
    path: str,
    token: str | None = None,
    payload: dict | None = None,
    connect_host: str = "127.0.0.1",
    host_header: str | None = None,
) -> tuple[int, dict]:
    headers = {"Host": host_header or authority(connect_host, port)}
    body = None
    if token is not None:
        headers["X-Geultu-Token"] = token
    if payload is not None:
        headers["Content-Type"] = "application/json"
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    connection = http.client.HTTPConnection(connect_host, port, timeout=3)
    try:
        connection.request(method, path, body=body, headers=headers)
        response = connection.getresponse()
        return response.status, read_json(response.read())
    finally:
        connection.close()


def request_bytes(
    port: int,
    method: str,
    path: str,
    token: str | None = None,
    connect_host: str = "127.0.0.1",
) -> tuple[int, bytes]:
    headers = {"Host": authority(connect_host, port)}
    if token is not None:
        headers["X-Geultu-Token"] = token
    connection = http.client.HTTPConnection(connect_host, port, timeout=3)
    try:
        connection.request(method, path, headers=headers)
        response = connection.getresponse()
        return response.status, response.read()
    finally:
        connection.close()


def assert_static_studio_flow(port: int) -> None:
    status, html = request_bytes(port, "GET", "/")
    page = html.decode("utf-8")
    if status != 200:
        raise AssertionError(f"Studio page failed: status={status}")
    for marker in (
        "글투 참고값",
        "글투 관찰값 만들기",
        "작업 지시 복사",
        "수정안 붙여넣기",
        "글투 비교",
        "수정안 검토 준비",
        "사람의 결정",
        "검토자 ID",
        "현재 원고에는 아직 반영되지 않았습니다.",
    ):
        if marker not in page:
            raise AssertionError(f"Studio page missing workflow marker: {marker!r}")
    for forbidden in ("r1", "s2", "revision", "provider", "model"):
        if forbidden in page.lower():
            raise AssertionError(f"ordinary Studio page exposes technical label: {forbidden!r}")
    for path, marker in (("/app.js", "voice.profile.create"), ("/style.css", "focus-visible")):
        asset_status, asset = request_bytes(port, "GET", path)
        if asset_status != 200 or marker.encode("utf-8") not in asset:
            raise AssertionError(f"Studio asset failed: path={path}, status={asset_status}")
    app_status, app_js = request_bytes(port, "GET", "/app.js")
    for marker in ("source_content_revision", "TextDecoder", "fatal", "TXT 또는 MD"):
        if app_status != 200 or marker.encode("utf-8") not in app_js:
            raise AssertionError(f"Studio app missing stale/file guard: {marker!r}")
    app_text = app_js.decode("utf-8")
    for marker in (
        "candidate.subject",
        "expected_artifact_revision",
        "expected_artifact_digest",
        "검토 대상이 바뀌었습니다.",
    ):
        if marker not in app_text:
            raise AssertionError(f"Studio app missing subject-bound approval guard: {marker!r}")
    for marker in (
        "function sceneTextIsDirty()",
        "function candidateTextIsDraft()",
        "function projectSceneReadIsCoherent(project, scene, projectId, sceneId)",
        "function editorSnapshotUnchanged(snapshot)",
        "function reconcileFailedSceneSave(snapshot)",
        "window.addEventListener('beforeunload'",
        "현재 편집 내용은 그대로 두었습니다.",
    ):
        if marker not in app_text:
            raise AssertionError(f"Studio scene-draft recovery guard is missing: {marker!r}")
    project_load = app_text.find("async function selectProject(id)")
    project_read = app_text.find("call('project.get', {project_id: id})", project_load)
    project_snapshot_check = app_text.find("editorSnapshotUnchanged(snapshot)", project_read)
    project_state_change = app_text.find("state.project = project;", project_read)
    if min(project_load, project_read, project_snapshot_check, project_state_change) < 0:
        raise AssertionError("Studio project navigation is missing its in-flight draft recheck")
    if not project_read < project_snapshot_check < project_state_change:
        raise AssertionError("Studio project navigation mutates selection before rechecking editor input")
    scene_load = app_text.find("async function selectScene(id)")
    scene_read = app_text.find("call('project.get', {project_id: projectId})", scene_load)
    scene_snapshot_check = app_text.find("editorSnapshotUnchanged(snapshot)", scene_read)
    scene_state_change = app_text.find("state.project = project;", scene_read)
    if min(scene_load, scene_read, scene_snapshot_check, scene_state_change) < 0:
        raise AssertionError("Studio scene navigation is missing its in-flight draft recheck")
    if not scene_read < scene_snapshot_check < scene_state_change:
        raise AssertionError("Studio scene navigation mutates selection before rechecking editor input")
    save_start = app_text.find("async function saveSceneText()")
    submitted_text = app_text.find("submittedText: $('text').value", save_start)
    save_effect = app_text.find("call('scene.set_text'", save_start)
    save_readback = app_text.find("reconcileFailedSceneSave(snapshot)", save_effect)
    if min(save_start, submitted_text, save_effect, save_readback) < 0:
        raise AssertionError("Studio Save is missing its captured input or owner readback")
    if not submitted_text < save_effect < save_readback:
        raise AssertionError("Studio Save reads or replaces editor state before reconciling its captured text")
    reconcile_start = app_text.find("async function reconcileFailedSceneSave(snapshot)")
    reconcile_body = app_text[reconcile_start:app_text.find("async function rereadSceneSaveOutcome()", reconcile_start)]
    project_read = reconcile_body.find("call('project.get'")
    scene_read = reconcile_body.find("call('scene.get'", project_read)
    coherence_check = reconcile_body.find("projectSceneReadIsCoherent(project, scene, snapshot.projectId, snapshot.sceneId)", scene_read)
    owner_cache_update = reconcile_body.find("state.project = project;", coherence_check)
    submitted_confirmation = reconcile_body.find("scene.text === snapshot.submittedText", coherence_check)
    if min(reconcile_start, project_read, scene_read, coherence_check, owner_cache_update, submitted_confirmation) < 0:
        raise AssertionError("Studio Save must compare project and scene snapshots before confirming or replacing cached owner state")
    if not scene_read < coherence_check < owner_cache_update < submitted_confirmation:
        raise AssertionError("Studio Save may confirm a scene from a torn project/scene readback")
    if "if (!projectSceneReadIsCoherent(project, scene, snapshot.projectId, snapshot.sceneId)) return 'unknown';" not in reconcile_body:
        raise AssertionError("A torn owner readback must remain unknown for explicit observation, not become a save success")
    save_body = app_text[save_start:app_text.find("$('project-form')", save_start)]
    if "await selectScene(" in save_body:
        raise AssertionError("Studio Save reselects the scene and may overwrite text typed after Save began")
    approve_start = app_text.find("async function approveCandidate()")
    approve_body = app_text[approve_start:app_text.find("async function rejectCandidate()", approve_start)]
    approve_call = approve_body.find("await call('changeset.approve'")
    approve_project_read = approve_body.find("call('project.get'", approve_call)
    approve_scene_read = approve_body.find("call('scene.get'", approve_project_read)
    approve_candidate_check = approve_body.find("approvedCandidate?.status !== 'approved'", approve_scene_read)
    approve_source_check = approve_body.find("const appliedToCurrentScene = scene.text === candidateText", approve_candidate_check)
    approve_state_update = approve_body.find("state.project = project;", approve_source_check)
    if min(approve_start, approve_call, approve_project_read, approve_scene_read, approve_candidate_check, approve_source_check, approve_state_update) < 0:
        raise AssertionError("Studio approval must read and confirm the owner’s applied candidate before updating the editor")
    if not approve_call < approve_project_read < approve_scene_read < approve_candidate_check < approve_source_check < approve_state_update:
        raise AssertionError("Studio approval updates the view before confirming the owner’s applied candidate")
    if "project.scenes[sceneId]" in approve_body or "await selectScene(" in approve_body:
        raise AssertionError("Studio approval must read the selected scene from the owner without reselecting the editor")
    recovery_start = app_text.find("async function rereadApprovalOutcome()")
    recovery_body = app_text[recovery_start:app_text.find("function renderReview(", recovery_start)]
    if recovery_start < 0 or "subject.digest === snapshot.subject.digest" not in recovery_body:
        raise AssertionError("Studio approval recovery must recheck the exact owner subject hash")
    if "candidate.status === 'approved' && changeset.status === 'approved'" not in recovery_body:
        raise AssertionError("Studio approval recovery must confirm the owner’s decision status")
    if "changeset.approve" in recovery_body:
        raise AssertionError("Studio approval recovery must read state without automatically repeating approval")
    project_view = app_text[app_text.find("function renderProject()") : app_text.find("function renderVoice()")]
    if "$('text').value = state.scene.text" in project_view:
        raise AssertionError("Studio project rendering overwrites the editor with persisted text")
    compare_disabled = re.search(r"\$\('compare'\)\.disabled = ([^;]+);", app_text)
    if compare_disabled is None or "sourceDirty" in compare_disabled.group(1):
        raise AssertionError("Studio style comparison is incorrectly blocked by source-only edits")
    if "$('scene-empty h2')" in app_text:
        raise AssertionError("Studio still passes a descendant selector to the ID-only $ helper")
    if "document.querySelector('#scene-empty h2')" not in app_text:
        raise AssertionError("Studio scene-empty heading selector regression guard is missing")
    if "state.project.title + ' 작업을 열었습니다.'" not in app_text:
        raise AssertionError("Studio project-open success status is missing")
    profile_load = app_text.find("const voiceProfile = await call('voice.profile.get', {project_id: id});")
    profile_visibility_reset = app_text.find("state.voiceInputsOpen = !voiceProfile;", profile_load)
    if profile_load < 0 or profile_visibility_reset < profile_load:
        raise AssertionError("Studio does not reset sample visibility from the loaded profile")
    for marker in ("voiceInputsOpen: true", "state.voiceInputsOpen = !state.voiceInputsOpen;"):
        if marker not in app_text:
            raise AssertionError(f"Studio sample visibility interaction is missing: {marker!r}")
    prepare_disabled = re.search(r"\$\('prepare'\)\.disabled = ([^;]+);", app_text)
    if prepare_disabled is None or any(token in prepare_disabled.group(1) for token in ("state.guideBasis", "state.comparison", "state.voiceProfile")):
        raise AssertionError("Manual candidate review must not require a copied guide, voice profile, or optional comparison")
    prepare_start = app_text.find("async function prepareCandidate()")
    prepare_end = app_text.find("async function approveCandidate()", prepare_start)
    prepare_body = app_text[prepare_start:prepare_end]
    current_project = prepare_body.find("call('project.get'")
    current_scene = prepare_body.find("call('scene.get'", current_project)
    project_scene_coherence = prepare_body.find("projectSceneReadIsCoherent(project, scene, projectId, sceneId)", current_scene)
    source_digest = prepare_body.find("const sourceDigest = await sha256Hex(sourceText)", current_scene)
    basis_guard = prepare_body.find("guideBasis.source_content_revision !== project.content_revision")
    atomic_prepare = prepare_body.find("await call('scene.prepare_candidate'", source_digest)
    if min(prepare_start, current_project, current_scene, project_scene_coherence, source_digest, basis_guard, atomic_prepare) < 0:
        raise AssertionError("Candidate preparation must read a coherent current owner source before the atomic owner command")
    if not current_project < current_scene < project_scene_coherence < source_digest < basis_guard < atomic_prepare:
        raise AssertionError("Candidate preparation does not bind its single owner command to freshly read source identity")
    if "await call('candidate.create'" in prepare_body or "await call('changeset.create'" in prepare_body:
        raise AssertionError("Studio candidate preparation must not split the owner pair into separate mutations")
    if 'expected_content_revision: snapshot.expectedContentRevision' not in prepare_body or 'expected_scene_revision: snapshot.expectedSceneRevision' not in prepare_body or 'command_id: snapshot.commandId' not in prepare_body:
        raise AssertionError("The atomic prepare request must carry the reviewed source revisions and stable command identity")
    if "voice.guide" in prepare_body:
        raise AssertionError("Manual candidate preparation must not require a copied clipboard guide")
    review_filter = app_text[app_text.find("function restorableReviews(") : app_text.find("function showPendingReview(")]
    if "changeset.source_revision !== project.content_revision" in review_filter or "candidate.source_revision !== project.content_revision" in review_filter:
        raise AssertionError("Stale pending owner reviews are hidden instead of being restored with their current source context")
    if "stale: !sourceIsCurrent" not in app_text:
        raise AssertionError("Restored pending reviews must keep stale source status explicit")
    reject_disabled = re.search(r"\$\('reject'\)\.disabled = ([^;]+);", app_text)
    reject_body = app_text[app_text.find("async function rejectCandidate()") : app_text.find("function renderReview(")]
    if reject_disabled is None or "pendingSourceMatches" in reject_disabled.group(1):
        raise AssertionError("A stale pending candidate must remain rejectable when its exact subject is available")
    reject_subject = reject_body.find("candidate.subject")
    reject_call = reject_body.find("call('changeset.reject'", reject_subject)
    reject_project_read = reject_body.find("call('project.get'", reject_call)
    reject_scene_read = reject_body.find("call('scene.get'", reject_project_read)
    if min(reject_subject, reject_call, reject_project_read, reject_scene_read) < 0:
        raise AssertionError("Candidate rejection must bind its exact subject and then re-read owner project and scene")
    if not reject_subject < reject_call < reject_project_read < reject_scene_read:
        raise AssertionError("Candidate rejection updates the view without an authoritative owner readback")
    if "decision?.decision === 'REJECT'" not in reject_body or "rejectedChangeset.status === 'rejected'" not in reject_body:
        raise AssertionError("Candidate rejection must verify both the exact owner receipt and recorded terminal state")
    latest_start = app_text.find("async function openLatestApprovalState()")
    latest_end = app_text.find("async function rejectCandidate()", latest_start)
    latest_body = app_text[latest_start:latest_end]
    if latest_start < 0 or "state.approvalUnknowns.push" not in latest_body or "state.approvalUnknowns[last] = {...state.approvalUnknowns[last]" not in latest_body:
        raise AssertionError("A mismatched approval readback must preserve its unresolved request before opening current owner state")
    if "call('project.get'" not in latest_body or "call('scene.get'" not in latest_body or "changeset.approve" in latest_body:
        raise AssertionError("Opening latest state must use owner reads only and never retry approval")
    if "approval-outcome-notice" not in page or "state.approvalUnknowns" not in app_text:
        raise AssertionError("Approval uncertainty must remain visible after opening current owner state")
    print("studio-ui: static workflow, focus, and jargon checks=PASS")


def exact_group_members(pgid: int) -> list[tuple[int, str]]:
    output = subprocess.check_output(
        ["ps", "-axo", "pid=,pgid=,command="],
        text=True,
        stderr=subprocess.STDOUT,
    )
    members: list[tuple[int, str]] = []
    for line in output.splitlines():
        fields = line.strip().split(None, 2)
        if len(fields) < 2:
            continue
        try:
            pid, member_pgid = int(fields[0]), int(fields[1])
        except ValueError:
            continue
        if member_pgid == pgid:
            members.append((pid, fields[2] if len(fields) == 3 else ""))
    return members


def assert_group_clear(pgid: int, label: str, timeout: float) -> list[tuple[int, str]]:
    deadline = time.monotonic() + timeout
    members: list[tuple[int, str]] = []
    while time.monotonic() < deadline:
        members = exact_group_members(pgid)
        if not members:
            print(f"studio-cleanup: {label}=clear")
            return members
        time.sleep(0.05)
    raise AssertionError(f"Studio process group {pgid} survived {label} check: {members!r}")


def wait_for_token(process: subprocess.Popen[bytes], stderr_path: Path, bind_host: str, port: int) -> str:
    pattern = re.compile(
        rf"^open: http://{re.escape(authority(bind_host, port))}/\?token=([^\s]+)",
        re.MULTILINE,
    )
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if process.poll() is not None:
            break
        log = stderr_path.read_text(encoding="utf-8")
        match = pattern.search(log)
        if match:
            token = match.group(1)
            validate_session_token(token)
            print(f"studio-token: lowercase-hex entropy={TOKEN_HEX_LENGTH * 4}bits")
            return token
        time.sleep(0.05)
    if process.poll() is not None:
        raise AssertionError(
            f"Studio exited before announcing a local token (returncode={process.returncode})"
        )
    raise AssertionError("Studio did not announce a local token before timeout")


def cleanup_process(process: subprocess.Popen[bytes], pgid: int, label: str) -> bool:
    force_stopped = False
    if process.poll() is None:
        try:
            os.killpg(pgid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            force_stopped = True
            os.killpg(pgid, signal.SIGKILL)
            process.wait(timeout=3)
    else:
        process.wait(timeout=3)
    assert_group_clear(pgid, f"{label}-immediate", 1.0)
    time.sleep(0.25)
    assert_group_clear(pgid, f"{label}-delayed", 1.0)
    print(f"{label}-cleanup: returncode={process.returncode} force_stop={force_stopped}")
    return force_stopped


def reject_non_loopback(studio_binary: Path, workspace: Path) -> None:
    port = choose_port()
    result = subprocess.run(
        [str(studio_binary), "--workspace", str(workspace), "--listen", authority("0.0.0.0", port)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=5,
    )
    if result.returncode == 0:
        raise AssertionError("non-loopback Studio listener unexpectedly started")
    if "non-loopback" not in result.stderr:
        raise AssertionError(f"non-loopback rejection message missing: {result.stderr!r}")
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.bind(("0.0.0.0", port))
    print(f"studio-config: rejected non-loopback before bind (exit={result.returncode})")


def run_local_smoke(studio_binary: Path, workspace: Path) -> None:
    port = choose_port()
    stderr_path = workspace.parent / "studio.stderr.log"
    process: subprocess.Popen[bytes] | None = None
    pgid: int | None = None
    try:
        with stderr_path.open("wb") as stderr_file:
            process = subprocess.Popen(
                [str(studio_binary), "--workspace", str(workspace), "--listen", authority("127.0.0.1", port)],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=stderr_file,
                start_new_session=True,
            )
        pgid = os.getpgid(process.pid)
        if pgid == os.getpgrp():
            raise AssertionError("Studio did not receive a distinct process group")
        print(f"studio-custody: pid={process.pid} pgid={pgid}")
        token = wait_for_token(process, stderr_path, "127.0.0.1", port)
        assert_static_studio_flow(port)

        status, health = request(port, "GET", "/healthz")
        if status != 200 or health.get("ok") is not True:
            raise AssertionError(f"local health failed: status={status}, body={health!r}")
        print("studio-http: local health=200")

        status, unauthorized = request(port, "GET", "/api/tools")
        if status != 403 or unauthorized.get("ok") is not False:
            raise AssertionError(f"unauthenticated API was not rejected: status={status}, body={unauthorized!r}")
        print("studio-http: unauthenticated API rejected")

        def call(name: str, arguments: dict, command_id: str | None = None) -> dict:
            payload_arguments = dict(arguments)
            if command_id is not None:
                payload_arguments["command_id"] = command_id
            status, value = request(
                port,
                "POST",
                "/api/tools/call",
                token,
                {"name": name, "arguments": payload_arguments},
            )
            if status != 200 or value.get("ok") is not True:
                raise AssertionError(f"authenticated {name} failed: status={status}, body={value!r}")
            result = value.get("result")
            if not isinstance(result, dict):
                raise AssertionError(f"authenticated {name} returned invalid result: {value!r}")
            return result

        call("project.create", {"project_id": "studio", "title": "Studio smoke"}, "studio-create")
        call(
            "scene.add",
            {"project_id": "studio", "scene_id": "scene_1", "title": "Opening", "text": "first"},
            "studio-scene-add",
        )
        call(
            "scene.set_text",
            {"project_id": "studio", "scene_id": "scene_1", "text": "updated", "expected_scene_revision": 0},
            "studio-scene-set",
        )
        profile_commit = call(
            "voice.profile.create",
            {
                "project_id": "studio",
                "profile_id": "voice-main",
                "expected_project_revision": 3,
                "samples": ["updated. I kept the facts and names."],
            },
            "studio-voice-create",
        )
        if profile_commit.get("idempotent") is not False:
            raise AssertionError(f"voice profile was not activated: {profile_commit!r}")
        profile = call("voice.profile.get", {"project_id": "studio"})
        if profile.get("sample_count") != 1 or "summary" not in profile:
            raise AssertionError(f"voice profile get returned invalid result: {profile!r}")
        comparison = call(
            "voice.compare",
            {"project_id": "studio", "candidate_text": "updated. I kept the facts and names."},
        )
        if not isinstance(comparison.get("score"), int) or not comparison.get("label"):
            raise AssertionError(f"voice comparison was not readable: {comparison!r}")
        guide = call(
            "voice.guide",
            {"project_id": "studio", "scene_id": "scene_1"},
        )
        if "외부 AI" not in guide.get("instruction", "") or "작업 대상 장면" not in guide.get("instruction", ""):
            raise AssertionError(f"voice guide did not preserve the external boundary: {guide!r}")
        for field in (
            "source_project_revision",
            "source_content_revision",
            "source_scene_revision",
            "source_scene_digest",
        ):
            if field not in guide:
                raise AssertionError(f"voice guide did not retain source basis: {field!r}")
        candidate = call(
            "candidate.create",
            {
                "project_id": "studio",
                "candidate_id": "candidate-1",
                "kind": "style_aware_rewrite",
                "source_revision": 2,
                "content": "updated in a calmer voice.",
                "provider": "external_editor",
                "model": "user_provided",
            },
            "studio-candidate-create",
        )
        changeset = call(
            "changeset.create",
            {
                "project_id": "studio",
                "candidate_id": "candidate-1",
                "changeset_id": "changeset-1",
                "operations": [
                    {
                        "type": "replace_scene_text",
                        "scene_id": "scene_1",
                        "expected_scene_revision": 1,
                        "text": "updated in a calmer voice.",
                    }
                ],
            },
            "studio-changeset-create",
        )
        project = call("project.get", {"project_id": "studio"})
        subject = call(
            "candidate.subject",
            {"project_id": "studio", "candidate_id": "candidate-1"},
        )
        call(
            "changeset.approve",
            {
                "project_id": "studio",
                "changeset_id": "changeset-1",
                "expected_project_revision": project["revision"],
                "actor_id": "studio-smoke-human",
                "expected_artifact_revision": subject["revision"],
                "expected_artifact_digest": subject["digest"],
            },
            "studio-changeset-approve",
        )
        status, scene = request(
            port,
            "POST",
            "/api/tools/call",
            token,
            {"name": "scene.get", "arguments": {"project_id": "studio", "scene_id": "scene_1"}},
        )
        if status != 200 or scene.get("ok") is not True or scene.get("result", {}).get("text") != "updated in a calmer voice.":
            raise AssertionError(f"authenticated mutation was not observable: status={status}, body={scene!r}")
        if candidate.get("idempotent") or changeset.get("idempotent"):
            raise AssertionError("candidate/change-set creation unexpectedly idempotent")
        print("studio-http: authenticated tool call, 글투 flow, and approval=PASS")

        call("project.create", {"project_id": "manual-review", "title": "Manual review without voice"}, "manual-review-create")
        call(
            "scene.add",
            {"project_id": "manual-review", "scene_id": "scene_1", "title": "Source", "text": "original source"},
            "manual-review-scene-add",
        )
        manual_project = call("project.get", {"project_id": "manual-review"})
        manual_scene = manual_project["scenes"]["scene_1"]
        manual_candidate = call(
            "candidate.create",
            {
                "project_id": "manual-review",
                "candidate_id": "candidate-manual",
                "kind": "style_aware_rewrite",
                "source_revision": manual_project["content_revision"],
                "content": "manually supplied candidate",
                "provider": "external_editor",
                "model": "user_provided",
            },
            "manual-review-candidate-create",
        )
        manual_changeset = call(
            "changeset.create",
            {
                "project_id": "manual-review",
                "candidate_id": "candidate-manual",
                "changeset_id": "changeset-manual",
                "operations": [{
                    "type": "replace_scene_text",
                    "scene_id": "scene_1",
                    "expected_scene_revision": manual_scene["revision"],
                    "text": "manually supplied candidate",
                }],
            },
            "manual-review-changeset-create",
        )
        if manual_candidate.get("idempotent") or manual_changeset.get("idempotent"):
            raise AssertionError("manual candidate review unexpectedly replayed")
        manual_subject = call(
            "candidate.subject",
            {"project_id": "manual-review", "candidate_id": "candidate-manual"},
        )
        call(
            "scene.set_text",
            {
                "project_id": "manual-review",
                "scene_id": "scene_1",
                "text": "newer source",
                "expected_scene_revision": manual_scene["revision"],
            },
            "manual-review-source-update",
        )
        latest_manual_project = call("project.get", {"project_id": "manual-review"})
        status, stale_approval = request(
            port,
            "POST",
            "/api/tools/call",
            token,
            {"name": "changeset.approve", "arguments": {
                "project_id": "manual-review",
                "changeset_id": "changeset-manual",
                "expected_project_revision": latest_manual_project["revision"],
                "actor_id": "studio-smoke-human",
                "expected_artifact_revision": manual_subject["revision"],
                "expected_artifact_digest": manual_subject["digest"],
                "command_id": "manual-review-stale-approve",
            }},
        )
        if status != 409 or "candidate source content revision mismatch" not in stale_approval.get("error", {}).get("message", ""):
            raise AssertionError(f"stale candidate approval was not rejected by the owner revision guard: status={status}, body={stale_approval!r}")
        reject_receipt = call(
            "changeset.reject",
            {
                "project_id": "manual-review",
                "changeset_id": "changeset-manual",
                "actor_id": "studio-smoke-human",
                "expected_artifact_revision": manual_subject["revision"],
                "expected_artifact_digest": manual_subject["digest"],
            },
            "manual-review-stale-reject",
        )
        if reject_receipt.get("project_id") != "manual-review" or reject_receipt.get("decision", {}).get("decision") != "REJECT":
            raise AssertionError(f"stale rejection did not return its exact decision receipt: {reject_receipt!r}")
        rejected = call("project.get", {"project_id": "manual-review"})
        rejected_candidate = rejected["candidates"]["candidate-manual"]
        rejected_changeset = rejected["changesets"]["changeset-manual"]
        latest_scene = call("scene.get", {"project_id": "manual-review", "scene_id": "scene_1"})
        if rejected_candidate["status"] != "pending" or rejected_changeset["status"] != "rejected" or latest_scene["text"] != "newer source":
            raise AssertionError(f"exact-subject stale rejection must preserve current source and record a separate reject decision: candidate={rejected_candidate!r}, changeset={rejected_changeset!r}, scene={latest_scene!r}")
        print("studio-http: voice-optional candidate review and exact-subject stale rejection=PASS")
    finally:
        if process is not None:
            if pgid is None:
                raise RuntimeError("cannot clean up Studio safely without a process group")
            cleanup_process(process, pgid, "studio")


def run_ipv6_host_smoke(studio_binary: Path, workspace: Path) -> None:
    if not socket.has_ipv6:
        print("studio-ipv6: SKIPPED_ENV (Python reports IPv6 unavailable)")
        return
    try:
        with socket.socket(socket.AF_INET6, socket.SOCK_STREAM) as probe:
            probe.bind(("::1", 0))
            port = int(probe.getsockname()[1])
    except OSError as error:
        print(f"studio-ipv6: SKIPPED_ENV (cannot bind ::1: {error})")
        return

    stderr_path = workspace.parent / "studio-ipv6.stderr.log"
    process: subprocess.Popen[bytes] | None = None
    pgid: int | None = None
    try:
        with stderr_path.open("wb") as stderr_file:
            process = subprocess.Popen(
                [str(studio_binary), "--workspace", str(workspace), "--listen", authority("::1", port)],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=stderr_file,
                start_new_session=True,
            )
        pgid = os.getpgid(process.pid)
        if pgid == os.getpgrp():
            raise AssertionError("IPv6 Studio did not receive a distinct process group")
        print(f"studio-ipv6-custody: pid={process.pid} pgid={pgid}")
        token = wait_for_token(process, stderr_path, "::1", port)
        host_header = authority("::1", port)
        status, health = request(
            port,
            "GET",
            "/healthz",
            connect_host="::1",
            host_header=host_header,
        )
        if status != 200 or health.get("ok") is not True:
            raise AssertionError(f"IPv6 loopback Host request failed: status={status}, body={health!r}")
        print("studio-ipv6: bracketed loopback Host=PASS")

        status, unauthorized = request(
            port,
            "GET",
            "/api/tools",
            connect_host="::1",
            host_header=host_header,
        )
        if status != 403 or unauthorized.get("ok") is not False:
            raise AssertionError(
                f"IPv6 unauthenticated API was not rejected: status={status}, body={unauthorized!r}"
            )
        print("studio-ipv6: unauthenticated API rejected")

        status, tools = request(
            port,
            "GET",
            "/api/tools",
            token=token,
            connect_host="::1",
            host_header=host_header,
        )
        if status != 200 or tools.get("ok") is not True or not isinstance(tools.get("result"), list):
            raise AssertionError(f"IPv6 authenticated tool listing failed: status={status}, body={tools!r}")
        print("studio-ipv6: authenticated tool listing=PASS")

        def call(name: str, arguments: dict, command_id: str) -> dict:
            status, value = request(
                port,
                "POST",
                "/api/tools/call",
                token,
                {"name": name, "arguments": {**arguments, "command_id": command_id}},
                connect_host="::1",
                host_header=host_header,
            )
            if status != 200 or value.get("ok") is not True:
                raise AssertionError(f"IPv6 authenticated {name} failed: status={status}, body={value!r}")
            result = value.get("result")
            if not isinstance(result, dict):
                raise AssertionError(f"IPv6 authenticated {name} returned invalid result: {value!r}")
            return result

        call("project.create", {"project_id": "studio-ipv6", "title": "Studio IPv6 smoke"}, "studio-ipv6-create")
        call(
            "scene.add",
            {"project_id": "studio-ipv6", "scene_id": "scene_1", "title": "Opening", "text": "first"},
            "studio-ipv6-scene-add",
        )
        call(
            "scene.set_text",
            {
                "project_id": "studio-ipv6",
                "scene_id": "scene_1",
                "text": "updated",
                "expected_scene_revision": 0,
            },
            "studio-ipv6-scene-set",
        )
        status, scene = request(
            port,
            "POST",
            "/api/tools/call",
            token,
            {"name": "scene.get", "arguments": {"project_id": "studio-ipv6", "scene_id": "scene_1"}},
            connect_host="::1",
            host_header=host_header,
        )
        if status != 200 or scene.get("ok") is not True or scene.get("result", {}).get("text") != "updated":
            raise AssertionError(f"IPv6 authenticated mutation was not observable: status={status}, body={scene!r}")
        print("studio-ipv6: authenticated project/scene mutation=PASS")
    finally:
        if process is not None:
            if pgid is None:
                raise RuntimeError("cannot clean up IPv6 Studio safely without a process group")
            cleanup_process(process, pgid, "studio-ipv6")


def main() -> int:
    studio_binary, cli_binary = build_current_binaries()
    temporary = tempfile.TemporaryDirectory(prefix="geultu-studio-")
    workspace = Path(temporary.name) / "work"
    try:
        init_workspace(cli_binary, workspace)
        reject_non_loopback(studio_binary, workspace)
        run_ipv6_host_smoke(studio_binary, workspace)
        run_local_smoke(studio_binary, workspace)
    finally:
        temporary.cleanup()
    if workspace.exists():
        raise AssertionError(f"temporary Studio workspace survived cleanup: {workspace}")
    print("studio-smoke: PASS (workspace removed)")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:  # noqa: BLE001 - report bounded smoke failures clearly.
        print(f"studio-smoke: FAIL: {error}", file=sys.stderr)
        raise
