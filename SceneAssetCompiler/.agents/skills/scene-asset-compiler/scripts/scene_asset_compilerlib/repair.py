from __future__ import annotations
from pathlib import Path
from datetime import datetime, timezone
from .util import load_json, atomic_json, sha256_file
from .contract_primitives import ContractError
from .review_contracts import validate_visual_review
from .workspace import begin_attempt

CODE_INSTRUCTIONS={
 'REQUIRED_LAYER_MISSING':'Add only the missing required semantic layer(s) with the exact machine ID(s). Preserve every existing valid layer.',
 'DUPLICATE_LAYER_ID':'Resolve only duplicate machine layer IDs without redesigning the image.',
 'ALPHA_MISSING':'Correct only background transparency/alpha. Preserve subject pixels, identity, style, pose, composition, and lighting.',
 'ALPHA_MARGIN_TOO_SMALL':'Apply a targeted canvas-fit correction only: scale down and/or recenter the approved pixels, or extend the transparent canvas, until every edge has the required margin. Preserve approved pixels, identity, pose, style, composition, and lighting; never creatively reroll.',
 'ALPHA_FOREGROUND_CLIPPED':'Apply a targeted canvas-fit correction only: scale down and/or recenter the approved pixels, or extend the transparent canvas, until no foreground touches any edge and every edge has the required margin. Preserve approved pixels, identity, pose, style, composition, and lighting; never creatively reroll.',
 'PSD_INVALID':'Regenerate a valid layered PSD matching the same approved visual result and exact required layer IDs.',
 'PSD_FALLBACK':'If PSD capability is now available, produce the requested layered PSD from the approved visual; otherwise keep the raster candidate and do not claim PSD compliance.',
 'PREFERRED_OUTPUT_MISSING':'Packaging-only retry: reuse approved locked pixels and layout and request the missing PSD. Do not redraw, restyle, relight, recompose, or assemble PSD locally; if it is missing again, stop and retain previews/evidence without claiming PSD unsupported.',
 'IDENTITY_DRIFT':'Correct only the failed identity invariant(s) using the approved identity anchor.',
 'STYLE_DRIFT':'Correct only the failed visual-language traits. Preserve identity, pose, composition, lighting intent, and valid layer structure.',
 'NARRATIVE_MISMATCH':'Correct only the depicted action/state/fact that contradicts the narrative authority. Do not redesign unrelated visual choices.',
 'COMPOSITION_FAIL':'Correct only framing, placement, depth, or focal hierarchy that failed. Preserve identity, style, lighting intent, and scene facts.',
 'LIGHTING_COLOR_FAIL':'Correct only motivated light, contrast, palette, or value hierarchy. Preserve identity, pose, composition, materials, and structure.',
 'MATERIAL_FAIL':'Correct only the failed material response, texture, or surface cue. Preserve identity, composition, lighting intent, palette, and abstraction level.',
 'PHOTOREALISM_FAIL':'Correct only the reported physical/camera/anatomy/material naturalism defect. Preserve all passing identity, narrative, composition, lighting, color, and structure decisions.',
 'STRUCTURE_FAIL':'Separate only incorrectly merged or contaminated semantic parts. Preserve appearance/alignment of passing parts.',
 'POSE_SEAM':'Repair only the joint seam by extending internal overlap beneath neighboring parts without changing external silhouette.',
 'USABILITY_FAIL':'Correct only the readability, silhouette, crop/padding, or material legibility issue reported by review.'}

VISUAL_MAPPING={
 'identity':'IDENTITY_DRIFT','style':'STYLE_DRIFT','narrative':'NARRATIVE_MISMATCH','composition':'COMPOSITION_FAIL',
 'lighting_color':'LIGHTING_COLOR_FAIL','material':'MATERIAL_FAIL','photorealism':'PHOTOREALISM_FAIL','structure':'STRUCTURE_FAIL','articulation':'POSE_SEAM','usability':'USABILITY_FAIL',
 'genre_fidelity':'STYLE_DRIFT','layout_fidelity':'COMPOSITION_FAIL'}


