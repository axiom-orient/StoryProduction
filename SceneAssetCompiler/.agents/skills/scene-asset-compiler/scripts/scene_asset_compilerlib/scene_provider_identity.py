"""Resolve pinned Scene Provider and PerfectPixel component identity."""
from __future__ import annotations

from pathlib import Path
import json
import os
from typing import Any

from .scene_provider_protocol import (
    DEFAULT_COMPONENT_LOCK,
    DEFAULT_PERFECTPIXEL_SHA256,
    DEFAULT_PROVIDER_SHA256,
    PERFECTPIXEL_CLI_VERSION,
    PERFECTPIXEL_INSPECTION_SCHEMA,
    PERFECTPIXEL_INSPECTION_SCHEMA_VERSION,
    PERFECTPIXEL_ROLE,
    PROVIDER_BINARY_NAME,
    PROVIDER_PROTOCOL_SCHEMA,
    PROVIDER_PROTOCOL_VERSION,
    _reject_duplicate_keys,
)
from .util import sha256_file


def _provider_path(value: Path) -> Path:
    path = Path(value).expanduser()
    if not path.is_absolute():
        raise ValueError("SCENE_PROVIDER_PATH_NOT_ABSOLUTE")
    if path.is_symlink():
        raise ValueError("SCENE_PROVIDER_SYMLINK")
    if not path.is_file():
        raise ValueError("SCENE_PROVIDER_MISSING")
    if path.name != PROVIDER_BINARY_NAME:
        raise ValueError("SCENE_PROVIDER_CANONICAL_NAME_REQUIRED")
    if not os.access(path, os.X_OK):
        raise ValueError("SCENE_PROVIDER_NOT_EXECUTABLE")
    return path.resolve()


def _perfectpixel_path(value: Path) -> Path:
    path = Path(value).expanduser()
    if not path.is_absolute():
        raise ValueError("PERFECTPIXEL_PATH_NOT_ABSOLUTE")
    if path.is_symlink():
        raise ValueError("PERFECTPIXEL_SYMLINK")
    if not path.is_file():
        raise ValueError("PERFECTPIXEL_MISSING")
    if path.name != "perfectpixel":
        raise ValueError("PERFECTPIXEL_CANONICAL_NAME_REQUIRED")
    if not os.access(path, os.X_OK):
        raise ValueError("PERFECTPIXEL_NOT_EXECUTABLE")
    return path.resolve()





def _component_lock_path(value: Path | None) -> Path:
    path = Path(value or DEFAULT_COMPONENT_LOCK).expanduser()
    if not path.is_absolute():
        raise ValueError("COMPONENT_LOCK_PATH_NOT_ABSOLUTE")
    if path.is_symlink() or not path.is_file():
        raise ValueError("COMPONENT_LOCK_MISSING")
    return path.resolve()


def _component_lock(value: Path | None) -> tuple[dict[str, Any], Path, str]:
    path = _component_lock_path(value)
    try:
        lock = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_reject_duplicate_keys)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        raise ValueError(f"COMPONENT_LOCK_INVALID:{error}") from error
    if not isinstance(lock, dict):
        raise ValueError("COMPONENT_LOCK_TYPE")
    if set(lock) != {"$schema", "schemaVersion", "product", "provider", "perfectpixel"}:
        raise ValueError("COMPONENT_LOCK_FIELDS")
    if lock.get("$schema") != "https://json-schema.org/draft/2020-12/schema" or lock.get("schemaVersion") != 2 or lock.get("product") != "SceneAssetCompiler":
        raise ValueError("COMPONENT_LOCK_IDENTITY")
    provider = lock.get("provider")
    if not isinstance(provider, dict) or set(provider) != {
        "name", "protocolSchema", "protocolVersion", "schemaVersion", "sha256"
    }:
        raise ValueError("COMPONENT_LOCK_PROVIDER_FIELDS")
    if (
        provider.get("name") != PROVIDER_BINARY_NAME
        or provider.get("protocolSchema") != PROVIDER_PROTOCOL_SCHEMA
        or provider.get("protocolVersion") != PROVIDER_PROTOCOL_VERSION
        or provider.get("schemaVersion") != PROVIDER_PROTOCOL_VERSION
    ):
        raise ValueError("COMPONENT_LOCK_PROVIDER_PROTOCOL")
    if not isinstance(provider.get("sha256"), str) or len(provider["sha256"]) != 64 or any(c not in "0123456789abcdef" for c in provider["sha256"]):
        raise ValueError("COMPONENT_LOCK_PROVIDER_SHA256")
    perfectpixel = lock.get("perfectpixel")
    if not isinstance(perfectpixel, dict) or set(perfectpixel) != {
        "name",
        "role",
        "inspectionSchema",
        "inspectionSchemaVersion",
        "cliVersion",
        "commands",
        "sourceManifestSha256",
        "sha256",
    }:
        raise ValueError("COMPONENT_LOCK_PERFECTPIXEL_FIELDS")
    if (
        perfectpixel.get("name") != "perfectpixel"
        or perfectpixel.get("role") != PERFECTPIXEL_ROLE
        or perfectpixel.get("inspectionSchema") != PERFECTPIXEL_INSPECTION_SCHEMA
        or perfectpixel.get("inspectionSchemaVersion") != PERFECTPIXEL_INSPECTION_SCHEMA_VERSION
        or perfectpixel.get("cliVersion") != PERFECTPIXEL_CLI_VERSION
        or perfectpixel.get("commands") != ["schema", "inspect"]
    ):
        raise ValueError("COMPONENT_LOCK_PERFECTPIXEL_VERSION")
    source_manifest_sha256 = perfectpixel.get("sourceManifestSha256")
    if (
        not isinstance(source_manifest_sha256, str)
        or len(source_manifest_sha256) != 64
        or any(c not in "0123456789abcdef" for c in source_manifest_sha256)
    ):
        raise ValueError("COMPONENT_LOCK_PERFECTPIXEL_SOURCE_PIN")
    if not isinstance(perfectpixel.get("sha256"), str) or len(perfectpixel["sha256"]) != 64 or any(c not in "0123456789abcdef" for c in perfectpixel["sha256"]):
        raise ValueError("COMPONENT_LOCK_PERFECTPIXEL_SHA256")
    if path == DEFAULT_COMPONENT_LOCK.resolve():
        if provider["sha256"] != DEFAULT_PROVIDER_SHA256:
            raise ValueError("COMPONENT_LOCK_DEFAULT_PROVIDER_PIN")
        if perfectpixel["sha256"] != DEFAULT_PERFECTPIXEL_SHA256:
            raise ValueError("COMPONENT_LOCK_DEFAULT_PERFECTPIXEL_PIN")
    return lock, path, sha256_file(path)


def _provider_lock_identity(
    provider_bin: Path,
    component_lock: Path | None = None,
) -> tuple[Path, dict[str, Any], Path, str, str]:
    """Load the lock and seal the exact provider bytes before probing them."""
    provider = _provider_path(Path(provider_bin))
    lock, lock_path, lock_sha256 = _component_lock(component_lock)
    provider_sha256 = sha256_file(provider)
    expected_provider = lock["provider"]["sha256"]
    if provider_sha256 != expected_provider:
        raise ValueError(f"PROVIDER_IDENTITY_MISMATCH:expected={expected_provider}:observed={provider_sha256}")
    return provider, lock, lock_path, lock_sha256, provider_sha256
