#!/usr/bin/env python3
"""Verify the NarrativeSceneMap v2 contract and its positive fixture.

This CLI intentionally has no third-party dependencies. The Rust
implementation of NarrativeCourier's canonical hash helper is not a Python
callable, so ``scene_map_v2_validator.py`` contains a fixture-side
implementation of the same integer-only RFC 8785/JCS subset used by
``crates/runtime/src/token.rs``. The CLI applies it to the positive fixture
and an in-memory negative corpus; it does not write product state.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path
from typing import Any as _Any, Callable as _Callable


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "contracts" / "narrative-scene-map-v2.schema.json"
FIXTURE_PATH = ROOT / "contracts" / "examples" / "narrative-scene-map-v2.example.json"

from scene_map_v2_contract import KINDS as _KINDS, ContractError as _ContractError, require as _require
from scene_map_v2_identity import (
    expected_entry_hash as _expected_entry_hash,
    expected_manifest_hash as _expected_manifest_hash,
    expected_map_id as _expected_map_id,
)
from scene_map_v2_shape import _validate_schema_shape
from scene_map_v2_validator import validate_map as _validate_map

__all__ = ["main"]


def _rebind_hashes(document: dict[str, _Any]) -> None:
    """Recompute all dependent identities after a structural negative edit."""

    manifest = document["semantic_manifest"]
    for entry in manifest["entries"]:
        entry["entry_hash"] = _expected_entry_hash(entry)
    manifest["semantic_manifest_hash"] = _expected_manifest_hash(manifest)
    document["map_id"] = _expected_map_id(document)


def _entry_for(document: dict[str, _Any], kind: str, occurrence: int = 0) -> dict[str, _Any]:
    for entry in document["semantic_manifest"]["entries"]:
        locator = entry["source_locator"]
        if entry["kind"] == kind and locator["occurrence_index"] == occurrence:
            return entry
    raise _ContractError(f"fixture has no {kind}[{occurrence}] entry")


def _negative_cases(document: dict[str, _Any]) -> list[tuple[str, _Callable[[dict[str, _Any]], None], str]]:
    def duplicate_semantic_id(candidate: dict[str, _Any]) -> None:
        entries = candidate["semantic_manifest"]["entries"]
        entries[1]["semantic_id"] = entries[0]["semantic_id"]
        _rebind_hashes(candidate)

    def duplicate_locator(candidate: dict[str, _Any]) -> None:
        first = _entry_for(candidate, "dialogue", 0)
        second = _entry_for(candidate, "dialogue", 1)
        second["source_locator"] = copy.deepcopy(first["source_locator"])
        _rebind_hashes(candidate)

    def missing_action_occurrence(candidate: dict[str, _Any]) -> None:
        entry = _entry_for(candidate, "action", 0)
        entry["source_locator"]["occurrence_index"] = len(candidate["scenes"][0]["action"])
        _rebind_hashes(candidate)

    def missing_dialogue_occurrence(candidate: dict[str, _Any]) -> None:
        entry = _entry_for(candidate, "dialogue", 1)
        entry["source_locator"]["occurrence_index"] = len(candidate["scenes"][0]["dialogue"])
        entry["source_locator"]["dialogue_index"] = len(candidate["scenes"][0]["dialogue"])
        _rebind_hashes(candidate)

    def noncontiguous_directive_occurrence(candidate: dict[str, _Any]) -> None:
        entry = _entry_for(candidate, "emotion", 0)
        entry["source_locator"]["occurrence_index"] = 1
        _rebind_hashes(candidate)

    def entry_hash_drift(candidate: dict[str, _Any]) -> None:
        entry = _entry_for(candidate, "action", 0)
        entry["entry_hash"] = "sha256:" + ("0" * 64)

    def manifest_hash_drift(candidate: dict[str, _Any]) -> None:
        candidate["semantic_manifest"]["semantic_manifest_hash"] = "sha256:" + ("0" * 64)

    def map_id_hash_drift(candidate: dict[str, _Any]) -> None:
        candidate["map_id"] = "nsm2_" + ("0" * 64)

    def unsupported_kind(candidate: dict[str, _Any]) -> None:
        entry = _entry_for(candidate, "emotion", 0)
        entry["kind"] = "unsupported"

    def field_kind_mismatch(candidate: dict[str, _Any]) -> None:
        entry = _entry_for(candidate, "emotion", 0)
        entry["source_locator"]["field"] = "cameraMovement"
        _rebind_hashes(candidate)

    def source_text_whitespace_mutation(candidate: dict[str, _Any]) -> None:
        scene = candidate["scenes"][0]
        scene["action"][0] += " "

    def dialogue_character_drift(candidate: dict[str, _Any]) -> None:
        entry = _entry_for(candidate, "dialogue", 0)
        entry["source_locator"]["character"] = "다른사람"
        _rebind_hashes(candidate)

    def dialogue_index_drift(candidate: dict[str, _Any]) -> None:
        entry = _entry_for(candidate, "dialogue", 1)
        entry["source_locator"]["dialogue_index"] = len(candidate["scenes"][0]["dialogue"])
        _rebind_hashes(candidate)

    def swapped_dialogue_indices(candidate: dict[str, _Any]) -> None:
        first = _entry_for(candidate, "dialogue", 0)
        second = _entry_for(candidate, "dialogue", 1)
        first["source_locator"]["dialogue_index"], second["source_locator"]["dialogue_index"] = (
            second["source_locator"]["dialogue_index"],
            first["source_locator"]["dialogue_index"],
        )
        _rebind_hashes(candidate)

    def overlong_scene_id(candidate: dict[str, _Any]) -> None:
        candidate["scenes"][0]["scene_id"] = "SCN-" + ("1" * 61)

    return [
        ("duplicate semantic_id", duplicate_semantic_id, "duplicate semantic_id"),
        ("duplicate locator", duplicate_locator, "duplicate source_locator"),
        ("missing action occurrence", missing_action_occurrence, "action occurrence_index"),
        ("missing dialogue occurrence", missing_dialogue_occurrence, "dialogue occurrence_index"),
        ("noncontiguous directive occurrence", noncontiguous_directive_occurrence, "emotion manifest occurrence indices are not contiguous"),
        ("entry hash drift", entry_hash_drift, "entry_hash mismatch"),
        ("manifest hash drift", manifest_hash_drift, "semantic_manifest_hash mismatch"),
        ("map id hash drift", map_id_hash_drift, "map_id hash mismatch"),
        ("unsupported kind", unsupported_kind, "kind is unsupported"),
        ("field/kind mismatch", field_kind_mismatch, "source_locator.field must equal kind"),
        ("source text whitespace mutation", source_text_whitespace_mutation, "action source_text drift"),
        ("dialogue character drift", dialogue_character_drift, "dialogue character drift"),
        ("dialogue index drift", dialogue_index_drift, "dialogue_index"),
        ("swapped dialogue indices", swapped_dialogue_indices, "dialogue_index must equal occurrence_index"),
        ("overlong scene_id", overlong_scene_id, "at most 64 character(s)"),
    ]


def main() -> int:
    try:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        _validate_schema_shape(schema)
        document = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
        _validate_map(document)
        entries = document["semantic_manifest"]["entries"]
        kinds = {entry["kind"] for entry in entries}
        _require(kinds == set(_KINDS), "positive fixture does not cover all five semantic kinds")
        dialogue_lines = [
            entry["source_text"]
            for entry in entries
            if entry["kind"] == "dialogue"
        ]
        _require(len(dialogue_lines) >= 2 and len(set(dialogue_lines)) < len(dialogue_lines), "positive fixture lacks repeated dialogue text")
        _require(any("\n" in entry["source_text"] for entry in entries), "positive fixture lacks a newline source_text")
        _require(any(any(ord(character) > 127 for character in entry["source_text"]) for entry in entries), "positive fixture lacks Unicode source_text")
        print("PASS positive fixture: schema, JCS hashes, scene fields, and semantic bijections")

        negative_cases = _negative_cases(document)
        for name, mutate, expected_reason in negative_cases:
            candidate = copy.deepcopy(document)
            mutate(candidate)
            try:
                _validate_map(candidate)
            except _ContractError as error:
                reason = str(error)
                _require(expected_reason in reason, f"negative {name} rejected for unexpected reason: {reason}")
                print(f"PASS negative {name}: {reason}")
            else:
                raise _ContractError(f"negative {name} was accepted")
        print(f"PASS negative corpus: {len(negative_cases)} cases rejected")
        return 0
    except (_ContractError, OSError, json.JSONDecodeError) as error:
        print(f"FAIL narrative-scene-map-v2: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
