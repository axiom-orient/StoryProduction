from __future__ import annotations

import hashlib
from pathlib import Path

CHUNK_SIZE = 1024 * 1024


class GateError(RuntimeError):
    """A stable failure type shared by repository and standalone release gates."""


def sha(p: Path) -> str:
    digest = hashlib.sha256()
    with p.open("rb") as stream:
        for chunk in iter(lambda: stream.read(CHUNK_SIZE), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()
