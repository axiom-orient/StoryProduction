#!/usr/bin/env python3
"""Generate and verify the repository's deterministic source manifest.

The manifest is a source-integrity receipt, not a build inventory.  Product
data, local tooling state, OS metadata, Python caches, and generated build
trees are intentionally outside the source set.  Symlinks are never followed
or recorded.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import sys
from typing import Iterable

import digest as _digest


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "SOURCE_MANIFEST.sha256"

EXCLUDED_DIR_NAMES = {
    ".git",
    ".omo",
    ".codegraph",
    ".claude",
    "node_modules",
    "projects",
    "target",
    "temp_pages",
    "__pycache__",
}
EXCLUDED_DIR_PATHS = {
    "docs/.pdca-snapshots",
    "src-tauri/gen",
    "src-tauri/target",
}
EXCLUDED_FILE_NAMES = {
    ".DS_Store",
    "Thumbs.db",
    "desktop.ini",
    ".bkit-memory.json",
    ".pdca-status.json",
    "SOURCE_MANIFEST.sha256",
}
EXCLUDED_SUFFIXES = (".pyc", ".pyo")


def relative_path(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def excluded_directory(path: Path) -> bool:
    rel = relative_path(path)
    return path.name in EXCLUDED_DIR_NAMES or rel in EXCLUDED_DIR_PATHS


def excluded_file(path: Path) -> bool:
    return (
        path.name in EXCLUDED_FILE_NAMES
        or path.name.endswith(EXCLUDED_SUFFIXES)
        or path.is_symlink()
    )


def source_files() -> list[Path]:
    files: list[Path] = []
    for current, directory_names, file_names in os.walk(ROOT, topdown=True, followlinks=False):
        current_path = Path(current)
        directory_names[:] = [
            name
            for name in directory_names
            if not excluded_directory(current_path / name)
            and not (current_path / name).is_symlink()
        ]
        for name in file_names:
            path = current_path / name
            if not excluded_file(path) and path.is_file():
                files.append(path)
    return sorted(files, key=relative_path)


def manifest_lines(files: Iterable[Path]) -> list[str]:
    return [f"{_digest.sha256_file(path)}  {relative_path(path)}\n" for path in files]


def generate() -> int:
    files = source_files()
    data = "".join(manifest_lines(files)).encode("utf-8")
    MANIFEST.write_bytes(data)
    print(json.dumps({
        "status": "PASS",
        "operation": "generate",
        "manifest": str(MANIFEST),
        "fileCount": len(files),
        "manifestSha256": _digest.sha256_bytes(data),
    }, ensure_ascii=False, sort_keys=True))
    return 0


def parse_manifest() -> tuple[dict[str, str], list[str]]:
    entries: dict[str, str] = {}
    malformed: list[str] = []
    try:
        lines = MANIFEST.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError) as error:
        return {}, [f"manifest unreadable: {error}"]
    for line_number, line in enumerate(lines, start=1):
        if not line:
            malformed.append(f"line {line_number}: empty")
            continue
        digest, separator, path_text = line.partition("  ")
        if separator != "  " or len(digest) != 64 or any(character not in "0123456789abcdef" for character in digest) or not path_text:
            malformed.append(f"line {line_number}: invalid entry")
            continue
        pure = PurePosixPath(path_text)
        if pure.is_absolute() or any(part in {"", ".", ".."} for part in pure.parts):
            malformed.append(f"line {line_number}: unsafe path {path_text}")
            continue
        if path_text in entries:
            malformed.append(f"line {line_number}: duplicate path {path_text}")
            continue
        entries[path_text] = digest
    return entries, malformed


def verify() -> int:
    expected_files = source_files()
    expected = {relative_path(path): _digest.sha256_file(path) for path in expected_files}
    actual, malformed = parse_manifest()
    missing = sorted(set(expected) - set(actual))
    extra = sorted(set(actual) - set(expected))
    hash_mismatches = sorted(
        path for path in set(expected) & set(actual) if expected[path] != actual[path]
    )
    data = MANIFEST.read_bytes() if MANIFEST.is_file() else b""
    errors = malformed + [f"missing: {path}" for path in missing]
    errors += [f"extra: {path}" for path in extra]
    errors += [f"hash mismatch: {path}" for path in hash_mismatches]
    result = {
        "status": "PASS" if not errors else "FAIL",
        "operation": "verify",
        "manifest": str(MANIFEST),
        "expectedFileCount": len(expected),
        "manifestEntryCount": len(actual),
        "missing": missing,
        "extra": extra,
        "hashMismatches": hash_mismatches,
        "malformed": malformed,
        "manifestSha256": _digest.sha256_bytes(data),
    }
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0 if not errors else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("generate", "verify"))
    args = parser.parse_args()
    return generate() if args.operation == "generate" else verify()


if __name__ == "__main__":
    sys.exit(main())
