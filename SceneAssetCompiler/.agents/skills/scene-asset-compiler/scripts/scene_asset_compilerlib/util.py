from __future__ import annotations
from pathlib import Path
import hashlib, json, os, tempfile


def load_json(path: Path):
    with path.open('rb') as f:
        return load_json_bytes(f.read())


def unique_json_object(pairs, duplicate_code='DUPLICATE_JSON_KEY'):
    value = {}
    for key, child in pairs:
        if key in value:
            raise ValueError(f'{duplicate_code}:{key}')
        value[key] = child
    return value


def load_json_bytes(data: bytes, *, duplicate_code='DUPLICATE_JSON_KEY'):
    return json.loads(
        data,
        object_pairs_hook=lambda pairs: unique_json_object(pairs, duplicate_code),
    )


def atomic_json(path: Path, obj) -> None:
    data = (json.dumps(obj, ensure_ascii=False, indent=2) + '\n').encode('utf-8')
    atomic_bytes(path, data)


def exclusive_json(path: Path, obj) -> None:
    """Create one durable JSON claim without replacing an existing target."""
    path.parent.mkdir(parents=True, exist_ok=True)
    data = (json.dumps(obj, ensure_ascii=False, indent=2) + '\n').encode('utf-8')
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    if hasattr(os, 'O_NOFOLLOW'):
        flags |= os.O_NOFOLLOW
    fd = os.open(path, flags, 0o600)
    # Keep even a partial claim: after creation, retry safety outranks reuse.
    with os.fdopen(fd, 'wb') as f:
        f.write(data); f.flush(); os.fsync(f.fileno())
    directory_fd = os.open(path.parent, os.O_RDONLY)
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)


def atomic_text(path: Path, text: str) -> None:
    atomic_bytes(path, text.encode('utf-8'))


def atomic_bytes(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=path.name+'.', dir=str(path.parent))
    try:
        with os.fdopen(fd, 'wb') as f:
            f.write(data); f.flush(); os.fsync(f.fileno())
        os.replace(tmp, path)
    finally:
        if os.path.exists(tmp): os.unlink(tmp)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    h=hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda:f.read(1024*1024), b''): h.update(chunk)
    return h.hexdigest()


def safe_rel(root: Path, path: Path) -> str:
    root=root.resolve(); path=path.resolve()
    return str(path.relative_to(root))


def require_under(root: Path, candidate: Path) -> Path:
    root=root.resolve(); candidate=candidate.resolve()
    try: candidate.relative_to(root)
    except ValueError: raise ValueError(f'PATH_ESCAPE:{candidate}')
    return candidate
