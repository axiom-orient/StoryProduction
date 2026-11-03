"""NVC cinematic authority: facts, explicit candidate plans, and review bindings.

This module never invents shots, appearance, timing, or approval. JSON schemas in
../contracts are the public format; this module owns producer-side invariants.
The existing Rust delivery DirectorPlan and its five DTOs are not changed.
"""
from __future__ import annotations
from dataclasses import dataclass
from functools import lru_cache
import json
import os
import stat
import tempfile
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

import digest as _digest

CONTRACTS = Path(__file__).resolve().parents[1] / 'contracts'
MAX_SAFE_INTEGER = 9007199254740991
MAX_JSON_BYTES = 16 * 1024 * 1024  # Local input resource policy, not narrative meaning.

class CinematicError(ValueError):
    def __init__(self, message: str, *, outcome: str = 'not_applied'):
        super().__init__(message)
        self.outcome = outcome

def require(condition: bool, message: str) -> None:
    if not condition:
        raise CinematicError(message)

def canonical_bytes(value: Any) -> bytes:
    """RFC 8785 integer-only domain used by these contracts (no floats)."""
    if value is None or isinstance(value, bool):
        return json.dumps(value).encode('ascii')
    if isinstance(value, int):
        require(abs(value) <= MAX_SAFE_INTEGER, 'INTEGER_RANGE')
        return str(value).encode('ascii')
    if isinstance(value, str):
        try:
            return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode('utf-8')
        except UnicodeError as error:
            raise CinematicError('INVALID_UNICODE') from error
    if isinstance(value, list):
        return b'[' + b','.join(canonical_bytes(x) for x in value) + b']'
    if isinstance(value, dict):
        require(all(isinstance(k, str) for k in value), 'NON_STRING_KEY')
        keys = sorted(value, key=lambda k: k.encode('utf-16-be'))
        return b'{' + b','.join(canonical_bytes(k)+b':'+canonical_bytes(value[k]) for k in keys) + b'}'
    raise CinematicError('NON_CANONICAL_JSON_NUMBER_OR_VALUE')

def digest_bytes(data: bytes) -> str:
    return 'sha256:' + _digest.sha256_bytes(data)

def content_hash(value: dict) -> str:
    return digest_bytes(canonical_bytes({k:v for k,v in value.items() if k != 'contentHash'}))

def seal(value: dict) -> dict:
    return {**value, 'contentHash': content_hash(value)}

def _pairs(items: list) -> dict:
    result = {}
    for key, value in items:
        require(key not in result, 'DUPLICATE_JSON_KEY:' + key)
        result[key] = value
    return result

def decode(data: bytes) -> dict:
    require(len(data) <= MAX_JSON_BYTES, 'JSON_TOO_LARGE')
    try:
        value = json.loads(data.decode('utf-8'), object_pairs_hook=_pairs,
                           parse_constant=lambda x: (_ for _ in ()).throw(CinematicError('NON_FINITE_NUMBER')))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise CinematicError('INVALID_UTF8_JSON') from error
    require(isinstance(value, dict), 'OBJECT_REQUIRED')
    canonical_bytes(value)  # Reject floats, unsafe integers and invalid Unicode everywhere.
    return value

@lru_cache(maxsize=None)
def _validator(name: str) -> Draft202012Validator:
    schema = json.loads((CONTRACTS / (name + '.schema.json')).read_text())
    if name == 'animatic-packet-v1':
        # A closed local reference set. The packet uses the producer's original
        # plan/review schemas, without another embedded authority or HTTP lookup.
        from referencing import Registry, Resource
        registry = Registry()
        for dependency in ('animatic-plan-v1', 'animatic-review-v1'):
            referenced = json.loads((CONTRACTS / (dependency + '.schema.json')).read_text())
            registry = registry.with_resource(referenced['$id'], Resource.from_contents(referenced))
        return Draft202012Validator(schema, registry=registry)
    return Draft202012Validator(schema)


