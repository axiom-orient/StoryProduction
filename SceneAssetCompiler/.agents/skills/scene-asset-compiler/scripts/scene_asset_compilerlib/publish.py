from __future__ import annotations
from pathlib import Path
from datetime import datetime, timezone
import json, os, shutil, tempfile
from .util import load_json, load_json_bytes, atomic_json, atomic_text, sha256_bytes, sha256_file
from .review_contracts import validate_visual_review


def approve_attempt(run_dir:Path,task_id:str,attempt_no:int):
    td=run_dir/'tasks'/task_id; ad=td/'attempts'/f'{attempt_no:04d}'
    if (td/'selection.json').exists():
        existing=load_json(td/'selection.json')
        for artifact in existing.get('artifacts',[]):
            path=td/artifact['source_path']
            if not path.is_file() or sha256_file(path)!=artifact.get('sha256'):
                raise ValueError('APPROVE_BLOCKED:SELECTION_ARTIFACT_HASH_DRIFT')
        if existing.get('attempt_no')==attempt_no: return existing
        raise ValueError('SELECTION_IMMUTABLE')
    mech=load_json(ad/'mechanical-review.json'); vis=load_json(ad/'visual-review.json'); task=load_json(td/'task.json')
    if mech['status']!='PASS': raise ValueError('APPROVE_BLOCKED:MECHANICAL')
    result=load_json(ad/'result.json')
    current_hashes={}
    for artifact in result.get('artifacts',[]):
        path=ad/artifact['path']
        if not path.is_file(): raise ValueError('APPROVE_BLOCKED:ARTIFACT_MISSING')
        current=sha256_file(path)
        if current!=artifact.get('sha256'): raise ValueError('APPROVE_BLOCKED:ARTIFACT_HASH_DRIFT')
        current_hashes[artifact['artifact_id']]=current
    validate_visual_review(vis,task_id,attempt_no,task.get('prompt_profile','PROJECT_STYLE'),task.get('render_contract'),ad)
    if vis['status']!='APPROVED': raise ValueError('APPROVE_BLOCKED:VISUAL')
    if (task.get('render_contract') or {}).get('wireframe_layout'):
        primaries=[artifact for artifact in result.get('artifacts',[]) if artifact.get('role','PRIMARY')=='PRIMARY']
        if len(primaries)!=1: raise ValueError('APPROVE_BLOCKED:WIREFRAME_PRIMARY_COUNT')
        primary_sha256=current_hashes[primaries[0]['artifact_id']]
        if vis.get('artifact_sha256')!=primary_sha256: raise ValueError('APPROVE_BLOCKED:WIREFRAME_VISUAL_HASH')
        if (vis.get('layout_evidence') or {}).get('output_sha256')!=primary_sha256:
            raise ValueError('APPROVE_BLOCKED:WIREFRAME_LAYOUT_HASH')
    sel={'schema_version':1,'task_id':task_id,'attempt_no':attempt_no,'approved_at':datetime.now(timezone.utc).isoformat(),'artifacts':[{'artifact_id':a['artifact_id'],'role':a.get('role','PRIMARY'),'source_path':str(Path('attempts')/f'{attempt_no:04d}'/a['path']),'run_path':str(Path('tasks')/task_id/'attempts'/f'{attempt_no:04d}'/a['path']),'sha256':a['sha256'],'mime':a['mime'],'size_bytes':a['size_bytes']} for a in result['artifacts']]}
    atomic_json(td/'selection.json',sel); return sel


def _safe_file(root: Path, relative: str) -> Path:
    # Approval/publish is a trust boundary even when the CLI previously validated a plan.
    if not isinstance(relative, str) or not relative or "\\" in relative or "\0" in relative:
        raise ValueError('PUBLISH_BLOCKED:PATH')
    value = Path(relative)
    if value.is_absolute() or any(part in ('.', '..') for part in relative.split('/')):
        raise ValueError('PUBLISH_BLOCKED:PATH')
    current = root
    for part in value.parts:
        current = current / part
        if current.is_symlink(): raise ValueError('PUBLISH_BLOCKED:SYMLINK')
    if not current.is_file(): raise ValueError(f'PUBLISH_BLOCKED:FILE_MISSING:{relative}')
    return current


