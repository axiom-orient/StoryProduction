"""Pure validation of SMC-owned projection and scene graph contracts for SAC release."""
from __future__ import annotations

import base64
import re
from typing import Any

from .asset_release_contracts import (
    AssetReleaseError,
    SCENE_ID,
    SEMANTIC_ID,
    SEMANTIC_KINDS,
    SHOT_ID,
    canonical_hash,
    _hex,
    _prefixed_hash,
    _relative_path,
    _require_keys,
    _string,
)
from .util import load_json_bytes, sha256_bytes


def _validate_projection(receipt: dict[str, Any], label: str = "projection") -> None:
    _require_keys(
        receipt,
        {
            "schemaVersion", "projectionId", "narrativeId", "sourceNarrativeId",
            "bundleSha256", "packageSha256", "sceneMapSha256", "sceneGraphPath",
            "sourceSemanticManifestHash", "semanticManifestHash", "sceneGraphHash",
            "sceneCount", "sceneIds", "sceneToShot", "shotCount", "sourceArtifacts",
            "directorPlanSha256", "continuityBibleSha256", "directorApprovalSha256",
        },
        {
            "schemaVersion", "projectionId", "narrativeId", "sourceNarrativeId",
            "bundleSha256", "packageSha256", "sceneMapSha256", "sceneGraphPath",
            "sourceSemanticManifestHash", "semanticManifestHash", "sceneGraphHash",
            "sceneCount", "sceneIds", "sceneToShot", "shotCount", "sourceArtifacts",
            "directorPlanSha256", "continuityBibleSha256", "directorApprovalSha256",
        },
        label,
    )
    if receipt["schemaVersion"] != 3:
        raise AssetReleaseError(f"VERSION:{label}")
    for key in ("projectionId", "narrativeId", "sourceNarrativeId"):
        _string(receipt[key], f"{label}.{key}")
    _prefixed_hash(receipt["bundleSha256"], f"{label}.bundleSha256")
    _hex(receipt["packageSha256"], f"{label}.packageSha256")
    _hex(receipt["sceneMapSha256"], f"{label}.sceneMapSha256")
    _prefixed_hash(receipt["sourceSemanticManifestHash"], f"{label}.sourceSemanticManifestHash")
    _prefixed_hash(receipt["semanticManifestHash"], f"{label}.semanticManifestHash")
    graph_path = _relative_path(receipt["sceneGraphPath"], f"{label}.sceneGraphPath")
    if not re.fullmatch(r"projection/generations/[A-Za-z0-9._-]+/scene-graph\.json", graph_path):
        raise AssetReleaseError(f"PATH_UNSAFE:{label}.sceneGraphPath")
    _prefixed_hash(receipt["sceneGraphHash"], f"{label}.sceneGraphHash")
    if not isinstance(receipt["sceneCount"], int) or isinstance(receipt["sceneCount"], bool) or receipt["sceneCount"] < 1:
        raise AssetReleaseError(f"RANGE:{label}.sceneCount")
    scene_ids = receipt["sceneIds"]
    if not isinstance(scene_ids, list) or len(scene_ids) != receipt["sceneCount"] or len(set(scene_ids)) != len(scene_ids):
        raise AssetReleaseError(f"SCENE_IDS_INVALID:{label}")
    if any(not isinstance(item, str) or not re.fullmatch(r"SCN-[0-9]{3,}", item) for item in scene_ids):
        raise AssetReleaseError(f"SCENE_IDS_INVALID:{label}")
    mappings = receipt["sceneToShot"]
    if not isinstance(receipt["shotCount"], int) or isinstance(receipt["shotCount"], bool) or receipt["shotCount"] < 1:
        raise AssetReleaseError(f"RANGE:{label}.shotCount")
    if not isinstance(mappings, list) or len(mappings) != receipt["shotCount"]:
        raise AssetReleaseError(f"SCENE_MAP_INVALID:{label}")
    for index, mapping in enumerate(mappings):
        _require_keys(mapping, {"sceneId", "shotId", "order"}, {"sceneId", "shotId", "order"}, f"{label}.sceneToShot[{index}]")
        if mapping["sceneId"] not in scene_ids or mapping["order"] != index + 1:
            raise AssetReleaseError(f"SCENE_MAP_INVALID:{label}")
        if not isinstance(mapping["shotId"], str) or not SHOT_ID.fullmatch(mapping["shotId"]):
            raise AssetReleaseError(f"SCENE_MAP_INVALID:{label}")

    mapped_scenes = list(dict.fromkeys(item["sceneId"] for item in mappings))
    if mapped_scenes != scene_ids or len({item["shotId"] for item in mappings}) != len(mappings):
        raise AssetReleaseError(f"SCENE_MAP_INVALID:{label}")
    for key in ("directorPlanSha256", "continuityBibleSha256", "directorApprovalSha256"):
        _prefixed_hash(receipt[key], f"{label}.{key}")
    artifacts = receipt["sourceArtifacts"]
    names = {"sceneMap", "continuityBible", "directorPlan", "directorApproval"}
    _require_keys(artifacts, names, names, f"{label}.sourceArtifacts")
    for key, ref in artifacts.items():
        _require_keys(ref, {"encoding", "sha256", "bytesBase64"}, {"encoding", "sha256", "bytesBase64"}, f"{label}.sourceArtifacts.{key}")
        if ref["encoding"] != "base64url" or not isinstance(ref["bytesBase64"], str) or not re.fullmatch(r"[A-Za-z0-9_-]+", ref["bytesBase64"]):
            raise AssetReleaseError(f"CAPSULE_ENCODING:{label}.sourceArtifacts.{key}")
        _prefixed_hash(ref["sha256"], f"{label}.sourceArtifacts.{key}.sha256")


