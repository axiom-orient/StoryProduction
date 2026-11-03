#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import json, os, subprocess, sys, tempfile

from release_verifier_support import GateError, sha
from standalone_bundle_verifier import (
    STANDALONE_DOCS,
    STANDALONE_INSTALL_LINES,
    STANDALONE_PACKAGE_NAME,
    _extracted_integration_smoke,
    _standalone_probe_block,
    run_bounded,
    verify_standalone_bundle,
)

ROOT=Path(__file__).resolve().parents[1]
SKILL=ROOT/'.agents/skills/scene-asset-compiler'
PYTHON=sys.executable


def run(cmd, *, cwd=ROOT):
    env=os.environ.copy(); env['PYTHONDONTWRITEBYTECODE']='1'
    p=subprocess.run(cmd,cwd=cwd,text=True,capture_output=True,env=env)
    if p.returncode:
        raise GateError(f"COMMAND_FAIL:{' '.join(map(str,cmd))}\n{p.stdout}\n{p.stderr}")
    return p.stdout+p.stderr


def main():
    gates=[]

    # 1 clean source scan
    bad=[]
    for p in ROOT.rglob('*'):
        rel=p.relative_to(ROOT).as_posix()
        if p.is_symlink(): bad.append(f'SYMLINK:{rel}')
        if p.name=='.DS_Store' or p.name.startswith('._') or p.name=='__pycache__' or p.suffix=='.pyc': bad.append(f'JUNK:{rel}')
        if '..' in Path(rel).parts: bad.append(f'PATH_ESCAPE:{rel}')
    if bad: raise GateError('CLEAN_SCAN:'+','.join(bad))
    gates.append(('G01','PASS','clean source scan'))

    # 2 python syntax
    pyfiles=[p for p in ROOT.rglob('*.py') if '__pycache__' not in p.parts]
    for p in pyfiles: compile(p.read_text(encoding='utf-8'),str(p),'exec')
    gates.append(('G02','PASS',f'python syntax files={len(pyfiles)}'))

    # 3 JSON parse
    jsonfiles=list(ROOT.rglob('*.json'))
    for p in jsonfiles: json.loads(p.read_text(encoding='utf-8'))
    gates.append(('G03','PASS',f'json parse files={len(jsonfiles)}'))

    # 4 schema meta validation
    try:
        from jsonschema import Draft202012Validator
    except Exception as e:
        raise GateError('JSONSCHEMA_REQUIRED_FOR_RELEASE') from e
    schemas=list((SKILL/'schemas').glob('*.json'))
    for p in schemas: Draft202012Validator.check_schema(json.loads(p.read_text()))
    gates.append(('G04','PASS',f'Draft202012 schemas={len(schemas)}'))

    # 5 canonical templates + skill verifier (manifest handled again at G11)
    out=run([PYTHON,str(SKILL/'scripts/verify_skill.py')])
    gates.append(('G05','PASS','canonical template/schema/semantic validation'))

    # 6 regression tests
    out_tests=run([PYTHON,'-m','unittest','discover','-s','tests','-v'])
    import re
    m=re.search(r'Ran (\d+) tests',out_tests)
    count=int(m.group(1)) if m else -1
    if count <= 0: raise GateError('REGRESSION_TESTS_MISSING_OR_NOT_DISCOVERED')
    gates.append(('G06','PASS',f'unittest={count}'))

    # 7 scene CLI smoke
    plan=SKILL/'templates/scene-production-plan.example.json'
    check=json.loads(run([PYTHON,str(SKILL/'scripts/scene_asset_compiler.py'),'scene-plan-check',str(plan)]))
    with tempfile.TemporaryDirectory() as td:
        sd=Path(td)/'scene'
        run([PYTHON,str(SKILL/'scripts/scene_asset_compiler.py'),'scene-init',str(plan),str(sd)])
        idx=json.loads((sd/'units/index.json').read_text())
        units=[json.loads((sd/x).read_text()) for x in idx['units']]
        psd=[u for u in units if u['stage']=='PSD_PACKAGING']
        if len(psd)!=1: raise GateError('PSD_PACKAGING_UNIT_COUNT')
        capsule=(sd/'units/psd/01.md').read_text()
    gates.append(('G07','PASS',f"scene graph recommended={check['budget']['recommended_total']} PSD_PACKAGING=1"))

    # 8 PSD capsule/profile smoke
    required=['real editable Adobe Photoshop PSD','embedded sRGB IEC61966-2.1','pixel/user layer masks','layer vector mask','clip.character.white_dog','clip.foreground.plants','do NOT set a document clipping path']
    for x in required:
        if x not in capsule: raise GateError(f'PSD_CAPSULE_MISSING:{x}')
    if 'clip.background.backplate' in capsule: raise GateError('PSD_CAPSULE_MEANINGLESS_BACKGROUND_PATH')
    profile=json.loads((SKILL/'templates/psd-generation-profile.example.json').read_text())
    sys.path.insert(0,str(SKILL/'scripts'))
    from scene_asset_compilerlib.psd_profile import resolved_profile
    rp=resolved_profile(profile)
    if rp['path_layers'][0]['work_path_tolerance_px']!=1.5: raise GateError('PSD_PROFILE_TOLERANCE')
    gates.append(('G08','PASS','PSD generation capsule/profile'))

    # 9 real PSD postflight regression: re-read the actual legacy PSD bytes.
    fixture_root=ROOT/'tests/fixtures/legacy-psd'
    provenance_path=fixture_root/'PROVENANCE.json'
    if not provenance_path.is_file():
        raise GateError(f'G09_SOURCE_EVIDENCE_MISSING:{provenance_path.relative_to(ROOT).as_posix()}')
    try:
        provenance=json.loads(provenance_path.read_text(encoding='utf-8'))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise GateError(
            f'G09_SOURCE_EVIDENCE_INVALID:{provenance_path.relative_to(ROOT).as_posix()}:{error}'
        ) from error
    sys.path.insert(0,str(SKILL/'scripts'))
    from scene_asset_compilerlib.psd import validate_generated_psd
    for item in provenance['fixtures']:
        fp=fixture_root/item['file']
        if not fp.is_file():
            raise GateError(f'G09_PSD_FIXTURE_MISSING:{item["file"]}')
        if sha(fp)!=item['sha256']:
            raise GateError(f'LEGACY_PSD_FIXTURE_HASH:{item["file"]}')
        profile_path=ROOT/item['profile']
        if not profile_path.is_file():
            raise GateError(f'G09_PSD_PROFILE_MISSING:{item["profile"]}')
        profile=json.loads(profile_path.read_text(encoding='utf-8'))
        # Required layers are exactly the declared path layers plus any known opaque background from scene fixture.
        required=[x['layer_id'] for x in profile['path_layers']]
        if item['file'].startswith('scene_'):
            required=['background.backplate']+required
        r=validate_generated_psd(fp,profile,required)
        if r.get('findings')!=item['expected_findings']:
            raise GateError(f'LEGACY_PSD_REGRESSION:{item["file"]}:{r.get("findings")}')
    gates.append(('G09','PASS',f'real PSD postflight regression fixtures={len(provenance["fixtures"])}; expected ICC defect reproduced'))

    # 10 boundary scan is enforced by verify_skill, add whole-tree producer-file check
    forbidden=['cutout.py','pathing.py','psd_writer.py','psd_manifest.py','assemble_psd.py']
    for name in forbidden:
        hits=[p for p in SKILL.rglob(name)]
        if hits: raise GateError(f'LOCAL_PSD_PRODUCER:{name}')
    gates.append(('G10','PASS','no direct image API/local PSD producer'))

    # 11 exact skill, documentation, evidence, and release manifests.  The
    # shared verifier owns the exclusion rules so G11 cannot drift from the
    # documented generation command.
    manifest_report=json.loads(run([PYTHON,str(ROOT/'tools/source_manifest.py'),'verify']))
    if manifest_report.get('status')!='PASS':
        raise GateError(f'MANIFEST_VERIFY:{json.dumps(manifest_report,ensure_ascii=False,sort_keys=True)}')
    release_report=manifest_report['release']
    gates.append(('G11','PASS',f"exact manifests release_files={release_report['expectedFileCount']}"))

    print('\n'.join(f'{g} {s} {d}' for g,s,d in gates))
    print('G12 EXTERNAL package ZIP CRC/SHA-256 gate is executed after archive creation')

if __name__=='__main__':
    try:
        if len(sys.argv)==3 and sys.argv[1]=='--standalone-bundle':
            verify_standalone_bundle(sys.argv[2])
        else:
            main()
    except GateError as error:
        print(f'FAIL {error}', file=sys.stderr)
        raise SystemExit(1)
    except Exception as e:
        print(f'FAIL UNEXPECTED {e}', file=sys.stderr)
        raise