def _packaging_missing(task, mech):
    if task.get('output',{}).get('preferred')!='PSD':
        return False
    codes={f.get('code') for f in mech.get('findings',[]) if f.get('severity')=='FAIL'}
    if 'PREFERRED_OUTPUT_MISSING' not in codes:
        return False
    if not codes <= {'PREFERRED_OUTPUT_MISSING','UNEXPECTED_FORMAT','NO_PRIMARY_OUTPUT','PSD_PACKAGING_PRIMARY_PSD_COUNT_INVALID'}:
        return False
    return True


def _select_raster_substrate(attempt_dir, result):
    candidates=sorted(
        (
            item for item in result.get('artifacts',[])
            if Path(item.get('path','')).suffix.lower() in {'.png','.webp','.jpg','.jpeg'}
            and item.get('role','PRIMARY') in {'PRIMARY','PREVIEW'}
        ),
        key=lambda item:(0 if item.get('role','PRIMARY')=='PRIMARY' else 1,item.get('path',''),item.get('sha256','')),
    )
    for item in candidates:
        path=attempt_dir/item['path']
        if path.is_file() and sha256_file(path)==item.get('sha256'):
            return item
    return None


def _require_approved_substrate(task, visual, task_id, attempt_no, attempt_dir, substrate):
    if not visual or visual.get('status')!='APPROVED':
        raise ValueError('PSD_PACKAGING_SUBSTRATE_APPROVAL_REQUIRED')
    try:
        validate_visual_review(
            visual,
            task_id,
            attempt_no,
            task.get('prompt_profile','PROJECT_STYLE'),
            task.get('render_contract'),
            attempt_dir,
        )
    except ContractError as e:
        raise ValueError(f'PSD_PACKAGING_SUBSTRATE_APPROVAL_INVALID:{e}') from e
    if visual.get('artifact_sha256')!=substrate['sha256']:
        raise ValueError('PSD_PACKAGING_SUBSTRATE_HASH_MISMATCH')
    layout=(task.get('render_contract') or {}).get('wireframe_layout')
    if layout:
        evidence=visual.get('layout_evidence')
        if not evidence or evidence.get('output_sha256')!=substrate['sha256']:
            raise ValueError('PSD_PACKAGING_LAYOUT_SUBSTRATE_MISMATCH')


def _packaging_retry_count(task_dir):
    count=0
    for attempt_dir in sorted((task_dir/'attempts').glob('[0-9][0-9][0-9][0-9]')):
        attempt_path=attempt_dir/'attempt.json'
        if not attempt_path.is_file():
            continue
        repair=load_json(attempt_path).get('repair') or {}
        if repair.get('strategy')=='PSD_PACKAGING_RETRY':
            count += 1
    return count


def _stop_packaging_retry(task_dir, attempt_dir, task_id, attempt_no, result, retry_count):
    retained_attempts=[]
    for retained_dir in sorted((task_dir/'attempts').glob('[0-9][0-9][0-9][0-9]')):
        result_path=retained_dir/'result.json'
        if not result_path.is_file():
            continue
        retained_result=load_json(result_path)
        evidence=[]
        for name in ('execution-receipt.json','mechanical-review.json','visual-review.json','packaging-preservation.json'):
            path=retained_dir/name
            if path.is_file():
                evidence.append({'path':str(Path('attempts')/retained_dir.name/name),'sha256':sha256_file(path)})
        retained_attempts.append({
            'attempt_no':int(retained_dir.name),
            'artifacts':[
                {'path':artifact['path'],'sha256':artifact['sha256'],'role':artifact.get('role','PRIMARY')}
                for artifact in retained_result.get('artifacts',[])
            ],
            'evidence':evidence,
        })
    state={
        'schema_version':1,
        'task_id':task_id,
        'attempt_no':attempt_no,
        'created_at':datetime.now(timezone.utc).isoformat(),
        'status':'STOPPED',
        'reason':'PSD_PACKAGING_RETRY_EXHAUSTED',
        'retry_count':retry_count,
        'max_retries':1,
        'retained_artifacts':[
            {'path':artifact['path'],'sha256':artifact['sha256'],'role':artifact.get('role','PRIMARY')}
            for artifact in result.get('artifacts',[])
        ],
        'retained_attempts':retained_attempts,
    }
    atomic_json(attempt_dir/'packaging-retry.json',state)
    raise ValueError('PSD_PACKAGING_RETRY_EXHAUSTED')

