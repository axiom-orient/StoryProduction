"""Mechanical output review for generated attempt artifacts."""
from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path

from .util import load_json, atomic_json, sha256_file
from .psd import parse_psd, PsdError
from .psd_validation import validate_generated_psd
from .execution import validate_execution_receipt
from .contract_primitives import ContractError
from .png_inspection import PngError, inspect_png
from .request_contracts import required_transparent_margin_px
from .packaging_preservation import validate_packaging_preservation_evidence
from .scene_provider_protocol import (
    DISPATCH_EVIDENCE_NAME,
    POSTFLIGHT_ERROR_EVIDENCE_NAME,
    PROCESS_EVIDENCE_NAME,
    PROVIDER_TOOL,
)


def mechanical_review(run_dir:Path, task_id:str, attempt_no:int):
    td=run_dir/'tasks'/task_id; task=load_json(td/'task.json'); ad=td/'attempts'/f'{attempt_no:04d}'; attempt=load_json(ad/'attempt.json'); result=load_json(ad/'result.json'); findings=[]
    receipt_path=ad/'execution-receipt.json'
    receipt=None
    raw_receipt=None
    if not receipt_path.exists():
        findings.append({'code':'EXECUTION_RECEIPT_MISSING','severity':'FAIL','message':'No immutable generation execution receipt was recorded.'})
    else:
        try:
            raw_receipt=load_json(receipt_path)
            receipt=validate_execution_receipt(raw_receipt,task_id,attempt_no,ad)
            receipt_outputs={x['path']:x['sha256'] for x in receipt['outputs']}
            for artifact in result.get('artifacts',[]):
                if receipt_outputs.get(artifact['path'])!=artifact['sha256']:
                    findings.append({'code':'EXECUTION_OUTPUT_MISMATCH','severity':'FAIL','artifact_id':artifact['artifact_id'],'message':artifact['path']})
        except Exception as e:
            # Keep the raw receipt available for targeted evidence diagnostics.
            # In particular, a stale PerfectPixel hash should be reported as
            # stale evidence even though receipt validation also fails.
            if raw_receipt is None:
                try: raw_receipt=load_json(receipt_path)
                except Exception: raw_receipt=None
            receipt=raw_receipt
            findings.append({'code':'EXECUTION_RECEIPT_INVALID','severity':'FAIL','message':str(e)})
    # Every Scene Provider PNG attempt must carry an immutable, hash-bound
    # non-mutating PerfectPixel inspection before it can enter visual review.
    # A missing or stale inspection is a terminal mechanical failure; it is
    # never converted into a retry or a successful attempt.
    receipt_for_evidence=receipt if isinstance(receipt,dict) else {}
    canonical_provider_evidence = {
        DISPATCH_EVIDENCE_NAME,
        PROCESS_EVIDENCE_NAME,
        POSTFLIGHT_ERROR_EVIDENCE_NAME,
    }
    unsupported_provider_evidence = [
        path.name
        for path in ad.iterdir()
        if path.is_file()
        and path.name.endswith(("-dispatch.json", "-process.json", "-postflight-error.json"))
        and path.name not in canonical_provider_evidence
    ]
    if unsupported_provider_evidence:
        findings.append({
            'code': 'PROVIDER_EVIDENCE_UNSUPPORTED',
            'severity': 'FAIL',
            'message': 'attempt contains non-canonical provider evidence filenames',
        })
    if any((ad / name).is_file() for name in canonical_provider_evidence) or receipt_for_evidence.get('tool') == PROVIDER_TOOL:
        inspection_ref=receipt_for_evidence.get('perfectpixel_inspection')
        if not isinstance(inspection_ref,dict):
            findings.append({'code':'PERFECTPIXEL_INSPECTION_MISSING','severity':'FAIL','message':'Scene Provider attempts require PerfectPixel inspect evidence.'})
        else:
            inspection=None
            inspection_value=inspection_ref.get('path')
            inspection_path=None
            if not isinstance(inspection_value,str) or not inspection_value or Path(inspection_value).is_absolute() or '..' in Path(inspection_value).parts or Path(inspection_value).as_posix()!=inspection_value:
                findings.append({'code':'PERFECTPIXEL_INSPECTION_INVALID','severity':'FAIL','message':'inspection evidence path is not local'})
            else:
                inspection_path=ad/inspection_value
            if inspection_path is not None and (not inspection_path.is_file() or inspection_path.is_symlink()):
                findings.append({'code':'PERFECTPIXEL_INSPECTION_MISSING','severity':'FAIL','message':str(inspection_ref.get('path'))})
            elif inspection_path is not None:
                current=inspection_path
                symlinked=False
                while current!=ad:
                    if current.is_symlink(): symlinked=True; break
                    current=current.parent
                if symlinked:
                    findings.append({'code':'PERFECTPIXEL_INSPECTION_INVALID','severity':'FAIL','message':'inspection evidence path contains a symlink'})
                elif sha256_file(inspection_path)!=inspection_ref.get('sha256'):
                    findings.append({'code':'PERFECTPIXEL_INSPECTION_STALE','severity':'FAIL','message':'inspection evidence hash changed after the execution receipt was sealed'})
                if not symlinked:
                    try:
                        inspection=load_json(inspection_path)
                    except Exception as error:
                        findings.append({'code':'PERFECTPIXEL_INSPECTION_INVALID','severity':'FAIL','message':str(error)})
            if isinstance(inspection,dict):
                required={'schema_version','schema','tool','executable_sha256','inspection_schema','inspection_schema_version','cli_version','role','source_manifest_sha256','input','inspection','process'}
                if set(inspection)!=required or inspection.get('schema_version')!=1 or inspection.get('schema')!='perfectpixel.asset-inspection/1' or inspection.get('tool')!='perfectpixel':
                    findings.append({'code':'PERFECTPIXEL_INSPECTION_INVALID','severity':'FAIL','message':'inspection evidence identity fields are not exact'})
                if inspection.get('inspection_schema')!='perfectpixel.asset-inspection/1' or inspection.get('inspection_schema_version')!=1 or inspection.get('role')!='deterministic-asset-compiler':
                    findings.append({'code':'PERFECTPIXEL_INSPECTION_INVALID','severity':'FAIL','message':'inspection contract identity is not exact'})
                input_ref=inspection.get('input')
                if not isinstance(input_ref,dict) or set(input_ref)!={'path','sha256','size_bytes'} or input_ref.get('path')!='outputs/generated.png':
                    findings.append({'code':'PERFECTPIXEL_INSPECTION_INVALID','severity':'FAIL','message':'inspection input binding is invalid'})
                else:
                    generated=next((item for item in result.get('artifacts',[]) if item.get('path')=='outputs/generated.png'),None)
                    if generated is None or input_ref.get('sha256')!=generated.get('sha256'):
                        findings.append({'code':'PERFECTPIXEL_INSPECTION_STALE','severity':'FAIL','message':'inspection input SHA-256 does not match the current generated PNG'})
                    elif input_ref.get('size_bytes')!=generated.get('size_bytes'):
                        findings.append({'code':'PERFECTPIXEL_INSPECTION_STALE','severity':'FAIL','message':'inspection input byte count does not match the current generated PNG'})
                inspected=inspection.get('inspection')
                if not isinstance(inspected,dict):
                    findings.append({'code':'PERFECTPIXEL_INSPECTION_INVALID','severity':'FAIL','message':'inspection result is not an object'})
                else:
                    generated_path=ad/'outputs/generated.png'
                    if generated_path.is_file():
                        try:
                            png=inspect_png(generated_path.read_bytes())
                            if inspected.get('inputSha256')!=sha256_file(generated_path) or inspected.get('inputByteCount')!=generated_path.stat().st_size:
                                findings.append({'code':'PERFECTPIXEL_INSPECTION_STALE','severity':'FAIL','message':'inspection result hash or byte count is stale'})
                            if (inspected.get('width'),inspected.get('height'))!=(png['width'],png['height']):
                                findings.append({'code':'PERFECTPIXEL_INSPECTION_DIMENSIONS','severity':'FAIL','message':'inspection result dimensions do not match the PNG'})
                            if inspected.get('hasAlpha') is not png['has_transparent_pixels']:
                                findings.append({'code':'PERFECTPIXEL_INSPECTION_ALPHA','severity':'FAIL','message':'inspection result alpha does not match the PNG'})
                        except PngError as error:
                            findings.append({'code':'PERFECTPIXEL_INSPECTION_INVALID','severity':'FAIL','message':str(error)})
                process=inspection.get('process')
                if not isinstance(process,dict) or process.get('leader_reaped') is not True or process.get('no_survivors') is not True or process.get('pipes_closed') is not True:
                    findings.append({'code':'PERFECTPIXEL_INSPECTION_PROCESS_UNPROVEN','severity':'FAIL','message':'PerfectPixel process custody was not proven'})
    primary=[a for a in result['artifacts'] if a.get('role')=='PRIMARY']
    if not primary: findings.append({'code':'NO_PRIMARY_OUTPUT','severity':'FAIL','message':'No PRIMARY output artifact returned.'})
    has_preferred=False
    for art in result['artifacts']:
        p=ad/art['path']
        if not p.is_file(): findings.append({'code':'MISSING_FILE','severity':'FAIL','artifact_id':art['artifact_id'],'message':art['path']}); continue
        if sha256_file(p)!=art['sha256']: findings.append({'code':'HASH_MISMATCH','severity':'FAIL','artifact_id':art['artifact_id'],'message':art['path']})
        ext=p.suffix.lower(); data=p.read_bytes(); preferred='.'+task['output']['preferred'].lower()
        if art.get('role')=='PREVIEW':
            if ext=='.png' and data[:8]!=b'\x89PNG\r\n\x1a\n': findings.append({'code':'PREVIEW_INVALID','severity':'FAIL','artifact_id':art['artifact_id'],'message':'bad preview PNG'})
            continue
        if ext==preferred: has_preferred=True
        allowed={preferred}
        if task['output']['fallback_allowed'] and preferred=='.psd': allowed|={'.png','.webp'}
        if ext not in allowed: findings.append({'code':'UNEXPECTED_FORMAT','severity':'FAIL','artifact_id':art['artifact_id'],'message':f'{ext} not in {sorted(allowed)}'})
        if ext=='.png':
            try: png=inspect_png(data)
            except PngError as e:
                png=None; findings.append({'code':'PNG_INVALID','severity':'FAIL','artifact_id':art['artifact_id'],'message':str(e)})
            if png:
                try: expected_size=tuple(int(x) for x in task['output']['size'].lower().split('x',1))
                except Exception: expected_size=None
                if expected_size and (png['width'],png['height'])!=expected_size:
                    findings.append({'code':'PNG_CANVAS_MISMATCH','severity':'FAIL','artifact_id':art['artifact_id'],'message':f"{png['width']}x{png['height']} != {expected_size[0]}x{expected_size[1]}"})
            if task['output']['transparent_background'] and (not png or not png['has_transparent_pixels']): findings.append({'code':'ALPHA_MISSING','severity':'FAIL','artifact_id':art['artifact_id'],'message':'transparent PNG requires at least one non-opaque pixel'})
            if task['output']['transparent_background'] and png and png['has_transparent_pixels'] and task.get('kind') not in ('EFFECT','SHADOW_LIGHT_HELPER') and png['near_opaque_pixels']==0:
                findings.append({'code':'ALPHA_OPAQUE_CORE_MISSING','severity':'FAIL','artifact_id':art['artifact_id'],'message':'reusable subject layer requires a near-opaque foreground core'})
            if task['output']['transparent_background'] and png and png.get('alpha_bbox'):
                bbox=png['alpha_bbox']; margins=(bbox[0],bbox[1],png['width']-bbox[2],png['height']-bbox[3])
                reusable=task.get('kind') in ('CHARACTER_ANCHOR','CHARACTER_ASSET','PROP_ANCHOR','PROP_ASSET')
                try:
                    required_margin=required_transparent_margin_px(task) if reusable else 0
                except ContractError as e:
                    required_margin=0
                    findings.append({'code':'OUTPUT_MARGIN_INVALID','severity':'FAIL','artifact_id':art['artifact_id'],'message':str(e)})
                if min(margins)<required_margin:
                    code='ALPHA_FOREGROUND_CLIPPED' if min(margins)==0 else 'ALPHA_MARGIN_TOO_SMALL'
                    findings.append({'code':code,'severity':'FAIL','artifact_id':art['artifact_id'],'message':f'alpha margins={margins}, required>={required_margin}px'})
            if task['output']['transparent_background'] and png and task.get('kind') in ('CHARACTER_ANCHOR','CHARACTER_ASSET','PROP_ANCHOR','PROP_ASSET') and png.get('fully_transparent_pixels',0)==0:
                findings.append({'code':'ALPHA_FULLY_TRANSPARENT_BACKGROUND_MISSING','severity':'FAIL','artifact_id':art['artifact_id'],'message':'reusable transparent asset requires fully transparent background pixels'})
            if preferred=='.psd': findings.append({'code':'PSD_FALLBACK','severity':'WARN','artifact_id':art['artifact_id'],'message':'Raster fallback used; PSD layer structure unavailable.'})
        if ext=='.psd':
            try:
                if task['output'].get('psd_profile'):
                    try: expected_canvas=tuple(int(x) for x in task['output']['size'].lower().split('x',1))
                    except Exception: expected_canvas=None
                    strict=validate_generated_psd(p,task['output']['psd_profile'],task['structure']['required_layer_ids'],expected_canvas)
                    info=strict['psd']
                    for message in strict['findings']:
                        findings.append({'code':message.split(':',1)[0],'severity':'FAIL','artifact_id':art['artifact_id'],'message':message})
                else:
                    info=parse_psd(data); names=[l['name'] for l in info['layers']]
                    for n in sorted({n for n in names if names.count(n)>1}): findings.append({'code':'DUPLICATE_LAYER_ID','severity':'FAIL','artifact_id':art['artifact_id'],'layer_id':n,'message':n})
                    for n in task['structure']['required_layer_ids']:
                        if n not in names: findings.append({'code':'REQUIRED_LAYER_MISSING','severity':'FAIL','artifact_id':art['artifact_id'],'layer_id':n,'message':n})
                atomic_json(ad/'psd-inventory.json',info)
            except PsdError as e: findings.append({'code':'PSD_INVALID','severity':'FAIL','artifact_id':art['artifact_id'],'message':str(e)})
    if task['output']['preferred']=='PSD' and not has_preferred and not task['output']['fallback_allowed']:
        findings.append({'code':'PREFERRED_OUTPUT_MISSING','severity':'FAIL','message':'Required PSD primary output missing.'})
    if attempt.get('mode')=='PACKAGING_RETRY':
        primary_psds=sorted(
            (
                item for item in result.get('artifacts',[])
                if item.get('role')=='PRIMARY' and Path(item.get('path','')).suffix.lower()=='.psd'
            ),
            key=lambda item:(item.get('path',''),item.get('sha256','')),
        )
        if len(primary_psds)!=1:
            findings.append({'code':'PSD_PACKAGING_PRIMARY_PSD_COUNT_INVALID','severity':'FAIL','message':f'Packaging retry requires exactly one PRIMARY PSD; observed {len(primary_psds)}.'})
        packaging=(attempt.get('repair') or {}).get('packaging_retry') or {}
        substrate_sha256=packaging.get('substrate_sha256')
        contract_valid=(
            isinstance(substrate_sha256,str) and len(substrate_sha256)==64
            and packaging.get('mode')=='PSD_ONLY'
            and packaging.get('trigger')=='PREFERRED_OUTPUT_MISSING'
            and packaging.get('retry_index')==1
            and packaging.get('max_retries')==1
            and packaging.get('preserve_locked_pixels') is True
            and packaging.get('preserve_locked_layout') is True
            and packaging.get('creative_changes')=='FORBID'
        )
        if not contract_valid:
            findings.append({'code':'PSD_PACKAGING_RETRY_CONTRACT_INVALID','severity':'FAIL','message':'Packaging retry lacks the bound substrate SHA or required preservation flags.'})
        elif len(primary_psds)==1:
            preservation_path=ad/'packaging-preservation.json'
            if not preservation_path.is_file():
                findings.append({'code':'PSD_PACKAGING_PRESERVATION_EVIDENCE_MISSING','severity':'FAIL','message':'Packaging retry PSD requires a SUBMITTER_ATTESTED hash-bound comparison attestation. Acceptance checks declared zero-delta consistency and file binding, not external provenance or independent reproduction.'})
            else:
                try:
                    validate_packaging_preservation_evidence(
                        load_json(preservation_path),task_id,attempt_no,ad,substrate_sha256,primary_psds[0]['sha256']
                    )
                except ContractError as e:
                    code='PSD_PACKAGING_PRESERVATION_NOT_PROVEN' if str(e).startswith('PSD_PACKAGING_PRESERVATION_NOT_PROVEN') else 'PSD_PACKAGING_PRESERVATION_EVIDENCE_INVALID'
                    findings.append({'code':code,'severity':'FAIL','message':str(e)})
    req=load_json(run_dir/'request.json'); allow_warn=req['review_policy'].get('allow_warnings',True)
    fail=any(f['severity']=='FAIL' for f in findings) or (not allow_warn and any(f['severity']=='WARN' for f in findings))
    status='FAIL' if fail else 'PASS'
    out={'schema_version':1,'task_id':task_id,'attempt_no':attempt_no,'created_at':datetime.now(timezone.utc).isoformat(),'status':status,'findings':findings}
    atomic_json(ad/'mechanical-review.json',out); return out
