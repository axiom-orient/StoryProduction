from __future__ import annotations
from pathlib import Path
from datetime import datetime, timezone
import os, shutil, tempfile, uuid
from .util import load_json, load_json_bytes, atomic_json, atomic_text, sha256_file
from .request_contracts import validate_run_request, validate_visual_authority, validate_generation_plan
from .asset_release_projection_import import import_nvc_released_scene_projection
from .prompts import compile_prompt, compile_generation_capsule


def init_run(request_path:Path, run_dir:Path):
    request_path=Path(request_path)
    request_bytes=request_path.read_bytes()
    req=load_json_bytes(request_bytes); validate_run_request(req)
    run_dir=Path(run_dir).absolute()
    if os.path.lexists(run_dir): raise ValueError('RUN_DEST_EXISTS')
    run_dir.parent.mkdir(parents=True,exist_ok=True)
    staging=Path(tempfile.mkdtemp(prefix=f'.{run_dir.name}.staging-',dir=str(run_dir.parent)))
    try:
        for d in ['inputs','authority','plan','tasks','publish']: (staging/d).mkdir()
        (staging/'request.json').write_bytes(request_bytes)
        receipt={'schema_version':1,'run_id':f'RUN-{uuid.uuid4()}','project_id':req['project_id'],'created_at':datetime.now(timezone.utc).isoformat(),'request_sha256':sha256_file(staging/'request.json')}
        atomic_json(staging/'run.json',receipt)
        src=req['source']
        if src['kind']=='SMC_NARRATIVE_SCENE_PROJECTION':
            source_root=Path(src['project_root'])
            source_path=Path(src['path'])
            if not source_path.is_absolute(): source_path=source_root/source_path
            import_nvc_released_scene_projection(source_path,source_root,staging,src['sha256'])
        # Cooperative single-writer boundary: a concurrent creator after this check is unknown.
        if os.path.lexists(run_dir): raise ValueError('RUN_DEST_EXISTS')
        os.rename(staging,run_dir)
        staging=None
        return receipt
    finally:
        if staging is not None: shutil.rmtree(staging)


def validate_authority(run_dir:Path):
    a=load_json(run_dir/'authority'/'visual-authority.json'); validate_visual_authority(a); return a


def _mime_for(path:Path):
    return {'.png':'image/png','.webp':'image/webp','.jpg':'image/jpeg','.jpeg':'image/jpeg','.psd':'image/vnd.adobe.photoshop'}.get(path.suffix.lower(),'application/octet-stream')


