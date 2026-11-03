from __future__ import annotations
from .contract_primitives import validate_style_reference_separation
from .psd_profile import prompt_lines as psd_prompt_lines
from .request_contracts import render_contract_for_task, required_transparent_margin_px
from .render_contract_prompt import prompt_lines


def _csv(items):
    return '; '.join(str(x).strip() for x in items if str(x).strip()) or 'none'


def _kv(label, value):
    value=str(value).strip()
    return f'{label}: {value}' if value else None


def _validate_task_prompt_contract(task, resolved_references):
    render_contract_for_task(task)
    validate_style_reference_separation(task.get('references', []), 'task.references')
    required_transparent_margin_px(task)
    if resolved_references is not None:
        validate_style_reference_separation(resolved_references, 'resolved_references')


def photoreal_profile_lines(task):
    profile=task.get('prompt_profile','PROJECT_STYLE')
    if profile=='PROJECT_STYLE': return []
    if profile=='CUSTOM': return ['Prompt profile: CUSTOM. Apply only the explicit visual authority and brief; do not infer a generic prestige style.']
    vb=task['visual_brief']; comp=vb['composition']; light=vb['lighting']; auth=task.get('authority_snapshot',{})
    transport=('Preserve coherent self-shadowing and occlusion. Do not bake environmental contact/cast shadows into a transparent reusable asset.' if task.get('output',{}).get('transparent_background') else 'Preserve coherent cast shadows, contact shadows, reflections, and occlusion.')
    lines=[
      f'Prompt profile: {profile}. The result must read as a physically plausible photograph, not illustration, CGI, concept art, plastic skin, or a stock-photo composite.',
      f"Capture model: framing={comp['framing']}; camera angle={comp['camera_angle']}; perspective={comp['perspective']}; lens behavior={comp['lens_behavior']}; focus/depth={comp['focus_depth']}.",
      f"Light transport: motivated by {light['motivation']}; direction={light['direction']}; quality={light['quality']}; contrast={light['contrast']}; color relationship={light['color_relationship']}. {transport}",
      'Naturalism: keep plausible anatomy, joint count, hands/paws, weight, scale, gravity, surface microtexture, asymmetry, and restrained imperfections. Avoid waxy retouching, impossible depth of field, duplicated anatomy, fused subjects, and attribute swaps.',
    ]
    bindings=[]
    for c in auth.get('characters',[]):
        facts=c.get('identity_invariants',[])+c.get('silhouette',[])+c.get('face_anchors',[])+c.get('costume_rules',[])
        bindings.append(f"{c.get('name',c.get('character_id'))} ({c.get('character_id')}): {_csv(facts)}")
    if bindings: lines.append('Per-subject identity binding: '+' | '.join(bindings)+'. Do not transfer face, body, costume, pose, or attributes between subjects.')
    if profile=='PHOTOREAL_CHARACTER_ANCHOR':
        lines.append('Character-anchor capture: neutral production-reference pose and expression, low-distortion full identity read, even motivated light, true skin/hair/fabric detail, generous uncropped padding; no cinematic grading or beauty retouch that changes identity. Keep both hands fully outside pockets and away from the torso, with both wrists and all fingers visible for anatomy review.')
    elif profile=='PHOTOREAL_LAYER_ASSET':
        lines.append('Layer-asset capture: isolate the complete semantic subject with genuine transparent pixels; no checkerboard, backdrop, matte, edge halo, or baked shadow unless explicitly requested. Preserve hair/fur/translucent boundary detail in soft alpha.')
    else:
        lines.append('Scene capture: all subjects share one coherent camera, vanishing-point system, exposure, light field, atmospheric depth, and physical contact with the environment.')
    return lines


