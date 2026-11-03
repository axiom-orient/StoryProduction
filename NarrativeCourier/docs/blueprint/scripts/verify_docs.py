#!/usr/bin/env python3
from pathlib import Path
import sys
sys.dont_write_bytecode = True
import copy, hashlib, importlib.util, json, re
from jsonschema import Draft202012Validator

ROOT=Path(__file__).resolve().parents[1]
CANON=['narrative-package','delivery-request','director-plan','asset-index','delivery-set']
PROTO=['media-backend-request','media-backend-response','agent-executor-request','agent-executor-response']
errors=[]; passes=[]

def read_json(p): return json.loads(p.read_text(encoding='utf-8'))
def fixture_hash(obj):
    # Fixtures intentionally use only JCS-compatible primitive forms (no floats).
    def no_float(v):
        if isinstance(v,float): raise ValueError('fixture float requires a real RFC 8785 implementation')
        if isinstance(v,dict):
            for x in v.values(): no_float(x)
        if isinstance(v,list):
            for x in v: no_float(x)
    no_float(obj)
    return hashlib.sha256(json.dumps(obj,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()).hexdigest()
def schema_errors(schema,data): return list(Draft202012Validator(schema).iter_errors(data))

actual=sorted(p.name for p in (ROOT/'contracts').glob('*.schema.json')); expected=sorted(x+'.schema.json' for x in CANON)
if actual!=expected: errors.append(f'canonical schema set mismatch: {actual}')
else: passes.append('exactly five canonical durable schemas')

for n in CANON:
    s=read_json(ROOT/'contracts'/f'{n}.schema.json'); Draft202012Validator.check_schema(s)
    d=read_json(ROOT/'fixtures/minimal'/f'{n}.json'); es=schema_errors(s,d)
    if es: errors.append(f'{n}: '+ '; '.join(e.message for e in es))
    else: passes.append(f'{n} fixture validates')
for n in PROTO:
    s=read_json(ROOT/'protocol'/f'{n}.schema.json'); Draft202012Validator.check_schema(s)
    d=read_json(ROOT/'fixtures/protocol'/f'{n}.json'); es=schema_errors(s,d)
    if es: errors.append(f'protocol {n}: '+ '; '.join(e.message for e in es))
    else: passes.append(f'protocol {n} fixture validates')

F=ROOT/'fixtures/minimal'
np=read_json(F/'narrative-package.json'); dr=read_json(F/'delivery-request.json'); dp=read_json(F/'director-plan.json'); ai=read_json(F/'asset-index.json'); ds=read_json(F/'delivery-set.json')
checks=[(dr['narrative_package_sha256']==fixture_hash(np),'request→narrative'),(dp['narrative_package_sha256']==fixture_hash(np),'plan→narrative'),(dp['delivery_request_sha256']==fixture_hash(dr),'plan→request'),(ai['narrative_package_sha256']==fixture_hash(np),'assets→narrative'),(ai['director_plan_sha256']==fixture_hash(dp),'assets→plan'),(ds['narrative_package_sha256']==fixture_hash(np),'set→narrative'),(ds['delivery_request_sha256']==fixture_hash(dr),'set→request'),(ds['director_plan_sha256']==fixture_hash(dp),'set→plan'),(ds['asset_index_sha256']==fixture_hash(ai),'set→assets')]
for ok,label in checks:
    if not ok: errors.append('hash chain: '+label)
passes.append('fixture cross-hash chain checked on JCS-compatible subset')

spec=importlib.util.spec_from_file_location('invariants',ROOT/'scripts/validate_invariants.py'); inv=importlib.util.module_from_spec(spec); spec.loader.exec_module(inv)
try: inv.validate_bundle(np,dr,dp,ai,ds); passes.append('cross-document invariant validator passes canonical fixture')
except Exception as e: errors.append('canonical invariant failure: '+str(e))

# Negative gates: two schema-level contradictions + four semantic/cross-document contradictions.
negative_schema=[('agent-executor-ok-with-error.json','agent-executor-response'),('media-backend-describe-empty-result.json','media-backend-response'),('delivery-set-pass-with-semantic-fail.json','delivery-set'),('delivery-request-unsafe-id.json','delivery-request')]
for fn,schema_name in negative_schema:
    d=read_json(ROOT/'fixtures/negative'/fn); s=read_json(ROOT/('protocol' if 'executor' in schema_name or 'backend' in schema_name else 'contracts')/f'{schema_name}.schema.json')
    if not schema_errors(s,d): errors.append(f'negative fixture unexpectedly schema-valid: {fn}')
    else: passes.append(f'negative schema rejects {fn}')
for fn,which in [('delivery-request-duplicate-output-id.json','dr'),('director-plan-dangling-asset-ref.json','dp'),('director-plan-duplicate-unit-id.json','dp'),('asset-index-missing-required.json','ai')]:
    vals={'np':copy.deepcopy(np),'dr':copy.deepcopy(dr),'dp':copy.deepcopy(dp),'ai':copy.deepcopy(ai),'ds':copy.deepcopy(ds)}; vals[which]=read_json(ROOT/'fixtures/negative'/fn)
    try: inv.validate_bundle(vals['np'],vals['dr'],vals['dp'],vals['ai'],vals['ds'])
    except inv.InvariantError: passes.append(f'negative invariant rejects {fn}')
    else: errors.append(f'negative fixture unexpectedly invariant-valid: {fn}')

link_re=re.compile(r'\[[^\]]+\]\((?!https?://|mailto:|#)([^)]+)\)')
for md in ROOT.rglob('*.md'):
    txt=md.read_text(encoding='utf-8')
    for target in link_re.findall(txt):
        target=target.split('#',1)[0]
        if target and not (md.parent/target).resolve().exists(): errors.append(f'broken link {md.relative_to(ROOT)} → {target}')
passes.append('markdown relative links scanned')

joined='\n'.join((ROOT/x).read_text(encoding='utf-8') for x in ['README.md','PRODUCT_SPEC.md','SYSTEM_SPEC.md','docs/03_PIPELINE_AND_RUNTIME.md'])
for marker in ['NarrativePackage','DeliveryRequest','DirectorPlan','AssetIndex','DeliverySet','P0 Validate','P1 Direct','P2 Resolve Assets','P3 Render','P4 Verify + Package','AgentExecutor','MediaBackend']:
    if marker not in joined: errors.append('missing architecture marker: '+marker)
passes.append('5-contract/5-pass/2-boundary markers present')

for p in [ROOT/'README.md',ROOT/'ONE_PAGE.md',ROOT/'SYSTEM_SPEC.md']+list((ROOT/'docs').glob('0[0-6]_*.md')):
    t=p.read_text(encoding='utf-8')
    for banned in ['S00_PACKAGE_PREFLIGHT','NDS.out.','NVC.in.','genre→style preset engine']:
        if banned in t: errors.append(f'legacy core term {banned} in {p.name}')
passes.append('core legacy-term scan')

req=read_json(ROOT/'contracts/delivery-request.schema.json'); out_props=req['$defs']['output_request']['properties']
if 'medium' in out_props: errors.append('DeliveryRequest output still duplicates medium+format')
if 'backend_id' not in out_props: errors.append('DeliveryRequest lacks optional backend pin')
plan=read_json(ROOT/'contracts/director-plan.schema.json'); vg=plan['properties']['visual_grammar']
if 'required' in vg: errors.append('visual grammar axes must be optional-by-use')
for key in ['content','duration_frames']:
    if key not in plan['$defs']['projection_unit']['properties']: errors.append('ProjectionUnit missing '+key)
if 'source' not in plan['$defs']['asset_request']['properties']: errors.append('AssetRequest missing source provenance')
asset=read_json(ROOT/'contracts/asset-index.schema.json')
if 'backend_fingerprint' in asset['properties']: errors.append('AssetIndex still has global backend fingerprint')
for key in ['backend_id','backend_fingerprint']:
    if key not in asset['$defs']['asset']['required']: errors.append('asset provenance missing '+key)
passes.append('v1.2 ambiguity-removal decisions checked')

if errors:
    print('FAIL'); [print(' -',e) for e in errors]; sys.exit(1)
print(f'PASS — {len(passes)} checks'); [print(' -',p) for p in passes]
