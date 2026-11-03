"""Safe filesystem and byte-signature operations for SAC asset release."""
from __future__ import annotations

import json
import os
from pathlib import Path
import stat as stat_module
import tempfile
from typing import Any

from .asset_release_contracts import (
    AssetReleaseError,
    RUNTIME_MIME,
    SOURCE_MIME,
    _relative_path,
)
from .util import load_json_bytes, sha256_bytes, sha256_file


def _detected_mime(data: bytes) -> str | None:
    """Identify the supported file signature without trusting its filename."""

    if data.startswith(b"\x89PNG\r\n\x1a\n"):
        return "image/png"
    if data.startswith(b"\xff\xd8\xff"):
        return "image/jpeg"
    if len(data) >= 12 and data[:4] == b"RIFF" and data[8:12] == b"WEBP":
        return "image/webp"
    # A PSD/PSB header is 26 bytes: signature, version, reserved bytes,
    # channels, height, width, depth, and color mode.  Requiring the complete
    # header prevents a text file renamed to .psd from being accepted as
    # source provenance.
    if len(data) >= 26 and data[:4] == b"8BPS" and data[4:6] in {b"\x00\x01", b"\x00\x02"}:
        if data[6:12] != b"\x00" * 6:
            return None
        channels = int.from_bytes(data[12:14], "big")
        height = int.from_bytes(data[14:18], "big")
        width = int.from_bytes(data[18:22], "big")
        depth = int.from_bytes(data[22:24], "big")
        color_mode = int.from_bytes(data[24:26], "big")
        if channels >= 1 and width >= 1 and height >= 1 and depth in {1, 8, 16, 32} and color_mode <= 9:
            return "image/vnd.adobe.photoshop"
    return None


def _verify_signature(data: bytes, path: str, declared_mime: str, label: str, *, runtime: bool) -> None:
    suffix = Path(path).suffix.lower()
    expected_mime = RUNTIME_MIME.get(suffix) if runtime else SOURCE_MIME.get(suffix)
    detected_mime = _detected_mime(data)
    if expected_mime is None or declared_mime != expected_mime:
        code = "RUNTIME_FORMAT_INVALID" if runtime else "SOURCE_FORMAT_INVALID"
        public_code = "runtime-format-invalid" if runtime else "source-format-invalid"
        raise AssetReleaseError(
            f"{code}:{label}:{path}",
            code=public_code,
            details={"path": path, "declaredMime": declared_mime, "expectedMime": expected_mime, "detectedMime": detected_mime},
        )
    if detected_mime != expected_mime:
        code = "RUNTIME_SIGNATURE_MISMATCH" if runtime else "SOURCE_SIGNATURE_MISMATCH"
        public_code = "runtime-signature-mismatch" if runtime else "source-signature-mismatch"
        raise AssetReleaseError(
            f"{code}:{label}:{path}",
            code=public_code,
            details={"path": path, "declaredMime": declared_mime, "expectedMime": expected_mime, "detectedMime": detected_mime},
        )


def _strict_root(root: Path) -> Path:
    root = Path(root)
    try:
        info = root.lstat()
    except OSError as error:
        raise AssetReleaseError(f"INPUT_ROOT_UNREADABLE:{root}") from error
    if stat_module.S_ISLNK(info.st_mode) or not stat_module.S_ISDIR(info.st_mode):
        raise AssetReleaseError(f"PATH_UNSAFE:project_root:{root}")
    return root.absolute()


def _safe_file(root: Path, relative_path: Any, label: str) -> Path:
    root = _strict_root(root)
    relative = _relative_path(relative_path, f"{label}.path")
    current = root
    parts = relative.split("/")
    for index, part in enumerate(parts):
        current = current / part
        try:
            info = current.lstat()
        except OSError as error:
            raise AssetReleaseError(f"ARTIFACT_MISSING:{label}:{relative}") from error
        if stat_module.S_ISLNK(info.st_mode):
            raise AssetReleaseError(f"PATH_UNSAFE:{label}:symlink")
        if index < len(parts) - 1 and not stat_module.S_ISDIR(info.st_mode):
            raise AssetReleaseError(f"PATH_UNSAFE:{label}:parent-not-directory")
        if index == len(parts) - 1 and not stat_module.S_ISREG(info.st_mode):
            raise AssetReleaseError(f"ARTIFACT_NOT_REGULAR:{label}:{relative}")
    return current


