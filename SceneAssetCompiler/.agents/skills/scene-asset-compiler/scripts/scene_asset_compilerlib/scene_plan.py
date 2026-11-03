"""SceneProductionPlan validation and derived generation-budget policy."""
from __future__ import annotations

from .render_contract_validation import validate_render_contract


class SceneError(ValueError): pass

def validate_scene_plan(p):
    if p.get('schema_version')!=1: raise SceneError('SCENE_PLAN_VERSION')
    for k in ('scene_id','deliverables','visual_brief','composition_candidates','target_candidate_policy','layers','output','budgets'):
        if k not in p: raise SceneError(f'SCENE_PLAN_MISSING:{k}')
    if not p['composition_candidates']: raise SceneError('SCENE_PLAN_NO_COMPOSITIONS')
    if p.get('render_contract') is not None:
        try: validate_render_contract(p['render_contract'])
        except ValueError as e: raise SceneError(f'RENDER_CONTRACT:{e}') from e
    ids=[x['candidate_id'] for x in p['composition_candidates']]
    if len(ids)!=len(set(ids)): raise SceneError('SCENE_PLAN_DUPLICATE_COMPOSITION')
    lids=[x['layer_id'] for x in p['layers']]
    for x in p['layers']:
        if x.get('path_boundary') not in ('NONE','SAVED_PATH','SOFT_ENVELOPE'): raise SceneError(f"SCENE_PLAN_PATH_BOUNDARY:{x.get('layer_id')}")
        if x.get('edge_class') not in ('HARD','ORGANIC','SOFT_ALPHA'): raise SceneError(f"SCENE_PLAN_EDGE_CLASS:{x.get('layer_id')}")
        if x.get('path_boundary')=='SOFT_ENVELOPE' and x.get('edge_class')!='SOFT_ALPHA': raise SceneError(f"SCENE_PLAN_SOFT_PATH_CLASS:{x.get('layer_id')}")
    if len(lids)!=len(set(lids)): raise SceneError('SCENE_PLAN_DUPLICATE_LAYER')
    if 'PSD' in p['deliverables'] and not p['layers']: raise SceneError('SCENE_PLAN_PSD_REQUIRES_LAYERS')
    est=estimate_generation_budget(p)
    limit=int(p['budgets']['max_total_generations'])
    if est['recommended_total']>limit:
        raise SceneError(f"SCENE_PLAN_BUDGET:{est['recommended_total']}>{limit}")
    return p

def _policy_count(policy, default=1):
    if not policy: return default
    return int(policy.get('recommended',policy.get('minimum',default)))

def estimate_generation_budget(p):
    comps=len(p['composition_candidates'])
    targets=_policy_count(p['target_candidate_policy'],2)
    layout=p.get('render_contract',{}).get('wireframe_layout') if p.get('render_contract') else None
    layout_lock=1 if layout else 0
    placeholder_photo=len(layout.get('slots',[])) if layout else 0
    layers=sum(_policy_count(x.get('candidate_policy'),p['budgets'].get('default_asset_candidates',1)) for x in p['layers']) if 'PSD' in p['deliverables'] else 0
    psd_packaging=1 if 'PSD' in p['deliverables'] else 0
    return {'composition':comps,'target':targets,'layout_lock':layout_lock,'placeholder_photo':placeholder_photo,'layers':layers,'psd_packaging':psd_packaging,'recommended_total':comps+targets+layout_lock+placeholder_photo+layers+psd_packaging,'hard_ceiling':p['budgets']['max_total_generations']}

def _layout_slots(plan):
    contract=plan.get('render_contract') or {}
    layout=contract.get('wireframe_layout')
    return sorted(layout.get('slots',[]),key=lambda slot:slot['slot_id']) if layout else []
