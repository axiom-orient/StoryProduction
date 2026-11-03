"""Integer-only JCS encoding and SceneMap v2 content identity helpers."""

from __future__ import annotations

import copy
import json
from typing import Any

import digest as _digest
from scene_map_v2_contract import ContractError


def _utf16_key(value: str) -> bytes:
    # Rust's ``str::encode_utf16`` compares UTF-16 code units, as required by
    # RFC 8785 object-key ordering rather than Unicode code-point ordering.
    return value.encode("utf-16-be", "surrogatepass")


def jcs_bytes(value: Any) -> bytes:
    """Serialize the contract's integer-only RFC 8785/JCS subset as UTF-8."""

    out = bytearray()

    def write(item: Any) -> None:
        if item is None:
            out.extend(b"null")
        elif item is True:
            out.extend(b"true")
        elif item is False:
            out.extend(b"false")
        elif isinstance(item, int):
            if abs(item) > 9_007_199_254_740_991:
                raise ContractError("integer is outside the canonical safe-integer profile")
            out.extend(str(item).encode("ascii"))
        elif isinstance(item, float):
            # NarrativeCourier rejects non-integer JSON numbers in its JCS
            # helper instead of silently selecting a different formatter.
            raise ContractError("non-integer JSON number is outside JCS contract domain")
        elif isinstance(item, str):
            encoded = json.dumps(item, ensure_ascii=False, separators=(",", ":"), allow_nan=False)
            try:
                out.extend(encoded.encode("utf-8"))
            except UnicodeEncodeError as error:
                raise ContractError(f"string is not valid UTF-8: {error}") from error
        elif isinstance(item, list):
            out.append(ord("["))
            for index, child in enumerate(item):
                if index:
                    out.append(ord(","))
                write(child)
            out.append(ord("]"))
        elif isinstance(item, dict):
            out.append(ord("{"))
            if any(not isinstance(key, str) for key in item):
                raise ContractError("canonical object keys must be strings")
            pairs = sorted(item.items(), key=lambda pair: _utf16_key(pair[0]))
            for index, (key, child) in enumerate(pairs):
                if index:
                    out.append(ord(","))
                write(key)
                out.append(ord(":"))
                write(child)
            out.append(ord("}"))
        else:
            raise ContractError(f"unsupported JSON value type: {type(item).__name__}")

    write(value)
    return bytes(out)


def sha256_jcs(value: Any) -> str:
    return _digest.sha256_bytes(jcs_bytes(value))


def prefixed_hash(value: Any) -> str:
    return "sha256:" + sha256_jcs(value)


def expected_entry_hash(entry: dict[str, Any]) -> str:
    body = copy.deepcopy(entry)
    body.pop("entry_hash", None)
    return prefixed_hash(body)


def expected_manifest_hash(manifest: dict[str, Any]) -> str:
    body = copy.deepcopy(manifest)
    body.pop("semantic_manifest_hash", None)
    return prefixed_hash(body)


def expected_map_id(document: dict[str, Any]) -> str:
    body = copy.deepcopy(document)
    body["map_id"] = ""
    return "nsm2_" + sha256_jcs(body)