def validate_shape(name: str, value: dict) -> None:
    first_error = min(
        _validator(name).iter_errors(value),
        key=lambda error: str(error.path),
        default=None,
    )
    if first_error is not None:
        raise CinematicError(
            f'SCHEMA:{name}:{list(first_error.path)}:{first_error.message}'
        )

def assert_sealed(name: str, value: dict) -> None:
    canonical_bytes(value)
    validate_shape(name, value)
    require(value['contentHash'] == content_hash(value), 'CONTENT_HASH:' + name)

def safe_path(root: Path, relative: str, *, existing: bool = True) -> Path:
    require(root.is_dir() and not root.is_symlink(), 'PROJECT_ROOT_REQUIRED')
    rel = Path(relative)
    require(bool(relative) and not rel.is_absolute() and '\\' not in relative
            and not any(p in ('..', '') for p in rel.parts), 'UNSAFE_PATH:' + relative)
    current = root
    for part in rel.parts:
        current = current / part
        require(not current.is_symlink(), 'SYMLINK_PATH:' + relative)
    require(current.resolve().is_relative_to(root.resolve()), 'PATH_ESCAPE:' + relative)
    if existing:
        require(current.is_file(), 'FILE_REQUIRED:' + relative)
    return current

def _read_bounded_bytes(root: Path, relative: str, maximum: int) -> bytes:
    path = safe_path(root, relative)
    return _read_regular_bytes(path, maximum)


def _read_regular_bytes(path: Path, maximum: int) -> bytes:
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode), 'REGULAR_FILE_REQUIRED:' + str(path))
    fd = os.open(path, os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0) | getattr(os, 'O_NONBLOCK', 0))
    with os.fdopen(fd, 'rb') as handle:
        opened = os.fstat(handle.fileno())
        require(stat.S_ISREG(opened.st_mode)
                and (opened.st_dev, opened.st_ino, opened.st_size) == (before.st_dev, before.st_ino, before.st_size),
                'FILE_CHANGED_DURING_OPEN:' + str(path))
        data = handle.read(maximum + 1)
        require(len(data) > maximum or len(data) == opened.st_size, 'FILE_SIZE_CHANGED:' + str(path))
        return data

def read_artifact(root: Path, relative: str, expected: str | None = None) -> tuple[dict, str]:
    data = _read_bounded_bytes(root, relative, MAX_JSON_BYTES)
    value = decode(data)
    actual = digest_bytes(data)
    if expected is not None:
        require(actual == ('sha256:' + expected.removeprefix('sha256:')), 'ARTIFACT_HASH:' + relative)
    return value, actual

def _matches_bytes(path: Path, expected: bytes) -> bool:
    return _read_regular_bytes(path, len(expected)) == expected

def publish(root: Path, relative: str, value: dict) -> str:
    """No overwrite: same bytes are idempotent; changed approved data is a revision."""
    return publish_bytes(root, relative, json_artifact_bytes(value))


def json_artifact_bytes(value: dict) -> bytes:
    """The byte representation shared by immutable JSON publication and prepared writes."""
    return json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False).encode('utf-8') + b'\n'


