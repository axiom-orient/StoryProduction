"""Scene-local artifact path and human-review receipt primitives."""
from __future__ import annotations

from pathlib import Path

from .scene_plan import SceneError
from .util import load_json, sha256_file


def _scene_local(scene_dir:Path,path:Path,label:str):
    root=Path(scene_dir).resolve(); candidate=Path(path)
    probe=candidate if candidate.is_absolute() else root/candidate
    if probe.is_symlink(): raise SceneError(f'{label}_SYMLINK')
    resolved=probe.resolve()
    if resolved!=root and root not in resolved.parents: raise SceneError(f'{label}_OUTSIDE_SCENE')
    return resolved

def _review_receipt(scene_dir:Path, artifact_path:Path, review_path:Path):
    review_path=_scene_local(scene_dir,review_path,'LOCK_REVIEW')
    if not review_path.is_file(): raise SceneError('LOCK_REVIEW_MISSING')
    review=load_json(review_path)
    if review.get('schema_version')!=1 or review.get('status')!='APPROVED': raise SceneError('LOCK_REVIEW_NOT_APPROVED')
    if review.get('artifact_sha256')!=sha256_file(artifact_path): raise SceneError('LOCK_REVIEW_ARTIFACT_MISMATCH')
    if not isinstance(review.get('evidence'),list) or not any(isinstance(x,str) and x.strip() for x in review['evidence']): raise SceneError('LOCK_REVIEW_EVIDENCE_REQUIRED')
    return review_path

def _verify_lock_artifact(scene_dir,lock, label):
    artifact=_scene_local(scene_dir,Path(lock['artifact_path']),'LOCK_ARTIFACT')
    if not artifact.is_file(): raise SceneError(f'LOCK_ARTIFACT_MISSING:{label}')
    actual=sha256_file(artifact)
    if actual!=lock['artifact_sha256']: raise SceneError(f'LOCK_ARTIFACT_DRIFT:{label}')
    review=_scene_local(scene_dir,Path(lock.get('review_path','')),'LOCK_REVIEW')
    if not review.is_file(): raise SceneError(f'LOCK_REVIEW_MISSING:{label}')
    if sha256_file(review)!=lock.get('review_sha256'): raise SceneError(f'LOCK_REVIEW_DRIFT:{label}')
    if load_json(review).get('artifact_sha256')!=actual: raise SceneError(f'LOCK_REVIEW_ARTIFACT_MISMATCH:{label}')
