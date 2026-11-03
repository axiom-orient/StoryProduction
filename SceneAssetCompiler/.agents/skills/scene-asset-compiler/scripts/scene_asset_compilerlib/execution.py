from __future__ import annotations
from datetime import datetime, timezone
from pathlib import Path
from .util import load_json, atomic_json, sha256_file
from .scene_provider_protocol import PROVIDER_TOOL

IMAGEGEN_TOOL='$imagegen'
EXECUTION_TOOL_IDENTITIES=frozenset({IMAGEGEN_TOOL,PROVIDER_TOOL})

ALLOWED={'provider','tool','model','model_snapshot','request_id','started_at','finished_at','outcome','evidence_level','submitted_prompt_path','capsule_verbatim','notes','perfectpixel_inspection'}
OUTCOMES={'SUCCEEDED','FAILED','CANCELLED','UNKNOWN'}
EVIDENCE={'EXECUTOR_RECORDED','PROVIDER_RECEIPT'}
PACKAGING_RETRY_KEYS={'mode','trigger','retry_index','max_retries','substrate_sha256','preserve_locked_pixels','preserve_locked_layout','creative_changes'}


def _validate_packaging_retry_lock(lock):
    if not isinstance(lock,dict) or set(lock)!=PACKAGING_RETRY_KEYS: raise ValueError('EXECUTION_PACKAGING_RETRY_CONTRACT')
    substrate=lock.get('substrate_sha256')
    if not isinstance(substrate,str) or len(substrate)!=64 or any(char not in '0123456789abcdef' for char in substrate):
        raise ValueError('EXECUTION_PACKAGING_RETRY_SUBSTRATE')
    if lock.get('mode')!='PSD_ONLY' or lock.get('trigger')!='PREFERRED_OUTPUT_MISSING': raise ValueError('EXECUTION_PACKAGING_RETRY_CONTRACT')
    if not isinstance(lock.get('retry_index'),int) or isinstance(lock.get('retry_index'),bool) or lock['retry_index']<1: raise ValueError('EXECUTION_PACKAGING_RETRY_CONTRACT')
    if lock.get('max_retries')!=1: raise ValueError('EXECUTION_PACKAGING_RETRY_CONTRACT')
    if lock.get('preserve_locked_pixels') is not True or lock.get('preserve_locked_layout') is not True or lock.get('creative_changes')!='FORBID':
        raise ValueError('EXECUTION_PACKAGING_RETRY_PRESERVATION')
    return lock


def validate_execution_draft(draft):
    if not isinstance(draft,dict): raise ValueError('EXECUTION_DRAFT_TYPE')
    extra=set(draft)-ALLOWED
    if extra: raise ValueError(f'EXECUTION_DRAFT_UNKNOWN_FIELDS:{sorted(extra)}')
    for key in ('provider','tool','model','model_snapshot','request_id','started_at','finished_at','outcome','evidence_level'):
        if not isinstance(draft.get(key),str) or not draft[key].strip(): raise ValueError(f'EXECUTION_DRAFT_REQUIRED:{key}')
    if draft['tool'] not in EXECUTION_TOOL_IDENTITIES: raise ValueError('EXECUTION_TOOL_UNSUPPORTED')
    if draft['outcome'] not in OUTCOMES: raise ValueError('EXECUTION_OUTCOME')
    if draft['evidence_level'] not in EVIDENCE: raise ValueError('EXECUTION_EVIDENCE_LEVEL')
    if draft['evidence_level']=='PROVIDER_RECEIPT' and draft['request_id']=='UNKNOWN': raise ValueError('EXECUTION_PROVIDER_RECEIPT_ID_REQUIRED')
    if not isinstance(draft.get('submitted_prompt_path'),str) or not draft['submitted_prompt_path']: raise ValueError('EXECUTION_SUBMITTED_PROMPT_REQUIRED')
    prompt_path=Path(draft['submitted_prompt_path'])
    if prompt_path.is_absolute() or '..' in prompt_path.parts or prompt_path.as_posix()!=draft['submitted_prompt_path']: raise ValueError('EXECUTION_SUBMITTED_PROMPT_NOT_LOCAL')
    if not isinstance(draft.get('capsule_verbatim'),bool): raise ValueError('EXECUTION_CAPSULE_VERBATIM_REQUIRED')
    if 'notes' in draft and not isinstance(draft['notes'],list): raise ValueError('EXECUTION_NOTES_TYPE')
    inspection=draft.get('perfectpixel_inspection')
    if draft.get('tool')==PROVIDER_TOOL and draft.get('outcome')=='SUCCEEDED' and inspection is None:
        raise ValueError('EXECUTION_PERFECTPIXEL_INSPECTION_REQUIRED')
    if inspection is not None:
        if not isinstance(inspection,dict) or set(inspection)!={'path','sha256'}: raise ValueError('EXECUTION_PERFECTPIXEL_INSPECTION_FIELDS')
        if not isinstance(inspection['path'],str) or not inspection['path'] or Path(inspection['path']).is_absolute() or '..' in Path(inspection['path']).parts or Path(inspection['path']).as_posix()!=inspection['path']:
            raise ValueError('EXECUTION_PERFECTPIXEL_INSPECTION_PATH')
        if not isinstance(inspection['sha256'],str) or len(inspection['sha256'])!=64 or any(char not in '0123456789abcdef' for char in inspection['sha256']):
            raise ValueError('EXECUTION_PERFECTPIXEL_INSPECTION_HASH')
    return draft


