"""Initialize scenes and compile deterministic GenerationUnits and capsules."""
from __future__ import annotations

from pathlib import Path
import shutil

from .util import load_json, atomic_json, sha256_file
from .psd_profile import prompt_lines as psd_prompt_lines
from .render_contract_prompt import prompt_lines
from .scene_plan import _layout_slots, _policy_count, estimate_generation_budget, validate_scene_plan


def init_scene(plan_path:Path, scene_dir:Path):
    p=validate_scene_plan(load_json(plan_path)); scene_dir.mkdir(parents=True,exist_ok=False)
    shutil.copy2(plan_path,scene_dir/'scene-plan.json')
    for d in ('units/composition','units/target','units/layout','units/placeholders','units/layers','units/psd','locks','locks/placeholders','locks/layers','psd/validation'):
        (scene_dir/d).mkdir(parents=True,exist_ok=True)
    atomic_json(scene_dir/'scene.json',{'schema_version':1,'scene_id':p['scene_id'],'plan_sha256':sha256_file(plan_path),'locks':[]})
    units=compile_units(scene_dir,p)
    return {'scene_dir':str(scene_dir),'budget':estimate_generation_budget(p),'units':units}

def _capsule(p,stage,purpose,variant,layer=None,slot_id=None):
    vb=p['visual_brief']
    lines=[
        f"# Generation Capsule — {p['scene_id']} / {stage} / {variant}", '',
        f'Purpose: {purpose}', '',
        f"Primary request: {vb.get('primary_request',vb.get('narrative_function',''))}",
        f"Exact output canvas: {p['output']['size']}",
        f"Output format: {p['output']['target_format']}",
    ]
    for key,label in [('first_read','First read'),('composition','Composition'),('lighting','Lighting'),('color','Color'),('style_medium','Style/medium')]:
        if vb.get(key): lines.append(f'{label}: {vb[key]}')
    if p.get('render_contract'):
        lines += ['', *prompt_lines(
            p['render_contract'],
            transparent_background=bool(layer and layer.get('semantic_role')!='BACKGROUND'),
            slot_id=slot_id,
        )]
    if slot_id:
        lines += ['', f'Placeholder-photo slot: {slot_id}.', 'Generate this placeholder photo independently before PSD packaging; do not merge it with another slot.']
    if layer:
        lines += [
            '', f"Output semantic layer: {layer['layer_id']}",
            f"Role: {layer['semantic_role']}",
            f"Generation intent: {layer.get('generation_intent','')}",
            f"Occlusion policy: {layer['occlusion_policy']}",
            f"Registration: {layer['registration_policy']}",
            f"Transparency: {'opaque full-canvas backplate' if layer['semantic_role']=='BACKGROUND' else 'genuine transparent alpha outside the semantic subject'}",
            'Return at the exact scene canvas without trimming or rescaling; preserve locked registration coordinates.',
        ]
        profile=layer.get('prompt_profile','PROJECT_STYLE')
        lines += ['',f'Prompt profile: {profile}']
        if profile=='PHOTOREAL_LAYER_ASSET':
            lines += [
                '- Render as a physically plausible photograph, not illustration, CGI, concept art, or a product mockup.',
                '- Preserve natural camera perspective, coherent depth of field, motivated light direction, contact/occlusion cues, and material-specific surface response.',
                '- Keep anatomy, joints, hands/paws, silhouette, scale, and subject-specific attributes coherent; do not add or merge limbs or objects.',
                '- Isolate the complete semantic subject with genuine transparent pixels; no checkerboard, matte, halo, backdrop, text, or baked cast shadow unless the layer role explicitly requests a shadow.',
                '- Keep fine boundary detail such as hair, fur, and translucent edges in soft alpha rather than hard clipping.',
            ]
        elif profile=='CUSTOM':
            lines.append('- Apply only the explicit custom style and rendering constraints in this scene plan; do not infer an unrelated house style.')
    lines += ['','Constraints:']+[f'- {x}' for x in vb.get('hard_constraints',[])]+['','Do not include workflow/debug/UI/JSON content in the image.']
    return '\n'.join(lines)+'\n'

