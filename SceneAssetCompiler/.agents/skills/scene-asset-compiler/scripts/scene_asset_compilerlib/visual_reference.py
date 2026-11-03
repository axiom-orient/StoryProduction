"""SAC-owned visual reference candidates, explicit Human review, and derived memory.

No pixels, model calls, semantic facts or final releases are generated here.
The immutable pack is the index; media remains in the declared local workspace.
"""
from __future__ import annotations
from datetime import datetime, timezone
from pathlib import Path
import json
import os
import tempfile
from typing import Any
from .asset_release_contracts import (
    AssetReleaseError,
    PROJECTION_RECEIPT_RELATIVE_PATH,
    canonical_hash,
    _prefixed_hash,
    _require_keys,
    _relative_path,
)
from .asset_release_files import (
    _read_json,
    _safe_destination,
    _safe_file,
    _strict_root,
    _verify_signature,
)
from .asset_release_projection_contracts import (
    _validate_cinematic_sources,
    _validate_projection,
    _validate_scene_graph,
)
from .util import sha256_bytes

MAX_REFERENCE_BYTES = 128 * 1024 * 1024
VIEWS = {'front','threeQuarter','profile','fullBody','expression','establishing','detail'}
MIMES = {'image/png','image/jpeg','image/webp'}

def _check(ok: bool, reason: str) -> None:
    if not ok: raise AssetReleaseError('VISUAL_REFERENCE:' + reason)

def _keys(value, names: str, label: str):
    keys=set(names.split()); return _require_keys(value,keys,keys,label)

def _seal(value: dict) -> dict:
    return {**value,'contentHash':canonical_hash({k:v for k,v in value.items() if k!='contentHash'})}

def _list(value, label: str) -> list:
    _check(isinstance(value,list),label+':array'); return value

def _strings(value, label: str) -> list[str]:
    items=_list(value,label)
    for item in items: _check(isinstance(item,str) and bool(item.strip()) and len(item)<=2048,label+':string')
    _check(len(items)==len(set(items)),label+':duplicate');return items

def _projection(root: Path) -> tuple[dict,dict]:
    projection,_,_=_read_json(root,PROJECTION_RECEIPT_RELATIVE_PATH,'projection')
    _validate_projection(projection)
    graph,_,_=_read_json(root,projection['sceneGraphPath'],'sceneGraph')
    _validate_scene_graph(graph,projection['sceneGraphHash'],projection)
    _validate_cinematic_sources(projection,graph)
    return projection,graph

def _image(root: Path, value: dict) -> dict:
    _keys(value,'path sha256 mime','image')
    _prefixed_hash(value['sha256'],'image.sha256');_relative_path(value['path'],'image.path')
    _check(value['mime'] in MIMES,'image.mime')
    path=_safe_file(root,value['path'],'image.path')
    with path.open('rb') as f: raw=f.read(MAX_REFERENCE_BYTES+1)
    _check(0<len(raw)<=MAX_REFERENCE_BYTES,'image.size')
    _check('sha256:'+sha256_bytes(raw)==value['sha256'],'image.hash')
    _verify_signature(raw,value['path'],value['mime'],'visualReference',runtime=True)
    return value

def validate_pack(root: Path, value: dict) -> tuple[dict,dict]:
    root=_strict_root(root);projection,graph=_projection(root)
    _keys(value,'schemaVersion kind sceneGraphHash sourceNarrativeSha256 directorPlanSha256 continuityBibleSha256 entityReferences storyboardFrames contentHash','pack')
    _check(type(value['schemaVersion']) is int and value['schemaVersion']==1 and value['kind']=='VisualReferencePack','pack.version')
    _check(value['contentHash']==_seal(value)['contentHash'],'pack.hash')
    for key,expected in [('sceneGraphHash',projection['sceneGraphHash']),('sourceNarrativeSha256',graph['sourceNarrativeSha256']),('directorPlanSha256',graph['directorPlanSha256']),('continuityBibleSha256',graph['continuityBibleSha256'])]:
        _check(value[key]==expected,'stale.'+key)
    known={e['id']:role for group,role in [('characters','CHARACTER'),('environments','ENVIRONMENT'),('props','PROP')] for e in graph[group]}
    refs={}
    for entry in _list(value['entityReferences'],'entityReferences'):
        _keys(entry,'entityId role images invariants','entityReference')
        eid=entry['entityId'];_check(isinstance(eid,str) and eid in known and eid not in refs,'entity.id')
        _check(entry['role']==known[eid],'entity.role');_strings(entry['invariants'],'invariants')
        images=_list(entry['images'],'images');_check(bool(images),'images.empty');views=set()
        for image in images:
            _keys(image,'view artifact','reference.image');view=image['view']
            _check(isinstance(view,str) and view in VIEWS and view not in views,'image.view');views.add(view);_image(root,image['artifact'])
        refs[eid]=entry
    shots={s['id']:s for s in graph['shots']};frames={}
    for frame in _list(value['storyboardFrames'],'storyboardFrames'):
        _keys(frame,'shotId artifact referenceEntityIds','storyboardFrame')
        sid=frame['shotId'];_check(isinstance(sid,str) and sid in shots and sid not in frames,'frame.id')
        _image(root,frame['artifact']);ids=_strings(frame['referenceEntityIds'],'frame.references')
        shot=shots[sid];allowed={shot['environmentId'],*shot['characterIds'],*shot['propIds']}
        _check(set(ids)<=set(refs) and set(ids)<=allowed,'frame.reference_binding');frames[sid]=frame
    for shot in graph['shots']:
        policy=shot['referencePolicy'];required=set(shot['propIds'])
        if policy['requiresCharacterReference']:required.update(shot['characterIds'])
        if policy['requiresEnvironmentReference']:required.add(shot['environmentId'])
        _check(required<=set(refs),'required.entity_reference:'+shot['id'])
        if policy['requiresKeyframe']:
            _check(shot['id'] in frames,'required.keyframe:'+shot['id'])
        if shot['id'] in frames:_check(required<=set(frames[shot['id']]['referenceEntityIds']),'required.keyframe_refs:'+shot['id'])
    return projection,graph

