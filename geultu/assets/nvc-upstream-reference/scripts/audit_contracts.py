#!/usr/bin/env python3
import json, sys, warnings
warnings.filterwarnings('ignore', category=DeprecationWarning)
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
try:
    from jsonschema import Draft202012Validator, RefResolver
except Exception as e:
    print('ERROR jsonschema dependency:',e); sys.exit(2)

def load(p): return json.loads(p.read_text(encoding='utf-8'))
errors=[]; warnings=[]
# 1. schema meta-validation
schemas={p.name:load(p) for p in sorted((ROOT/'schemas').glob('*.json'))}
for name,s in schemas.items():
    try: Draft202012Validator.check_schema(s)
    except Exception as e: errors.append(f'SCHEMA_INVALID {name}: {e}')
# 2. stage contract validation
stage_schema=schemas['stage-contract.schema.json']; stage_v=Draft202012Validator(stage_schema)
stages={}
for p in sorted((ROOT/'contracts/stages').glob('*.json')):
    o=load(p); sid=o.get('stage_id')
    for e in stage_v.iter_errors(o): errors.append(f'STAGE_SCHEMA {p.name}: {e.message} path={list(e.path)}')
    if sid in stages: errors.append(f'DUP_STAGE {sid}')
    stages[sid]=o
# 3. stage refs: schemas, failures, next, substep failure subset
fcatalog=load(ROOT/'catalogs/failure-codes.json'); fcodes={x['code'] for x in fcatalog['codes']}
for sid,s in stages.items():
    for io in s['inputs']+s['outputs']:
        if not (ROOT/io['schema']).exists(): errors.append(f'DANGLING_SCHEMA {sid}:{io["schema"]}')
    for c in s['failure_codes']:
        if c not in fcodes: errors.append(f'DANGLING_FAILURE {sid}:{c}')
    for sub in s['substeps']:
        for c in sub['failure_codes']:
            if c not in fcodes: errors.append(f'DANGLING_SUBSTEP_FAILURE {sid}/{sub["step_id"]}:{c}')
            if c not in s['failure_codes']:
                errors.append(f'SUBSTEP_FAILURE_NOT_DECLARED_STAGE {sid}/{sub["step_id"]}:{c}')
    nxt=s['next']['on_pass']
    if nxt is not None and nxt not in stages: errors.append(f'DANGLING_NEXT {sid}->{nxt}')
# 4. linear DAG cycle/reachability
start='S00_PACKAGE_PREFLIGHT'; seen=[]; cur=start
while cur is not None:
    if cur in seen:
        errors.append('DAG_CYCLE '+' -> '.join(seen+[cur])); break
    if cur not in stages:
        errors.append(f'DAG_MISSING_STAGE {cur}'); break
    seen.append(cur); cur=stages[cur]['next']['on_pass']
if set(seen)!=set(stages): errors.append('UNREACHABLE_STAGES '+repr(sorted(set(stages)-set(seen))))
# 5. one producer per output kind
producer={}
for sid,s in stages.items():
    for out in s['outputs']:
        k=out['kind']
        if k in producer: errors.append(f'OUTPUT_KIND_MULTI_OWNER {k}:{producer[k]}|{sid}')
        producer[k]=sid
# 6. artifact catalog
ac=load(ROOT/'catalogs/artifact-catalog.json')
for k,p in ac.items():
    if not (ROOT/p).exists(): errors.append(f'ARTIFACT_CATALOG_DANGLING {k}:{p}')
# 7. prompt specs
pv=Draft202012Validator(schemas['prompt-spec.schema.json'])
for p in sorted((ROOT/'contracts/prompts').glob('*.json')):
    o=load(p)
    for e in pv.iter_errors(o): errors.append(f'PROMPT_SCHEMA {p.name}: {e.message}')
    if o['stage_id'] not in stages: errors.append(f'PROMPT_STAGE_DANGLING {p.name}:{o["stage_id"]}')
    if not (ROOT/o['output_schema']).exists(): errors.append(f'PROMPT_OUTPUT_SCHEMA_DANGLING {p.name}:{o["output_schema"]}')
# 8. director presets
dv=Draft202012Validator(schemas['director-preset.schema.json'])
for p in sorted((ROOT/'directors').glob('*.json')):
    for e in dv.iter_errors(load(p)): errors.append(f'DIRECTOR_SCHEMA {p.name}: {e.message}')

