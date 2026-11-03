"""Own scene locks, prerequisite revalidation, and immutable seal transitions."""
from __future__ import annotations

from pathlib import Path

from .util import load_json, atomic_json, sha256_file
from .scene_files import _review_receipt, _scene_local, _verify_lock_artifact
from .scene_plan import SceneError, _layout_slots, validate_scene_plan

LOCK_ORDER=['COMPOSITION_LOCK','TARGET_LOCK','LAYOUT_LOCK','COORDINATE_LOCK','ASSET_LOCK','COMPOSITE_LOCK','DELIVERY_LOCK']


def _pre_asset_locks(plan):
    if _layout_slots(plan):
        return ('COMPOSITION_LOCK','TARGET_LOCK','LAYOUT_LOCK','PLACEHOLDER_SET_LOCK','COORDINATE_LOCK')
    return ('COMPOSITION_LOCK','TARGET_LOCK','COORDINATE_LOCK')


def _verify_prerequisite(scene_dir:Path,kind:str):
    scene_id=load_json(scene_dir/'scene.json')['scene_id']; lp=scene_dir/'locks'/f'{kind.lower()}.json'
    if not lp.is_file(): raise SceneError(f'LOCK_PREREQUISITE:{kind}')
    lock=load_json(lp)
    if lock.get('schema_version')!=1 or lock.get('scene_id')!=scene_id or lock.get('lock_kind')!=kind: raise SceneError(f'LOCK_RECORD_INVALID:{kind}')
    plan=validate_scene_plan(load_json(scene_dir/'scene-plan.json'))
    if kind=='PLACEHOLDER_SET_LOCK':
        slots=_layout_slots(plan); expected=[slot['slot_id'] for slot in slots]
        if not slots: raise SceneError('LOCK_RECORD_INVALID:PLACEHOLDER_SET_LOCK')
        _verify_prerequisite(scene_dir,'LAYOUT_LOCK')
        layout_path=scene_dir/'locks/layout_lock.json'
        if lock.get('layout_lock_sha256')!=sha256_file(layout_path): raise SceneError('LOCK_DEPENDENCY_DRIFT:PLACEHOLDER_SET_LOCK:LAYOUT_LOCK')
        placeholders=lock.get('placeholders')
        if not isinstance(placeholders,list) or any(not isinstance(x,dict) for x in placeholders) or [x.get('slot_id') for x in placeholders]!=expected: raise SceneError('LOCK_RECORD_INVALID:PLACEHOLDER_SET_LOCK')
        lock_hashes=lock.get('placeholder_lock_sha256s')
        if not isinstance(lock_hashes,dict) or set(lock_hashes)!=set(expected): raise SceneError('LOCK_RECORD_INVALID:PLACEHOLDER_SET_LOCK')
        for item in placeholders:
            if item.get('schema_version')!=1 or item.get('scene_id')!=scene_id or item.get('lock_kind')!='PLACEHOLDER_LOCK': raise SceneError('LOCK_RECORD_INVALID:PLACEHOLDER_SET_LOCK')
            slot_id=item.get('slot_id'); individual_path=scene_dir/'locks/placeholders'/f'{slot_id}.json'
            if not individual_path.is_file(): raise SceneError(f'LOCK_PREREQUISITE:PLACEHOLDER_LOCK:{slot_id}')
            if sha256_file(individual_path)!=lock_hashes.get(slot_id): raise SceneError(f'LOCK_DEPENDENCY_DRIFT:PLACEHOLDER_SET_LOCK:PLACEHOLDER_LOCK:{slot_id}')
            if load_json(individual_path)!=item: raise SceneError(f'LOCK_RECORD_INVALID:PLACEHOLDER_SET_LOCK:{slot_id}')
            _verify_lock_artifact(scene_dir,item,f"PLACEHOLDER_LOCK:{item.get('slot_id')}")
    elif kind=='ASSET_LOCK':
        expected=[x['layer_id'] for x in plan['layers']]
        layers=lock.get('layers')
        if not isinstance(layers,list) or any(not isinstance(x,dict) for x in layers) or [x.get('layer_id') for x in layers]!=expected: raise SceneError('LOCK_RECORD_INVALID:ASSET_LOCK')
        for item in layers:
            if item.get('schema_version')!=1 or item.get('scene_id')!=scene_id or item.get('lock_kind')!='LAYER_LOCK': raise SceneError('LOCK_RECORD_INVALID:ASSET_LOCK')
            _verify_lock_artifact(scene_dir,item,f"LAYER_LOCK:{item.get('layer_id')}")
        if _layout_slots(plan):
            _verify_prerequisite(scene_dir,'LAYOUT_LOCK')
            _verify_prerequisite(scene_dir,'PLACEHOLDER_SET_LOCK')
            if lock.get('layout_lock_sha256')!=sha256_file(scene_dir/'locks/layout_lock.json'): raise SceneError('LOCK_DEPENDENCY_DRIFT:ASSET_LOCK:LAYOUT_LOCK')
            if lock.get('placeholder_set_lock_sha256')!=sha256_file(scene_dir/'locks/placeholder_set_lock.json'): raise SceneError('LOCK_DEPENDENCY_DRIFT:ASSET_LOCK:PLACEHOLDER_SET_LOCK')
    else:
        _verify_lock_artifact(scene_dir,lock,kind)
    return lock