def validate_execution_receipt(receipt, task_id=None, attempt_no=None, attempt_dir=None):
    if receipt.get('schema_version')!=1: raise ValueError('EXECUTION_RECEIPT_VERSION')
    if task_id is not None and receipt.get('task_id')!=task_id: raise ValueError('EXECUTION_RECEIPT_TASK')
    if attempt_no is not None and receipt.get('attempt_no')!=attempt_no: raise ValueError('EXECUTION_RECEIPT_ATTEMPT')
    for key in ('capsule_sha256','prompt_sha256'):
        value=receipt.get(key)
        if not isinstance(value,str) or len(value)!=64: raise ValueError(f'EXECUTION_RECEIPT_HASH:{key}')
    if not isinstance(receipt.get('submitted_prompt_sha256'),str) or len(receipt['submitted_prompt_sha256'])!=64: raise ValueError('EXECUTION_RECEIPT_HASH:submitted_prompt_sha256')
    if receipt.get('capsule_verbatim') and receipt['submitted_prompt_sha256']!=receipt['capsule_sha256']: raise ValueError('EXECUTION_CAPSULE_VERBATIM_MISMATCH')
    validate_execution_draft({k:receipt[k] for k in ALLOWED if k in receipt})
    if not isinstance(receipt.get('inputs'),list) or not isinstance(receipt.get('outputs'),list): raise ValueError('EXECUTION_RECEIPT_IO')
    packaging_retry=receipt.get('packaging_retry')
    if packaging_retry is not None:
        _validate_packaging_retry_lock(packaging_retry)
    if receipt['outcome']=='SUCCEEDED' and not receipt['outputs']: raise ValueError('EXECUTION_RECEIPT_OUTPUT_REQUIRED')
    if receipt.get('tool')==PROVIDER_TOOL and receipt['outcome']=='SUCCEEDED' and receipt.get('perfectpixel_inspection') is None:
        raise ValueError('EXECUTION_PERFECTPIXEL_INSPECTION_REQUIRED')
    if attempt_dir is not None:
        ad=Path(attempt_dir).resolve(); submitted=(ad/receipt['submitted_prompt_path'])
        attempt=load_json(ad/'attempt.json')
        if attempt.get('mode')=='PACKAGING_RETRY':
            if packaging_retry is None: raise ValueError('EXECUTION_PACKAGING_RETRY_REQUIRED')
            attempt_lock=(attempt.get('repair') or {}).get('packaging_retry')
            _validate_packaging_retry_lock(attempt_lock)
            if packaging_retry!=attempt_lock: raise ValueError('EXECUTION_PACKAGING_RETRY_MISMATCH')
        elif packaging_retry is not None:
            raise ValueError('EXECUTION_PACKAGING_RETRY_UNEXPECTED')
        current=submitted
        while current!=ad:
            if current.is_symlink(): raise ValueError('EXECUTION_SUBMITTED_PROMPT_SYMLINK')
            if ad not in current.parents: raise ValueError('EXECUTION_SUBMITTED_PROMPT_NOT_LOCAL')
            current=current.parent
        if not submitted.is_file(): raise ValueError('EXECUTION_SUBMITTED_PROMPT_MISSING')
        if sha256_file(submitted)!=receipt['submitted_prompt_sha256']: raise ValueError('EXECUTION_SUBMITTED_PROMPT_DRIFT')
        inspection=receipt.get('perfectpixel_inspection')
        if inspection is not None:
            evidence=ad/inspection['path']
            if ad not in evidence.parents:
                raise ValueError('EXECUTION_PERFECTPIXEL_INSPECTION_NOT_LOCAL')
            current=evidence
            while current!=ad:
                if current.is_symlink(): raise ValueError('EXECUTION_PERFECTPIXEL_INSPECTION_SYMLINK')
                current=current.parent
            if evidence.is_symlink() or not evidence.is_file(): raise ValueError('EXECUTION_PERFECTPIXEL_INSPECTION_MISSING')
            if sha256_file(evidence)!=inspection['sha256']: raise ValueError('EXECUTION_PERFECTPIXEL_INSPECTION_DRIFT')
    return receipt