def _immutable(root:Path,relative:str,value:dict) -> str:
    target=_safe_destination(root,relative);raw=(json.dumps(value,ensure_ascii=False,indent=2)+'\n').encode()
    if target.exists():
        _check(_safe_file(root,relative,'output').read_bytes()==raw,'immutable.conflict');return 'sha256:'+sha256_bytes(raw)
    target.parent.mkdir(parents=True,exist_ok=True);target=_safe_destination(root,relative)
    fd,name=tempfile.mkstemp(prefix='.reference-',dir=target.parent)
    try:
        with os.fdopen(fd,'wb') as f:f.write(raw);f.flush();os.fsync(f.fileno())
        try:os.link(name,target,follow_symlinks=False)
        except FileExistsError:_check(_safe_file(root,relative,'output').read_bytes()==raw,'immutable.race')
        directory=os.open(target.parent,os.O_RDONLY)
        try:os.fsync(directory)
        finally:os.close(directory)
    finally:
        if os.path.exists(name):os.unlink(name)
    return 'sha256:'+sha256_bytes(raw)

def prepare(root:Path,draft_path:str,output:str)->dict:
    root=_strict_root(root);projection,graph=_projection(root)
    draft,_,_=_read_json(root,draft_path,'visualReferenceDraft')
    _keys(draft,'entityReferences storyboardFrames','draft')
    pack=_seal({'schemaVersion':1,'kind':'VisualReferencePack','sceneGraphHash':projection['sceneGraphHash'],
        'sourceNarrativeSha256':graph['sourceNarrativeSha256'],'directorPlanSha256':graph['directorPlanSha256'],
        'continuityBibleSha256':graph['continuityBibleSha256'],**draft})
    validate_pack(root,pack);raw_hash=_immutable(root,output,pack)
    return {'status':'CANDIDATE','path':output,'sha256':raw_hash,'packHash':pack['contentHash']}

def _validate_approval(value:dict,pack:dict)->None:
    _keys(value,'schemaVersion kind decision packHash reviewer reviewedAt contentHash','approval')
    _check(type(value['schemaVersion']) is int and value['schemaVersion']==1 and value['kind']=='VisualReferenceApproval' and value['decision']=='APPROVED','approval.kind')
    _check(value['packHash']==pack['contentHash'] and value['contentHash']==_seal(value)['contentHash'],'approval.stale')
    _check(isinstance(value['reviewer'],str) and bool(value['reviewer'].strip()),'approval.reviewer')
    try:_check(datetime.fromisoformat(value['reviewedAt'].replace('Z','+00:00')).tzinfo is not None,'approval.timezone')
    except (ValueError,TypeError):raise AssetReleaseError('VISUAL_REFERENCE:approval.time')

def _inspect_pack_snapshot(
    root: Path,
    pack_path: str,
    approval_path: str | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    """Validate one parsed pack snapshot and, when supplied, its approval."""
    root=_strict_root(root)
    pack,_,_=_read_json(root,pack_path,'visualReferencePack')
    validate_pack(root,pack)
    result={'status':'VALID_CANDIDATE','packHash':pack['contentHash'],'entities':len(pack['entityReferences']),'keyframes':len(pack['storyboardFrames'])}
    if approval_path:
        approval,_,_=_read_json(root,approval_path,'visualReferenceApproval');_validate_approval(approval,pack)
        result.update(status='VALID_APPROVED',approvalHash=approval['contentHash'])
    return pack,result

def approve(root:Path,pack_path:str,reviewer:str,expected_hash:str,output:str)->dict:
    root=_strict_root(root);pack,_,_=_read_json(root,pack_path,'visualReferencePack');validate_pack(root,pack)
    _check(expected_hash==pack['contentHash'],'review.expected_hash');_check(bool(reviewer.strip()),'review.reviewer')
    approval=_seal({'schemaVersion':1,'kind':'VisualReferenceApproval','decision':'APPROVED','packHash':pack['contentHash'],
        'reviewer':reviewer.strip(),'reviewedAt':datetime.now(timezone.utc).isoformat()})
    if _safe_destination(root,output).exists():
        existing,_,_=_read_json(root,output,'existingApproval');_validate_approval(existing,pack)
        _check(existing['reviewer']==reviewer.strip(),'review.conflict');approval=existing
    _validate_approval(approval,pack);_immutable(root,output,approval)
    return {'status':'APPROVED','path':output,'packHash':pack['contentHash'],'approvalHash':approval['contentHash']}

def inspect(root:Path,pack_path:str,approval_path:str|None=None)->dict:
    _,result=_inspect_pack_snapshot(root,pack_path,approval_path)
    return result

def memory_index(root:Path,pack_path:str,approval_path:str)->dict:
    pack,result=_inspect_pack_snapshot(root,pack_path,approval_path)
    entries=[{'subjectRef':entry['entityId'],'view':image['view'],'artifact':image['artifact']} for entry in pack['entityReferences'] for image in entry['images']]
    entries.extend({'subjectRef':frame['shotId'],'view':'keyframe','artifact':frame['artifact']} for frame in pack['storyboardFrames'])
    return {'schemaVersion':1,'kind':'DerivedVisualMemoryIndex','packHash':result['packHash'],'approvalHash':result['approvalHash'],'entries':entries}
