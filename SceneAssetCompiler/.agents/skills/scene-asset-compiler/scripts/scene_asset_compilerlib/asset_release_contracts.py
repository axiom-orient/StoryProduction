"""SAC-owned value constraints and canonical identities for scene release."""
from __future__ import annotations

from datetime import datetime
import json
import re
from typing import Any

from .util import sha256_bytes

HEX_SHA256 = re.compile(r"^[0-9a-f]{64}$")

PREF_SHA256 = re.compile(r"^sha256:[0-9a-f]{64}$")

SAFE_RELATIVE_PATH = re.compile(r"^[^/\\]+(?:/[^/\\]+)*$")

IDENTIFIER = re.compile(r"^[A-Za-z][A-Za-z0-9._:-]{1,127}$")

REQUIREMENT_ID = re.compile(r"^(?:CHARACTER_SPRITE|ENVIRONMENT_PLATE|PROP_CUTOUT|SHOT_KEYFRAME):[A-Za-z][A-Za-z0-9._:-]{1,127}$")

RUNTIME_MIME = {
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".webp": "image/webp",
}

SOURCE_MIME = {
    **RUNTIME_MIME,
    ".psd": "image/vnd.adobe.photoshop",
    ".psb": "image/vnd.adobe.photoshop",
}

REQUIREMENT_TYPES = (
    "CHARACTER_SPRITE",
    "ENVIRONMENT_PLATE",
    "PROP_CUTOUT",
    "SHOT_KEYFRAME",
)

SEMANTIC_KINDS = (
    "action",
    "dialogue",
    "emotion",
    "lightingTransition",
    "cameraMovement",
)

SEMANTIC_ID = re.compile(r"^semantic\.[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")

SCENE_ID = re.compile(r"^SCN-[0-9]{3,}$")

SHOT_ID = re.compile(r"^shot\.[a-z0-9][a-z0-9._-]{0,126}$")

ASSET_KINDS = (
    "illustration",
    "photo",
    "texture",
    "product",
    "icon",
    "vector-decoration",
)

RIGHTS_STATUSES = (
    "USER_OWNED",
    "LICENSED",
    "PUBLIC_DOMAIN",
    "CC0",
)

SOURCE_KIND = "SMC_NARRATIVE_SCENE_PROJECTION"

PROJECTION_RECEIPT_RELATIVE_PATH = "projection/narrative-scene-projection.v3.json"

ASSET_PACK_RELATIVE_PATH = "handoff/asset-pack.v2.json"

AUDIT_RELATIVE_PATH = "handoff/asset-pack-audit.v1.json"

APPROVAL_RELATIVE_PATH = "handoff/scene-asset-release-approval.v1.json"

BINDING_RELATIVE_PATH = "handoff/binding-manifest.v1.json"

HANDOFF_RELATIVE_PATH = "handoff/scene-motion-asset-handoff.v2.json"

class AssetReleaseError(ValueError):
    """Stable, machine-readable producer boundary failure."""

    def __init__(self, message: str, *, code: str | None = None, details: dict[str, Any] | None = None):
        super().__init__(message)
        token = message.split(":", 1)[0]
        self.code = code or token.lower().replace("_", "-")
        self.details = details if details is not None else {}


