"""SceneMap v2 scene, entry, locator, and schema-shape checks."""

from __future__ import annotations

from typing import Any

from scene_map_v2_contract import (
    ENTRY_REQUIRED,
    HASH64,
    KINDS,
    LOCATOR_REQUIRED,
    SAFE_SEMANTIC_ID,
    SCENE_FIELDS,
    SCENE_ID,
    SCENE_REQUIRED,
    expect_array,
    expect_int,
    expect_object,
    expect_string,
    require,
)


def _validate_dialogue(dialogue: Any, path: str) -> None:
    value = expect_object(dialogue, path, frozenset({"character", "line"}), frozenset({"character", "line"}))
    expect_string(value["character"], f"{path}.character", 1, 256)
    expect_string(value["line"], f"{path}.line", 1, 16384)


def _validate_scene(scene: Any, index: int) -> None:
    path = f"scenes[{index}]"
    value = expect_object(scene, path, SCENE_REQUIRED, SCENE_REQUIRED)
    require(SCENE_ID.fullmatch(expect_string(value["scene_id"], f"{path}.scene_id", 1, 64)) is not None, f"{path}.scene_id is invalid")
    expect_int(value["order"], f"{path}.order", 1, 10000)
    expect_string(value["heading"], f"{path}.heading", 1, 16384)
    require(value["setting"] in {"INT", "EXT", "INT/EXT", "I/E"}, f"{path}.setting is unsupported")
    expect_string(value["location"], f"{path}.location", 1, 4096)
    require(value["time_of_day"] in {"dawn", "morning", "noon", "day", "afternoon", "dusk", "night"}, f"{path}.time_of_day is unsupported")
    expect_string(value["time_of_day_label"], f"{path}.time_of_day_label", 1, 256)
    require(isinstance(value["silent"], bool), f"{path}.silent must be a boolean")

    characters = expect_array(value["characters"], f"{path}.characters", 0, 256)
    seen_characters: set[str] = set()
    for character_index, character in enumerate(characters):
        text = expect_string(character, f"{path}.characters[{character_index}]", 1, 256)
        require(text not in seen_characters, f"{path}.characters contains a duplicate")
        seen_characters.add(text)

    actions = expect_array(value["action"], f"{path}.action", 0, 10000)
    for action_index, action in enumerate(actions):
        expect_string(action, f"{path}.action[{action_index}]", 1, 16384)

    dialogues = expect_array(value["dialogue"], f"{path}.dialogue", 0, 10000)
    for dialogue_index, dialogue in enumerate(dialogues):
        _validate_dialogue(dialogue, f"{path}.dialogue[{dialogue_index}]")

    requirements = expect_array(value["asset_requirements"], f"{path}.asset_requirements", 0, 1000)
    for requirement_index, requirement in enumerate(requirements):
        requirement_path = f"{path}.asset_requirements[{requirement_index}]"
        requirement_value = expect_object(
            requirement,
            requirement_path,
            frozenset({"kind", "description", "source"}),
            frozenset({"kind", "description", "source"}),
        )
        expect_string(requirement_value["kind"], f"{requirement_path}.kind", 1, 256)
        expect_string(requirement_value["description"], f"{requirement_path}.description", 1, 4096)
        expect_string(requirement_value["source"], f"{requirement_path}.source", 1, 4096)

    require(value["silent"] == (len(dialogues) == 0), f"{path}.silent does not match dialogue presence")


def _validate_locator(locator: Any, path: str) -> dict[str, Any]:
    value = expect_object(
        locator,
        path,
        LOCATOR_REQUIRED,
        frozenset({"scene_id", "field", "occurrence_index", "character", "dialogue_index"}),
    )
    require(SCENE_ID.fullmatch(expect_string(value["scene_id"], f"{path}.scene_id", 1, 64)) is not None, f"{path}.scene_id is invalid")
    require(isinstance(value["field"], str), f"{path}.field must be a string")
    require(value["field"] in SCENE_FIELDS, f"{path}.field is unsupported")
    expect_int(value["occurrence_index"], f"{path}.occurrence_index", 0, 10000)
    if "character" in value:
        expect_string(value["character"], f"{path}.character", 1, 256)
    if "dialogue_index" in value:
        expect_int(value["dialogue_index"], f"{path}.dialogue_index", 0, 10000)
    if value["field"] == "dialogue":
        require("character" in value, f"{path}.character is required for dialogue")
        require("dialogue_index" in value, f"{path}.dialogue_index is required for dialogue")
        require(value["dialogue_index"] == value["occurrence_index"], f"{path}.dialogue_index must equal occurrence_index")
    return value