def compile_prompt(task, attempt_no:int, repair=None, resolved_references=None):
    _validate_task_prompt_contract(task, resolved_references)
    vb=task['visual_brief']; comp=vb['composition']; light=vb['lighting']; color=vb['color']; auth=task.get('authority_snapshot',{})
    style=auth.get('style',{}); vl=style.get('visual_language',{})
    characters=auth.get('characters',[]); props=auth.get('props',[]); beats=auth.get('color_script',[])

    identity=[]; allowed=[]
    for c in characters:
        identity += [f"{c['name']}: {x}" for x in c.get('identity_invariants',[])]
        identity += [f"{c['name']} silhouette: {x}" for x in c.get('silhouette',[])]
        identity += [f"{c['name']} face: {x}" for x in c.get('face_anchors',[])]
        identity += [f"{c['name']} palette: {x}" for x in c.get('palette',[])]
        identity += [f"{c['name']} costume: {x}" for x in c.get('costume_rules',[])]
        allowed += [f"{c['name']}: {x}" for x in c.get('allowed_variation',[])]
    for p in props:
        identity += [f"{p['name']}: {x}" for x in p.get('identity_invariants',[])]
        allowed += [f"{p['name']}: {x}" for x in p.get('allowed_variation',[])]

    lines=[
      f"Use case: {task['kind'].lower().replace('_','-')}",
      f"Asset: {task['asset_id']}",
    ]
    if repair:
        strategy=repair.get('strategy','TARGETED_EDIT')
        reconstruction=strategy in ('RECONSTRUCT_FROM_AUTHORITY','TARGETED_REGENERATION','REFERENCE_ABLATION','RESEED_ANCHOR','RESET_EXECUTION_CONTEXT')
        priority='[PSD PACKAGING RETRY PRIORITY]' if strategy=='PSD_PACKAGING_RETRY' else ('[RECONSTRUCTION PRIORITY]' if reconstruction else '[TARGETED EDIT PRIORITY]')
        lines += [
          '',
          priority,
          repair['instruction'],
        ]
        if repair.get('edit_target'):
            et=repair['edit_target']; lines.append(f"Edit target: {et.get('run_path',et.get('path',''))} (sha256 {et.get('sha256','')})")
        if repair.get('reference_policy'): lines.append(f"Reference policy: {repair['reference_policy']}")
        if repair.get('preserve') and not reconstruction:
            lines += ['Preserve exactly:', *[f'- {x}' for x in repair['preserve']]]
        lines.append('')
    lines += [
      f"Narrative function: {vb['narrative_function']}",
      f"Emotional intent: {vb['emotional_intent']}",
      f"Primary moment: {task['content']['subject']}; {task['content']['pose_or_state']}.",
      f"First read: {vb['first_read']}",
      f"Focal hierarchy: {_csv(vb['focal_hierarchy'])}",
      f"Composition: framing {comp['framing']}; camera {comp['camera_angle']}; perspective {comp['perspective']}; lens behavior {comp['lens_behavior']}; focus/depth {comp['focus_depth']}; subject placement {comp['subject_placement']}; depth planes {_csv(comp['depth_layers'])}; negative space {comp['negative_space']}.",
      f"Lighting: motivated by {light['motivation']}; direction {light['direction']}; quality {light['quality']}; contrast {light['contrast']}; color relationship {light['color_relationship']}.",
      f"Color/value: role {color['palette_role']}; dominant {_csv(color['dominant'])}; accent {_csv(color['accent'])}; value structure {color['value_structure']}.",
      f"Materials/textures: {_csv(vb['materials'])}",
      f"Atmosphere: {_csv(vb['atmosphere'])}",
      f"Environment rules: {_csv(vb['environment_rules'])}",
      f"Continuity: {_csv(vb['continuity'])}",
      f"Style system: {style.get('render_family','')}; shape {_csv(vl.get('shape_language',[]))}; edges {_csv(vl.get('line_edge',[]))}; shading {_csv(vl.get('shading',[]))}; texture {_csv(vl.get('texture',[]))}; palette {_csv(vl.get('palette',[]))}; detail density {_csv(vl.get('detail_density',[]))}.",
      f"Project lighting language: {_csv(vl.get('lighting_principles',[]))}",
      f"Project camera language: {_csv(vl.get('camera_principles',[]))}",
      f"Project material language: {_csv(vl.get('material_principles',[]))}",
    ]
    if identity: lines.append(f"Identity/prop invariants: {_csv(identity)}")
    if allowed: lines.append(f"Allowed variation: {_csv(allowed)}")
    if auth.get('world'): lines.append(f"World invariants: {_csv(auth['world'])}")
    facts=[f"{x['source_ref']}: {x['fact']}" for x in auth.get('protected_visual_facts',[])]
    if facts: lines.append(f"Protected visual facts: {_csv(facts)}")
    if beats:
        lines.append('Color-script beat: '+_csv([f"{b['beat_id']} / {b['function']} / palette {_csv(b['palette'])} / values {b['value_structure']} / light {_csv(b['lighting'])} / transition {b['transition']}" for b in beats]))
    lines += photoreal_profile_lines(task)

    refs=resolved_references or []
    if refs:
        lines.append('Input images:')
        for i,r in enumerate(refs,1):
            source_note=f"task={r.get('source_task_id')} asset={r.get('asset_id')}" if r.get('source_kind')=='TASK_OUTPUT' else 'project reference'
            lines.append(f"- Image {i}: role={r['role']}; source={r['path']} ({source_note}); preserve={_csv(r['preserve'])}; ignore={_csv(r['ignore'])}.")
    if task.get('render_contract'):
        lines += prompt_lines(
            task['render_contract'],
            references=refs,
            transparent_background=task['output'].get('transparent_background',False),
        )

    margin=required_transparent_margin_px(task)
    if margin is not None:
        lines.append(
            f"Transparent reusable-asset fit: minimum {margin}px fully transparent margin on every canvas edge at the output canvas size; show the complete character/object extent; no edge contact."
        )

    style_avoid=style.get('forbidden_drift',[])
    avoid=list(vb['avoid'])+list(style_avoid)
    lines += [
      f"Structure/output: split {task['split_profile']}; required layers {_csv(task['structure']['required_layer_ids'])}; preferred {task['output']['preferred']}; fallback_allowed={task['output']['fallback_allowed']}; transparent={task['output']['transparent_background']}; size={task['output']['size']}.",
      f"Avoid: {_csv(avoid)}",
      "Hard constraints: obey approved identity, style, world, and protected narrative facts. Do not add unrequested text, logos, watermarks, characters, props, story beats, costume changes, or decorative focal points.",
      "Reference rule: each image controls only its declared role and preserve fields. Never import identity/style/composition from an ignored axis.",
      "Craft rule: narrative function and first-read hierarchy outrank decorative detail. Keep secondary contrast/detail subordinate. Use concrete visual decisions, not generic prestige tags.",
      "Generation rule: use the typed Scene Provider only for a reference-free PNG GENERATE attempt; otherwise delegate raster generation/editing to the installed $imagegen skill. Inspect the actual candidate before persisting it.",
      "PSD rule: when preferred=PSD, the image generator itself must create the PSD and path metadata. SceneAssetCompiler only specifies and validates the contract; never locally assemble, vectorize, or fake PSD output.",
    ]
    if task['split_profile'] in ('L2','L3'):
        lines.append('Articulation rule: create hidden overlap around moving joints so the planned motion cannot expose holes; preserve a clean external silhouette.')
    return '\n'.join(x for x in lines if x)+'\n'