def canonical_json_bytes(value: Any) -> bytes:
    try:
        return json.dumps(
            value,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
    except (TypeError, ValueError) as error:
        raise AssetReleaseError(f"CANONICAL_JSON_INVALID:{error}") from error


def canonical_hash(value: Any) -> str:
    return "sha256:" + sha256_bytes(canonical_json_bytes(value))


def _require_keys(value: Any, required: set[str], allowed: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise AssetReleaseError(f"TYPE:{label}:object")
    missing = sorted(required - set(value))
    if missing:
        raise AssetReleaseError(f"MISSING:{label}:{','.join(missing)}")
    extra = sorted(set(value) - allowed)
    if extra:
        raise AssetReleaseError(f"UNKNOWN_FIELDS:{label}:{','.join(extra)}")
    return value


def _string(value: Any, label: str, *, nonempty: bool = True) -> str:
    if not isinstance(value, str) or (nonempty and not value):
        raise AssetReleaseError(f"TYPE:{label}:string")
    return value


def _relative_path(value: Any, label: str) -> str:
    value = _string(value, label)
    if "\x00" in value or "\\" in value or value.startswith("/") or ":" in value:
        raise AssetReleaseError(f"PATH_UNSAFE:{label}:{value}")
    if not SAFE_RELATIVE_PATH.fullmatch(value):
        raise AssetReleaseError(f"PATH_UNSAFE:{label}:{value}")
    parts = value.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        raise AssetReleaseError(f"PATH_UNSAFE:{label}:{value}")
    return value


def _hex(value: Any, label: str) -> str:
    value = _string(value, label)
    if not HEX_SHA256.fullmatch(value):
        raise AssetReleaseError(f"FORMAT:{label}:sha256")
    return value


def _prefixed_hash(value: Any, label: str) -> str:
    value = _string(value, label)
    if not PREF_SHA256.fullmatch(value):
        raise AssetReleaseError(f"FORMAT:{label}:sha256")
    return value


def _asset_kind(value: Any, label: str) -> str:
    value = _string(value, label)
    if value not in ASSET_KINDS:
        raise AssetReleaseError(f"VALUE:{label}:kind")
    return value


def _source_region_id(value: Any, label: str) -> str:
    value = _string(value, label)
    if not re.fullmatch(r"[A-Za-z][A-Za-z0-9._:-]{0,127}", value):
        raise AssetReleaseError(f"FORMAT:{label}:sourceRegionId")
    return value


def _identifier(value: Any, label: str) -> str:
    value = _string(value, label)
    if not IDENTIFIER.fullmatch(value):
        raise AssetReleaseError(f"FORMAT:{label}:identifier")
    return value


def _requirement_id(value: Any, label: str) -> str:
    value = _string(value, label)
    if not REQUIREMENT_ID.fullmatch(value):
        raise AssetReleaseError(f"FORMAT:{label}:requirementId")
    return value


def _rights(value: Any, label: str) -> dict[str, str]:
    value = _require_keys(value, {"status", "holder", "source"}, {"status", "holder", "source"}, label)
    if value["status"] not in RIGHTS_STATUSES:
        raise AssetReleaseError(f"VALUE:{label}.status:rights")
    for key in ("holder", "source"):
        if not isinstance(value[key], str) or not value[key].strip():
            raise AssetReleaseError(f"TYPE:{label}.{key}:string")
    if len(value["holder"]) > 256 or len(value["source"]) > 1024:
        raise AssetReleaseError(f"RANGE:{label}:rights")
    return {"status": value["status"], "holder": value["holder"], "source": value["source"]}


def _crop(value: Any, label: str) -> dict[str, int]:
    value = _require_keys(value, {"x", "y", "width", "height"}, {"x", "y", "width", "height"}, label)
    for key in ("x", "y", "width", "height"):
        number = value[key]
        if isinstance(number, bool) or not isinstance(number, int):
            raise AssetReleaseError(f"TYPE:{label}.{key}:integer")
    if value["x"] < 0 or value["y"] < 0 or value["width"] < 1 or value["height"] < 1:
        raise AssetReleaseError(f"RANGE:{label}:crop")
    return {key: value[key] for key in ("x", "y", "width", "height")}


def _is_valid_datetime(value: str) -> bool:
    try:
        datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return False
    return True


def _validate_release_request(request: dict[str, Any]) -> dict[str, Any]:
    allowed = {"schemaVersion", "releaseId", "createdAt", "projectionReceiptPath", "assets", "requirements"}
    _require_keys(request, allowed, allowed, "sceneAssetReleaseRequest")
    if request["schemaVersion"] != 1:
        raise AssetReleaseError("VERSION:sceneAssetReleaseRequest")
    _identifier(request["releaseId"], "sceneAssetReleaseRequest.releaseId")
    created_at = _string(request["createdAt"], "sceneAssetReleaseRequest.createdAt")
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})", created_at) or not _is_valid_datetime(created_at):
        raise AssetReleaseError("FORMAT:sceneAssetReleaseRequest.createdAt:date-time")
    _relative_path(request["projectionReceiptPath"], "sceneAssetReleaseRequest.projectionReceiptPath")
    if not isinstance(request["assets"], list) or not request["assets"]:
        raise AssetReleaseError("EMPTY:sceneAssetReleaseRequest.assets")
    if not isinstance(request["requirements"], list) or not request["requirements"]:
        raise AssetReleaseError("EMPTY:sceneAssetReleaseRequest.requirements")
    if len({item.get("assetId") for item in request["assets"] if isinstance(item, dict)}) != len(request["assets"]):
        raise AssetReleaseError("DUPLICATE:sceneAssetReleaseRequest.assetId")
    for index, item in enumerate(request["assets"]):
        _require_keys(item, {"assetId", "kind", "rights", "sourceRegionId", "sourceArtifact", "runtimeDerivative"}, {"assetId", "kind", "rights", "sourceRegionId", "sourceArtifact", "runtimeDerivative"}, f"sceneAssetReleaseRequest.assets[{index}]")
        _identifier(item["assetId"], f"sceneAssetReleaseRequest.assets[{index}].assetId")
        _asset_kind(item["kind"], f"sceneAssetReleaseRequest.assets[{index}].kind")
        _rights(item["rights"], f"sceneAssetReleaseRequest.assets[{index}].rights")
        _source_region_id(item["sourceRegionId"], f"sceneAssetReleaseRequest.assets[{index}].sourceRegionId")
        for name in ("sourceArtifact", "runtimeDerivative"):
            ref = item[name]
            _require_keys(ref, {"path", "sha256", "mime", "sizeBytes"}, {"path", "sha256", "mime", "sizeBytes"}, f"sceneAssetReleaseRequest.assets[{index}].{name}")
            _relative_path(ref["path"], f"sceneAssetReleaseRequest.assets[{index}].{name}")
            _hex(ref["sha256"], f"sceneAssetReleaseRequest.assets[{index}].{name}.sha256")
            _string(ref["mime"], f"sceneAssetReleaseRequest.assets[{index}].{name}.mime")
            if not isinstance(ref["sizeBytes"], int) or isinstance(ref["sizeBytes"], bool) or ref["sizeBytes"] < 1:
                raise AssetReleaseError(f"RANGE:sceneAssetReleaseRequest.assets[{index}].{name}.sizeBytes")
    seen_requirements: set[str] = set()
    for index, item in enumerate(request["requirements"]):
        _require_keys(item, {"requirementId", "requirementType", "sceneEntityId", "assetId", "kind", "rights", "sourceRegionId"}, {"requirementId", "requirementType", "sceneEntityId", "assetId", "kind", "rights", "sourceRegionId", "allowedUse", "crop"}, f"sceneAssetReleaseRequest.requirements[{index}]")
        rid = _requirement_id(item["requirementId"], f"sceneAssetReleaseRequest.requirements[{index}].requirementId")
        if rid in seen_requirements:
            raise AssetReleaseError(f"DUPLICATE_REQUIREMENT:{rid}")
        seen_requirements.add(rid)
        if item["requirementType"] not in REQUIREMENT_TYPES:
            raise AssetReleaseError(f"REQUIREMENT_TYPE_INVALID:{rid}")
        _identifier(item["sceneEntityId"], f"sceneAssetReleaseRequest.requirements[{index}].sceneEntityId")
        _identifier(item["assetId"], f"sceneAssetReleaseRequest.requirements[{index}].assetId")
        _asset_kind(item["kind"], f"sceneAssetReleaseRequest.requirements[{index}].kind")
        _rights(item["rights"], f"sceneAssetReleaseRequest.requirements[{index}].rights")
        _source_region_id(item["sourceRegionId"], f"sceneAssetReleaseRequest.requirements[{index}].sourceRegionId")
        if "allowedUse" in item:
            allowed_use = _string(item["allowedUse"], f"sceneAssetReleaseRequest.requirements[{index}].allowedUse")
            if not allowed_use.strip():
                raise AssetReleaseError(f"TYPE:sceneAssetReleaseRequest.requirements[{index}].allowedUse:string")
        if "crop" in item:
            _crop(item["crop"], f"sceneAssetReleaseRequest.requirements[{index}].crop")
    return request