def compile_plan(run_dir:Path):
    authority=validate_authority(run_dir)
    plan=load_json(run_dir/'plan'/'generation-plan.json'); validate_generation_plan(plan)
    if plan['authority_id']!=authority['authority_id']: raise ValueError('AUTHORITY_PLAN_MISMATCH')
    task_by_id={t['task_id']:t for t in plan['tasks']}; task_ids=set(task_by_id)
    chars={c['character_id']:c for c in authority['characters']}
    props={p['prop_id']:p for p in authority['props']}
    beats={b['beat_id']:b for b in authority['color_script']}
    char_anchors={cid:[t['task_id'] for t in plan['tasks'] if t['kind']=='CHARACTER_ANCHOR' and cid in t['authority_refs']['character_ids']] for cid in chars}
    prop_anchors={pid:[t['task_id'] for t in plan['tasks'] if t['kind']=='PROP_ANCHOR' and pid in t['authority_refs']['prop_ids']] for pid in props}

    # Cross-contract gates that require both VisualAuthority and GenerationPlan.
    for task in plan['tasks']:
        tid=task['task_id']; ar=task['authority_refs']; deps=set(task['depends_on'])
        missing_chars=[x for x in ar['character_ids'] if x not in chars]
        missing_props=[x for x in ar['prop_ids'] if x not in props]
        missing_beats=[x for x in ar['color_script_beat_ids'] if x not in beats]
        if missing_chars: raise ValueError(f'UNKNOWN_AUTHORITY_CHARACTER:{tid}:{missing_chars}')
        if missing_props: raise ValueError(f'UNKNOWN_AUTHORITY_PROP:{tid}:{missing_props}')
        if missing_beats: raise ValueError(f'UNKNOWN_COLOR_BEAT:{tid}:{missing_beats}')

        output_refs=[r for r in task['references'] if r['source_kind']=='TASK_OUTPUT']
        for r in output_refs:
            if r['source'] not in task_ids: raise ValueError(f'UNKNOWN_REFERENCE_TASK:{tid}:{r["source"]}')
            if r['source'] not in deps: raise ValueError(f'REFERENCE_TASK_MUST_BE_DEPENDENCY:{tid}:{r["source"]}')

        if task['kind']!='CHARACTER_ANCHOR':
            for cid in ar['character_ids']:
                if chars[cid]['anchor_required']:
                    valid=set(char_anchors[cid])
                    ok=any(r['role']=='IDENTITY' and r['source'] in valid for r in output_refs)
                    if not ok: raise ValueError(f'CHARACTER_ANCHOR_REFERENCE_REQUIRED:{tid}:{cid}')
        if task['kind']!='PROP_ANCHOR':
            for pid in ar['prop_ids']:
                if props[pid]['anchor_required']:
                    valid=set(prop_anchors[pid])
                    ok=any(r['role']=='PROP' and r['source'] in valid for r in output_refs)
                    if not ok: raise ValueError(f'PROP_ANCHOR_REFERENCE_REQUIRED:{tid}:{pid}')

        nrefs=set(task['content'].get('narrative_refs',[]))
        relevant=[b['beat_id'] for b in authority['color_script'] if nrefs and set(b['narrative_refs']) & nrefs]
        if task['kind'] in ['KEY_VISUAL','MOTION_SOURCE'] and relevant and not (set(ar['color_script_beat_ids']) & set(relevant)):
            raise ValueError(f'COLOR_SCRIPT_BEAT_REQUIRED:{tid}:{relevant}')
        if nrefs:
            for bid in ar['color_script_beat_ids']:
                b=beats[bid]
                if b['narrative_refs'] and not (set(b['narrative_refs']) & nrefs):
                    raise ValueError(f'COLOR_SCRIPT_BEAT_MISMATCH:{tid}:{bid}')

    for folder in (run_dir/'tasks', run_dir/'plan'):
        if folder.is_symlink() or not folder.is_dir(): raise ValueError('UNSAFE_PLAN_DIRECTORY')
    for task in plan['tasks']:
        if os.path.lexists(run_dir/'tasks'/task['task_id']): raise ValueError(f'TASK_ALREADY_EXISTS:{task["task_id"]}')
    if os.path.lexists(run_dir/'plan'/'compiled.json'): raise ValueError('PLAN_ALREADY_COMPILED')
    staging=Path(tempfile.mkdtemp(prefix='.compile-',dir=run_dir))
    published=[]
    try:
        for task in plan['tasks']:
            td=staging/task['task_id']
            (td/'attempts').mkdir(parents=True); (td/'references').mkdir()
            compiled={**task, 'references': []}
            for idx, ref in enumerate(task['references'],1):
                if ref['source_kind']=='TASK_OUTPUT':
                    compiled['references'].append(ref)
                    continue
                source=Path(ref['source'])
                if not source.is_absolute(): source=(run_dir/source).resolve()
                if not source.is_file(): raise ValueError(f'REFERENCE_MISSING:{task["task_id"]}:{ref["source"]}')
                dest=td/'references'/f'{idx:02d}-{source.name}'
                shutil.copy2(source,dest)
                compiled['references'].append({**ref,'source':str(Path('tasks')/task['task_id']/'references'/dest.name),'sha256':sha256_file(dest)})
    
            ar=task['authority_refs']; narrative_refs=set(task['content'].get('narrative_refs',[]))
            snapshot={
              'style':authority['style'],
              'characters':[chars[x] for x in ar['character_ids']],
              'props':[props[x] for x in ar['prop_ids']],
              'world':authority['world'],
              'color_script':[beats[x] for x in ar['color_script_beat_ids']],
              'protected_visual_facts':[f for f in authority['protected_visual_facts'] if not narrative_refs or f['source_ref'] in narrative_refs],
            }
            atomic_json(td/'task.json',{'schema_version':2,**compiled,'authority_snapshot':snapshot})
        for task in plan['tasks']:
            dest=run_dir/'tasks'/task['task_id']
            if os.path.lexists(dest): raise ValueError(f'TASK_ALREADY_EXISTS:{task["task_id"]}')
            os.rename(staging/task['task_id'],dest)
            published.append(dest)
        atomic_json(run_dir/'plan'/'compiled.json',{'schema_version':2,'plan_id':plan['plan_id'],'task_ids':[t['task_id'] for t in plan['tasks']]})
    except BaseException:
        for dest in reversed(published): shutil.rmtree(dest)
        raise
    finally:
        shutil.rmtree(staging)
    return [t['task_id'] for t in plan['tasks']]

