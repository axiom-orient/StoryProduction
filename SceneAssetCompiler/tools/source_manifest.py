#!/usr/bin/env python3
"""Generate and verify SceneAssetCompiler source and release manifests.

The root release manifest covers the complete current release tree: source,
contracts, tests, fixtures, experiments, documentation, and release tools.
The omitted paths are repository metadata (``.git``), Finder metadata,
Python bytecode/cache, Rust build output (``target``), and the manifest file
being generated. Live run output remains in an operator-owned workspace.

The skill and documentation manifests are subordinate integrity scopes.
``SOURCE_RELEASE_MANIFEST.sha256`` at the repository root is the release-scope
authority. Historical release-evidence snapshot manifests are intentionally not
regenerated; they duplicate the same source truth and create avoidable cycles.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
from typing import Iterable

from release_verifier_support import sha


ROOT = Path(__file__).resolve().parents[1]
SKILL_ROOT = ROOT / ".agents/skills/scene-asset-compiler"
DOCS_ROOT = ROOT / "docs"
RELEASE_MANIFEST = ROOT / "SOURCE_RELEASE_MANIFEST.sha256"
SKILL_MANIFEST = SKILL_ROOT / "SOURCE_MANIFEST.sha256"
DOCS_MANIFEST = DOCS_ROOT / "DOCS_MANIFEST.sha256"
EXCLUDED_DIRECTORY_NAMES = {".git", "__pycache__", "target"}
EXCLUDED_FILE_NAMES = {".DS_Store"}
EXCLUDED_SUFFIXES = {".pyc", ".pyo"}


def relative(path: Path, base: Path) -> str:
    return path.relative_to(base).as_posix()


def source_files(base: Path, *, excluded: set[str]) -> list[Path]:
    files: list[Path] = []
    for current, directory_names, file_names in os.walk(base, topdown=True, followlinks=False):
        current_path = Path(current)
        directory_names[:] = [
            name
            for name in directory_names
            if name not in EXCLUDED_DIRECTORY_NAMES
            and not (current_path / name).is_symlink()
        ]
        for name in file_names:
            path = current_path / name
            rel = relative(path, base)
            if rel in excluded or path.is_symlink():
                continue
            if (
                path.name in EXCLUDED_FILE_NAMES
                or path.name.startswith("._")
                or path.suffix in EXCLUDED_SUFFIXES
            ):
                continue
            if path.is_file():
                files.append(path)
    return sorted(files, key=lambda path: relative(path, base))


def lines_for(files: Iterable[Path], base: Path) -> list[str]:
    return [f"{sha(path)}  {relative(path, base)}\n" for path in files]


def write_manifest(path: Path, files: Iterable[Path], base: Path) -> dict[str, object]:
    data = "".join(lines_for(files, base)).encode("utf-8")
    path.write_bytes(data)
    return {"path": str(path), "fileCount": len(data.splitlines()), "sha256": hashlib.sha256(data).hexdigest()}


def release_files() -> list[Path]:
    return source_files(ROOT, excluded={RELEASE_MANIFEST.relative_to(ROOT).as_posix()})


def skill_files() -> list[Path]:
    return source_files(SKILL_ROOT, excluded={SKILL_MANIFEST.relative_to(SKILL_ROOT).as_posix()})


def docs_files() -> list[Path]:
    return source_files(DOCS_ROOT, excluded={DOCS_MANIFEST.relative_to(DOCS_ROOT).as_posix()})


def generate_skill() -> dict[str, object]:
    return write_manifest(SKILL_MANIFEST, skill_files(), SKILL_ROOT)


def generate_all() -> dict[str, object]:
    # Order is deterministic: subordinate scopes are sealed before the root.
    skill = generate_skill()
    docs = write_manifest(DOCS_MANIFEST, docs_files(), DOCS_ROOT)
    release = write_manifest(RELEASE_MANIFEST, release_files(), ROOT)
    return {"skill": skill, "docs": docs, "release": release}


def parse_manifest(path: Path) -> tuple[dict[str, str], list[str]]:
    entries: dict[str, str] = {}
    errors: list[str] = []
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError) as error:
        return {}, [f"unreadable: {error}"]
    for line_number, line in enumerate(lines, start=1):
        if not line:
            errors.append(f"line {line_number}: empty")
            continue
        digest, separator, rel = line.partition("  ")
        if separator != "  " or len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest) or not rel:
            errors.append(f"line {line_number}: malformed")
            continue
        if rel in entries:
            errors.append(f"line {line_number}: duplicate {rel}")
            continue
        entries[rel] = digest
    return entries, errors


def verify_manifest(path: Path, files: list[Path], base: Path) -> dict[str, object]:
    expected = {relative(file, base): sha(file) for file in files}
    actual, malformed = parse_manifest(path)
    missing = sorted(set(expected) - set(actual))
    extra = sorted(set(actual) - set(expected))
    mismatches = sorted(name for name in set(expected) & set(actual) if expected[name] != actual[name])
    errors = malformed + [f"missing:{name}" for name in missing]
    errors += [f"extra:{name}" for name in extra]
    errors += [f"hash:{name}" for name in mismatches]
    data = path.read_bytes() if path.is_file() else b""
    return {
        "status": "PASS" if not errors else "FAIL",
        "path": str(path),
        "expectedFileCount": len(expected),
        "manifestEntryCount": len(actual),
        "missing": missing,
        "extra": extra,
        "hashMismatches": mismatches,
        "malformed": malformed,
        "sha256": hashlib.sha256(data).hexdigest(),
    }


def verify_skill() -> dict[str, object]:
    return verify_manifest(SKILL_MANIFEST, skill_files(), SKILL_ROOT)


def verify_all() -> dict[str, object]:
    result = {
        "skill": verify_skill(),
        "docs": verify_manifest(DOCS_MANIFEST, docs_files(), DOCS_ROOT),
        "release": verify_manifest(RELEASE_MANIFEST, release_files(), ROOT),
    }
    result["status"] = "PASS" if all(item["status"] == "PASS" for item in result.values()) else "FAIL"
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("generate", "verify"))
    args = parser.parse_args()
    result = generate_all() if args.operation == "generate" else verify_all()
    if args.operation == "verify" and result["status"] != "PASS":
        exit_code = 1
    else:
        exit_code = 0
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