def _validate_handoff(handoff: dict[str, Any]) -> None:
    fields = {
        "schemaVersion", "handoffId", "sourceKind", "projectionReceiptPath", "projectionReceiptSha256",
        "sceneGraphHash", "sourceSemanticManifestHash", "semanticManifestHash", "assetPackPath",
        "assetPackSha256", "assetPackHash", "assetPackAuditPath", "assetPackAuditSha256", "assetPackAuditHash",
        "bindingManifestPath", "bindingManifestSha256", "bindingManifestHash", "approvalPath",
        "approvalSha256", "approvalSubjectHash",
    }
    _require_keys(handoff, fields, fields, "sceneMotionAssetHandoff")
    if handoff["schemaVersion"] != 2:
        raise AssetReleaseError("VERSION:sceneMotionAssetHandoff")
    if not isinstance(handoff["handoffId"], str) or not re.fullmatch(r"SAC-SMC-[0-9a-f]{64}", handoff["handoffId"]):
        raise AssetReleaseError("FORMAT:sceneMotionAssetHandoff.handoffId")
    if handoff["sourceKind"] != SOURCE_KIND:
        raise AssetReleaseError("VALUE:sceneMotionAssetHandoff.sourceKind")
    expected_paths = {
        "projectionReceiptPath": PROJECTION_RECEIPT_RELATIVE_PATH,
        "assetPackPath": ASSET_PACK_RELATIVE_PATH,
        "assetPackAuditPath": AUDIT_RELATIVE_PATH,
        "bindingManifestPath": BINDING_RELATIVE_PATH,
        "approvalPath": APPROVAL_RELATIVE_PATH,
    }
    for key, expected in expected_paths.items():
        if handoff[key] != expected:
            raise AssetReleaseError(f"PATH_UNSUPPORTED:sceneMotionAssetHandoff.{key}")
    for key in ("projectionReceiptSha256", "assetPackSha256", "assetPackAuditSha256", "bindingManifestSha256", "approvalSha256"):
        _hex(handoff[key], f"sceneMotionAssetHandoff.{key}")
    for key in ("sceneGraphHash", "sourceSemanticManifestHash", "semanticManifestHash", "assetPackHash", "assetPackAuditHash", "bindingManifestHash", "approvalSubjectHash"):
        _prefixed_hash(handoff[key], f"sceneMotionAssetHandoff.{key}")
    identity = dict(handoff)
    identity.pop("handoffId")
    expected_id = "SAC-SMC-" + canonical_hash(identity).split(":", 1)[1]
    if handoff["handoffId"] != expected_id:
        raise AssetReleaseError("HASH_MISMATCH:sceneMotionAssetHandoff.handoffId")