def _publish_inputs(run_dir: Path):
    """One owner of the exact input/read set used by final approval and publication."""
    subject_files = {}
    def read(relative):
        path = _safe_file(run_dir, relative)
        raw = path.read_bytes()
        subject_files[relative] = sha256_bytes(raw)
        return load_json_bytes(raw)
    req = read('request.json')
    compiled = read('plan/compiled.json')
    run = read('run.json')
    if run.get('request_sha256') != subject_files['request.json'] or run.get('project_id') != req['project_id']:
        raise ValueError('PUBLISH_BLOCKED:RUN_REQUEST_DRIFT')
    task_ids = compiled['task_ids']
    if not isinstance(task_ids, list) or len(task_ids) != len(set(task_ids)):
        raise ValueError('PUBLISH_BLOCKED:TASK_IDS')
    selected = []
    for tid in task_ids:
        if not isinstance(tid, str) or not tid or '/' in tid or '\\' in tid or tid in ('.', '..'):
            raise ValueError('PUBLISH_BLOCKED:TASK_ID')
        td = run_dir / 'tasks' / tid
        task = read(f'tasks/{tid}/task.json')
        if not task['required'] and not (td/'selection.json').exists(): continue
        if not (td/'selection.json').exists(): raise ValueError(f'PUBLISH_BLOCKED:UNAPPROVED:{tid}')
        sel = read(f'tasks/{tid}/selection.json')
        if not sel.get('artifacts'): raise ValueError(f'PUBLISH_BLOCKED:EMPTY_SELECTION:{tid}')
        primary_count = 0
        for artifact in sel['artifacts']:
            relative = f'tasks/{tid}/{artifact["source_path"]}'
            src = _safe_file(run_dir, relative)
            digest = sha256_file(src)
            if digest != artifact['sha256'] or src.stat().st_size != artifact['size_bytes']:
                raise ValueError(f'PUBLISH_BLOCKED:HASH:{tid}')
            subject_files[relative] = digest
            if artifact.get('role', 'PRIMARY') != 'PRIMARY': continue
            primary_count += 1
            selected.append((tid, task, sel, artifact, src))
        if not primary_count: raise ValueError(f'PUBLISH_BLOCKED:NO_PRIMARY:{tid}')
    if not selected: raise ValueError('PUBLISH_BLOCKED:NO_ASSETS')
    subject = sha256_bytes(json.dumps(subject_files, sort_keys=True, separators=(',', ':')).encode('utf-8'))
    return req, compiled, run, selected, subject


def human_approve(run_dir: Path, reviewer: str, note: str = ''):
    if not isinstance(reviewer, str) or not reviewer.strip(): raise ValueError('HUMAN_REVIEWER_REQUIRED')
    if not isinstance(note, str): raise ValueError('HUMAN_REVIEW_NOTE_INVALID')
    _, _, _, _, subject = _publish_inputs(run_dir)
    receipt = {'schema_version':1,'status':'APPROVED','reviewer':reviewer,'approved_at':datetime.now(timezone.utc).isoformat(),'note':note}
    publish_dir = run_dir/'publish'
    if publish_dir.is_symlink() or (publish_dir.exists() and not publish_dir.is_dir()):
        raise ValueError('PUBLISH_BLOCKED:PUBLISH_SYMLINK')
    hp = publish_dir/'human-approval.json'
    atomic_json(hp, receipt)
    # Keep the public human-approval v1 DTO unchanged. The internal commit marker binds
    # both the receipt bytes and the complete input read set. A torn pair fails closed.
    atomic_json(run_dir/'publish'/'human-approval-subject.json', {
        'schema_version': 1, 'subject_sha256': subject, 'approval_sha256': sha256_file(hp)
    })
    return receipt


def _assert_human_approval(run_dir: Path, subject: str):
    hp = run_dir/'publish'/'human-approval.json'
    bp = run_dir/'publish'/'human-approval-subject.json'
    if not hp.is_file() or not bp.is_file() or hp.is_symlink() or bp.is_symlink():
        raise ValueError('PUBLISH_BLOCKED:HUMAN_FINAL_REVIEW')
    approval_bytes = hp.read_bytes()
    receipt = load_json_bytes(approval_bytes)
    binding = load_json(bp)
    if (receipt.get('status') != 'APPROVED' or not isinstance(receipt.get('reviewer'), str)
            or not receipt['reviewer'].strip() or binding.get('schema_version') != 1
            or binding.get('approval_sha256') != sha256_bytes(approval_bytes)
            or binding.get('subject_sha256') != subject):
        raise ValueError('PUBLISH_BLOCKED:HUMAN_APPROVAL_STALE')


