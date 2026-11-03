"""Revalidate the current SAC scene-release subject before handoff publication."""
from __future__ import annotations

from pathlib import Path
from typing import Any

from .asset_release_contracts import (
    APPROVAL_RELATIVE_PATH,
    ASSET_PACK_RELATIVE_PATH,
    AUDIT_RELATIVE_PATH,
    BINDING_RELATIVE_PATH,
    PROJECTION_RECEIPT_RELATIVE_PATH,
    AssetReleaseError,
    canonical_hash,
    _asset_kind,
    _require_keys,
    _rights,
    _source_region_id,
)
from .asset_release_files import _read_json, _strict_root, _verify_file_ref
from .asset_release_projection_contracts import (
    _validate_cinematic_sources,
    _validate_projection,
    _validate_scene_graph,
    expected_requirements,
)


def _load_prepared(run_dir: Path) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any], str, str, str]:
    run_dir = _strict_root(run_dir)
    pack, pack_bytes, pack_raw = _read_json(run_dir, ASSET_PACK_RELATIVE_PATH, "assetPack")
    audit, audit_bytes, audit_raw = _read_json(run_dir, AUDIT_RELATIVE_PATH, "assetPackAudit")
    binding, binding_bytes, binding_raw = _read_json(run_dir, BINDING_RELATIVE_PATH, "bindingManifest")
    return pack, audit, binding, {"pack": pack_bytes, "audit": audit_bytes, "binding": binding_bytes}, pack_raw, audit_raw, binding_raw