def make_repair(run_dir:Path,task_id:str,attempt_no:int):
    ad=run_dir/'tasks'/task_id/'attempts'/f'{attempt_no:04d}'; task_dir=run_dir/'tasks'/task_id; task=load_json(task_dir/'task.json'); mech=load_json(ad/'mechanical-review.json'); vis=load_json(ad/'visual-review.json') if (ad/'visual-review.json').exists() else None
    codes=[]; details=[]
    for f in mech['findings']:
        if f['severity']=='FAIL': codes.append(f['code']); details.append(f.get('message',''))
    if vis and vis['status']=='REJECTED':
        for k,v in vis['checks'].items():
            if v=='FAIL': codes.append(VISUAL_MAPPING[k]); details.extend(vis.get('evidence',{}).get(k,[]))
    if not codes: raise ValueError('NO_REPAIRABLE_FAILURE')
    result=load_json(ad/'result.json'); primary=[a for a in result['artifacts'] if a.get('role','PRIMARY')=='PRIMARY']; target=primary[0] if primary else None

    if _packaging_missing(task,mech):
        retry_count=_packaging_retry_count(task_dir)
        if retry_count>=1:
            _stop_packaging_retry(task_dir,ad,task_id,attempt_no,result,retry_count+1)
        substrate=_select_raster_substrate(ad,result)
        if substrate is None:
            raise ValueError('PSD_PACKAGING_RASTER_SUBSTRATE_REQUIRED')
        _require_approved_substrate(task,vis,task_id,attempt_no,ad,substrate)
        edit_target={**substrate,'run_path':str(Path('tasks')/task_id/'attempts'/f'{attempt_no:04d}'/substrate['path'])}
        repair={
            'schema_version':1,
            'source_attempt':attempt_no,
            'failure_codes':['PREFERRED_OUTPUT_MISSING'],
            'strategy':'PSD_PACKAGING_RETRY',
            'repair_scope':'PSD_PACKAGING',
            'instruction':CODE_INSTRUCTIONS['PREFERRED_OUTPUT_MISSING'],
            'preserve':['approved locked pixels','approved locked layout','identity','registration','z-order','color and alpha'],
            'edit_target':edit_target,
            'packaging_retry':{
                'mode':'PSD_ONLY',
                'trigger':'PREFERRED_OUTPUT_MISSING',
                'retry_index':retry_count+1,
                'max_retries':1,
                'substrate_sha256':substrate['sha256'],
                'preserve_locked_pixels':True,
                'preserve_locked_layout':True,
                'creative_changes':'FORBID',
            },
        }
        return begin_attempt(run_dir,task_id,repair)

    unique_codes=list(dict.fromkeys(codes))
    margin_codes={'ALPHA_MARGIN_TOO_SMALL','ALPHA_FOREGROUND_CLIPPED'}
    if margin_codes.intersection(unique_codes) and not set(unique_codes) <= margin_codes:
        raise ValueError('CANVAS_FIT_MIXED_FAILURE_REQUIRES_SEPARATION')
    instructions=[CODE_INSTRUCTIONS.get(c,f'Correct only failure {c}.') for c in unique_codes]
    
    edit_target=None
    if target:
        edit_target={**target,'run_path':str(Path('tasks')/task_id/'attempts'/f'{attempt_no:04d}'/target['path'])}
    observed=' | '.join(str(x) for x in details if str(x).strip())
    instruction=' '.join(instructions)+(f' Observed failures: {observed}' if observed else '')
    repair={'schema_version':1,'source_attempt':attempt_no,'failure_codes':unique_codes,'strategy':'TARGETED_EDIT','instruction':instruction,'preserve':['all unreported identity invariants','all passing visual-language traits','all passing narrative facts','all passing composition/light/color/material decisions','all passing semantic layers','all approved explicit references'],'edit_target':edit_target}
    if any(code in unique_codes for code in ('ALPHA_MARGIN_TOO_SMALL','ALPHA_FOREGROUND_CLIPPED')):
        repair['repair_scope']='CANVAS_FIT'
    return begin_attempt(run_dir,task_id,repair)
