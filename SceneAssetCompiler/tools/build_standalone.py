#!/usr/bin/env python3
"""Build a deterministic SceneAssetCompiler standalone ZIP.

The staging tree is the release boundary.  This command owns the only ZIP
writer used for the 0.13 candidate: it regenerates the content manifest,
sorts every member, fixes DOS timestamps and Unix mode bits, and can rebuild
the same staging tree twice to prove byte identity.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import stat
import tempfile
import zipfile

from release_verifier_support import sha


PACKAGE_NAME = "SceneAssetCompiler-Standalone-0.13.0"
MANIFEST_NAME = "BUNDLE_MANIFEST.sha256"
FIXED_DATE_TIME = (2020, 1, 1, 0, 0, 0)


class BuildError(RuntimeError):
    pass


def _regular_files(staging_root: Path) -> list[tuple[str, Path]]:
    files: list[tuple[str, Path]] = []
    for path in staging_root.rglob("*"):
        if path.is_symlink():
            raise BuildError(f"STAGING_SYMLINK:{path.relative_to(staging_root)}")
        if not path.is_file():
            continue
        relative = path.relative_to(staging_root).as_posix()
        if not relative or relative.startswith("/") or ".." in Path(relative).parts:
            raise BuildError(f"STAGING_PATH:{relative}")
        if relative == MANIFEST_NAME:
            continue
        files.append((relative, path))
    if not files:
        raise BuildError("STAGING_EMPTY")
    return sorted(files)


def _write_manifest(staging_root: Path, files: list[tuple[str, Path]]) -> Path:
    manifest = staging_root / MANIFEST_NAME
    lines = [f"{sha(path)}  ./{relative}" for relative, path in files]
    manifest.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    return manifest


def _zip_info(name: str, mode: int) -> zipfile.ZipInfo:
    info = zipfile.ZipInfo(name, FIXED_DATE_TIME)
    info.create_system = 3
    info.create_version = 20
    info.extract_version = 20
    info.flag_bits = 0
    info.compress_type = zipfile.ZIP_DEFLATED
    info.external_attr = (stat.S_IFREG | (mode & 0o7777)) << 16
    info.internal_attr = 0
    info.extra = b""
    info.comment = b""
    return info


def build_bundle(staging_root: Path, output: Path) -> dict[str, object]:
    staging_root = staging_root.expanduser().resolve()
    output = output.expanduser().resolve()
    if staging_root.name != PACKAGE_NAME:
        raise BuildError(f"STAGING_PACKAGE_NAME:{staging_root.name}")
    if not staging_root.is_dir() or staging_root.is_symlink():
        raise BuildError("STAGING_ROOT_NOT_DIRECTORY")
    try:
        output.relative_to(staging_root)
    except ValueError:
        pass
    else:
        raise BuildError("OUTPUT_INSIDE_STAGING")

    files = _regular_files(staging_root)
    manifest = _write_manifest(staging_root, files)
    all_files = sorted([*files, (MANIFEST_NAME, manifest)])
    output.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary_name = tempfile.mkstemp(prefix=f".{output.name}.", suffix=".tmp", dir=output.parent)
    os.close(fd)
    temporary = Path(temporary_name)
    try:
        with zipfile.ZipFile(
            temporary,
            "w",
            compression=zipfile.ZIP_DEFLATED,
            compresslevel=9,
            allowZip64=True,
        ) as archive:
            for relative, path in all_files:
                mode = stat.S_IMODE(path.stat().st_mode)
                archive.writestr(_zip_info(f"{PACKAGE_NAME}/{relative}", mode), path.read_bytes())
        os.replace(temporary, output)
    finally:
        if temporary.exists():
            temporary.unlink()
    return {
        "path": str(output),
        "sha256": sha(output),
        "bytes": output.stat().st_size,
        "manifestEntries": len(all_files) - 1,
        "regularFileEntries": len(all_files),
    }


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Build a deterministic SceneAssetCompiler standalone 0.13 ZIP")
    parser.add_argument("--staging-root", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument(
        "--deterministic-check",
        action="store_true",
        help="build a second ZIP from the same staging tree and require identical bytes",
    )
    return parser


def main() -> int:
    args = _parser().parse_args()
    first = build_bundle(args.staging_root, args.output)
    if not args.deterministic_check:
        print(
            f"BUNDLE_BUILD path={first['path']} sha256={first['sha256']} "
            f"bytes={first['bytes']} entries={first['regularFileEntries']}"
        )
        return 0
    repeat = args.output.with_name(f"{args.output.stem}.repeat{args.output.suffix}")
    second = build_bundle(args.staging_root, repeat)
    if first["sha256"] != second["sha256"] or first["bytes"] != second["bytes"]:
        raise BuildError(
            f"DETERMINISTIC_REBUILD_MISMATCH:first={first['sha256']}:{first['bytes']};"
            f"second={second['sha256']}:{second['bytes']}"
        )
    print(
        f"DETERMINISTIC_REBUILD PASS command=tools/build_standalone.py "
        f"sha256={first['sha256']} bytes={first['bytes']} entries={first['regularFileEntries']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except BuildError as error:
        print(f"FAIL {error}")
        raise SystemExit(1) from error