def _validate_entry(entry: Any, index: int) -> dict[str, Any]:
    path = f"semantic_manifest.entries[{index}]"
    value = expect_object(entry, path, ENTRY_REQUIRED, ENTRY_REQUIRED)
    semantic_id = expect_string(value["semantic_id"], f"{path}.semantic_id", 1, 137)
    require(SAFE_SEMANTIC_ID.fullmatch(semantic_id) is not None, f"{path}.semantic_id is unsafe")
    require(HASH64.fullmatch(expect_string(value["entry_hash"], f"{path}.entry_hash", 71, 71)) is not None, f"{path}.entry_hash is invalid")
    require(SCENE_ID.fullmatch(expect_string(value["scene_id"], f"{path}.scene_id", 1, 64)) is not None, f"{path}.scene_id is invalid")
    require(isinstance(value["kind"], str), f"{path}.kind must be a string")
    require(value["kind"] in KINDS, f"{path}.kind is unsupported: {value['kind']!r}")
    expect_string(value["source_text"], f"{path}.source_text", 1, 16384)
    _validate_locator(value["source_locator"], f"{path}.source_locator")
    require(value["source_locator"]["field"] == value["kind"], f"{path}.source_locator.field must equal kind")
    require(value["source_locator"]["scene_id"] == value["scene_id"], f"{path}.source_locator.scene_id must equal scene_id")
    return value


def _validate_schema_shape(schema: dict[str, Any]) -> None:
    """Check the schema's public identity and bounded strict objects.

    This is intentionally a focused self-check rather than a second JSON
    Schema implementation. The data validator separately enforces the same
    contract rules for in-memory values and the fixture's negative corpus.
    """

    require(schema.get("$schema") == "https://json-schema.org/draft/2020-12/schema", "schema is not Draft 2020-12")
    require(schema.get("$id") == "https://narrative-courier.local/contracts/narrative-scene-map/v2.schema.json", "schema $id is not v2")
    require(schema.get("title") == "NarrativeSceneMap v2", "schema title is not v2")
    require(schema.get("type") == "object" and schema.get("additionalProperties") is False, "top-level schema must be strict")
    top = schema.get("properties", {})
    require(top.get("schema_version", {}).get("const") == 2, "schema_version is not const 2")
    require(top.get("map_id", {}).get("pattern") == r"^nsm2_[0-9a-f]{64}$", "map_id pattern is not nsm2_<64hex>")
    require("semantic_manifest" in top and "semantic_manifest" in schema.get("required", []), "semantic_manifest is not required")

    defs = schema.get("$defs", {})
    for name in ("scene", "dialogue", "asset_requirement", "semantic_manifest", "semantic_entry", "source_locator"):
        require(defs.get(name, {}).get("additionalProperties") is False, f"$defs.{name} must be strict")
    manifest = defs["semantic_manifest"]
    require(manifest["properties"]["schema_version"].get("const") == 1, "semantic manifest schema_version is not const 1")
    require(manifest["properties"]["semantic_manifest_hash"].get("pattern") == r"^sha256:[0-9a-f]{64}$", "manifest hash pattern is invalid")
    entry = defs["semantic_entry"]
    require(entry["properties"]["source_text"].get("minLength") == 1 and entry["properties"]["source_text"].get("maxLength") == 16384, "source_text bounds are invalid")
    require(entry["properties"]["kind"].get("enum") == list(KINDS), "semantic kind enum is invalid")
    locator = defs["source_locator"]
    require(locator["properties"]["field"].get("enum") == list(KINDS), "locator field enum is invalid")
    for name, definition in (
        ("scene.scene_id", defs["scene"]["properties"]["scene_id"]),
        ("semantic_entry.scene_id", defs["semantic_entry"]["properties"]["scene_id"]),
        ("source_locator.scene_id", locator["properties"]["scene_id"]),
    ):
        require(definition.get("maxLength") == 64, f"{name} maxLength must be 64")
