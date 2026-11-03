"""Validate attempt-bound PSD packaging preservation and comparison evidence."""
from __future__ import annotations

from pathlib import Path

from .contract_primitives import ContractError, _SHA256_RE, reject_unknown
from .util import load_json, sha256_file


_PACKAGING_METRIC_KEYS={'canvas_match','pixel_exact','changed_pixels','max_channel_delta','alpha_match'}


def _validate_packaging_metrics(metrics):
    if not isinstance(metrics,dict) or set(metrics)!=_PACKAGING_METRIC_KEYS:
        raise ContractError('PACKAGING_PRESERVATION_METRICS_INVALID')
    exact=(
        metrics.get('canvas_match') is True
        and metrics.get('pixel_exact') is True
        and isinstance(metrics.get('changed_pixels'),int) and not isinstance(metrics.get('changed_pixels'),bool) and metrics['changed_pixels']==0
        and isinstance(metrics.get('max_channel_delta'),(int,float)) and not isinstance(metrics.get('max_channel_delta'),bool) and metrics['max_channel_delta']==0
        and metrics.get('alpha_match') is True
    )
    if not exact: raise ContractError('PACKAGING_PRESERVATION_METRICS_NOT_EXACT')
    return metrics


def _comparison_receipt_path(attempt_dir, value):
    if not isinstance(value,str) or not value: raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_PATH')
    relative=Path(value)
    if relative.is_absolute() or '..' in relative.parts or relative.as_posix()!=value:
        raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_PATH')
    root=Path(attempt_dir).resolve(); current=root
    for part in relative.parts:
        current=current/part
        if current.is_symlink(): raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_PATH')
    resolved=current.resolve()
    if root not in resolved.parents: raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_PATH')
    if not resolved.is_file(): raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_MISSING')
    return resolved


def validate_packaging_preservation_evidence(o, task_id, attempt_no, attempt_dir, expected_substrate_sha256, expected_psd_sha256):
    if not isinstance(o,dict): raise ContractError('TYPE:packaging_preservation')
    reject_unknown(o,['schema_version','task_id','attempt_no','evidence_level','substrate_sha256','psd_sha256','render_back_sha256','reviewer','tool','tool_version','comparison_receipt','metrics','verdict','evidence'],'packaging_preservation')
    if o.get('schema_version')!=1: raise ContractError('VERSION:packaging_preservation')
    if o.get('task_id')!=task_id or o.get('attempt_no')!=attempt_no: raise ContractError('LINEAGE:packaging_preservation')
    if o.get('evidence_level')!='SUBMITTER_ATTESTED': raise ContractError('PACKAGING_PRESERVATION_EVIDENCE_LEVEL')
    for key in ('substrate_sha256','psd_sha256','render_back_sha256'):
        value=o.get(key)
        if not isinstance(value,str) or not _SHA256_RE.fullmatch(value): raise ContractError(f'PACKAGING_PRESERVATION_HASH:{key}')
    for key in ('reviewer','tool','tool_version'):
        if not isinstance(o.get(key),str) or not o[key].strip(): raise ContractError(f'PACKAGING_PRESERVATION_REQUIRED:{key}')
    if o['substrate_sha256']!=expected_substrate_sha256: raise ContractError('PACKAGING_PRESERVATION_SUBSTRATE_MISMATCH')
    if o['psd_sha256']!=expected_psd_sha256: raise ContractError('PACKAGING_PRESERVATION_PSD_MISMATCH')
    if o.get('verdict')!='PASS': raise ContractError('PSD_PACKAGING_PRESERVATION_NOT_PROVEN')
    evidence=o.get('evidence')
    if not isinstance(evidence,list) or not any(isinstance(item,str) and item.strip() for item in evidence):
        raise ContractError('PACKAGING_PRESERVATION_EVIDENCE_REQUIRED')
    metrics=_validate_packaging_metrics(o.get('metrics'))
    comparison_ref=o.get('comparison_receipt')
    if not isinstance(comparison_ref,dict) or set(comparison_ref)!={'path','sha256'}:
        raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_INVALID')
    if not isinstance(comparison_ref.get('sha256'),str) or not _SHA256_RE.fullmatch(comparison_ref['sha256']):
        raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_INVALID')

    ad=Path(attempt_dir); result=load_json(ad/'result.json'); artifacts=result.get('artifacts',[])
    primary_psds=[
        item for item in artifacts
        if item.get('role')=='PRIMARY' and Path(item.get('path','')).suffix.lower()=='.psd'
    ]
    if len(primary_psds)!=1: raise ContractError('PSD_PACKAGING_PRIMARY_PSD_COUNT_INVALID')
    if primary_psds[0].get('sha256')!=o['psd_sha256']:
        raise ContractError('PACKAGING_PRESERVATION_PSD_MISMATCH')
    render_matches=[
        item for item in artifacts
        if item.get('sha256')==o['render_back_sha256']
        and item.get('role') in {'PRIMARY','PREVIEW'}
        and Path(item.get('path','')).suffix.lower()=='.png'
    ]
    if not render_matches: raise ContractError('PACKAGING_PRESERVATION_RENDER_BACK_UNBOUND')
    for item in primary_psds+render_matches:
        path=ad/item['path']
        if not path.is_file() or sha256_file(path)!=item['sha256']:
            raise ContractError('PACKAGING_PRESERVATION_OUTPUT_HASH_MISMATCH')

    comparison_path=_comparison_receipt_path(ad,comparison_ref.get('path'))
    if sha256_file(comparison_path)!=comparison_ref['sha256']:
        raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_HASH_MISMATCH')
    try:
        comparison=load_json(comparison_path)
    except Exception as e:
        raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_INVALID') from e
    reject_unknown(comparison,['schema_version','substrate_sha256','psd_sha256','render_back_sha256','metrics'],'packaging_preservation.comparison_receipt')
    if comparison.get('schema_version')!=1: raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_INVALID')
    if any(comparison.get(key)!=o[key] for key in ('substrate_sha256','psd_sha256','render_back_sha256')):
        raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_BINDING_MISMATCH')
    comparison_metrics=_validate_packaging_metrics(comparison.get('metrics'))
    if comparison_metrics!=metrics: raise ContractError('PACKAGING_PRESERVATION_COMPARISON_RECEIPT_METRICS_MISMATCH')
    return o