def _validate_semantic_locator(locator: Any, label: str, kind: str | None = None) -> None:
    if not isinstance(locator, dict):
        raise AssetReleaseError(f"TYPE:{label}:object")
    required = {"sceneId", "field", "occurrenceIndex"}
    allowed = {"sceneId", "field", "occurrenceIndex", "character", "dialogueIndex"}
    if kind == "dialogue":
        required |= {"character", "dialogueIndex"}
    _require_keys(locator, required, allowed, label)
    if not SCENE_ID.fullmatch(locator["sceneId"]):
        raise AssetReleaseError(f"FORMAT:{label}.sceneId")
    if locator["field"] not in SEMANTIC_KINDS:
        raise AssetReleaseError(f"ENUM:{label}.field")
    if not isinstance(locator["occurrenceIndex"], int) or isinstance(locator["occurrenceIndex"], bool) or locator["occurrenceIndex"] < 0:
        raise AssetReleaseError(f"RANGE:{label}.occurrenceIndex")
    if locator["field"] == "dialogue":
        if not isinstance(locator.get("character"), str) or not locator["character"].strip():
            raise AssetReleaseError(f"TYPE:{label}.character:string")
        if not isinstance(locator.get("dialogueIndex"), int) or isinstance(locator["dialogueIndex"], bool) or locator["dialogueIndex"] != locator["occurrenceIndex"]:
            raise AssetReleaseError(f"RANGE:{label}.dialogueIndex")
    elif "character" in locator or "dialogueIndex" in locator:
        raise AssetReleaseError(f"KEYS:{label}")


