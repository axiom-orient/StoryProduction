#!/usr/bin/env python3
from __future__ import annotations
from pathlib import Path
import argparse, json, sys
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from scene_asset_compilerlib.util import load_json
from scene_asset_compilerlib.workspace import init_run, validate_authority, compile_plan, all_status, begin_attempt
from scene_asset_compilerlib.output_manifest import manifest_outputs
from scene_asset_compilerlib.mechanical_review import mechanical_review
from scene_asset_compilerlib.visual_review_template import visual_template
from scene_asset_compilerlib.repair import make_repair
from scene_asset_compilerlib.publish import approve_attempt, human_approve, publish
from scene_asset_compilerlib.review_contracts import validate_visual_review
from scene_asset_compilerlib.doctor import build_audit_template, validate_output_audit, diagnose, build_reconstruction_plan
from scene_asset_compilerlib.reconstruct import reconstruct
from scene_asset_compilerlib.scene_plan import validate_scene_plan, estimate_generation_budget
from scene_asset_compilerlib.scene_units import init_scene
from scene_asset_compilerlib.scene_locks import create_lock, lock_placeholder, seal_placeholders, lock_layer, seal_assets, validate_psd_entry
from scene_asset_compilerlib.psd_validation import validate_generated_psd
from scene_asset_compilerlib.psd_profile import validate_profile as validate_psd_profile, resolved_profile
from scene_asset_compilerlib.execution import record_execution
from scene_asset_compilerlib.scene_provider import execute_scene_provider
from scene_asset_compilerlib.scene_provider_preflight import probe_scene_provider_protocol


def _print_json(obj): print(json.dumps(obj,ensure_ascii=False,indent=2))

