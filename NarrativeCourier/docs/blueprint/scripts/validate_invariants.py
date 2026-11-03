#!/usr/bin/env python3
from __future__ import annotations
from pathlib import Path
import sys
sys.dont_write_bytecode = True
import json

class InvariantError(ValueError): pass

def safe_delivery_id(value, label):
    import re
    if not isinstance(value, str) or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,119}", value) is None:
        raise InvariantError(f"{label} must be a safe path segment")

def unique(values, label):
    seen=set(); dup=[]
    for v in values:
        if v in seen and v not in dup: dup.append(v)
        seen.add(v)
    if dup: raise InvariantError(f"{label} duplicates: {dup}")

def validate_artifact_paths(obj):
    def walk(v):
        if isinstance(v, dict):
            if 'path' in v and 'sha256' in v and isinstance(v['path'], str):
                p=v['path'].replace('\\','/')
                if p.startswith('/') or ':' in p.split('/')[0] or any(x == '..' for x in p.split('/')):
                    raise InvariantError(f"artifact path must be project-relative: {v['path']}")
            for x in v.values(): walk(x)
        elif isinstance(v, list):
            for x in v: walk(x)
    walk(obj)

def validate_bundle(np, dr, dp, ai, ds):
    validate_artifact_paths(np); validate_artifact_paths(dr); validate_artifact_paths(dp); validate_artifact_paths(ai); validate_artifact_paths(ds)
    safe_delivery_id(dr['request_id'],'DeliveryRequest.request_id')
    output_ids=[o['output_id'] for o in dr['outputs']]; unique(output_ids,'DeliveryRequest.output_id')
    for output_id in output_ids: safe_delivery_id(output_id,'DeliveryRequest.output_id')
    asset_inputs=dr.get('asset_inputs',[]); input_ids=[a['asset_id'] for a in asset_inputs]; unique(input_ids,'DeliveryRequest.asset_input.asset_id')
    for a in asset_inputs:
        safe_delivery_id(a['asset_id'],'DeliveryRequest.asset_input.asset_id')
        if a['generation_mode'] not in {'USER','DERIVED'}: raise InvariantError(f"{a['asset_id']} generation_mode must be USER or DERIVED")
    asset_req_ids=[a['asset_id'] for a in dp['asset_requests']]; unique(asset_req_ids,'DirectorPlan.asset_id')
    asset_req_set=set(asset_req_ids)
    plan_output_ids=[d['output_request_id'] for d in dp['deliveries']]; unique(plan_output_ids,'DirectorPlan.output_request_id')
    if set(plan_output_ids) != set(output_ids): raise InvariantError('DirectorPlan deliveries must match DeliveryRequest outputs exactly')
    req_by_id={o['output_id']:o for o in dr['outputs']}
    all_units=[]
    for delivery in dp['deliveries']:
        unit_ids=[u['unit_id'] for u in delivery['units']]; unique(unit_ids,f"{delivery['output_request_id']}.unit_id")
        orders=[u['order'] for u in delivery['units']]; unique(orders,f"{delivery['output_request_id']}.order")
        if sorted(orders) != list(range(len(orders))): raise InvariantError(f"{delivery['output_request_id']}.order must be contiguous from 0")
        for u in delivery['units']:
            dangling=sorted(set(u.get('asset_refs',[]))-asset_req_set)
            if dangling: raise InvariantError(f"{u['unit_id']} dangling asset_refs: {dangling}")
        if delivery['presentation_structure']=='TIMED':
            expected=req_by_id[delivery['output_request_id']].get('duration_frames')
            actual=sum(u['duration_frames'] for u in delivery['units'])
            if expected is None or actual != expected: raise InvariantError(f"{delivery['output_request_id']} timed duration mismatch: plan={actual}, request={expected}")
        all_units.extend(unit_ids)
    unique(all_units,'DirectorPlan global unit_id')

    assets=ai['assets']; unique([a['asset_id'] for a in assets],'AssetIndex.asset_id')
    source_ids=[a['source_request_id'] for a in assets]; unique(source_ids,'AssetIndex.source_request_id')
    if set(source_ids) != asset_req_set: raise InvariantError('AssetIndex must materialize every DirectorPlan AssetRequest exactly once')
    req_by_asset={a['asset_id']:a for a in dp['asset_requests']}
    for a in assets:
        if a['source_request_id'] not in asset_req_set: raise InvariantError(f"AssetIndex unknown source_request_id: {a['source_request_id']}")
        if a['kind'] != req_by_asset[a['source_request_id']]['kind']: raise InvariantError(f"AssetIndex kind mismatch: {a['source_request_id']}")

    delivery_ids=[d['output_request_id'] for d in ds['deliveries']]; unique(delivery_ids,'DeliverySet.output_request_id')
    if set(delivery_ids) != set(output_ids): raise InvariantError('DeliverySet deliveries must match DeliveryRequest outputs exactly')
    for d in ds['deliveries']:
        v=d['verification']
        if d['status']=='PASS':
            if v['semantic']['status']!='PASS' or v['technical']['status']!='PASS' or v['visual']['status'] not in {'PASS','NOT_APPLICABLE'}:
                raise InvariantError(f"{d['output_request_id']} false PASS")
        for layer_name, layer in v.items():
            if layer['status'] in {'PASS','NOT_APPLICABLE'} and layer['findings']:
                raise InvariantError(f"{d['output_request_id']}.{layer_name} {layer['status']} cannot carry findings")
            if layer['status'] in {'WARN','FAIL'} and not layer['findings']:
                raise InvariantError(f"{d['output_request_id']}.{layer_name} {layer['status']} requires evidence")
    if ds['status']=='PASS' and any(d['status']!='PASS' for d in ds['deliveries']): raise InvariantError('DeliverySet false PASS')

def load(p): return json.loads(Path(p).read_text(encoding='utf-8'))

def main(argv):
    if len(argv)!=6:
        print('usage: validate_invariants.py NARRATIVE DELIVERY_REQUEST DIRECTOR_PLAN ASSET_INDEX DELIVERY_SET',file=sys.stderr); return 2
    try: validate_bundle(*(load(p) for p in argv[1:]))
    except InvariantError as e:
        print('FAIL —',e); return 1
    print('PASS — cross-document invariants'); return 0
if __name__=='__main__': raise SystemExit(main(sys.argv))