def _validate_graph_semantic_manifest(manifest: Any, shots: list[dict[str, Any]], projection: dict[str, Any] | None = None) -> None:
    if not isinstance(manifest, dict):
        raise AssetReleaseError("TYPE:sceneGraph.semanticManifest:object")
    _require_keys(
        manifest,
        {"schemaVersion", "sourceSemanticManifestHash", "semanticManifestHash", "entries"},
        {"schemaVersion", "sourceSemanticManifestHash", "semanticManifestHash", "entries"},
        "sceneGraph.semanticManifest",
    )
    if manifest["schemaVersion"] != 1:
        raise AssetReleaseError("VERSION:sceneGraph.semanticManifest")
    _prefixed_hash(manifest["sourceSemanticManifestHash"], "sceneGraph.semanticManifest.sourceSemanticManifestHash")
    _prefixed_hash(manifest["semanticManifestHash"], "sceneGraph.semanticManifest.semanticManifestHash")
    if not isinstance(manifest["entries"], list):
        raise AssetReleaseError("TYPE:sceneGraph.semanticManifest.entries:array")
    expected_manifest_hash = canonical_hash({
        "schemaVersion": manifest["schemaVersion"],
        "sourceSemanticManifestHash": manifest["sourceSemanticManifestHash"],
        "entries": manifest["entries"],
    })
    if manifest["semanticManifestHash"] != expected_manifest_hash:
        raise AssetReleaseError("HASH_MISMATCH:sceneGraph.semanticManifest")
    shot_ids = {shot.get("id") for shot in shots}
    shot_scene_by_id = {shot.get("id"): shot.get("sourceSceneId") for shot in shots}
    semantic_ids: set[str] = set()
    locator_keys: set[tuple[Any, ...]] = set()
    for index, entry in enumerate(manifest["entries"]):
        label = f"sceneGraph.semanticManifest.entries[{index}]"
        if not isinstance(entry, dict):
            raise AssetReleaseError(f"TYPE:{label}:object")
        _require_keys(
            entry,
            {"semanticId", "entryHash", "shotId", "sourceSceneId", "kind", "sourceText", "sourceLocator"},
            {"semanticId", "entryHash", "shotId", "sourceSceneId", "kind", "sourceText", "sourceLocator"},
            label,
        )
        semantic_id = entry["semanticId"]
        if not isinstance(semantic_id, str) or not SEMANTIC_ID.fullmatch(semantic_id):
            raise AssetReleaseError(f"FORMAT:{label}.semanticId")
        if semantic_id in semantic_ids:
            raise AssetReleaseError(f"DUPLICATE:{label}.semanticId")
        semantic_ids.add(semantic_id)
        _prefixed_hash(entry["entryHash"], f"{label}.entryHash")
        _string(entry["sourceSceneId"], f"{label}.sourceSceneId")
        if not SCENE_ID.fullmatch(entry["sourceSceneId"]):
            raise AssetReleaseError(f"FORMAT:{label}.sourceSceneId")
        if entry["kind"] not in SEMANTIC_KINDS:
            raise AssetReleaseError(f"ENUM:{label}.kind")
        if not isinstance(entry["sourceText"], str) or not entry["sourceText"]:
            raise AssetReleaseError(f"TYPE:{label}.sourceText:string")
        _validate_semantic_locator(entry["sourceLocator"], f"{label}.sourceLocator", entry["kind"])
        locator = entry["sourceLocator"]
        locator_key = (locator["sceneId"], locator["field"], locator["occurrenceIndex"])
        if locator_key in locator_keys:
            raise AssetReleaseError(f"DUPLICATE:{label}.sourceLocator")
        locator_keys.add(locator_key)
        if entry["sourceSceneId"] != locator["sceneId"] or entry["kind"] != locator["field"]:
            raise AssetReleaseError(f"BINDING_MISMATCH:{label}")
        if entry["shotId"] not in shot_ids or shot_scene_by_id.get(entry["shotId"]) != entry["sourceSceneId"]:
            raise AssetReleaseError(f"BINDING_MISMATCH:{label}.shotId")
        expected_entry_hash = canonical_hash({key: value for key, value in entry.items() if key != "entryHash"})
        if entry["entryHash"] != expected_entry_hash:
            raise AssetReleaseError(f"HASH_MISMATCH:{label}.entryHash")
    if projection is not None:
        if projection["sourceSemanticManifestHash"] != manifest["sourceSemanticManifestHash"] or projection["semanticManifestHash"] != manifest["semanticManifestHash"]:
            raise AssetReleaseError("PROJECTION_SEMANTIC_BINDING_MISMATCH")


