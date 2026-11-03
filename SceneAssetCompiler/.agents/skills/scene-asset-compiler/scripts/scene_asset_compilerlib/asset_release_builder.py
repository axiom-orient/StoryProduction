"""Build SAC scene-release pack, audit, and binding candidates."""
from __future__ import annotations

from pathlib import Path
from typing import Any

from .asset_release_contracts import (
    PROJECTION_RECEIPT_RELATIVE_PATH,
    AssetReleaseError,
    canonical_hash,
    _asset_kind,
    _crop,
    _identifier,
    _relative_path,
    _rights,
    _source_region_id,
    _string,
)
from .asset_release_files import _read_json, _safe_file, _strict_root, _verify_file_ref
from .asset_release_projection_contracts import (
    _validate_cinematic_sources,
    _validate_projection,
    _validate_scene_graph,
    expected_requirements,
)
from .util import load_json


def _build_release_objects(
    run_dir: Path,
    request: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any]]:
    run_dir = _strict_root(run_dir)
    projection_path = _relative_path(request["projectionReceiptPath"], "projectionReceiptPath")
    projection, _, projection_raw_hash = _read_json(run_dir, projection_path, "projectionReceipt")
    _validate_projection(projection)
    graph, _, _ = _read_json(run_dir, projection["sceneGraphPath"], "sceneGraph")
    _validate_scene_graph(graph, projection["sceneGraphHash"], projection)
    _validate_cinematic_sources(projection, graph)
    if projection_path != PROJECTION_RECEIPT_RELATIVE_PATH:
        raise AssetReleaseError(f"PROJECTION_PATH_UNSUPPORTED:{projection_path}")

    expected = expected_requirements(graph)
    expected_by_id = {item["requirementId"]: item for item in expected}
    req_by_id = {item["requirementId"]: item for item in request["requirements"]}
    if set(req_by_id) != set(expected_by_id):
        missing = sorted(set(expected_by_id) - set(req_by_id))
        extra = sorted(set(req_by_id) - set(expected_by_id))
        raise AssetReleaseError(f"REQUIREMENTS_MISMATCH:missing={','.join(missing)}:extra={','.join(extra)}")
    for requirement_id, expected_item in expected_by_id.items():
        given = req_by_id[requirement_id]
        if any(given[key] != expected_item[key] for key in ("requirementType", "sceneEntityId")):
            raise AssetReleaseError(f"REQUIREMENT_BINDING_MISMATCH:{requirement_id}")

    assets_by_id = {item["assetId"]: item for item in request["assets"]}
    used_asset_ids: set[str] = set()
    normalized_assets = []
    for item in sorted(request["assets"], key=lambda entry: entry["assetId"]):
        asset_id = item["assetId"]
        source = _verify_file_ref(run_dir, item["sourceArtifact"], f"asset.{asset_id}.sourceArtifact")
        runtime = _verify_file_ref(
            run_dir,
            item["runtimeDerivative"],
            f"asset.{asset_id}.runtimeDerivative",
            runtime=True,
        )
        if source["path"] == runtime["path"] or source["sha256"] == runtime["sha256"]:
            raise AssetReleaseError(f"SOURCE_RUNTIME_ALIAS:{asset_id}")
        normalized_assets.append({
            "assetId": asset_id,
            "kind": _asset_kind(item["kind"], f"asset.{asset_id}.kind"),
            "rights": _rights(item["rights"], f"asset.{asset_id}.rights"),
            "sourceRegionId": _source_region_id(item["sourceRegionId"], f"asset.{asset_id}.sourceRegionId"),
            "sourceArtifact": source,
            "runtimeDerivative": runtime,
        })

    normalized_requirements = []
    for item in sorted(req_by_id.values(), key=lambda entry: entry["requirementId"]):
        requirement_id = item["requirementId"]
        asset_id = item["assetId"]
        if asset_id not in assets_by_id:
            raise AssetReleaseError(f"REQUIREMENT_ASSET_MISSING:{requirement_id}:{asset_id}")
        used_asset_ids.add(asset_id)
        asset = assets_by_id[asset_id]
        if item["kind"] != asset["kind"] or item["rights"] != asset["rights"] or item["sourceRegionId"] != asset["sourceRegionId"]:
            raise AssetReleaseError(f"REQUIREMENT_ASSET_METADATA_MISMATCH:{requirement_id}")
        allowed_use = item.get("allowedUse", f"SceneMotionCompiler:{item['requirementType']}")
        allowed_use = _string(allowed_use, f"requirement.{requirement_id}.allowedUse")
        if not allowed_use.strip():
            raise AssetReleaseError(f"TYPE:requirement.{requirement_id}.allowedUse:string")
        crop = _crop(
            item.get("crop", {"x": 0, "y": 0, "width": 1, "height": 1}),
            f"requirement.{requirement_id}.crop",
        )
        normalized_requirements.append({
            "requirementId": requirement_id,
            "requirementType": item["requirementType"],
            "sceneEntityId": item["sceneEntityId"],
            "assetId": asset_id,
            "kind": item["kind"],
            "rights": item["rights"],
            "sourceRegionId": item["sourceRegionId"],
            "allowedUse": allowed_use.strip(),
            "crop": crop,
        })
    if used_asset_ids != set(assets_by_id):
        raise AssetReleaseError(f"UNBOUND_ASSETS:{','.join(sorted(set(assets_by_id) - used_asset_ids))}")

    projection_ref = {
        "receiptSha256": projection_raw_hash,
        "sceneGraphHash": projection["sceneGraphHash"],
        "sceneCount": projection["sceneCount"],
        "sceneIds": projection["sceneIds"],
    }
    run_request_path = _safe_file(run_dir, "request.json", "runRequest")
    project_id = _identifier(load_json(run_request_path).get("project_id", "scene-asset-compiler"), "run.project_id")
    pack_identity = {
        "projectId": project_id,
        "projection": projection_ref,
        "assets": normalized_assets,
        "requirements": normalized_requirements,
    }
    pack = {
        "schemaVersion": 2,
        "packId": "SAP2-" + canonical_hash(pack_identity).split(":", 1)[1],
        "projectId": project_id,
        "createdAt": request["createdAt"],
        "projection": projection_ref,
        "assets": normalized_assets,
        "requirements": normalized_requirements,
    }
    binding_identity = {"projection": projection_ref, "requirements": normalized_requirements}
    binding = {
        "schemaVersion": 1,
        "manifestId": "BIND1-" + canonical_hash(binding_identity).split(":", 1)[1],
        "projection": projection_ref,
        "requirements": normalized_requirements,
    }
    pack_hash = canonical_hash(pack)
    binding_hash = canonical_hash(binding)
    checks = []
    for asset in normalized_assets:
        checks.extend([
            {
                "checkId": f"source:{asset['assetId']}",
                "targetKind": "SOURCE_ARTIFACT",
                "targetId": asset["assetId"],
                "status": "PASS",
                "evidence": [f"sha256:{asset['sourceArtifact']['sha256']}"],
            },
            {
                "checkId": f"runtime:{asset['assetId']}",
                "targetKind": "RUNTIME_DERIVATIVE",
                "targetId": asset["assetId"],
                "status": "PASS",
                "evidence": [f"sha256:{asset['runtimeDerivative']['sha256']}", asset["runtimeDerivative"]["mime"]],
            },
        ])
    for item in normalized_requirements:
        checks.append({
            "checkId": f"requirement:{item['requirementId']}",
            "targetKind": "REQUIREMENT",
            "targetId": item["requirementId"],
            "status": "PASS",
            "evidence": [f"asset:{item['assetId']}"],
        })
    audit_identity = {
        "projection": projection_ref,
        "packHash": pack_hash,
        "bindingManifestHash": binding_hash,
        "checks": checks,
    }
    audit = {
        "schemaVersion": 1,
        "auditId": "SAA1-" + canonical_hash(audit_identity).split(":", 1)[1],
        "status": "COMPLETE",
        "projection": projection_ref,
        "assetPackHash": pack_hash,
        "bindingManifestHash": binding_hash,
        "assetIds": [asset["assetId"] for asset in normalized_assets],
        "requirementIds": [item["requirementId"] for item in normalized_requirements],
        "checks": checks,
    }
    return pack, audit, binding, {
        "projectionPath": projection_path,
        "projection": projection,
        "projectionRawHash": projection_raw_hash,
    }