def _psd_capsule(p):
    layer_ids=[x["layer_id"] for x in p["layers"]]
    path_layers=[]
    for x in p["layers"]:
        if x.get("path_boundary") in ("SAVED_PATH","SOFT_ENVELOPE"):
            path_layers.append({"layer_id":x["layer_id"],"edge_class":x["edge_class"]})
    profile={
      "schema_version":1,
      "document_role":"SCENE_MASTER",
      "path_layers":path_layers,
      "primary_cutout_layer_id":None,
      "document_clipping_path":"NONE",
    }
    lines=[
      f"# Generation Capsule — {p['scene_id']} / PSD_PACKAGING", "",
      "Purpose: package the already-approved scene assets into the final editable PSD without creative redesign.",
      "Input images: use the approved TARGET_LOCK render only as composition/reference evidence and every approved LAYER_LOCK asset as the actual layer source.",
      "Pixel preservation: preserve approved layer pixels, registration, scale, z-order, color and alpha exactly; do not redraw, restyle, relight, recompose, inpaint or invent hidden content during packaging.",
      "Scene coordinate rule: use the exact locked scene canvas and placements. Do not trim layers.",
      f"Exact PSD canvas: {p['output']['size']}.",
      "Primary output format: PSD. Any PNG is preview-only and cannot satisfy this unit.",
      "Packaging retry discipline: PREFERRED_OUTPUT_MISSING is a format/package failure, not a visual failure. Permit at most one PSD-only retry; reuse approved locked pixels and layout exactly; do not redraw, restyle, relight, recompose, or assemble PSD locally. If PSD is missing a second time, stop and retain previews/evidence; do not claim PSD unsupported.",
      "",
    ]
    if p.get('render_contract'):
        lines.extend(prompt_lines(p['render_contract']))
        if _layout_slots(p):
            lines.append("Layout evidence: use the approved LAYOUT_LOCK and every PLACEHOLDER_LOCK in PLACEHOLDER_SET_LOCK as slot-specific geometry/content evidence; preserve the sealed bounds, focal points, crop-safe insets, z-order, and tolerances.")
        lines.append('')
    lines.extend(psd_prompt_lines(profile,layer_ids))
    return "\n".join(lines)+"\n"