def _validate_scene_graph(graph: dict[str, Any], expected_hash: str, projection: dict[str, Any] | None = None) -> None:
    if graph.get("schemaVersion") != 3:
        raise AssetReleaseError("VERSION:sceneGraph")
    for key in ("characters", "environments", "props", "shots"):
        if not isinstance(graph.get(key), list):
            raise AssetReleaseError(f"TYPE:sceneGraph.{key}:array")
    for key in ("characters", "environments", "props", "shots"):
        ids = []
        for index, entity in enumerate(graph[key]):
            if not isinstance(entity, dict) or not isinstance(entity.get("id"), str) or not entity["id"]:
                raise AssetReleaseError(f"SCENE_GRAPH_ENTITY_INVALID:{key}[{index}]")
            ids.append(entity["id"])
        if len(ids) != len(set(ids)):
            raise AssetReleaseError(f"SCENE_GRAPH_ENTITY_DUPLICATE:{key}")
    if not graph["environments"]:
        raise AssetReleaseError("SCENE_GRAPH_ENVIRONMENTS_EMPTY")
    env_ids = {entity["id"] for entity in graph["environments"]}
    char_ids = {entity["id"] for entity in graph["characters"]}
    prop_ids = {entity["id"] for entity in graph["props"]}
    for shot in graph["shots"]:
        if shot.get("environmentId") not in env_ids:
            raise AssetReleaseError("SCENE_GRAPH_REFERENCE_INVALID:environment")
        if any(item not in char_ids for item in shot.get("characterIds", [])):
            raise AssetReleaseError("SCENE_GRAPH_REFERENCE_INVALID:character")
        if any(item not in prop_ids for item in shot.get("propIds", [])):
            raise AssetReleaseError("SCENE_GRAPH_REFERENCE_INVALID:prop")
    timeline = graph.get("timeline", {})
    fps = timeline.get("fps")
    if not isinstance(fps, int) or isinstance(fps, bool) or not 1 <= fps <= 240:
        raise AssetReleaseError("SCENE_GRAPH_FPS")
    frames = [shot.get("durationFrames") for shot in graph["shots"]]
    if not frames or any(not isinstance(n, int) or isinstance(n, bool) or n < 1 for n in frames):
        raise AssetReleaseError("SCENE_GRAPH_DURATION")
    if sum(frames) > 9007199254740991 or sum(frames) != timeline.get("totalDurationFrames"):
        raise AssetReleaseError("SCENE_GRAPH_DURATION_SUM")
    if projection is not None:
        mappings = [{"sceneId":shot["sourceSceneId"],"shotId":shot["id"],"order":i+1} for i,shot in enumerate(graph["shots"])]
        if mappings != projection["sceneToShot"]:
            raise AssetReleaseError("SCENE_GRAPH_SHOT_MAPPING")
        for key in ("directorPlanSha256", "continuityBibleSha256", "directorApprovalSha256"):
            if graph.get(key) != projection[key]:
                raise AssetReleaseError("SCENE_GRAPH_CINEMATIC_BINDING:" + key)
        if graph.get("sourceNarrativeSha256") != "sha256:" + projection["packageSha256"] or graph.get("sourceSceneMapSha256") != "sha256:" + projection["sceneMapSha256"]:
            raise AssetReleaseError("SCENE_GRAPH_SOURCE_BINDING")
    _validate_graph_semantic_manifest(graph.get("semanticManifest"), graph["shots"], projection)
    actual = canonical_hash(graph)
    if actual != expected_hash:
        raise AssetReleaseError(f"HASH_MISMATCH:sceneGraph:{expected_hash}:{actual}")