def publish_bytes(root: Path, relative: str, data: bytes) -> str:
    """Publish exact bytes with the same immutable policy for JSON and text."""
    target = safe_path(root, relative, existing=False)
    if target.exists():
        require(
            not target.is_symlink() and _matches_bytes(target, data),
            'IMMUTABLE_OUTPUT_EXISTS:' + relative,
        )
        return digest_bytes(data)
    target.parent.mkdir(parents=True, exist_ok=True)
    safe_path(root, relative, existing=False)
    fd, temporary = tempfile.mkstemp(prefix='.cinematic-', dir=target.parent)
    published = False
    try:
        with os.fdopen(fd, 'wb') as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, target, follow_symlinks=False)
            published = True
        except FileExistsError:
            require(
                not target.is_symlink() and _matches_bytes(target, data),
                'IMMUTABLE_OUTPUT_RACE:' + relative,
            )
            published = True
        directory = os.open(target.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    except (CinematicError, OSError, ValueError) as error:
        if published:
            raise CinematicError(
                'PUBLICATION_NOT_CONFIRMED:' + relative + ':' + str(error),
                outcome='outcome_unknown',
            ) from error
        raise
    finally:
        try:
            os.unlink(temporary)
        except OSError as error:
            raise CinematicError(
                'PUBLICATION_CLEANUP_FAILED:' + relative + ':' + str(error),
                outcome='outcome_unknown',
            ) from error
    return digest_bytes(data)

def _derived(value: dict, field: str, prefix: str, blank: bool = False) -> None:
    identity = dict(value)
    if blank:
        identity[field] = ''
    else:
        identity.pop(field, None)
    require(value[field] == prefix + digest_bytes(canonical_bytes(identity))[7:], 'DERIVED_ID:' + field)


@dataclass(frozen=True)
class _CompletedRelease:
    package: dict
    scene_map: dict
    binding: dict
    state: dict
    bundle_sha256: str


def load_completed_release(root: Path, relative: str) -> _CompletedRelease:
    bundle, bundle_hash = read_artifact(root, relative)
    validate_shape('release-bundle', bundle)
    _derived(bundle, 'bundle_id', 'nvcb_')
    def artifact(ref: dict) -> tuple[dict, str]:
        return read_artifact(root, ref['path'], ref['sha256'])
    state, _ = artifact(bundle['completed_state'])
    require(state.get('status') == 'completed' and state.get('nodes', {}).get('final_release', {}).get('status') == 'completed', 'NARRATIVE_NOT_COMPLETED')
    require(digest_bytes(canonical_bytes(state.get('source_snapshot')))[7:] == bundle['source_snapshot_sha256'], 'SOURCE_SNAPSHOT_BINDING')
    package, package_hash = artifact(bundle['narrative_package'])
    scene_map, scene_hash = artifact(bundle['scene_map'])
    validate_shape('narrative-package', package)
    validate_shape('narrative-scene-map-v2', scene_map)
    _derived(package, 'package_id', 'nvc2_')
    require(package_hash[7:] == bundle['package_sha256'] and package['package_id'] == bundle['package_id'], 'PACKAGE_BINDING')
    require(package['source_snapshot_sha256'] == bundle['source_snapshot_sha256'], 'PACKAGE_SNAPSHOT_BINDING')
    require(package['narrative_id'] == bundle['project_name'] == scene_map['narrative_id'], 'NARRATIVE_ID_BINDING')
    require(package['scene_map']['path'] == bundle['scene_map']['path']
            and package['scene_map']['sha256'] == scene_hash[7:], 'SCENE_MAP_BINDING')
    validate_scene_map(scene_map)
    binding = {'narrativePackageSha256': package_hash, 'sceneMapSha256': scene_hash}
    return _CompletedRelease(package, scene_map, binding, state, bundle_hash)

def validate_scene_map(scene_map: dict) -> None:
    """Validate occurrence identity, not only JSON shape or a self-asserted hash."""
    validate_shape('narrative-scene-map-v2', scene_map)
    scenes = scene_map['scenes']
    require(len({s['scene_id'] for s in scenes}) == len(scenes), 'SCENE_ID_DUPLICATE')
    manifest = scene_map['semantic_manifest']

    def without_hash(value: dict, field: str) -> str:
        return digest_bytes(canonical_bytes({k:v for k,v in value.items() if k != field}))

    entries = manifest['entries']
    seen_ids = set()
    locators = set()
    by_scene = {s['scene_id']: s for s in scenes}
    for index, scene in enumerate(scenes):
        require(scene['order'] == index + 1, 'SCENE_ORDER')
        require(scene['silent'] == (not scene['dialogue']), 'SCENE_SILENCE')
        require(all(d['character'] in scene['characters'] for d in scene['dialogue']), 'SCENE_CHARACTER')
    for entry in entries:
        sid = entry['scene_id']
        loc = entry['source_locator']
        eid = entry['semantic_id']
        require(sid in by_scene and loc['scene_id'] == sid and entry['kind'] == loc['field'], 'SEMANTIC_SOURCE')
        key = (sid, loc['field'], loc['occurrence_index'])
        require(eid not in seen_ids and key not in locators, 'SEMANTIC_DUPLICATE')
        seen_ids.add(eid)
        locators.add(key)
        scene = by_scene[sid]
        occurrence = loc['occurrence_index']
        if entry['kind'] == 'action':
            require(occurrence < len(scene['action']) and entry['source_text'] == scene['action'][occurrence], 'ACTION_SOURCE')
        elif entry['kind'] == 'dialogue':
            require(occurrence < len(scene['dialogue']), 'DIALOGUE_SOURCE')
            dialogue = scene['dialogue'][occurrence]
            require(loc['dialogue_index'] == occurrence and loc['character'] == dialogue['character']
                    and entry['source_text'] == dialogue['line'], 'DIALOGUE_SOURCE')
        require(entry['entry_hash'] == without_hash(entry, 'entry_hash'), 'SEMANTIC_ENTRY_HASH')

    for scene in scenes:
        for kind in ('action', 'dialogue'):
            indices = {index for sid, field, index in locators if sid == scene['scene_id'] and field == kind}
            require(indices == set(range(len(scene[kind]))), 'SEMANTIC_OCCURRENCE_MISSING')

    directive_refs = {}
    for scene_id, field, occurrence in locators:
        if field not in {'action', 'dialogue'}:
            directive_refs.setdefault((scene_id, field), set()).add(occurrence)
    for (scene_id, kind), occurrences in sorted(directive_refs.items()):
        require(
            occurrences == set(range(len(occurrences))),
            f'{kind} manifest occurrence indices are not contiguous for {scene_id}',
        )

    require(manifest['semantic_manifest_hash'] == without_hash(manifest, 'semantic_manifest_hash'), 'SEMANTIC_MANIFEST_HASH')
    _derived(scene_map, 'map_id', 'nsm2_', True)

def entity_id(kind: str, identity: str) -> str:
    prefix = {'CHARACTER': 'char', 'ENVIRONMENT': 'env', 'PROP': 'prop'}[kind]
    return prefix + '.h' + _digest.sha256_bytes(identity.encode('utf-8'))[:24]

def extract_continuity(scene_map: dict, source: dict) -> dict:
    validate_scene_map(scene_map)
    entities: dict[str, dict] = {}; states = []
    def add(kind: str, name: str, identity: str, sid: str) -> str:
        eid = entity_id(kind, identity)
        previous = entities.get(eid)
        if previous:
            require(previous['kind'] == kind and previous['name'] == name, 'ENTITY_ID_COLLISION')
            if sid not in previous['sourceSceneIds']:
                previous['sourceSceneIds'].append(sid)
        else:
            entities[eid] = {'entityId':eid,'kind':kind,'name':name,'sourceSceneIds':[sid],'visualState':'UNRESOLVED_VISUAL'}
        return eid
    for scene in scene_map['scenes']:
        sid = scene['scene_id']
        # Geography identity is independent from time/lighting state.
        env = add('ENVIRONMENT', scene['location'], scene['setting'] + '|' + scene['location'], sid)
        chars = [add('CHARACTER', name, name, sid) for name in dict.fromkeys(scene['characters'])]
        props = list(dict.fromkeys(add('PROP', r['description'], r['kind'] + '|' + r['description'], sid) for r in scene['asset_requirements']))
        states.append({'sceneId':sid,'environmentId':env,'characterIds':chars,'propIds':props,'timeOfDay':scene['time_of_day']})
    bible = seal({'schemaVersion':1,'narrativeId':scene_map['narrative_id'],**source,'entities':list(entities.values()),'sceneStates':states})
    assert_sealed('continuity-bible-v1', bible)
    return bible

def validate_plan(scene_map: dict, bible: dict, plan: dict, source: dict) -> None:
    assert_sealed('continuity-bible-v1', bible)
    assert_sealed('cinematic-director-plan-v1', plan)
    require(bible == extract_continuity(scene_map, source), 'CONTINUITY_FACTS_CHANGED')
    require(all(plan[k] == v for k,v in source.items()), 'PLAN_SOURCE_STALE')
    require(plan['continuityBibleSha256'] == bible['contentHash'], 'CONTINUITY_BINDING')
    require([s['sceneId'] for s in plan['scenes']] == [s['scene_id'] for s in scene_map['scenes']], 'PLAN_SCENE_ORDER')
    source_entries = {e['semantic_id']:e for e in scene_map['semantic_manifest']['entries']}
    assigned: set[str] = set(); shot_ids: set[str] = set(); order = 0; total = 0
    for scene, state in zip(plan['scenes'], bible['sceneStates']):
        sid = scene['sceneId']; allowed = {state['environmentId'], *state['characterIds'], *state['propIds']}
        require(scene['sceneDurationFrames'] == sum(s['durationFrames'] for s in scene['shots']), 'SCENE_DURATION_SUM')
        dialogue_order = []
        for shot in scene['shots']:
            order += 1; total += shot['durationFrames']
            require(total <= MAX_SAFE_INTEGER, 'TIMELINE_OVERFLOW')
            require(shot['order'] == order and shot['shotId'] not in shot_ids, 'SHOT_ID_OR_ORDER')
            shot_ids.add(shot['shotId'])
            require(shot['narrativeRefs'] == [sid] and shot['continuityRefs'] == [sid], 'SHOT_SCENE_BINDING')
            require(set(shot['requiredEntities']) <= allowed and state['environmentId'] in shot['requiredEntities'], 'SHOT_ENTITY_BINDING')
            for ref in shot['semanticRefs']:
                require(ref in source_entries and source_entries[ref]['scene_id'] == sid and ref not in assigned, 'SEMANTIC_ASSIGNMENT')
                assigned.add(ref)
            dialogue = [r for r in shot['semanticRefs'] if source_entries[r]['kind'] == 'dialogue']
            require(shot['dialogueRefs'] == dialogue, 'DIALOGUE_ASSIGNMENT')
            dialogue_order.extend(source_entries[r]['source_locator']['dialogue_index'] for r in dialogue)
            for ref in dialogue:
                name = source_entries[ref]['source_locator']['character']
                require(entity_id('CHARACTER', name) in shot['requiredEntities'], 'DIALOGUE_CHARACTER_MISSING')
            policy = shot['referencePolicy']
            if any(e.startswith('char.') for e in shot['requiredEntities']):
                require(policy['requiresCharacterReference'] and policy['requiresKeyframe'], 'CHARACTER_REFERENCE_REQUIRED')
            transition = shot['transitionOut']
            require((transition['type'] == 'CUT') == (transition['durationFrames'] == 0), 'TRANSITION_DURATION')
        require(dialogue_order == sorted(dialogue_order), 'DIALOGUE_ORDER_CHANGED')
    require(assigned == set(source_entries), 'SEMANTIC_OCCURRENCE_LOSS')
    shots = [shot for scene in plan['scenes'] for shot in scene['shots']]
    for index, shot in enumerate(shots):
        frames = shot['transitionOut']['durationFrames']
        if index + 1 == len(shots):
            require(frames == 0, 'FINAL_SHOT_TRANSITION')
        else:
            require(frames <= min(shot['durationFrames'], shots[index+1]['durationFrames']), 'TRANSITION_EXCEEDS_SHOT')

def validate_approval(plan: dict, bible: dict, approval: dict) -> None:
    assert_sealed('cinematic-approval-v1', approval)
    require(approval['directorPlanSha256'] == plan['contentHash']
            and approval['continuityBibleSha256'] == bible['contentHash']
            and all(approval[k] == plan[k] for k in ('narrativePackageSha256','sceneMapSha256')), 'APPROVAL_STALE')
