"""Run-request, visual-authority, and generation-plan contract validation."""
from __future__ import annotations

import math
import re

from .psd_profile import validate_profile as validate_psd_profile
from .contract_primitives import (
    ContractError,
    PROMPT_PROFILES,
    REUSABLE_TRANSPARENT_KINDS,
    _SHA256_RE,
    reject_unknown,
    req,
    validate_render_contract,
    validate_style_reference_separation,
)

_SOURCE_KINDS = {'SMC_NARRATIVE_SCENE_PROJECTION','SCREENPLAY','SYNOPSIS','FREEFORM'}
_SOURCE_FIELDS = {'kind','path','project_root','sha256'}
def output_canvas_dimensions(output, where='output'):
    value=output.get('size') if isinstance(output,dict) else None
    if not isinstance(value,str) or not re.fullmatch(r'[1-9][0-9]*x[1-9][0-9]*',value):
        raise ContractError(f'FORMAT:{where}.size')
    width,height=(int(x) for x in value.split('x'))
    return width,height


def required_transparent_margin_px(task, where='task'):
    """Return the effective positive per-edge margin for reusable transparent assets."""

    output=task.get('output') if isinstance(task,dict) else None
    if not isinstance(output,dict):
        raise ContractError(f'TYPE:{where}.output')
    if task.get('kind') not in REUSABLE_TRANSPARENT_KINDS or output.get('transparent_background') is not True:
        return None
    explicit=output.get('min_transparent_margin_px')
    if explicit is not None:
        if isinstance(explicit,bool) or not isinstance(explicit,int):
            raise ContractError(f'TYPE:{where}.output.min_transparent_margin_px:TRANSPARENT_MARGIN')
        if explicit <= 0:
            raise ContractError(f'RANGE:{where}.output.min_transparent_margin_px:TRANSPARENT_MARGIN_POSITIVE')
        return explicit
    width,height=output_canvas_dimensions(output,where)
    return max(1,math.ceil(min(width,height)*0.02))

def render_contract_for_task(task):
    profile=task.get('prompt_profile','PROJECT_STYLE')
    supplied=task.get('render_contract')
    if supplied is None:
        role='SCENE'
        if profile=='PHOTOREAL_CHARACTER_ANCHOR': role='CHARACTER'
        elif profile=='PHOTOREAL_LAYER_ASSET': role='OBJECT'
        return {'schema_version':1,'capture_profile':'PHOTOREAL' if profile.startswith('PHOTOREAL_') else 'NONE','genre_profile':'PHOTOREAL' if profile.startswith('PHOTOREAL_') else 'PROJECT_STYLE','semantic_role':role,'_explicit':False}
    validate_render_contract(supplied)
    out=dict(supplied); out.setdefault('capture_profile','NONE'); out.setdefault('genre_profile','PROJECT_STYLE'); out['_explicit']=True
    if profile.startswith('PHOTOREAL_') and out['capture_profile']!='PHOTOREAL':
        raise ContractError(f'RENDER_CONTRACT_CONFLICT:{task.get("task_id","task")}:LEGACY_PROMPT_PROFILE')
    return out

def validate_run_request(o):
    reject_unknown(o,['schema_version','project_id','goal','source','outputs','runtime_target','review_policy','user_direction','reference_paths'],'run_request')
    if req(o,'schema_version')!=1: raise ContractError('VERSION:run_request')
    req(o,'project_id',str); req(o,'goal',str); source=req(o,'source',dict); req(o,'outputs',list); rp=req(o,'review_policy',dict)
    if set(source)-_SOURCE_FIELDS: raise ContractError('UNKNOWN_FIELDS:source')
    if 'kind' not in source: raise ContractError('MISSING:source.kind')
    kind=source['kind']
    if not isinstance(kind,str): raise ContractError('TYPE:source.kind')
    if kind not in _SOURCE_KINDS: raise ContractError('SOURCE_KIND')
    for field in ['path','project_root','sha256']:
        if field in source and not isinstance(source[field],str): raise ContractError(f'TYPE:source.{field}')
    if 'sha256' in source and not _SHA256_RE.fullmatch(source['sha256']): raise ContractError('FORMAT:source.sha256')
    if kind=='SMC_NARRATIVE_SCENE_PROJECTION':
        for field in ['path','project_root','sha256']:
            if field not in source: raise ContractError(f'MISSING:source.{field}')
            value=source[field]
            if not isinstance(value,str): raise ContractError(f'TYPE:source.{field}')
            if field in ['path','project_root'] and not value: raise ContractError(f'MISSING:source.{field}')
    if rp.get('codex_visual_review') is not True: raise ContractError('POLICY:codex_visual_review must be true')