def _validate_cinematic_sources(projection: dict[str, Any], graph: dict[str, Any]) -> None:
    artifacts = {}
    for key, ref in projection["sourceArtifacts"].items():
        encoded = ref["bytesBase64"]
        if len(encoded) > 11184811:
            raise AssetReleaseError("CINEMATIC_CAPSULE_SIZE")
        try:
            raw = base64.urlsafe_b64decode(encoded + "=" * (-len(encoded) % 4))
            if base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=") != encoded or "sha256:" + sha256_bytes(raw) != ref["sha256"]:
                raise AssetReleaseError("CINEMATIC_CAPSULE_HASH:" + key)
            value = load_json_bytes(raw)
        except (ValueError, UnicodeError) as error:
            raise AssetReleaseError("CINEMATIC_CAPSULE_INVALID:" + key) from error
        artifacts[key] = value
        if key != "sceneMap":
            expected = canonical_hash({k:v for k,v in value.items() if k != "contentHash"})
            if value.get("contentHash") != expected:
                raise AssetReleaseError("CINEMATIC_CONTENT_HASH:" + key)
    plan = artifacts["directorPlan"]; bible = artifacts["continuityBible"]; approval = artifacts["directorApproval"]
    for key, artifact in (("directorPlanSha256",plan),("continuityBibleSha256",bible),("directorApprovalSha256",approval)):
        if artifact["contentHash"] != projection[key]:
            raise AssetReleaseError("CINEMATIC_BINDING:" + key)
    for value in (plan,bible,approval):
        if value.get("narrativePackageSha256") != "sha256:" + projection["packageSha256"] or value.get("sceneMapSha256") != "sha256:" + projection["sceneMapSha256"]:
            raise AssetReleaseError("CINEMATIC_SOURCE_STALE")
    if projection["sourceArtifacts"]["sceneMap"]["sha256"] != "sha256:" + projection["sceneMapSha256"]:
        raise AssetReleaseError("CINEMATIC_SCENE_MAP_HASH")
    if approval.get("decision") != "APPROVED" or approval.get("directorPlanSha256") != plan["contentHash"] or approval.get("continuityBibleSha256") != bible["contentHash"] or plan.get("continuityBibleSha256") != bible["contentHash"]:
        raise AssetReleaseError("CINEMATIC_APPROVAL_STALE")
    intended = [shot for scene in plan["scenes"] for shot in scene["shots"]]
    if len(intended) != len(graph["shots"]) or plan["fps"] != graph["timeline"]["fps"]:
        raise AssetReleaseError("CINEMATIC_SHOT_COUNT_OR_FPS")
    for source, target in zip(intended, graph["shots"]):
        if source["shotId"] != target["id"] or source["durationFrames"] != target["durationFrames"] or source["subjectAction"] != target["action"]:
            raise AssetReleaseError("CINEMATIC_SHOT_PROJECTION")
        for key in ("shotSize","cameraAngle","cameraMovement","lensIntent"):
            if source[key] != target["camera"][key]:
                raise AssetReleaseError("CINEMATIC_CAMERA_PROJECTION")
        for key in ("semanticRefs","narrativeRefs","continuityRefs","referencePolicy","composition","performanceBeat","visualPriority"):
            if source[key] != target[key]:
                raise AssetReleaseError("CINEMATIC_INTENT_PROJECTION:" + key)


def expected_requirements(graph: dict[str, Any]) -> list[dict[str, str]]:
    result: list[dict[str, str]] = []
    for key, requirement_type in (
        ("characters", "CHARACTER_SPRITE"),
        ("environments", "ENVIRONMENT_PLATE"),
        ("props", "PROP_CUTOUT"),
        ("shots", "SHOT_KEYFRAME"),
    ):
        for entity in graph[key]:
            entity_id = entity["id"]
            result.append({
                "requirementId": f"{requirement_type}:{entity_id}",
                "requirementType": requirement_type,
                "sceneEntityId": entity_id,
            })
    return result