def create_lock(scene_dir:Path,kind:str,artifact_path:Path,review_path:Path,note:str=''):
    if kind not in LOCK_ORDER: raise SceneError(f'LOCK_KIND:{kind}')
    plan=validate_scene_plan(load_json(scene_dir/'scene-plan.json'))
    if kind=='LAYOUT_LOCK' and not _layout_slots(plan): raise SceneError('LAYOUT_LOCK_UNEXPECTED')
    lock_path=scene_dir/'locks'/f'{kind.lower()}.json'
    if lock_path.exists(): raise SceneError(f'LOCK_IMMUTABLE:{kind}')
    required={
      'TARGET_LOCK':['COMPOSITION_LOCK'],
      'LAYOUT_LOCK':['COMPOSITION_LOCK','TARGET_LOCK'],
      'COORDINATE_LOCK':['COMPOSITION_LOCK','TARGET_LOCK'],
      'ASSET_LOCK':['COMPOSITION_LOCK','TARGET_LOCK','COORDINATE_LOCK'],
      'COMPOSITE_LOCK':['COMPOSITION_LOCK','TARGET_LOCK','COORDINATE_LOCK','ASSET_LOCK'],
      'DELIVERY_LOCK':['COMPOSITION_LOCK','TARGET_LOCK','COORDINATE_LOCK','ASSET_LOCK','COMPOSITE_LOCK'],
    }.get(kind,[])
    if kind=='COORDINATE_LOCK' and _layout_slots(plan): required=[*required,'LAYOUT_LOCK']
    if kind=='ASSET_LOCK' and _layout_slots(plan): required=[*required,'LAYOUT_LOCK','PLACEHOLDER_SET_LOCK']
    for prev in required:
        _verify_prerequisite(scene_dir,prev)
    artifact_path=_scene_local(scene_dir,artifact_path,'LOCK_ARTIFACT')
    if not artifact_path.is_file(): raise SceneError('LOCK_ARTIFACT_MISSING')
    review_path=_review_receipt(scene_dir,artifact_path,review_path)
    rec={'schema_version':1,'scene_id':load_json(scene_dir/'scene.json')['scene_id'],'lock_kind':kind,'artifact_path':str(artifact_path),'artifact_sha256':sha256_file(artifact_path),'review_path':str(review_path),'review_sha256':sha256_file(review_path),'note':note}
    atomic_json(lock_path,rec); return rec

def lock_placeholder(scene_dir:Path,slot_id:str,artifact_path:Path,review_path:Path,note:str=''):
    plan=validate_scene_plan(load_json(scene_dir/'scene-plan.json'))
    allowed={slot['slot_id'] for slot in _layout_slots(plan)}
    if slot_id not in allowed: raise SceneError(f'PLACEHOLDER_LOCK_UNKNOWN:{slot_id}')
    for prev in ('COMPOSITION_LOCK','TARGET_LOCK','LAYOUT_LOCK'):
        _verify_prerequisite(scene_dir,prev)
    d=scene_dir/'locks/placeholders'; d.mkdir(parents=True,exist_ok=True)
    lp=d/f'{slot_id}.json'
    if lp.exists(): raise SceneError(f'PLACEHOLDER_LOCK_IMMUTABLE:{slot_id}')
    artifact_path=_scene_local(scene_dir,artifact_path,'LOCK_ARTIFACT')
    if not artifact_path.is_file(): raise SceneError('LOCK_ARTIFACT_MISSING')
    review_path=_review_receipt(scene_dir,artifact_path,review_path)
    rec={'schema_version':1,'scene_id':plan['scene_id'],'lock_kind':'PLACEHOLDER_LOCK','slot_id':slot_id,'artifact_path':str(artifact_path),'artifact_sha256':sha256_file(artifact_path),'review_path':str(review_path),'review_sha256':sha256_file(review_path),'note':note}
    atomic_json(lp,rec); return rec

