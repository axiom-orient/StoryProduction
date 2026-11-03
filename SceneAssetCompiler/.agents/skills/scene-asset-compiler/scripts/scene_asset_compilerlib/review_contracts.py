"""Visual-review contract validation."""
from __future__ import annotations

from pathlib import Path

from .render_contract_validation import RenderContractError
from .render_layout_evidence import validate_layout_evidence as _validate_layout_evidence
from .util import load_json, sha256_file
from .contract_primitives import (
    ContractError,
    PROMPT_PROFILES,
    _SHA256_RE,
    validate_render_contract,
)


def validate_visual_review(o, task_id, attempt_no, expected_prompt_profile=None, expected_render_contract=None, attempt_dir=None):
    if o.get('schema_version')!=3: raise ContractError('VERSION:visual_review')
    if o.get('task_id')!=task_id or o.get('attempt_no')!=attempt_no: raise ContractError('LINEAGE:visual_review')
    checks=o.get('checks')
    if not isinstance(checks,dict): raise ContractError('TYPE:checks')
    keys=['identity','style','narrative','composition','lighting_color','material','photorealism','structure','articulation','usability']
    extra=set(checks)-set(keys)
    if extra-set(['layout_fidelity','genre_fidelity']): raise ContractError('VISUAL_CHECK_KEYS')
    snapshot=o.get('render_contract_snapshot')
    if expected_render_contract is not None:
        validate_render_contract(expected_render_contract,'expected_render_contract')
    if snapshot is not None:
        validate_render_contract(snapshot,'visual_review.render_contract_snapshot')
    if expected_render_contract is not None and snapshot is not None and snapshot!=expected_render_contract:
        raise ContractError('VISUAL_RENDER_CONTRACT_MISMATCH')
    contract=expected_render_contract or snapshot or {}
    explicit=expected_render_contract is not None or snapshot is not None
    layout_required=explicit and bool(contract.get('wireframe_layout'))
    genre_required=explicit
    if layout_required and 'layout_fidelity' not in checks: raise ContractError('VISUAL_LAYOUT_FIDELITY_REQUIRED')
    if genre_required and 'genre_fidelity' not in checks: raise ContractError('VISUAL_GENRE_FIDELITY_REQUIRED')
    if layout_required and checks.get('layout_fidelity')=='NA': raise ContractError('VISUAL_LAYOUT_FIDELITY_REQUIRED')
    if genre_required and checks.get('genre_fidelity')=='NA': raise ContractError('VISUAL_GENRE_FIDELITY_REQUIRED')
    if 'layout_fidelity' in checks and not layout_required: raise ContractError('VISUAL_LAYOUT_FIDELITY_UNEXPECTED')
    if 'genre_fidelity' in checks and not genre_required: raise ContractError('VISUAL_GENRE_FIDELITY_UNEXPECTED')
    layout_evidence=o.get('layout_evidence')
    if layout_required and layout_evidence is None:
        raise ContractError('VISUAL_LAYOUT_EVIDENCE_REQUIRED')
    artifact_sha256=o.get('artifact_sha256')
    if layout_required and artifact_sha256 is None:
        raise ContractError('VISUAL_ARTIFACT_HASH_REQUIRED')
    if artifact_sha256 is not None:
        if not isinstance(artifact_sha256,str) or not _SHA256_RE.fullmatch(artifact_sha256):
            raise ContractError('VISUAL_ARTIFACT_HASH_INVALID')
        if attempt_dir is not None:
            ad=Path(attempt_dir); result=load_json(ad/'result.json')
            matches=[item for item in result.get('artifacts',[]) if item.get('sha256')==artifact_sha256]
            if not matches: raise ContractError('VISUAL_ARTIFACT_HASH_UNBOUND')
            for item in matches:
                path=ad/item['path']
                if not path.is_file() or sha256_file(path)!=artifact_sha256:
                    raise ContractError('VISUAL_ARTIFACT_HASH_MISMATCH')
    keys_for_validation=keys+sorted(extra)
    for key in keys_for_validation:
        if checks.get(key) not in ['PASS','FAIL','NA']: raise ContractError(f'VISUAL_CHECK:{key}')
    evidence=o.get('evidence')
    if not isinstance(evidence,dict) or set(evidence)!=set(keys_for_validation): raise ContractError('VISUAL_EVIDENCE_KEYS')
    for key in keys_for_validation:
        ev=evidence.get(key)
        if not isinstance(ev,list): raise ContractError(f'TYPE:evidence.{key}')
        if checks[key] in ['PASS','FAIL'] and not any(isinstance(x,str) and x.strip() for x in ev):
            raise ContractError(f'VISUAL_EVIDENCE_REQUIRED:{key}')
    if layout_required:
        try:
            _validate_layout_evidence(
                layout_evidence,
                contract,
                attempt_dir=attempt_dir,
                require_pass=checks.get('layout_fidelity')=='PASS',
            )
        except RenderContractError as error:
            raise ContractError(str(error)) from error
    profile=o.get('prompt_profile_snapshot')
    if profile not in PROMPT_PROFILES: raise ContractError('VISUAL_PROMPT_PROFILE')
    if expected_prompt_profile is not None and profile!=expected_prompt_profile: raise ContractError('VISUAL_PROMPT_PROFILE_MISMATCH')
    if (profile.startswith('PHOTOREAL_') or contract.get('capture_profile')=='PHOTOREAL') and checks['photorealism']=='NA': raise ContractError('VISUAL_PHOTOREALISM_REQUIRED')
    expected='APPROVED' if all(v in ['PASS','NA'] for v in checks.values()) else 'REJECTED'
    if o.get('status')!=expected: raise ContractError(f'VISUAL_STATUS_MISMATCH:{expected}')