def record_execution(run_dir:Path, task_id:str, attempt_no:int, draft):
    ad=Path(run_dir)/'tasks'/task_id/'attempts'/f'{attempt_no:04d}'
    target=ad/'execution-receipt.json'
    if target.exists(): raise ValueError('EXECUTION_RECEIPT_IMMUTABLE')
    draft=load_json(Path(draft)) if isinstance(draft,(str,Path)) else draft
    validate_execution_draft(draft)
    attempt=load_json(ad/'attempt.json'); task=load_json(Path(run_dir)/'tasks'/task_id/'task.json')
    if attempt.get('task_id')!=task_id or attempt.get('attempt_no')!=attempt_no: raise ValueError('EXECUTION_ATTEMPT_LINEAGE')
    inputs=[]
    for item in attempt.get('resolved_references',[]):
        inputs.append({k:item[k] for k in ('role','path','sha256') if k in item})
    edit_target=(attempt.get('repair') or {}).get('edit_target')
    if edit_target:
        inputs.append({'role':'EDIT_TARGET','path':edit_target.get('run_path',edit_target.get('path')),'sha256':edit_target['sha256']})
    outputs=[]
    for path in sorted((ad/'outputs').iterdir()):
        if path.is_file(): outputs.append({'path':str(Path('outputs')/path.name),'sha256':sha256_file(path),'size_bytes':path.stat().st_size})
    submitted_value=Path(draft['submitted_prompt_path'])
    submitted=ad/submitted_value
    unresolved=submitted
    while unresolved!=ad:
        if unresolved.is_symlink(): raise ValueError('EXECUTION_SUBMITTED_PROMPT_SYMLINK')
        unresolved=unresolved.parent
    submitted=submitted.resolve()
    if ad.resolve() not in submitted.parents: raise ValueError('EXECUTION_SUBMITTED_PROMPT_NOT_LOCAL')
    if not submitted.is_file(): raise ValueError('EXECUTION_SUBMITTED_PROMPT_MISSING')
    submitted_receipt_path=submitted.relative_to(ad.resolve()).as_posix()
    receipt={
      'schema_version':1,'task_id':task_id,'attempt_no':attempt_no,'created_at':datetime.now(timezone.utc).isoformat(),
      **draft,
      'capsule_sha256':sha256_file(ad/'generation-capsule.md'),'prompt_sha256':sha256_file(ad/'prompt.md'),
      'submitted_prompt_path':submitted_receipt_path,'submitted_prompt_sha256':sha256_file(submitted),
      'requested':{'format':task['output']['preferred'],'size':task['output']['size'],'transparent_background':task['output']['transparent_background']},
      'inputs':inputs,'outputs':outputs,
    }
    if attempt.get('mode')=='PACKAGING_RETRY':
        packaging=(attempt.get('repair') or {}).get('packaging_retry') or {}
        receipt['packaging_retry']=dict(packaging)
    validate_execution_receipt(receipt,task_id,attempt_no,ad)
    atomic_json(target,receipt); return receipt