def _publish_blob(src: Path, dest: Path, digest: str, tid: str):
    if dest.is_symlink(): raise ValueError(f'PUBLISH_BLOCKED:BLOB_SYMLINK:{tid}')
    if dest.exists():
        if not dest.is_file() or sha256_file(dest) != digest:
            raise ValueError(f'PUBLISH_BLOCKED:BLOB_HASH:{tid}')
        return
    fd, temp = tempfile.mkstemp(prefix='.blob-', dir=dest.parent)
    try:
        with os.fdopen(fd, 'wb') as output, src.open('rb') as source:
            shutil.copyfileobj(source, output)
            output.flush(); os.fsync(output.fileno())
        if sha256_file(Path(temp)) != digest: raise ValueError(f'PUBLISH_BLOCKED:SOURCE_CHANGED:{tid}')
        os.replace(temp, dest)
    finally:
        if os.path.exists(temp): os.unlink(temp)


def publish(run_dir: Path):
    req, compiled, run, selected, subject = _publish_inputs(run_dir)
    if req['review_policy'].get('human_final_review', False): _assert_human_approval(run_dir, subject)
    blobs = run_dir/'publish'/'blobs'
    if blobs.is_symlink() or (run_dir/'publish').is_symlink(): raise ValueError('PUBLISH_BLOCKED:PUBLISH_SYMLINK')
    blobs.mkdir(parents=True, exist_ok=True)
    assets = []
    for tid, task, sel, artifact, src in selected:
        dest = blobs/f'{artifact["sha256"]}{src.suffix.lower()}'
        _publish_blob(src, dest, artifact['sha256'], tid)
        assets.append({'asset_id':task['asset_id'],'task_id':tid,'attempt_no':sel['attempt_no'],'path':str(Path('blobs')/dest.name),'sha256':artifact['sha256'],'mime':artifact['mime'],'size_bytes':artifact['size_bytes']})
    # Recheck the read set before the commit marker. Copied blobs may remain after an
    # interrupted publication, but no pack can claim success for changed approval inputs.
    if _publish_inputs(run_dir)[4] != subject: raise ValueError('PUBLISH_BLOCKED:INPUT_CHANGED')
    if req['review_policy'].get('human_final_review', False): _assert_human_approval(run_dir, subject)
    run_id = run['run_id']
    identity={'project_id':req['project_id'],'plan_id':compiled['plan_id'],'run_id':run_id,'assets':assets}
    identity_bytes=json.dumps(identity,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode('utf-8')
    pack_id='AP-'+sha256_bytes(identity_bytes)
    pack_path=run_dir/'publish'/'asset-pack.json'
    existing=None
    if pack_path.exists():
        try:
            existing=load_json(pack_path)
        except Exception as exc:
            raise ValueError('PUBLISH_BLOCKED:PACK_INVALID') from exc
        expected_lineage={'run_id':run_id,'plan_id':compiled['plan_id']}
        if (existing.get('schema_version')!=1 or existing.get('revision')!=1 or
                existing.get('pack_id')!=pack_id or existing.get('project_id')!=req['project_id'] or
                existing.get('assets')!=assets or existing.get('lineage')!=expected_lineage):
            raise ValueError('PUBLISH_BLOCKED:PACK_CONFLICT')
        for asset in existing['assets']:
            blob=run_dir/'publish'/asset['path']
            if not blob.is_file() or sha256_file(blob)!=asset['sha256']:
                raise ValueError('PUBLISH_BLOCKED:PACK_BLOB_HASH')
    pack=existing or {'schema_version':1,'pack_id':pack_id,'revision':1,'project_id':req['project_id'],'created_at':datetime.now(timezone.utc).isoformat(),'assets':assets,'lineage':{'run_id':run_id,'plan_id':compiled['plan_id']}}
    if existing is None:
        atomic_json(pack_path,pack)
    # portable checksum manifest
    lines=[f'{sha256_file(run_dir/"publish"/a["path"])}  {a["path"]}' for a in assets]
    manifest='\n'.join(lines)+'\n'
    manifest_path=run_dir/'publish'/'MANIFEST.sha256'
    if not manifest_path.exists() or manifest_path.read_text(encoding='utf-8')!=manifest:
        atomic_text(manifest_path,manifest)
    return pack