def compile_units(scene_dir:Path,p=None):
    p=p or validate_scene_plan(load_json(scene_dir/'scene-plan.json')); units=[]
    for c in p['composition_candidates']:
        u={
            'schema_version':1,
            'unit_id':f"GU.{p['scene_id']}.composition.{c['candidate_id']}",
            'scene_id':p['scene_id'],'stage':'COMPOSITION',
            'purpose':c.get('variation_intent','composition exploration'),
            'candidate_index':1,'depends_on_locks':[],
            'output_role':'COMPOSITION_CANDIDATE','stop_condition':'ONE_APPROVABLE_CANDIDATE',
        }
        path=scene_dir/'units/composition'/f"{c['candidate_id']}.json"
        atomic_json(path,u); path.with_suffix('.md').write_text(_capsule(p,'COMPOSITION',u['purpose'],c['candidate_id']),encoding='utf-8')
        units.append(str(path.relative_to(scene_dir)))
    n=_policy_count(p['target_candidate_policy'],2)
    for i in range(1,n+1):
        u={'schema_version':1,'unit_id':f"GU.{p['scene_id']}.target.{i:02d}",'scene_id':p['scene_id'],'stage':'TARGET','purpose':'integrated scene target','candidate_index':i,'depends_on_locks':['COMPOSITION_LOCK'],'output_role':'TARGET_CANDIDATE','stop_condition':'ONE_APPROVABLE_CANDIDATE'}
        path=scene_dir/'units/target'/f'{i:02d}.json'; atomic_json(path,u); path.with_suffix('.md').write_text(_capsule(p,'TARGET',u['purpose'],f'{i:02d}'),encoding='utf-8')
        units.append(str(path.relative_to(scene_dir)))
    layout=p.get('render_contract',{}).get('wireframe_layout') if p.get('render_contract') else None
    layout_dependencies=['LAYOUT_LOCK','PLACEHOLDER_SET_LOCK'] if layout else []
    if layout:
        u={'schema_version':1,'unit_id':f"GU.{p['scene_id']}.layout.01",'scene_id':p['scene_id'],'stage':'LAYOUT_LOCK','purpose':'lock wireframe slot geometry before placeholder-photo generation','candidate_index':1,'depends_on_locks':['TARGET_LOCK'],'output_role':'LAYOUT_LOCK','stop_condition':'ONE_APPROVABLE_LAYOUT_LOCK'}
        path=scene_dir/'units/layout'/'01.json'; atomic_json(path,u); path.with_suffix('.md').write_text(_capsule(p,'LAYOUT_LOCK',u['purpose'],'01'),encoding='utf-8')
        units.append(str(path.relative_to(scene_dir)))
        for slot in sorted(layout['slots'],key=lambda x:x['slot_id']):
            slot_id=slot['slot_id']; d=scene_dir/'units/placeholders'/slot_id; d.mkdir(parents=True,exist_ok=True)
            u={'schema_version':1,'unit_id':f"GU.{p['scene_id']}.placeholder.{slot_id}.01",'scene_id':p['scene_id'],'stage':'PLACEHOLDER_PHOTO','purpose':f"independent placeholder photo for layout slot {slot_id}",'candidate_index':1,'depends_on_locks':['LAYOUT_LOCK'],'output_role':'PLACEHOLDER_PHOTO','slot_id':slot_id,'stop_condition':'ONE_APPROVABLE_PLACEHOLDER_PHOTO'}
            path=d/'01.json'; atomic_json(path,u); path.with_suffix('.md').write_text(_capsule(p,'PLACEHOLDER_PHOTO',u['purpose'],'01',slot_id=slot_id),encoding='utf-8')
            units.append(str(path.relative_to(scene_dir)))
    if 'PSD' in p['deliverables']:
        for layer in p['layers']:
            n=_policy_count(layer.get('candidate_policy'),p['budgets'].get('default_asset_candidates',1))
            d=scene_dir/'units/layers'/layer['layer_id']; d.mkdir(parents=True,exist_ok=True)
            for i in range(1,n+1):
                u={'schema_version':1,'unit_id':f"GU.{p['scene_id']}.layer.{layer['layer_id']}.{i:02d}",'scene_id':p['scene_id'],'stage':'LAYER','purpose':layer.get('generation_intent',layer['semantic_role']),'candidate_index':i,'depends_on_locks':['TARGET_LOCK','COORDINATE_LOCK',*layout_dependencies],'output_role':'SEMANTIC_LAYER','layer_id':layer['layer_id'],'stop_condition':'ONE_APPROVABLE_CANDIDATE'}
                path=d/f'{i:02d}.json'; atomic_json(path,u); path.with_suffix('.md').write_text(_capsule(p,'LAYER',u['purpose'],f'{i:02d}',layer),encoding='utf-8')
                units.append(str(path.relative_to(scene_dir)))
        u={'schema_version':1,'unit_id':f"GU.{p['scene_id']}.psd.01",'scene_id':p['scene_id'],'stage':'PSD_PACKAGING','purpose':'package approved layer assets into final editable PSD','candidate_index':1,'depends_on_locks':['ASSET_LOCK',*layout_dependencies],'output_role':'PSD_MASTER','packaging_retry_policy':{'mode':'PSD_ONLY','trigger':'PREFERRED_OUTPUT_MISSING','max_retries':1,'preserve_locked_pixels':True,'preserve_locked_layout':True,'creative_changes':'FORBID'},'stop_condition':'ONE_STRUCTURALLY_VALID_PSD'}
        path=scene_dir/'units/psd/01.json'; atomic_json(path,u); path.with_suffix('.md').write_text(_psd_capsule(p),encoding='utf-8')
        units.append(str(path.relative_to(scene_dir)))
    atomic_json(scene_dir/'units/index.json',{'schema_version':1,'scene_id':p['scene_id'],'units':units,'budget':estimate_generation_budget(p)})
    return units