def seal_placeholders(scene_dir:Path):
    plan=validate_scene_plan(load_json(scene_dir/'scene-plan.json')); slots=_layout_slots(plan)
    if not slots: raise SceneError('PLACEHOLDER_SET_LAYOUT_REQUIRED')
    target=scene_dir/'locks/placeholder_set_lock.json'
    if target.exists(): raise SceneError('LOCK_IMMUTABLE:PLACEHOLDER_SET_LOCK')
    for prev in ('COMPOSITION_LOCK','TARGET_LOCK','LAYOUT_LOCK'):
        _verify_prerequisite(scene_dir,prev)
    entries=[]
    for slot in slots:
        slot_id=slot['slot_id']; lp=scene_dir/'locks/placeholders'/f'{slot_id}.json'
        if not lp.exists(): raise SceneError(f'PLACEHOLDER_SET_MISSING_SLOT:{slot_id}')
        lock=load_json(lp)
        if lock.get('schema_version')!=1 or lock.get('scene_id')!=plan['scene_id'] or lock.get('lock_kind')!='PLACEHOLDER_LOCK' or lock.get('slot_id')!=slot_id: raise SceneError(f'LOCK_RECORD_INVALID:PLACEHOLDER_LOCK:{slot_id}')
        _verify_lock_artifact(scene_dir,lock,f'PLACEHOLDER_LOCK:{slot_id}')
        entries.append(lock)
    lock_hashes={slot['slot_id']:sha256_file(scene_dir/'locks/placeholders'/f"{slot['slot_id']}.json") for slot in slots}
    rec={'schema_version':1,'scene_id':plan['scene_id'],'lock_kind':'PLACEHOLDER_SET_LOCK','layout_lock_sha256':sha256_file(scene_dir/'locks/layout_lock.json'),'placeholder_lock_sha256s':lock_hashes,'placeholders':entries}
    atomic_json(target,rec); return rec

def lock_layer(scene_dir:Path,layer_id:str,artifact_path:Path,review_path:Path,note:str=''):
    plan=validate_scene_plan(load_json(scene_dir/'scene-plan.json'))
    for prev in _pre_asset_locks(plan):
        _verify_prerequisite(scene_dir,prev)
    allowed={x['layer_id'] for x in plan['layers']}
    if layer_id not in allowed: raise SceneError(f'LAYER_LOCK_UNKNOWN:{layer_id}')
    d=scene_dir/'locks/layers'; d.mkdir(parents=True,exist_ok=True)
    safe=layer_id.replace('/','_')
    lp=d/f'{safe}.json'
    if lp.exists(): raise SceneError(f'LAYER_LOCK_IMMUTABLE:{layer_id}')
    artifact_path=_scene_local(scene_dir,artifact_path,'LOCK_ARTIFACT')
    if not artifact_path.is_file(): raise SceneError('LOCK_ARTIFACT_MISSING')
    review_path=_review_receipt(scene_dir,artifact_path,review_path)
    rec={'schema_version':1,'scene_id':plan['scene_id'],'lock_kind':'LAYER_LOCK','layer_id':layer_id,'artifact_path':str(artifact_path),'artifact_sha256':sha256_file(artifact_path),'review_path':str(review_path),'review_sha256':sha256_file(review_path),'note':note}
    atomic_json(lp,rec); return rec

def seal_assets(scene_dir:Path):
    plan=validate_scene_plan(load_json(scene_dir/'scene-plan.json'))
    if (scene_dir/'locks/asset_lock.json').exists(): raise SceneError('LOCK_IMMUTABLE:ASSET_LOCK')
    for kind in _pre_asset_locks(plan):
        lp=scene_dir/'locks'/f'{kind.lower()}.json'
        if not lp.exists(): raise SceneError(f'LOCK_PREREQUISITE:{kind}')
        _verify_prerequisite(scene_dir,kind)
    entries=[]
    for layer in plan['layers']:
        lp=scene_dir/'locks/layers'/f"{layer['layer_id'].replace('/','_')}.json"
        if not lp.exists(): raise SceneError(f"ASSET_LOCK_MISSING_LAYER:{layer['layer_id']}")
        lock=load_json(lp)
        if lock.get('schema_version')!=1 or lock.get('scene_id')!=plan['scene_id'] or lock.get('lock_kind')!='LAYER_LOCK' or lock.get('layer_id')!=layer['layer_id']: raise SceneError(f"LOCK_RECORD_INVALID:LAYER_LOCK:{layer['layer_id']}")
        _verify_lock_artifact(scene_dir,lock,f"LAYER_LOCK:{layer['layer_id']}")
        entries.append(lock)
    rec={'schema_version':1,'scene_id':plan['scene_id'],'lock_kind':'ASSET_LOCK','layers':entries}
    if _layout_slots(plan):
        rec['layout_lock_sha256']=sha256_file(scene_dir/'locks/layout_lock.json')
        rec['placeholder_set_lock_sha256']=sha256_file(scene_dir/'locks/placeholder_set_lock.json')
    atomic_json(scene_dir/'locks/asset_lock.json',rec); return rec

def validate_psd_entry(scene_dir:Path):
    plan=validate_scene_plan(load_json(scene_dir/'scene-plan.json'))
    if 'PSD' not in plan['deliverables']: raise SceneError('PSD_ENTRY_NOT_DECLARED')
    _verify_prerequisite(scene_dir,'ASSET_LOCK')
    return {'schema_version':1,'scene_id':plan['scene_id'],'stage':'PSD_PACKAGING','status':'LOCKS_VERIFIED'}
