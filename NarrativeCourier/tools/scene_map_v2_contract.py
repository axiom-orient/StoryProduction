"""Fixture-side field bounds and strict value guards for SceneMap v2."""

from __future__ import annotations

import re
from typing import Any

KINDS = (
    "action",
    "dialogue",
    "emotion",
    "lightingTransition",
    "cameraMovement",
)
SCENE_FIELDS = frozenset(KINDS)
SCENE_REQUIRED = frozenset(
    {
        "scene_id",
        "order",
        "heading",
        "setting",
        "location",
        "time_of_day",
        "time_of_day_label",
        "silent",
        "characters",
        "action",
        "dialogue",
        "asset_requirements",
    }
)
ENTRY_REQUIRED = frozenset(
    {"semantic_id", "entry_hash", "scene_id", "kind", "source_text", "source_locator"}
)
LOCATOR_REQUIRED = frozenset({"scene_id", "field", "occurrence_index"})
SAFE_SEMANTIC_ID = re.compile(r"^semantic\.[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")
SCENE_ID = re.compile(r"^SCN-[0-9]{3,}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
HASH64 = re.compile(r"^sha256:[0-9a-f]{64}$")
MAP_ID = re.compile(r"^nsm2_[0-9a-f]{64}$")


class ContractError(ValueError):
    """A deterministic contract rejection with a user-facing reason."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def expect_object(
    value: Any,
    path: str,
    required: frozenset[str],
    allowed: frozenset[str],
) -> dict[str, Any]:
    require(isinstance(value, dict), f"{path} must be an object")
    missing = sorted(required - value.keys())
    require(not missing, f"{path} missing required field(s): {', '.join(missing)}")
    unknown = sorted(value.keys() - allowed)
    require(not unknown, f"{path} has unsupported field(s): {', '.join(unknown)}")
    return value


def expect_string(value: Any, path: str, minimum: int = 1, maximum: int | None = None) -> str:
    require(isinstance(value, str), f"{path} must be a string")
    require(len(value) >= minimum, f"{path} must contain at least {minimum} character(s)")
    if maximum is not None:
        require(len(value) <= maximum, f"{path} must contain at most {maximum} character(s)")
    return value


def expect_int(value: Any, path: str, minimum: int | None = None, maximum: int | None = None) -> int:
    require(isinstance(value, int) and not isinstance(value, bool), f"{path} must be an integer")
    if minimum is not None:
        require(value >= minimum, f"{path} must be at least {minimum}")
    if maximum is not None:
        require(value <= maximum, f"{path} must be at most {maximum}")
    return value


def expect_array(value: Any, path: str, minimum: int = 0, maximum: int | None = None) -> list[Any]:
    require(isinstance(value, list), f"{path} must be an array")
    require(len(value) >= minimum, f"{path} must contain at least {minimum} item(s)")
    if maximum is not None:
        require(len(value) <= maximum, f"{path} must contain at most {maximum} item(s)")
    return value