def validate_scene_asset_release(run_dir: Path) -> dict[str, Any]:
    """Validate all producer outputs and return hashes for the SMC handoff."""

    run_dir = _strict_root(run_dir)
    pack, audit, binding, _, pack_raw, audit_raw, binding_raw = _load_prepared(run_dir)
    _require_keys(pack, {"schemaVersion", "packId", "projectId", "createdAt", "projection", "assets", "requirements"}, {"schemaVersion", "packId", "projectId", "createdAt", "projection", "assets", "requirements"}, "assetPack")
    if pack["schemaVersion"] != 2 or not isinstance(pack["assets"], list) or not pack["assets"]:
        raise AssetReleaseError("ASSET_PACK_INVALID")
    _require_keys(audit, {"schemaVersion", "auditId", "status", "projection", "assetPackHash", "bindingManifestHash", "assetIds", "requirementIds", "checks"}, {"schemaVersion", "auditId", "status", "projection", "assetPackHash", "bindingManifestHash", "assetIds", "requirementIds", "checks"}, "assetPackAudit")
    if audit["schemaVersion"] != 1 or audit["status"] != "COMPLETE":
        raise AssetReleaseError("AUDIT_INCOMPLETE")
    if (
        not isinstance(audit["assetIds"], list)
        or not all(isinstance(item, str) for item in audit["assetIds"])
        or len(set(audit["assetIds"])) != len(audit["assetIds"])
        or not isinstance(audit["requirementIds"], list)
        or not all(isinstance(item, str) for item in audit["requirementIds"])
        or len(set(audit["requirementIds"])) != len(audit["requirementIds"])
        or not isinstance(audit["checks"], list)
    ):
        raise AssetReleaseError("AUDIT_INVALID")
    _require_keys(binding, {"schemaVersion", "manifestId", "projection", "requirements"}, {"schemaVersion", "manifestId", "projection", "requirements"}, "bindingManifest")
    if binding["schemaVersion"] != 1 or not isinstance(binding["requirements"], list):
        raise AssetReleaseError("BINDING_MANIFEST_INVALID")
    projection, _, projection_raw = _read_json(run_dir, PROJECTION_RECEIPT_RELATIVE_PATH, "projectionReceipt")
    _validate_projection(projection)
    graph, _, _ = _read_json(run_dir, projection["sceneGraphPath"], "sceneGraph")
    _validate_scene_graph(graph, projection["sceneGraphHash"], projection)
    _validate_cinematic_sources(projection, graph)
    expected_ref = {"receiptSha256": projection_raw, "sceneGraphHash": projection["sceneGraphHash"], "sceneCount": projection["sceneCount"], "sceneIds": projection["sceneIds"]}
    if pack["projection"] != expected_ref or audit["projection"] != expected_ref or binding["projection"] != expected_ref:
        raise AssetReleaseError("PROJECTION_BINDING_MISMATCH")
    if canonical_hash(pack) != audit["assetPackHash"] or canonical_hash(binding) != audit["bindingManifestHash"]:
        raise AssetReleaseError("AUDIT_HASH_MISMATCH")
    assets_by_id = {}
    for item in pack["assets"]:
        _require_keys(item, {"assetId", "kind", "rights", "sourceRegionId", "sourceArtifact", "runtimeDerivative"}, {"assetId", "kind", "rights", "sourceRegionId", "sourceArtifact", "runtimeDerivative"}, "assetPack.assets")
        if item["assetId"] in assets_by_id:
            raise AssetReleaseError(f"DUPLICATE_ASSET:{item['assetId']}")
        assets_by_id[item["assetId"]] = item
        _asset_kind(item["kind"], f"assetPack.assets.{item['assetId']}.kind")
        _rights(item["rights"], f"assetPack.assets.{item['assetId']}.rights")
        _source_region_id(item["sourceRegionId"], f"assetPack.assets.{item['assetId']}.sourceRegionId")
        _verify_file_ref(run_dir, item["sourceArtifact"], f"asset.{item['assetId']}.sourceArtifact")
        _verify_file_ref(run_dir, item["runtimeDerivative"], f"asset.{item['assetId']}.runtimeDerivative", runtime=True)
    expected = expected_requirements(graph)
    expected_ids = {item["requirementId"] for item in expected}
    binding_ids = {item.get("requirementId") for item in binding["requirements"]}
    if binding_ids != expected_ids or len(binding_ids) != len(binding["requirements"]):
        raise AssetReleaseError("BINDING_REQUIREMENTS_MISMATCH")
    for item in binding["requirements"]:
        _require_keys(item, {"requirementId", "requirementType", "sceneEntityId", "assetId", "kind", "rights", "sourceRegionId", "allowedUse", "crop"}, {"requirementId", "requirementType", "sceneEntityId", "assetId", "kind", "rights", "sourceRegionId", "allowedUse", "crop"}, "bindingManifest.requirements")
        if item.get("assetId") not in assets_by_id:
            raise AssetReleaseError(f"REQUIREMENT_ASSET_MISSING:{item.get('requirementId')}")
        asset = assets_by_id[item["assetId"]]
        if item.get("kind") != asset.get("kind") or item.get("rights") != asset.get("rights") or item.get("sourceRegionId") != asset.get("sourceRegionId"):
            raise AssetReleaseError(f"REQUIREMENT_ASSET_METADATA_MISMATCH:{item.get('requirementId')}")
    if set(audit["assetIds"]) != set(assets_by_id) or set(audit["requirementIds"]) != expected_ids:
        raise AssetReleaseError("AUDIT_COVERAGE_MISMATCH")
    expected_check_ids = {
        *(f"source:{asset_id}" for asset_id in assets_by_id),
        *(f"runtime:{asset_id}" for asset_id in assets_by_id),
        *(f"requirement:{requirement_id}" for requirement_id in expected_ids),
    }
    actual_check_ids = []
    for check in audit["checks"]:
        if not isinstance(check, dict) or not isinstance(check.get("checkId"), str):
            raise AssetReleaseError("AUDIT_INVALID")
        actual_check_ids.append(check["checkId"])
    if len(actual_check_ids) != len(set(actual_check_ids)) or set(actual_check_ids) != expected_check_ids:
        raise AssetReleaseError("AUDIT_CHECKS_MISMATCH")
    if any(check.get("status") != "PASS" for check in audit["checks"]):
        raise AssetReleaseError("AUDIT_CHECK_FAILED")
    approval, _, approval_raw = _read_json(run_dir, APPROVAL_RELATIVE_PATH, "sceneAssetReleaseApproval")
    _require_keys(approval, {"schemaVersion", "approvalId", "status", "reviewer", "approvedAt", "note", "subject", "subjectHash"}, {"schemaVersion", "approvalId", "status", "reviewer", "approvedAt", "note", "subject", "subjectHash"}, "sceneAssetReleaseApproval")
    if approval["schemaVersion"] != 1 or approval["status"] != "APPROVED" or not str(approval["reviewer"]).strip():
        raise AssetReleaseError("APPROVAL_INVALID")
    subject = {"sceneGraphHash": projection["sceneGraphHash"], "assetPackHash": canonical_hash(pack), "assetPackAuditHash": canonical_hash(audit), "bindingManifestHash": canonical_hash(binding)}
    if approval["subject"] != subject or approval["subjectHash"] != canonical_hash(subject):
        raise AssetReleaseError("APPROVAL_STALE")
    return {
        "pack": pack,
        "audit": audit,
        "binding": binding,
        "approval": approval,
        "projection": projection,
        "projectionReceiptSha256": projection_raw,
        "assetPackSha256": pack_raw,
        "assetPackAuditSha256": audit_raw,
        "bindingManifestSha256": binding_raw,
        "approvalSha256": approval_raw,
        "assetPackHash": subject["assetPackHash"],
        "assetPackAuditHash": subject["assetPackAuditHash"],
        "bindingManifestHash": subject["bindingManifestHash"],
        "approvalSubjectHash": approval["subjectHash"],
        "sourceSemanticManifestHash": projection["sourceSemanticManifestHash"],
        "semanticManifestHash": projection["semanticManifestHash"],
    }