def main(argv=None):
    ap=argparse.ArgumentParser(prog='scene-asset-compiler',description='Codex-native visual asset workflow controller')
    sp=ap.add_subparsers(dest='cmd',required=True)
    x=sp.add_parser('init'); x.add_argument('request'); x.add_argument('run_dir')
    x=sp.add_parser('validate-authority'); x.add_argument('run_dir')
    x=sp.add_parser('compile-plan'); x.add_argument('run_dir')
    x=sp.add_parser('status'); x.add_argument('run_dir')
    x=sp.add_parser('ready'); x.add_argument('run_dir')
    x=sp.add_parser('begin'); x.add_argument('run_dir'); x.add_argument('task_id')
    x=sp.add_parser('manifest'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('record-execution'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int); x.add_argument('draft')
    x=sp.add_parser('execute-scene-provider', help='run one reference-free PNG attempt through the strict Scene Provider v2')
    x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x.add_argument('--provider-bin',required=True); x.add_argument('--quality',choices=['auto','low','medium','high'],required=True)
    x.add_argument('--perfectpixel-bin',required=True); x.add_argument('--component-lock')
    x.add_argument('--timeout-ms',type=int,default=600000); x.add_argument('--dry-run',action='store_true')
    x=sp.add_parser('probe-scene-provider', help='verify the packaged Scene Provider v2 rejects unsupported operations')
    x.add_argument('--provider-bin',required=True); x.add_argument('--component-lock'); x.add_argument('--timeout-seconds',type=float,default=10.0)
    x=sp.add_parser('review'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('visual-template'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('validate-visual'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('repair'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('audit-template'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('validate-audit'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('doctor'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('reconstruction-plan'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('reconstruct'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('approve'); x.add_argument('run_dir'); x.add_argument('task_id'); x.add_argument('attempt',type=int)
    x=sp.add_parser('human-approve'); x.add_argument('run_dir'); x.add_argument('--reviewer',required=True); x.add_argument('--note',default='')
    x=sp.add_parser('publish'); x.add_argument('run_dir')
    x=sp.add_parser('scene-release', help='publish the versioned SceneAssetCompiler to SceneMotionCompiler boundary')
    xsp=x.add_subparsers(dest='scene_release_cmd', required=True)
    y=xsp.add_parser('prepare'); y.add_argument('run_dir'); y.add_argument('request')
    y=xsp.add_parser('approve'); y.add_argument('run_dir'); y.add_argument('--reviewer', required=True); y.add_argument('--note', default='')
    y=xsp.add_parser('validate'); y.add_argument('run_dir')
    y=xsp.add_parser('publish'); y.add_argument('run_dir')
    x=sp.add_parser('scene-plan-check'); x.add_argument('scene_plan')
    x=sp.add_parser('scene-init'); x.add_argument('scene_plan'); x.add_argument('scene_dir')
    x=sp.add_parser('scene-lock'); x.add_argument('scene_dir'); x.add_argument('lock_kind'); x.add_argument('artifact'); x.add_argument('--review',required=True); x.add_argument('--note',default='')
    x=sp.add_parser('scene-placeholder-lock'); x.add_argument('scene_dir'); x.add_argument('slot_id'); x.add_argument('artifact'); x.add_argument('--review',required=True); x.add_argument('--note',default='')
    x=sp.add_parser('scene-seal-placeholders'); x.add_argument('scene_dir')
    x=sp.add_parser('scene-layer-lock'); x.add_argument('scene_dir'); x.add_argument('layer_id'); x.add_argument('artifact'); x.add_argument('--review',required=True); x.add_argument('--note',default='')
    x=sp.add_parser('scene-seal-assets'); x.add_argument('scene_dir')
    x=sp.add_parser('scene-psd-entry-check'); x.add_argument('scene_dir')
    x=sp.add_parser('psd-profile-check'); x.add_argument('profile')
    x=sp.add_parser('psd-validate-profile'); x.add_argument('psd'); x.add_argument('profile'); x.add_argument('--required-layer',action='append',default=[]); x.add_argument('--expected-size')
    x=sp.add_parser('visual-reference', help='prepare/review hash-bound references without generating pixels')
    vsp=x.add_subparsers(dest='visual_reference_cmd',required=True)
    y=vsp.add_parser('prepare');y.add_argument('run_dir');y.add_argument('--draft',required=True);y.add_argument('--out',required=True)
    y=vsp.add_parser('approve');y.add_argument('run_dir');y.add_argument('--pack',required=True);y.add_argument('--reviewer',required=True);y.add_argument('--expected-pack-hash',required=True);y.add_argument('--out',required=True)
    y=vsp.add_parser('inspect');y.add_argument('run_dir');y.add_argument('--pack',required=True);y.add_argument('--approval')
    y=vsp.add_parser('index');y.add_argument('run_dir');y.add_argument('--pack',required=True);y.add_argument('--approval',required=True)
    a=ap.parse_args(sys.argv[1:] if argv is None else argv); rd=Path(getattr(a,'run_dir','')) if hasattr(a,'run_dir') else None
    if a.cmd=='visual-reference':
        from scene_asset_compilerlib import visual_reference as vr
        from scene_asset_compilerlib.asset_release import AssetReleaseError
        try:
            if a.visual_reference_cmd=='prepare':_print_json(vr.prepare(rd,a.draft,a.out))
            elif a.visual_reference_cmd=='approve':_print_json(vr.approve(rd,a.pack,a.reviewer,a.expected_pack_hash,a.out))
            elif a.visual_reference_cmd=='inspect':_print_json(vr.inspect(rd,a.pack,a.approval))
            else:_print_json(vr.memory_index(rd,a.pack,a.approval))
            return 0
        except (AssetReleaseError,OSError,KeyError,TypeError,ValueError) as error:
            _print_json({'ok':False,'error':{'code':getattr(error,'code','visual-reference-invalid'),'message':str(error)}});return 1
    if a.cmd=='init': _print_json(init_run(Path(a.request),Path(a.run_dir)))
    elif a.cmd=='validate-authority': validate_authority(rd); print('PASS')
    elif a.cmd=='compile-plan': _print_json({'tasks':compile_plan(rd)})
    elif a.cmd=='status': _print_json(all_status(rd))
    elif a.cmd=='ready': _print_json([x for x in all_status(rd) if x['status']=='READY'])
    elif a.cmd=='begin': print(begin_attempt(rd,a.task_id))
    elif a.cmd=='manifest': _print_json(manifest_outputs(rd,a.task_id,a.attempt))
    elif a.cmd=='record-execution': _print_json(record_execution(rd,a.task_id,a.attempt,Path(a.draft)))
    elif a.cmd=='execute-scene-provider':
        result=execute_scene_provider(rd,a.task_id,a.attempt,Path(a.provider_bin),quality=a.quality,timeout_ms=a.timeout_ms,dry_run=a.dry_run,perfectpixel_bin=Path(a.perfectpixel_bin),component_lock=Path(a.component_lock) if a.component_lock else None)
        _print_json(result)
        if not a.dry_run and result['status']!='READY_FOR_VISUAL_REVIEW': return 1
    elif a.cmd=='probe-scene-provider':
        _print_json(probe_scene_provider_protocol(Path(a.provider_bin),component_lock=Path(a.component_lock) if a.component_lock else None,timeout_seconds=a.timeout_seconds))
    elif a.cmd=='review': _print_json(mechanical_review(rd,a.task_id,a.attempt))
    elif a.cmd=='visual-template': _print_json(visual_template(rd,a.task_id,a.attempt))
    elif a.cmd=='validate-visual':
        td=rd/'tasks'/a.task_id; ad=td/'attempts'/f'{a.attempt:04d}'; v=load_json(ad/'visual-review.json'); task=load_json(td/'task.json'); validate_visual_review(v,a.task_id,a.attempt,task.get('prompt_profile','PROJECT_STYLE'),task.get('render_contract'),ad); print('PASS')
    elif a.cmd=='repair': print(make_repair(rd,a.task_id,a.attempt))
    elif a.cmd=='audit-template': _print_json(build_audit_template(rd,a.task_id,a.attempt))
    elif a.cmd=='validate-audit':
        v=load_json(rd/'tasks'/a.task_id/'attempts'/f'{a.attempt:04d}'/'output-audit.json'); validate_output_audit(v,a.task_id,a.attempt,True); print('PASS')
    elif a.cmd=='doctor': _print_json(diagnose(rd,a.task_id,a.attempt))
    elif a.cmd=='reconstruction-plan': _print_json(build_reconstruction_plan(rd,a.task_id,a.attempt))
    elif a.cmd=='reconstruct': print(reconstruct(rd,a.task_id,a.attempt))
    elif a.cmd=='approve': _print_json(approve_attempt(rd,a.task_id,a.attempt))
    elif a.cmd=='human-approve': _print_json(human_approve(rd,a.reviewer,a.note))
    elif a.cmd=='publish': _print_json(publish(rd))
    elif a.cmd=='scene-release':
        from scene_asset_compilerlib.asset_release import (
            approve_scene_asset_release,
            prepare_scene_asset_release,
            publish_scene_assets,
        )
        from scene_asset_compilerlib.asset_release_contracts import AssetReleaseError
        from scene_asset_compilerlib.asset_release_validation import validate_scene_asset_release
        try:
            if a.scene_release_cmd=='prepare': _print_json(prepare_scene_asset_release(rd,Path(a.request)))
            elif a.scene_release_cmd=='approve': _print_json(approve_scene_asset_release(rd,a.reviewer,a.note))
            elif a.scene_release_cmd=='validate':
                verified=validate_scene_asset_release(rd)
                _print_json({'status':'VALID','sceneGraphHash':verified['projection']['sceneGraphHash'],'assetPackHash':verified['assetPackHash'],'assetPackAuditHash':verified['assetPackAuditHash'],'bindingManifestHash':verified['bindingManifestHash'],'approvalSubjectHash':verified['approvalSubjectHash']})
            elif a.scene_release_cmd=='publish': _print_json(publish_scene_assets(rd))
        except AssetReleaseError as error:
            _print_json({'ok':False,'error':{'code':error.code,'message':str(error),'details':error.details}})
            return 1
    elif a.cmd=='scene-plan-check':
        plan=load_json(Path(a.scene_plan)); validate_scene_plan(plan); _print_json({'status':'PASS','budget':estimate_generation_budget(plan)})
    elif a.cmd=='scene-init': _print_json(init_scene(Path(a.scene_plan),Path(a.scene_dir)))
    elif a.cmd=='scene-lock': _print_json(create_lock(Path(a.scene_dir),a.lock_kind,Path(a.artifact),Path(a.review),a.note))
    elif a.cmd=='scene-placeholder-lock': _print_json(lock_placeholder(Path(a.scene_dir),a.slot_id,Path(a.artifact),Path(a.review),a.note))
    elif a.cmd=='scene-seal-placeholders': _print_json(seal_placeholders(Path(a.scene_dir)))
    elif a.cmd=='scene-layer-lock': _print_json(lock_layer(Path(a.scene_dir),a.layer_id,Path(a.artifact),Path(a.review),a.note))
    elif a.cmd=='scene-seal-assets': _print_json(seal_assets(Path(a.scene_dir)))
    elif a.cmd=='scene-psd-entry-check': _print_json(validate_psd_entry(Path(a.scene_dir)))
    elif a.cmd=='psd-profile-check': _print_json(resolved_profile(validate_psd_profile(load_json(Path(a.profile)))))
    elif a.cmd=='psd-validate-profile':
        canvas=tuple(int(x) for x in a.expected_size.lower().split('x',1)) if a.expected_size else None
        _print_json(validate_generated_psd(Path(a.psd),load_json(Path(a.profile)),a.required_layer,canvas))
if __name__=='__main__': sys.exit(main() or 0)