def task_status(run_dir:Path, task_id:str):
    td=run_dir/'tasks'/task_id; task=load_json(td/'task.json')
    sel=td/'selection.json'
    if sel.exists():
        selection=load_json(sel)
        for artifact in selection.get('artifacts',[]):
            path=run_dir/artifact['run_path']
            if not path.is_file() or sha256_file(path)!=artifact.get('sha256'):
                return 'SELECTION_ARTIFACT_DRIFT'
        return 'APPROVED'
    deps=task['depends_on']
    if any(not (run_dir/'tasks'/d/'selection.json').exists() for d in deps): return 'BLOCKED'
    attempts=sorted((td/'attempts').glob('[0-9][0-9][0-9][0-9]'))
    if not attempts: return 'READY'
    a=attempts[-1]
    if not (a/'result.json').exists(): return 'GENERATING'
    if not (a/'mechanical-review.json').exists(): return 'RETURNED'
    mech=load_json(a/'mechanical-review.json')
    if mech['status']!='PASS':
        fail_codes={f.get('code') for f in mech.get('findings',[]) if f.get('severity')=='FAIL'}
        if task.get('output',{}).get('preferred')=='PSD' and 'PREFERRED_OUTPUT_MISSING' in fail_codes and fail_codes <= {'PREFERRED_OUTPUT_MISSING','UNEXPECTED_FORMAT','NO_PRIMARY_OUTPUT','PSD_PACKAGING_PRIMARY_PSD_COUNT_INVALID'}:
            if (a/'packaging-retry.json').exists(): return 'PSD_PACKAGING_STOPPED'
            retry_count=0
            for previous in attempts:
                repair=(load_json(previous/'attempt.json').get('repair') or {}) if (previous/'attempt.json').is_file() else {}
                if repair.get('strategy')=='PSD_PACKAGING_RETRY': retry_count+=1
            if retry_count>=1: return 'PSD_PACKAGING_STOPPED'
            visual_path=a/'visual-review.json'
            if not visual_path.is_file() or load_json(visual_path).get('status')!='APPROVED': return 'NEEDS_VISUAL_REVIEW'
            return 'READY_TO_PACKAGE_RETRY'
        if not (a/'doctor-report.json').exists(): return 'NEEDS_DOCTOR'
        if not (a/'reconstruction-plan.json').exists(): return 'DIAGNOSED'
        return 'READY_TO_RECONSTRUCT'
    if not (a/'visual-review.json').exists(): return 'NEEDS_VISUAL_REVIEW'
    vis=load_json(a/'visual-review.json')
    if vis['status']=='APPROVED': return 'READY_TO_APPROVE'
    if not (a/'doctor-report.json').exists(): return 'NEEDS_DOCTOR'
    if not (a/'reconstruction-plan.json').exists(): return 'DIAGNOSED'
    return 'READY_TO_RECONSTRUCT'


