#!/usr/bin/env python3
from pathlib import Path
import hashlib, json, sys

ROOT=Path(__file__).resolve().parents[1]
SCRIPTS=ROOT/'scripts'
sys.path.insert(0,str(SCRIPTS))
from scene_asset_compilerlib.contract_primitives import validate_render_contract
from scene_asset_compilerlib.request_contracts import validate_run_request, validate_visual_authority, validate_generation_plan
from scene_asset_compilerlib.scene_plan import validate_scene_plan

errors=[]

# JSON syntax + JSON Schema meta-validation + canonical template semantic validation.
try:
    from jsonschema import Draft202012Validator
except Exception:
    Draft202012Validator=None
    errors.append('JSONSCHEMA_REQUIRED')
for p in list((ROOT/'schemas').glob('*.json'))+list((ROOT/'templates').glob('*.json')):
    try:
        obj=json.loads(p.read_text(encoding='utf-8'))
        if p.parent.name=='schemas' and Draft202012Validator is not None: Draft202012Validator.check_schema(obj)
    except Exception as e: errors.append(f'JSON:{p.relative_to(ROOT)}:{e}')
try:
    validate_run_request(json.loads((ROOT/'templates/run-request.json').read_text()))
    validate_visual_authority(json.loads((ROOT/'templates/visual-authority.json').read_text()))
    validate_generation_plan(json.loads((ROOT/'templates/generation-plan.json').read_text()))
    validate_generation_plan(json.loads((ROOT/'templates/generation-plan-psd.example.json').read_text()))
    validate_render_contract(json.loads((ROOT/'templates/render-contract.example.json').read_text()))
    validate_scene_plan(json.loads((ROOT/'templates/scene-production-plan.example.json').read_text()))
except Exception as e:
    errors.append(f'TEMPLATE_CONTRACT:{e}')

if Draft202012Validator is not None:
    template_schema_pairs=[
      ('scene-production-plan.example.json','scene-production-plan.schema.json'),
      ('character-asset-contract.example.json','character-asset-contract.schema.json'),
      ('psd-generation-profile.example.json','psd-generation-profile.schema.json'),
      ('generation-plan.json','generation-plan.schema.json'),
      ('generation-plan-psd.example.json','generation-plan.schema.json'),
      ('render-contract.example.json','render-contract.schema.json'),
    ]
    try:
        from referencing import Registry, Resource
        resources=[]
        for sp in (ROOT/'schemas').glob('*.json'):
            sobj=json.loads(sp.read_text())
            sid=sobj.get('$id')
            if sid: resources.append((sid,Resource.from_contents(sobj)))
        registry=Registry().with_resources(resources)
    except Exception:
        registry=None
    for tname,sname in template_schema_pairs:
        try:
            schema=json.loads((ROOT/'schemas'/sname).read_text())
            validator=Draft202012Validator(schema,registry=registry) if registry is not None else Draft202012Validator(schema)
            validator.validate(json.loads((ROOT/'templates'/tname).read_text()))
        except Exception as e: errors.append(f'TEMPLATE_SCHEMA:{tname}:{e}')

# Python syntax and no direct Images API implementation in deterministic scripts.
for p in (ROOT/'scripts').rglob('*.py'):
    if '__pycache__' in p.parts: continue
    try: compile(p.read_text(encoding='utf-8'),str(p),'exec')
    except Exception as e: errors.append(f'PYTHON:{p.relative_to(ROOT)}:{e}')
    if p.name=='verify_skill.py': continue
    s=p.read_text(encoding='utf-8',errors='ignore')
    for bad in ['api.openai.com','/images/generations','/images/edits','OPENAI_API_KEY']:
        if bad in s: errors.append(f'DIRECT_IMAGE_API:{p.relative_to(ROOT)}:{bad}')


# Canonical boundary: no local PSD construction/cutout/vectorization modules or commands.
for forbidden in ['scene_asset_compilerlib/psd_writer.py','scene_asset_compilerlib/cutout.py','scene_asset_compilerlib/pathing.py','scene_asset_compilerlib/psd_manifest.py']:
    if (ROOT/'scripts'/forbidden.split('/',1)[1]).exists(): errors.append(f'FORBIDDEN_LOCAL_PSD_PRODUCER:{forbidden}')
cli_text=(ROOT/'scripts/scene_asset_compiler.py').read_text(encoding='utf-8',errors='ignore')
for bad in ["add_parser('psd-build')","add_parser('cutout')","write_layered_psd","apply_matte("]:
    if bad in cli_text: errors.append(f'FORBIDDEN_LOCAL_PSD_COMMAND:{bad}')
for p in (ROOT/'scripts/scene_asset_compilerlib').glob('*.py'):
    if p.name in {'psd.py','psd_profile.py'}: continue
    body=p.read_text(encoding='utf-8',errors='ignore')
    for bad in ['import cv2','from cv2','import PIL','from PIL','import psd_tools','from psd_tools','import openai','from openai']:
        if bad in body: errors.append(f'FORBIDDEN_PRODUCER_DEP:{p.relative_to(ROOT)}:{bad}')

# Skill frontmatter sanity.
skill=ROOT/'SKILL.md'
try:
    lines=skill.read_text(encoding='utf-8').splitlines()
    if not lines or lines[0].strip()!='---': errors.append('SKILL_FRONTMATTER:missing opening delimiter')
    end=lines[1:].index('---')+1
    front='\n'.join(lines[1:end])
    if 'name: scene-asset-compiler' not in front: errors.append('SKILL_FRONTMATTER:name')
    if 'description:' not in front: errors.append('SKILL_FRONTMATTER:description')
except Exception as e: errors.append(f'SKILL_FRONTMATTER:{e}')

# Source manifest must be exact: no missing, stale, or untracked files.
manifest=ROOT/'SOURCE_MANIFEST.sha256'
expected={p.relative_to(ROOT).as_posix() for p in ROOT.rglob('*') if p.is_file() and p!=manifest and '__pycache__' not in p.parts and p.suffix!='.pyc'}
tracked={}
if not manifest.exists():
    errors.append('MANIFEST_MISSING')
else:
    for line in manifest.read_text(encoding='utf-8').splitlines():
        if not line.strip(): continue
        try: h,rel=line.split('  ',1)
        except ValueError: errors.append(f'MANIFEST_LINE:{line}'); continue
        if rel in tracked: errors.append(f'MANIFEST_DUPLICATE:{rel}')
        tracked[rel]=h
        p=ROOT/rel
        if not p.is_file(): errors.append(f'MANIFEST_MISSING_FILE:{rel}'); continue
        actual=hashlib.sha256(p.read_bytes()).hexdigest()
        if actual!=h: errors.append(f'MANIFEST_HASH:{rel}')
    for rel in sorted(expected-set(tracked)): errors.append(f'MANIFEST_UNTRACKED:{rel}')
    for rel in sorted(set(tracked)-expected): errors.append(f'MANIFEST_EXTRA:{rel}')

if errors:
    print('FAIL'); print('\n'.join(errors)); sys.exit(1)
print(f'PASS files={len(expected)}')
