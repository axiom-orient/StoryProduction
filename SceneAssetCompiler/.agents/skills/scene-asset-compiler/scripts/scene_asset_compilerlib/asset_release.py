"""Prepare, approve, validate, and publish the SAC-owned scene asset release."""
from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .asset_release_builder import _build_release_objects
from .asset_release_validation import _load_prepared, validate_scene_asset_release
from .asset_release_contracts import (
    ASSET_PACK_RELATIVE_PATH,
    APPROVAL_RELATIVE_PATH,
    AUDIT_RELATIVE_PATH,
    BINDING_RELATIVE_PATH,
    HANDOFF_RELATIVE_PATH,
    PROJECTION_RECEIPT_RELATIVE_PATH,
    SOURCE_KIND,
    AssetReleaseError,
    canonical_hash,
    _string,
    _validate_handoff,
    _validate_release_request,
)
from .asset_release_files import (
    _read_json,
    _safe_destination,
    _strict_root,
)
from .util import atomic_json, load_json, sha256_file


def prepare_scene_asset_release(run_dir: Path, request_path: Path | str) -> dict[str, Any]:
    """Verify selected runtime bytes and publish pack/audit/binding inputs."""

    run_dir = _strict_root(run_dir)
    request_path = Path(request_path)
    if request_path.is_absolute():
        try:
            request_relative = request_path.relative_to(run_dir)
        except ValueError as error:
            raise AssetReleaseError("PATH_UNSAFE:releaseRequest") from error
    else:
        request_relative = request_path
    request, _, _ = _read_json(run_dir, request_relative.as_posix(), "sceneAssetReleaseRequest")
    _validate_release_request(request)
    pack, audit, binding, context = _build_release_objects(run_dir, request)
    pack_path = _safe_destination(run_dir, ASSET_PACK_RELATIVE_PATH)
    audit_path = _safe_destination(run_dir, AUDIT_RELATIVE_PATH)
    binding_path = _safe_destination(run_dir, BINDING_RELATIVE_PATH)
    atomic_json(pack_path, pack)
    atomic_json(binding_path, binding)
    # Recompute after serialization so every generated file is tied to the
    # exact JSON object read by the consumer.
    pack_hash = canonical_hash(load_json(pack_path))
    binding_hash = canonical_hash(load_json(binding_path))
    audit["assetPackHash"] = pack_hash
    audit["bindingManifestHash"] = binding_hash
    atomic_json(audit_path, audit)
    return {
        "status": "PREPARED",
        "packPath": ASSET_PACK_RELATIVE_PATH,
        "auditPath": AUDIT_RELATIVE_PATH,
        "bindingManifestPath": BINDING_RELATIVE_PATH,
        "assetPackHash": pack_hash,
        "assetPackSha256": sha256_file(pack_path),
        "assetPackAuditHash": canonical_hash(load_json(audit_path)),
        "assetPackAuditSha256": sha256_file(audit_path),
        "bindingManifestHash": binding_hash,
        "bindingManifestSha256": sha256_file(binding_path),
        "sceneGraphHash": context["projection"]["sceneGraphHash"],
        "projectionReceiptPath": context["projectionPath"],
        "projectionReceiptSha256": context["projectionRawHash"],
    }


def approve_scene_asset_release(run_dir: Path, reviewer: str, note: str = "") -> dict[str, Any]:
    """Record the explicit human release decision for the prepared subject."""

    run_dir = _strict_root(run_dir)
    reviewer = _string(reviewer, "reviewer").strip()
    if not reviewer:
        raise AssetReleaseError("MISSING:reviewer")
    pack, audit, binding, _, _, _, _ = _load_prepared(run_dir)
    subject = {
        "sceneGraphHash": pack["projection"]["sceneGraphHash"],
        "assetPackHash": canonical_hash(pack),
        "assetPackAuditHash": canonical_hash(audit),
        "bindingManifestHash": canonical_hash(binding),
    }
    approval = {
        "schemaVersion": 1,
        "approvalId": "SARA1-" + canonical_hash({"subject": subject, "reviewer": reviewer}).split(":", 1)[1],
        "status": "APPROVED",
        "reviewer": reviewer,
        "approvedAt": datetime.now(timezone.utc).isoformat(),
        "note": str(note),
        "subject": subject,
        "subjectHash": canonical_hash(subject),
    }
    approval_path = _safe_destination(run_dir, APPROVAL_RELATIVE_PATH)
    if approval_path.exists():
        existing = load_json(approval_path)
        if existing.get("subjectHash") != approval["subjectHash"] or existing.get("subject") != subject:
            raise AssetReleaseError("APPROVAL_CONFLICT")
        return {"status": "APPROVED", "approvalPath": APPROVAL_RELATIVE_PATH, "subjectHash": existing["subjectHash"], "reused": True}
    atomic_json(approval_path, approval)
    return {"status": "APPROVED", "approvalPath": APPROVAL_RELATIVE_PATH, "subjectHash": approval["subjectHash"], "reused": False}


def publish_scene_assets(run_dir: Path) -> dict[str, Any]:
    """Publish the final SceneMotionAssetHandoff after explicit approval."""

    run_dir = _strict_root(run_dir)
    verified = validate_scene_asset_release(run_dir)
    handoff = {
        "schemaVersion": 2,
        "sourceKind": SOURCE_KIND,
        "projectionReceiptPath": PROJECTION_RECEIPT_RELATIVE_PATH,
        "projectionReceiptSha256": verified["projectionReceiptSha256"],
        "sceneGraphHash": verified["projection"]["sceneGraphHash"],
        "sourceSemanticManifestHash": verified["sourceSemanticManifestHash"],
        "semanticManifestHash": verified["semanticManifestHash"],
        "assetPackPath": ASSET_PACK_RELATIVE_PATH,
        "assetPackSha256": verified["assetPackSha256"],
        "assetPackHash": verified["assetPackHash"],
        "assetPackAuditPath": AUDIT_RELATIVE_PATH,
        "assetPackAuditSha256": verified["assetPackAuditSha256"],
        "assetPackAuditHash": verified["assetPackAuditHash"],
        "bindingManifestPath": BINDING_RELATIVE_PATH,
        "bindingManifestSha256": verified["bindingManifestSha256"],
        "bindingManifestHash": verified["bindingManifestHash"],
        "approvalPath": APPROVAL_RELATIVE_PATH,
        "approvalSha256": verified["approvalSha256"],
        "approvalSubjectHash": verified["approvalSubjectHash"],
    }
    handoff["handoffId"] = "SAC-SMC-" + canonical_hash(handoff).split(":", 1)[1]
    _validate_handoff(handoff)
    handoff_path = _safe_destination(run_dir, HANDOFF_RELATIVE_PATH)
    if handoff_path.exists():
        existing = load_json(handoff_path)
        if existing != handoff:
            raise AssetReleaseError("HANDOFF_CONFLICT")
        return {"status": "PUBLISHED", "handoffPath": HANDOFF_RELATIVE_PATH, "handoff": existing, "reused": True}
    atomic_json(handoff_path, handoff)
    return {"status": "PUBLISHED", "handoffPath": HANDOFF_RELATIVE_PATH, "handoff": handoff, "reused": False}