def validate_visual_authority(o):
    reject_unknown(o,['schema_version','authority_id','style','characters','props','world','color_script','protected_visual_facts'],'visual_authority')
    if req(o,'schema_version')!=2: raise ContractError('VERSION:visual_authority')
    req(o,'authority_id',str); style=req(o,'style',dict); req(o,'characters',list); req(o,'props',list); req(o,'world',list); req(o,'color_script',list); req(o,'protected_visual_facts',list)
    for k in ['style_id','render_family','visual_language','forbidden_drift']: req(style,k)
    vl=req(style,'visual_language',dict)
    for k in ['shape_language','line_edge','shading','texture','palette','lighting_principles','camera_principles','material_principles','detail_density']: req(vl,k,list)
    ids=set()
    for c in o['characters']:
        cid=req(c,'character_id',str)
        if cid in ids: raise ContractError(f'DUPLICATE_CHARACTER:{cid}')
        ids.add(cid); inv=req(c,'identity_invariants',list)
        if not inv: raise ContractError(f'EMPTY_IDENTITY:{cid}')
        for k in ['silhouette','face_anchors','palette','costume_rules','allowed_variation','scene_variants']: req(c,k,list)
        req(c,'anchor_required',bool)
    pids=set()
    for p in o['props']:
        pid=req(p,'prop_id',str)
        if pid in pids: raise ContractError(f'DUPLICATE_PROP:{pid}')
        pids.add(pid); req(p,'identity_invariants',list); req(p,'allowed_variation',list); req(p,'anchor_required',bool)
    bids=set()
    for b in o['color_script']:
        bid=req(b,'beat_id',str)
        if bid in bids: raise ContractError(f'DUPLICATE_COLOR_BEAT:{bid}')
        bids.add(bid)
        for k in ['narrative_refs','palette','lighting']: req(b,k,list)
        for k in ['function','value_structure','transition']: req(b,k,str)

def _validate_visual_brief(v, tid):
    for k in ['narrative_function','emotional_intent','first_read','focal_hierarchy','composition','lighting','color','materials','atmosphere','environment_rules','continuity','avoid']: req(v,k)
    if not v['focal_hierarchy'] or len(v['focal_hierarchy'])>3: raise ContractError(f'FOCAL_HIERARCHY:{tid}')
    for k in ['framing','camera_angle','perspective','lens_behavior','focus_depth','subject_placement','depth_layers','negative_space']: req(v['composition'],k)
    for k in ['motivation','direction','quality','contrast','color_relationship']: req(v['lighting'],k)
    for k in ['palette_role','dominant','accent','value_structure']: req(v['color'],k)

def validate_generation_plan(o):
    reject_unknown(o,['schema_version','plan_id','authority_id','tasks'],'generation_plan')
    if req(o,'schema_version')!=2: raise ContractError('VERSION:generation_plan')
    req(o,'plan_id',str); req(o,'authority_id',str); tasks=req(o,'tasks',list)
    if not tasks: raise ContractError('EMPTY_PLAN')
    ids=set()
    for t in tasks:
        for k in ['task_id','asset_id','kind','required','depends_on','split_profile','authority_refs','content','visual_brief','references','structure','output','acceptance']: req(t,k)
        tid=t['task_id']
        if tid in ids: raise ContractError(f'DUPLICATE_TASK:{tid}')
        ids.add(tid)
        if t['split_profile'] not in ['L0','L1','L2','L3']: raise ContractError(f'SPLIT_PROFILE:{tid}')
        if t.get('prompt_profile','PROJECT_STYLE') not in PROMPT_PROFILES: raise ContractError(f'PROMPT_PROFILE:{tid}')
        render_contract_for_task(t)
        required_transparent_margin_px(t, tid)
        if not t['structure'].get('required_layer_ids'): raise ContractError(f'NO_REQUIRED_LAYERS:{tid}')
        _validate_visual_brief(t['visual_brief'],tid)
        ar=t['authority_refs']
        for k in ['character_ids','prop_ids','color_script_beat_ids']: req(ar,k,list)
        output=t['output']
        if output.get('preferred')=='PSD':
            pp=output.get('psd_profile')
            if not isinstance(pp,dict): raise ContractError(f'PSD_PROFILE_REQUIRED:{tid}')
            try: validate_psd_profile(pp)
            except Exception as e: raise ContractError(f'PSD_PROFILE:{tid}:{e}')
        acc=t['acceptance']
        for k in ['identity','style','narrative','composition','lighting_color','material','structure','articulation','usability']: req(acc,k,list)
        for ref in t['references']:
            reject_unknown(ref,['source_kind','role','source','preserve','ignore','sha256'],'reference')
            if req(ref,'source_kind',str) not in ['FILE','TASK_OUTPUT']: raise ContractError(f'REFERENCE_SOURCE_KIND:{tid}')
            if req(ref,'role',str) not in ['IDENTITY','STYLE','POSE','COSTUME','PROP','COMPOSITION','ENVIRONMENT','MATERIAL']: raise ContractError(f'REFERENCE_ROLE:{tid}')
            req(ref,'source',str); req(ref,'preserve',list); req(ref,'ignore',list)
        validate_style_reference_separation(t['references'],f'{tid}.references')
    for t in tasks:
        for dep in t['depends_on']:
            if dep not in ids: raise ContractError(f'UNKNOWN_DEP:{t["task_id"]}:{dep}')
        if t['task_id'] in t['depends_on']: raise ContractError(f'SELF_DEP:{t["task_id"]}')
    deps={t['task_id']:set(t['depends_on']) for t in tasks}; done=set()
    while len(done)<len(deps):
        ready=[k for k,v in deps.items() if k not in done and v <= done]
        if not ready: raise ContractError('DEPENDENCY_CYCLE')
        done.update(ready)