def all_status(run_dir:Path):
    out=[]
    for td in sorted((run_dir/'tasks').iterdir() if (run_dir/'tasks').exists() else []):
        if td.is_dir() and (td/'task.json').exists(): out.append({'task_id':td.name,'status':task_status(run_dir,td.name)})
    return out


def begin_attempt(run_dir:Path,task_id:str, repair=None):
    td=run_dir/'tasks'/task_id
    if not td.exists(): raise ValueError('UNKNOWN_TASK')
    task=load_json(td/'task.json')
    if (td/'selection.json').exists(): raise ValueError('TASK_ALREADY_APPROVED')
    if any(not (run_dir/'tasks'/d/'selection.json').exists() for d in task['depends_on']): raise ValueError('DEPENDENCIES_NOT_APPROVED')
    attempts=sorted((td/'attempts').glob('[0-9][0-9][0-9][0-9]'))
    if attempts and repair is None: raise ValueError('ATTEMPT_EXISTS_USE_REPAIR')
    no=len(attempts)+1; ad=td/'attempts'/f'{no:04d}'

    resolved=[]
    for ref in task['references']:
        if ref['source_kind']=='FILE':
            p=run_dir/ref['source']
            if not p.is_file(): raise ValueError(f'REFERENCE_MISSING_AT_ATTEMPT:{ref["source"]}')
            actual=sha256_file(p)
            if actual!=ref.get('sha256'): raise ValueError(f'REFERENCE_HASH_MISMATCH:{ref["source"]}')
            resolved.append({'source_kind':'FILE','role':ref['role'],'path':ref['source'],'sha256':actual,'mime':_mime_for(p),'preserve':ref['preserve'],'ignore':ref['ignore']})
            continue
        source_tid=ref['source']; source_task=load_json(run_dir/'tasks'/source_tid/'task.json'); sel=load_json(run_dir/'tasks'/source_tid/'selection.json')
        primaries=[a for a in sel['artifacts'] if a.get('role','PRIMARY')=='PRIMARY']
        if not primaries: raise ValueError(f'REFERENCE_SOURCE_NO_PRIMARY:{task_id}:{source_tid}')
        for a in primaries:
            p=run_dir/a['run_path']
            if sha256_file(p)!=a['sha256']: raise ValueError(f'REFERENCE_SOURCE_HASH_MISMATCH:{source_tid}:{a["artifact_id"]}')
            resolved.append({'source_kind':'TASK_OUTPUT','role':ref['role'],'source_task_id':source_tid,'asset_id':source_task['asset_id'],'artifact_id':a['artifact_id'],'path':a['run_path'],'sha256':a['sha256'],'mime':a['mime'],'preserve':ref['preserve'],'ignore':ref['ignore']})

    if repair and repair.get('edit_target'):
        et=repair['edit_target']; ep=run_dir/et['run_path']
        if not ep.is_file(): raise ValueError('EDIT_TARGET_MISSING')
        if sha256_file(ep)!=et['sha256']: raise ValueError('EDIT_TARGET_HASH_MISMATCH')
    mode='PACKAGING_RETRY' if repair and repair.get('strategy')=='PSD_PACKAGING_RETRY' else ('EDIT' if repair and repair.get('edit_target') else 'GENERATE')
    attempt={'schema_version':2,'task_id':task_id,'attempt_no':no,'mode':mode,'created_at':datetime.now(timezone.utc).isoformat(),'resolved_references':resolved,'repair':repair}
    # Pure preparation and all reference validation happen before an attempt exists.
    prompt = compile_prompt(task,no,repair,resolved)
    capsule = compile_generation_capsule(task,no,repair,resolved)
    ad.mkdir()  # Exclusive ownership; never clean a directory created by another caller.
    try:
        atomic_json(ad/'attempt.json',attempt)
        atomic_text(ad/'prompt.md',prompt)
        atomic_text(ad/'generation-capsule.md',capsule)
        (ad/'outputs').mkdir()
    except BaseException:
        shutil.rmtree(ad)
        raise
    return ad