# 8b. external port contracts
port_v=Draft202012Validator(schemas['port-contract.schema.json'])
for p in sorted((ROOT/'contracts/external').glob('*.json')):
    o=load(p)
    for e in port_v.iter_errors(o): errors.append(f'PORT_SCHEMA {p.name}: {e.message}')
    if not (ROOT/o['schema']).exists(): errors.append(f'PORT_SCHEMA_DANGLING {p.name}:{o["schema"]}')
# 8c. invariant catalog integrity
invcat=load(ROOT/'catalogs/invariants.json')
inv_ids=[x['id'] for x in invcat['invariants']]
if len(inv_ids)!=len(set(inv_ids)): errors.append('DUP_INVARIANT_ID')

# 9. fixture schema validation (resolve local refs)
fixture_map={
 'narrative-package.json':'narrative-package.schema.json','story-ir.json':'story-ir.schema.json','meaning-spec.json':'meaning-spec.schema.json',
 'director-ir.json':'director-ir.schema.json','adaptation-ir.json':'adaptation-ir.schema.json','scene-ir.json':'scene-ir.schema.json',
 'shot-ir.json':'shot-ir.schema.json','asset-manifest.json':'asset-manifest.schema.json'}
store={str((ROOT/'schemas'/n).resolve().as_uri()):s for n,s in schemas.items()}
for fn,sn in fixture_map.items():
    inst=load(ROOT/'fixtures/shadow-story'/fn); sch=schemas[sn]
    resolver=RefResolver(base_uri=(ROOT/'schemas'/sn).resolve().as_uri(),referrer=sch,store=store)
    v=Draft202012Validator(sch,resolver=resolver)
    for e in v.iter_errors(inst): errors.append(f'FIXTURE_SCHEMA {fn}: {e.message} path={list(e.path)}')
# 10. fixture cross refs
shot=load(ROOT/'fixtures/shadow-story/shot-ir.json'); assets=load(ROOT/'fixtures/shadow-story/asset-manifest.json')
aids={a['asset_id'] for a in assets['assets']}
for sh in shot['shots']:
    for a in sh['asset_refs']:
        if a not in aids: errors.append(f'FIXTURE_ASSET_DANGLING {sh["shot_id"]}:{a}')
ms=load(ROOT/'fixtures/shadow-story/meaning-spec.json'); obligations={x['obligation_id'] for x in ms['transition_obligations']}
hard={x['obligation_id'] for x in ms['transition_obligations'] if x['criticality']=='hard'}
used={}
for fn,key in [('adaptation-ir.json','selected_scenes'),('scene-ir.json','scenes'),('shot-ir.json','shots')]:
    data=load(ROOT/'fixtures/shadow-story'/fn); got=set()
    for x in data[key]:
        for ob in x.get('obligations',[]):
            got.add(ob)
            if ob not in obligations: errors.append(f'FIXTURE_OBLIGATION_DANGLING {fn}:{ob}')
    used[fn]=got
if not hard <= used['adaptation-ir.json']: errors.append('HARD_OBLIGATION_MISSING_ADAPTATION '+repr(sorted(hard-used['adaptation-ir.json'])))
if not hard <= used['shot-ir.json']: errors.append('HARD_OBLIGATION_MISSING_SHOT '+repr(sorted(hard-used['shot-ir.json'])))
# 11. no unused fixture assets
consumed={a for sh in shot['shots'] for a in sh['asset_refs']}
unused=aids-consumed
if unused: errors.append('FIXTURE_UNUSED_ASSETS '+repr(sorted(unused)))
# output
print(f'Schemas: {len(schemas)}')
print(f'Stages: {len(stages)}')
print(f'Substeps: {sum(len(s["substeps"]) for s in stages.values())}')
print(f'Prompt specs: {len(list((ROOT/"contracts/prompts").glob("*.json")))}')
print(f'Director presets: {len(list((ROOT/"directors").glob("*.json")))}')
print(f'Failure codes: {len(fcodes)}')
print(f'Runtime invariants: {len(load(ROOT/"catalogs/invariants.json")["invariants"])}')
print(f'External ports: {len(list((ROOT/"contracts/external").glob("*.json")))}')
print(f'Fixture artifacts: {len(fixture_map)}')
print('Stage chain:', ' -> '.join(seen))
if warnings:
    print('WARNINGS:')
    for w in warnings: print(' -',w)
if errors:
    print('AUDIT: FAIL')
    for e in errors: print(' -',e)
    sys.exit(1)
print('AUDIT: PASS')
