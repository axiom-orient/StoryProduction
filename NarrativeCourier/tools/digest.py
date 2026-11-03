"""Raw SHA-256 primitives shared by NarrativeCourier maintenance tools.

Callers retain ownership of canonicalization, domain framing, prefixes, and IDs.
"""

from __future__ import annotations

import hashlib
from pathlib import Path

_CHUNK_SIZE = 1024 * 1024


def sha256_bytes(data: bytes) -> str:
    """Return the lowercase, unprefixed SHA-256 digest of bytes in memory."""

    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    """Hash a file incrementally and return lowercase, unprefixed hex."""

    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(_CHUNK_SIZE):
            digest.update(chunk)
    return digest.hexdigest()
