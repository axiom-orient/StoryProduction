"""Whole-map SceneMap v2 validation for the fixture-side maintenance tool.

The helper remains read-only and fixture-scoped. It composes the owner-local
value guards, canonical identity rules, and shape checks with cross-scene
occurrence-bijection validation. ``validate_map`` is its only maintained
module entrypoint; the CLI imports fixture helpers from their owning modules.
"""

from __future__ import annotations

from typing import Any as _Any

from scene_map_v2_contract import (
    HASH64 as _HASH64,
    HEX64 as _HEX64,
    MAP_ID as _MAP_ID,
    expect_array as _expect_array,
    expect_object as _expect_object,
    expect_string as _expect_string,
    require as _require,
)
from scene_map_v2_identity import (
    expected_entry_hash as _expected_entry_hash,
    expected_manifest_hash as _expected_manifest_hash,
    expected_map_id as _expected_map_id,
)
from scene_map_v2_shape import (
    _validate_entry,
    _validate_scene,
)

__all__ = ["validate_map"]


def validate_map(document: _Any, *, check_hashes: bool = True) -> None:
    top = _expect_object(
        document,
        "map",
        frozenset({"schema_version", "map_id", "narrative_id", "screenplay_sha256", "scene_ir_sha256", "scenes", "semantic_manifest"}),
        frozenset({"schema_version", "map_id", "narrative_id", "screenplay_sha256", "scene_ir_sha256", "scenes", "semantic_manifest"}),
    )
    _require(top["schema_version"] == 2, "schema_version must be 2")
    _require(_MAP_ID.fullmatch(_expect_string(top["map_id"], "map.map_id", 69, 69)) is not None, "map_id is invalid")
    _expect_string(top["narrative_id"], "map.narrative_id", 1, 512)
    _require(_HEX64.fullmatch(_expect_string(top["screenplay_sha256"], "map.screenplay_sha256", 64, 64)) is not None, "screenplay_sha256 is invalid")
    _require(_HEX64.fullmatch(_expect_string(top["scene_ir_sha256"], "map.scene_ir_sha256", 64, 64)) is not None, "scene_ir_sha256 is invalid")

    scenes = _expect_array(top["scenes"], "map.scenes", 1, 10000)
    scene_by_id: dict[str, dict[str, Any]] = {}
    for index, scene in enumerate(scenes):
        _validate_scene(scene, index)
        scene_id = scene["scene_id"]
        _require(scene_id not in scene_by_id, f"duplicate scene_id: {scene_id}")
        _require(scene["order"] == index + 1, f"scene order is not contiguous at {scene_id}")
        scene_by_id[scene_id] = scene

    manifest = _expect_object(
        top["semantic_manifest"],
        "map.semantic_manifest",
        frozenset({"schema_version", "semantic_manifest_hash", "entries"}),
        frozenset({"schema_version", "semantic_manifest_hash", "entries"}),
    )
    _require(manifest["schema_version"] == 1, "semantic_manifest.schema_version must be 1")
    _require(_HASH64.fullmatch(_expect_string(manifest["semantic_manifest_hash"], "semantic_manifest.semantic_manifest_hash", 71, 71)) is not None, "semantic_manifest_hash is invalid")
    entries = _expect_array(manifest["entries"], "semantic_manifest.entries", 0, 100000)

    semantic_ids: set[str] = set()
    locators: set[tuple[str, str, int]] = set()
    action_refs: dict[str, set[int]] = {scene_id: set() for scene_id in scene_by_id}
    dialogue_refs: dict[str, set[int]] = {scene_id: set() for scene_id in scene_by_id}
    directive_refs: dict[tuple[str, str], set[int]] = {}
    parsed_entries: list[dict[str, Any]] = []
    for index, entry in enumerate(entries):
        value = _validate_entry(entry, index)
        parsed_entries.append(value)
        semantic_id = value["semantic_id"]
        _require(semantic_id not in semantic_ids, f"duplicate semantic_id: {semantic_id}")
        semantic_ids.add(semantic_id)

        locator = value["source_locator"]
        locator_key = (locator["scene_id"], locator["field"], locator["occurrence_index"])
        _require(locator_key not in locators, f"duplicate source_locator: {locator_key}")
        locators.add(locator_key)

        scene = scene_by_id.get(value["scene_id"])
        _require(scene is not None, f"entry {semantic_id} references unknown scene_id")
        _require(locator["scene_id"] == value["scene_id"], f"entry {semantic_id} locator scene_id drift")
        kind = value["kind"]
        occurrence = locator["occurrence_index"]
        if kind == "action":
            _require(occurrence < len(scene["action"]), f"action occurrence_index {occurrence} is missing in {value['scene_id']}")
            _require(value["source_text"] == scene["action"][occurrence], f"action source_text drift at {value['scene_id']}[{occurrence}]")
            action_refs[value["scene_id"]].add(occurrence)
        elif kind == "dialogue":
            _require(occurrence < len(scene["dialogue"]), f"dialogue occurrence_index {occurrence} is missing in {value['scene_id']}")
            dialogue_index = locator["dialogue_index"]
            _require(dialogue_index < len(scene["dialogue"]), f"dialogue_index {dialogue_index} is missing in {value['scene_id']}")
            dialogue = scene["dialogue"][dialogue_index]
            _require(locator["character"] == dialogue["character"], f"dialogue character drift at {value['scene_id']}[{dialogue_index}]")
            _require(value["source_text"] == dialogue["line"], f"dialogue source_text drift at {value['scene_id']}[{dialogue_index}]")
            _require(dialogue_index not in dialogue_refs[value["scene_id"]], f"duplicate dialogue_index {dialogue_index} in {value['scene_id']}")
            dialogue_refs[value["scene_id"]].add(dialogue_index)
        else:
            directive_refs.setdefault((value["scene_id"], kind), set()).add(occurrence)

    # Action and dialogue channels are the v1 arrays. Every item must have
    # exactly one manifest entry and every entry must point to one item.
    for scene_id, scene in scene_by_id.items():
        expected_actions = set(range(len(scene["action"])))
        _require(action_refs[scene_id] == expected_actions, f"action manifest is not a bijection for {scene_id}")
        expected_dialogue = set(range(len(scene["dialogue"])))
        _require(dialogue_refs[scene_id] == expected_dialogue, f"dialogue manifest is not a bijection for {scene_id}")

    for (scene_id, kind), occurrences in sorted(directive_refs.items()):
        expected = set(range(len(occurrences)))
        _require(occurrences == expected, f"{kind} manifest occurrence indices are not contiguous for {scene_id}")

    if check_hashes:
        for index, entry in enumerate(parsed_entries):
            _require(entry["entry_hash"] == _expected_entry_hash(entry), f"entry_hash mismatch at semantic_manifest.entries[{index}]")
        _require(manifest["semantic_manifest_hash"] == _expected_manifest_hash(manifest), "semantic_manifest_hash mismatch")
        _require(top["map_id"] == _expected_map_id(top), "map_id hash mismatch")