def _verify_file_ref(root: Path, ref: dict[str, Any], label: str, *, runtime: bool = False) -> dict[str, Any]:
    path = _relative_path(ref["path"], f"{label}.path")
    target = _safe_file(root, path, label)
    try:
        data = target.read_bytes()
    except OSError as error:
        raise AssetReleaseError(f"ARTIFACT_UNREADABLE:{label}:{path}") from error
    actual_hash = sha256_bytes(data)
    actual_size = len(data)
    if actual_hash != ref["sha256"]:
        raise AssetReleaseError(f"HASH_MISMATCH:{label}:{path}")
    if actual_size != ref["sizeBytes"]:
        raise AssetReleaseError(f"SIZE_MISMATCH:{label}:{path}")
    _verify_signature(data, path, ref["mime"], label, runtime=runtime)
    return {**ref, "path": path, "sha256": actual_hash, "sizeBytes": actual_size}


def _safe_destination(root: Path, relative: str) -> Path:
    root = _strict_root(root)
    relative = _relative_path(relative, "destination")
    parts = relative.split("/")
    current = root
    for part in parts[:-1]:
        current = current / part
        try:
            info = current.lstat()
        except FileNotFoundError:
            current.mkdir()
            info = current.lstat()
        if stat_module.S_ISLNK(info.st_mode) or not stat_module.S_ISDIR(info.st_mode):
            raise AssetReleaseError(f"PATH_UNSAFE:destination:{relative}")
    target = current / parts[-1]
    if os.path.lexists(target):
        info = target.lstat()
        if stat_module.S_ISLNK(info.st_mode) or not stat_module.S_ISREG(info.st_mode):
            raise AssetReleaseError(f"PATH_UNSAFE:destination:{relative}")
    return target


def _read_json(root: Path, relative: str, label: str, expected_sha256: str | None = None) -> tuple[dict[str, Any], bytes, str]:
    target = _safe_file(root, relative, label)
    try:
        data = target.read_bytes()
        value = load_json_bytes(data)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        raise AssetReleaseError(f"ARTIFACT_INVALID:{label}:{relative}") from error
    actual = sha256_bytes(data)
    if expected_sha256 is not None and actual != expected_sha256:
        raise AssetReleaseError(f"HASH_MISMATCH:{label}:{relative}")
    if not isinstance(value, dict):
        raise AssetReleaseError(f"TYPE:{label}:object")
    return value, data, actual


def _copy_exact(data: bytes, expected_sha256: str, destination_root: Path, destination_relative: str, label: str) -> str:
    actual_sha256 = sha256_bytes(data)
    if actual_sha256 != expected_sha256:
        raise AssetReleaseError(f"HASH_MISMATCH:{label}:validated-bytes")
    target = _safe_destination(destination_root, destination_relative)
    if target.exists():
        if target.stat().st_size == len(data) and sha256_file(target) == actual_sha256:
            return actual_sha256
        raise AssetReleaseError(f"DESTINATION_CONFLICT:{label}:{destination_relative}")

    fd, temporary = tempfile.mkstemp(prefix=".projection-", dir=target.parent)
    try:
        with os.fdopen(fd, "wb") as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        try:
            os.link(temporary, target, follow_symlinks=False)
        except FileExistsError:
            # Another publisher won the create race. Reuse only identical bytes.
            target = _safe_destination(destination_root, destination_relative)
            if target.stat().st_size != len(data) or sha256_file(target) != actual_sha256:
                raise AssetReleaseError(f"DESTINATION_CONFLICT:{label}:{destination_relative}")
        return actual_sha256
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