def compile_generation_capsule(task, attempt_no:int, repair=None, resolved_references=None):
    """Return the task-scoped visual request handed verbatim to the selected executor.

    Deliberately excludes SceneAssetCompiler/Doctor/workflow/QA vocabulary so executor context cannot
    become accidental visual content. prompt.md remains the auditable full brief; this capsule
    is the actual generation boundary.
    """
    _validate_task_prompt_contract(task, resolved_references)
    vb=task['visual_brief']; comp=vb['composition']; light=vb['lighting']; color=vb['color']; auth=task.get('authority_snapshot',{})
    style=auth.get('style',{}); vl=style.get('visual_language',{})
    subject=task['content']['subject']
    pose=task['content'].get('pose_or_state','')
    refs=resolved_references or []

    lines=[
      f"Use case: {task['kind'].lower().replace('_','-')}",
      f"Asset type: {task['kind'].lower().replace('_',' ')}",
      f"Primary request: {subject}" + (f"; {pose}." if pose else '.'),
      f"Scene/backdrop: {_csv(vb.get('environment_rules',[]))}",
      f"Subject: {subject}",
      f"Style/medium: {style.get('render_family','')}; edges {_csv(vl.get('line_edge',[]))}; shading {_csv(vl.get('shading',[]))}; texture {_csv(vl.get('texture',[]))}",
      f"Composition/framing: first read {vb['first_read']}; focal hierarchy {_csv(vb['focal_hierarchy'])}; {comp['framing']}; camera {comp['camera_angle']}; {comp['perspective']}; subject placement {comp['subject_placement']}",
      f"Lighting/mood: {vb['emotional_intent']}; motivated by {light['motivation']}; {light['direction']}; {light['quality']}; {light['contrast']}",
      f"Color palette: dominant {_csv(color['dominant'])}; accent {_csv(color['accent'])}; {color['value_structure']}",
      f"Materials/textures: {_csv(vb['materials'])}",
    ]
    lines += photoreal_profile_lines(task)
    if task.get('render_contract'):
        lines += prompt_lines(
            task['render_contract'],
            references=refs,
            transparent_background=task['output'].get('transparent_background',False),
        )
    margin=required_transparent_margin_px(task)
    if margin is not None:
        lines.append(
            f"Transparent reusable-asset fit: minimum {margin}px fully transparent margin on every canvas edge at the output canvas size; show the complete character/object extent; no edge contact."
        )
    if refs:
        lines.append('Input images:')
        for i,r in enumerate(refs,1):
            lines.append(f"- Image {i}: role={r['role']}; preserve={_csv(r['preserve'])}; ignore={_csv(r['ignore'])}.")

    # Put task truth before optional polish. For a global reconstruction, only canonical
    # positive/negative constraints are carried; rejected-pixel descriptions stay out.
    constraints=[]
    if vb.get('narrative_function'): constraints.append('Narrative purpose: '+vb['narrative_function'])
    constraints.append('First read must be: '+vb['first_read'])
    for x in auth.get('world',[]): constraints.append('World invariant: '+x)
    for x in auth.get('protected_visual_facts',[]): constraints.append('Protected fact: '+x.get('fact',''))
    for axis,items in task.get('acceptance',{}).items():
        for x in items:
            if axis in ('narrative','identity','composition','structure','articulation','usability'): constraints.append(x)
    if task['split_profile'] in ('L2','L3'):
        constraints.append('Moving joints need hidden texture overlap so motion cannot expose holes.')
    if task['structure'].get('required_layer_ids'):
        constraints.append('Required semantic layers: '+_csv(task['structure']['required_layer_ids']))
    if repair and (repair.get('edit_target') or repair.get('strategy')=='PSD_PACKAGING_RETRY'):
        constraints.append(repair['instruction'])
        constraints += [str(x) for x in repair.get('preserve',[])]
        if repair.get('strategy')=='PSD_PACKAGING_RETRY':
            constraints.append('Packaging-only retry: reuse approved locked pixels and layout exactly; do not redraw, restyle, relight, recompose, or assemble PSD locally.')
    elif repair and repair.get('strategy') in ('RECONSTRUCT_FROM_AUTHORITY','TARGETED_REGENERATION','REFERENCE_ABLATION','RESEED_ANCHOR','RESET_EXECUTION_CONTEXT'):
        # Do not inject rejected candidate descriptions. Critical truth is already represented
        # by the canonical subject/world/acceptance lines above.
        constraints.append('Rebuild from the canonical subject, setting, action and first-read above; do not inherit the rejected candidate.')
    lines.append('Constraints: '+_csv(constraints))

    avoid=list(vb.get('avoid',[]))+list(style.get('forbidden_drift',[]))+[
      'unrequested text','logos','watermarks','workflow diagrams','dashboards','infographics','JSON/code rendered into the image'
    ]
    lines.append('Avoid: '+_csv(avoid))
    margin=required_transparent_margin_px(task)
    margin_text=f'; minimum transparent margin on every canvas edge={margin}px' if margin is not None else ''
    lines.append(f"Output intent: {task['output']['size']} {task['output']['preferred']}; transparent={task['output']['transparent_background']}; split profile {task['split_profile']}{margin_text}.")
    if task['output'].get('preferred')=='PSD':
        lines.append('')
        lines.append('PSD boundary: the image generator itself must create the PSD and path metadata; SceneAssetCompiler only specifies and validates the returned file.')
        lines.extend(psd_prompt_lines(task['output']['psd_profile'], task['structure'].get('required_layer_ids',[])))
    return '\n'.join(x for x in lines if x)+'\n'
