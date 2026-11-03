from __future__ import annotations
from pathlib import Path
from datetime import datetime, timezone
import hashlib, uuid
from .util import load_json, atomic_json

AXES=['identity','style','narrative','composition','lighting_color','material','photorealism','structure','articulation','usability']
CORE_AXES={'identity','narrative','structure'}
VALID_STATUSES={'PASS','MINOR','MAJOR','NA','UNKNOWN'}


def _claim(cid, axis, source, kind, critical, expected):
    return {
        'claim_id':cid,'axis':axis,'source':source,'kind':kind,'critical':bool(critical),
        'expected':str(expected),'actual':'','status':'UNKNOWN','evidence':[]
    }


def build_audit_template(run_dir:Path, task_id:str, attempt_no:int):
    td=run_dir/'tasks'/task_id; ad=td/'attempts'/f'{attempt_no:04d}'
    task=load_json(td/'task.json'); result=load_json(ad/'result.json')
    claims=[]; n=0
    def add(axis,source,kind,critical,expected):
        nonlocal n
        if expected is None or (isinstance(expected,str) and not expected.strip()): return
        n+=1; claims.append(_claim(f'C-{n:03d}',axis,source,kind,critical,expected))

    add('narrative','content.subject','MUST_HAVE',True,task['content'].get('subject',''))
    add('narrative','content.pose_or_state','MUST_HAVE',False,task['content'].get('pose_or_state',''))
    vb=task['visual_brief']
    add('narrative','visual_brief.narrative_function','MUST_SATISFY',True,vb.get('narrative_function',''))
    add('composition','visual_brief.first_read','MUST_SATISFY',False,vb.get('first_read',''))
    for x in vb.get('environment_rules',[]): add('narrative','visual_brief.environment_rules','MUST_SATISFY',False,x)
    for x in vb.get('continuity',[]): add('narrative','visual_brief.continuity','PRESERVE',False,x)
    for x in vb.get('avoid',[]): add('usability','visual_brief.avoid','MUST_NOT_HAVE',False,x)

    auth=task.get('authority_snapshot',{})
    for c in auth.get('characters',[]):
        for x in c.get('identity_invariants',[]): add('identity',f'authority.character.{c.get("character_id","?")}.identity','PRESERVE',True,x)
    style=auth.get('style',{})
    for x in style.get('forbidden_drift',[]): add('style','authority.style.forbidden_drift','MUST_NOT_HAVE',False,x)
    for x in auth.get('protected_visual_facts',[]): add('narrative','authority.protected_visual_facts','PRESERVE',True,x.get('fact',''))
    if task.get('prompt_profile','PROJECT_STYLE').startswith('PHOTOREAL_'):
        add('photorealism','prompt_profile','MUST_SATISFY',True,'physically plausible camera, lighting, anatomy, materials, depth, contact and natural imperfections; no CGI/plastic/composite artifacts')

    for axis in AXES:
        for x in task.get('acceptance',{}).get(axis,[]): add(axis,f'acceptance.{axis}','MUST_SATISFY',axis in CORE_AXES,x)
    for x in task.get('structure',{}).get('required_layer_ids',[]): add('structure','structure.required_layer_ids','MUST_HAVE',True,x)

    refs=load_json(ad/'attempt.json').get('resolved_references',[]) if (ad/'attempt.json').exists() else []
    out={
      'schema_version':1,'audit_id':f'AUD-{uuid.uuid4()}','task_id':task_id,'attempt_no':attempt_no,
      'reviewer':'CODEX','created_at':datetime.now(timezone.utc).isoformat(),'status':'INCOMPLETE',
      'artifact_ids':[a['artifact_id'] for a in result.get('artifacts',[]) if a.get('role','PRIMARY')=='PRIMARY'],
      'claims':claims,
      'reference_observations':[{'reference_index':i+1,'role':r.get('role','OTHER'),'status':'NA','evidence':[]} for i,r in enumerate(refs)],
      'global_observations':[]
    }
    atomic_json(ad/'output-audit.json',out); return out


def validate_output_audit(o, task_id:str, attempt_no:int, require_complete=True):
    if o.get('schema_version')!=1: raise ValueError('AUDIT_VERSION')
    if o.get('task_id')!=task_id or o.get('attempt_no')!=attempt_no: raise ValueError('AUDIT_LINEAGE')
    claims=o.get('claims');
    if not isinstance(claims,list) or not claims: raise ValueError('AUDIT_CLAIMS_EMPTY')
    seen=set(); incomplete=[]
    for c in claims:
        cid=c.get('claim_id'); axis=c.get('axis'); status=c.get('status')
        if cid in seen: raise ValueError(f'AUDIT_DUPLICATE_CLAIM:{cid}')
        seen.add(cid)
        if axis not in AXES: raise ValueError(f'AUDIT_AXIS:{axis}')
        if status not in VALID_STATUSES: raise ValueError(f'AUDIT_STATUS:{cid}:{status}')
        if status=='UNKNOWN': incomplete.append(cid)
        if status in {'PASS','MINOR','MAJOR'}:
            if not isinstance(c.get('actual'),str) or not c['actual'].strip(): raise ValueError(f'AUDIT_ACTUAL_REQUIRED:{cid}')
            ev=c.get('evidence')
            if not isinstance(ev,list) or not any(isinstance(x,str) and x.strip() for x in ev): raise ValueError(f'AUDIT_EVIDENCE_REQUIRED:{cid}')
    expected='COMPLETE' if not incomplete else 'INCOMPLETE'
    if o.get('status')!=expected: raise ValueError(f'AUDIT_STATUS_MISMATCH:{expected}')
    if require_complete and incomplete: raise ValueError(f'AUDIT_INCOMPLETE:{incomplete}')
    for ro in o.get('reference_observations',[]):
        if ro.get('status') not in {'CLEAN','CONTAMINATION_SUSPECTED','CONFLICT_SUSPECTED','NA'}: raise ValueError('AUDIT_REFERENCE_STATUS')
        if ro.get('status') not in {'CLEAN','NA'} and not ro.get('evidence'): raise ValueError('AUDIT_REFERENCE_EVIDENCE_REQUIRED')
    return o


def _signature(failed):
    raw='|'.join(sorted(f"{c['axis']}:{c['source']}:{c['expected']}" for c in failed)).encode('utf-8')
    return hashlib.sha256(raw).hexdigest()[:16]


def diagnose(run_dir:Path, task_id:str, attempt_no:int):
    td=run_dir/'tasks'/task_id; ad=td/'attempts'/f'{attempt_no:04d}'
    task=load_json(td/'task.json'); audit=load_json(ad/'output-audit.json'); validate_output_audit(audit,task_id,attempt_no,True)
    mech=load_json(ad/'mechanical-review.json') if (ad/'mechanical-review.json').exists() else {'status':'PASS','findings':[]}
    vis=load_json(ad/'visual-review.json') if (ad/'visual-review.json').exists() else None
    attempt=load_json(ad/'attempt.json')
    metric_doc=load_json(ad/'metric-evidence.json') if (ad/'metric-evidence.json').exists() else {'metrics':[]}

    failed=[c for c in audit['claims'] if c['status'] in {'MINOR','MAJOR'}]
    axis={a:{'minor':0,'major':0,'critical_major':0,'score':0} for a in AXES}
    for c in failed:
        w=1 if c['status']=='MINOR' else 3
        axis[c['axis']]['score']+=w; axis[c['axis']][c['status'].lower()]+=1
        if c['status']=='MAJOR' and c.get('critical'): axis[c['axis']]['critical_major']+=1
    mech_fail=[f for f in mech.get('findings',[]) if f.get('severity')=='FAIL']
    majors=sum(x['major'] for x in axis.values()); minors=sum(x['minor'] for x in axis.values()); critical=sum(x['critical_major'] for x in axis.values())
    if not failed and not mech_fail: scope='CLEAN'
    elif critical or majors>=3 or len([a for a,v in axis.items() if v['major']])>=3: scope='GLOBAL'
    elif majors or minors>=3 or mech_fail: scope='REGIONAL'
    else: scope='LOCAL'

    hypotheses=[]
    def hyp(code,confidence,evidence): hypotheses.append({'code':code,'confidence':confidence,'evidence':evidence})
    passing_style=(vis is not None and vis.get('checks',{}).get('style')=='PASS') or axis['style']['score']==0
    core_major=[a for a in CORE_AXES if axis[a]['major']]
    if core_major and passing_style:
        hyp('PROMPT_PRIORITY_INVERSION',0.78,[f'core axes failed: {sorted(core_major)}','style did not fail'])
    refs=attempt.get('resolved_references',[])
    suspicious=[x for x in audit.get('reference_observations',[]) if x['status'] in {'CONTAMINATION_SUSPECTED','CONFLICT_SUSPECTED'}]
    context_obs=[x for x in audit.get('global_observations',[]) if isinstance(x,str) and (x.startswith('EXECUTOR_CONTEXT_LEAKAGE:') or x.startswith('META_CONTEXT_LEAKAGE:'))]
    if context_obs:
        hyp('EXECUTOR_CONTEXT_CONTAMINATION',0.97,context_obs)
    if suspicious:
        hyp('REFERENCE_CONFLICT_OR_CONTAMINATION',0.9,[e for x in suspicious for e in x.get('evidence',[])])
    elif len(refs)>=4 and (axis['identity']['score'] or axis['style']['score'] or axis['composition']['score']):
        hyp('REFERENCE_PRESSURE',0.55,[f'{len(refs)} simultaneous image references with drift'])
    if not refs and scope=='GLOBAL':
        hyp('UNDERCONSTRAINED_REQUEST',0.62,['global divergence with no image anchors'])
    if len(audit['claims'])>28 and len(refs)>3:
        hyp('OVERCONSTRAINED_REQUEST',0.5,[f'{len(audit["claims"])} atomic claims and {len(refs)} references'])
    if mech_fail:
        hyp('OUTPUT_CONTRACT_FAILURE',0.99,[f.get('code','') for f in mech_fail])
    repeated_context=False
    if attempt_no>1:
        prev=sorted(td.glob('attempts/[0-9][0-9][0-9][0-9]/doctor-report.json'))
        sig=_signature(failed)
        previous_reports=[load_json(p) for p in prev if p.parent.name!=f'{attempt_no:04d}']
        if any(r.get('gap_signature')==sig for r in previous_reports):
            hyp('REPEATED_FAILURE',0.95,[f'gap signature {sig} repeated'])
        if context_obs and any(any(h.get('code')=='EXECUTOR_CONTEXT_CONTAMINATION' for h in r.get('hypotheses',[])) for r in previous_reports):
            repeated_context=True
            hyp('REPEATED_EXECUTOR_CONTEXT_CONTAMINATION',0.995,['executor meta-context leakage repeated after task-scoped reconstruction'])

    if scope=='CLEAN': strategy='NONE'
    elif repeated_context: strategy='RESET_EXECUTION_CONTEXT'
    elif any(h['code']=='EXECUTOR_CONTEXT_CONTAMINATION' for h in hypotheses): strategy='RECONSTRUCT_FROM_AUTHORITY'
    elif any(h['code']=='REPEATED_FAILURE' for h in hypotheses) and axis['identity']['score']: strategy='RESEED_ANCHOR'
    elif any(h['code']=='REFERENCE_CONFLICT_OR_CONTAMINATION' for h in hypotheses): strategy='REFERENCE_ABLATION'
    elif scope=='GLOBAL': strategy='RECONSTRUCT_FROM_AUTHORITY'
    elif scope=='REGIONAL' and any(axis[a]['major'] for a in CORE_AXES): strategy='TARGETED_REGENERATION'
    elif scope=='REGIONAL': strategy='TARGETED_EDIT'
    else: strategy='SURGICAL_EDIT'

    failed_axes=[a for a,v in axis.items() if v['score']]
    passed_axes=[a for a in AXES if a not in failed_axes and (not vis or vis.get('checks',{}).get(a) in {'PASS','NA'})]
    experiments=[]
    if strategy in {'RECONSTRUCT_FROM_AUTHORITY','TARGETED_REGENERATION','RESET_EXECUTION_CONTEXT'}:
        experiments.append({'kind':'MINIMAL_CORE_PROMPT','change':'keep subject/setting/action/first-read + hard constraints; remove decorative/nonessential direction','metric':'atomic claim pass rate','keep_if':'fewer MAJOR core-claim failures'})
    if any(h['code']=='EXECUTOR_CONTEXT_CONTAMINATION' for h in hypotheses):
        experiments.insert(0,{'kind':'TASK_SCOPED_EXECUTION_CAPSULE','change':'send generation-capsule.md as the sole visual brief; exclude workflow/Doctor/meta-project prose from the image request','metric':'meta-content leakage + atomic core-claim pass rate','keep_if':'no workflow/UI/meta-project content appears and core claims improve'})
    if refs and (strategy=='REFERENCE_ABLATION' or len(refs)>=3):
        experiments.append({'kind':'REFERENCE_ABLATION','change':'test the smallest role-complete reference diet, one removable reference/role at a time','metric':'identity/style/composition gap vector','keep_if':'target axis improves without regression on preserved axes'})
    if axis['identity']['score']:
        experiments.append({'kind':'ANCHOR_ONLY_CONTROL','change':'identity anchor + minimal neutral prompt only','metric':'identity critical-claim pass rate','keep_if':'identity passes'})

    report={
      'schema_version':1,'doctor_id':f'DOC-{uuid.uuid4()}','task_id':task_id,'attempt_no':attempt_no,'created_at':datetime.now(timezone.utc).isoformat(),
      'scope':scope,'gap_signature':_signature(failed),'gap_vector':axis,'failed_claims':[c['claim_id'] for c in failed],
      'mechanical_failures':[f.get('code','UNKNOWN') for f in mech_fail],
      'metric_evidence':metric_doc.get('metrics',[]),
      'hypotheses':hypotheses,'recommended_strategy':strategy,'preserve_axes':passed_axes,'rewrite_axes':failed_axes,'experiments':experiments,
      'summary':_summary(scope,strategy,failed,mech_fail)
    }
    atomic_json(ad/'doctor-report.json',report); return report


def _summary(scope,strategy,failed,mech):
    if scope=='CLEAN': return 'No material input/output divergence detected by completed audit and mechanical review.'
    parts=[]
    if failed: parts.append(f'{len(failed)} visual contract claim(s) diverged')
    if mech: parts.append(f'{len(mech)} mechanical gate(s) failed')
    return f"{scope} divergence: "+', '.join(parts)+f'. Recommended strategy: {strategy}.'


def build_reconstruction_plan(run_dir:Path, task_id:str, attempt_no:int):
    ad=run_dir/'tasks'/task_id/'attempts'/f'{attempt_no:04d}'; report=load_json(ad/'doctor-report.json'); audit=load_json(ad/'output-audit.json')
    result=load_json(ad/'result.json'); primary=next((a for a in result['artifacts'] if a.get('role','PRIMARY')=='PRIMARY'),None)
    failed=[c for c in audit['claims'] if c['status'] in {'MINOR','MAJOR'}]
    strategy=report['recommended_strategy']
    if strategy=='NONE': raise ValueError('RECONSTRUCTION_NOT_NEEDED')
    use_edit=strategy in {'SURGICAL_EDIT','TARGETED_EDIT'} and primary is not None
    edit_target=None
    if use_edit:
        edit_target={**primary,'run_path':str(Path('tasks')/task_id/'attempts'/f'{attempt_no:04d}'/primary['path'])}
    critical=[c for c in failed if c.get('critical')]
    directives=[]
    if strategy in {'RECONSTRUCT_FROM_AUTHORITY','TARGETED_REGENERATION','RESEED_ANCHOR','RESET_EXECUTION_CONTEXT'}:
        directives += [
          'Rebuild the generation brief from canonical authority and the task VisualBrief; do not inherit failed visual choices from the rejected output.',
          'Place core subject, setting/action, first-read, and MUST_NOT constraints before decorative/style detail.',
          'Use the smallest reference set that still covers required roles; remove or ablate ambiguous references.'
        ]
    else:
        directives.append('Change only the failed claim(s); repeat all preserved invariants explicitly and keep every passing axis unchanged.')
    if critical:
        directives.append('Critical failed claims must be stated as explicit must-have/must-not-have constraints near the top of the generation brief.')
    # A rejected candidate is evidence, not prompt material. For a fresh reconstruction,
    # never restate its wrong visual content: doing so can prime the next generation toward
    # the exact failure we are trying to remove. Only carry canonical expected constraints.
    if use_edit:
        mismatch=' | '.join(f"[{c['axis']}] expected={c['expected']} ; observed={c['actual']}" for c in failed)
        instruction=' '.join(directives)+(' Mismatches to correct: '+mismatch if mismatch else '')
    else:
        must_haves=[]
        for c in critical:
            prefix='MUST NOT' if c.get('kind')=='MUST_NOT_HAVE' else 'MUST HAVE'
            item=f"{prefix}: {c['expected']}"
            if item not in must_haves: must_haves.append(item)
        instruction=' '.join(directives)
        if must_haves:
            instruction += ' Core reconstruction constraints: ' + ' | '.join(must_haves)
        instruction += ' Treat the rejected candidate only as diagnostic evidence; do not describe, imitate, or inherit its wrong subject, setting, composition, lighting, or incidental objects.'
    plan={
      'schema_version':1,'reconstruction_id':f'REC-{uuid.uuid4()}','task_id':task_id,'source_attempt':attempt_no,'created_at':datetime.now(timezone.utc).isoformat(),
      'strategy':strategy,'reset_scope':('OUTPUT' if use_edit else ('EXECUTOR_CONTEXT_AND_OUTPUT' if strategy=='RESET_EXECUTION_CONTEXT' else 'PROMPT_AND_OUTPUT')),'edit_target':edit_target,
      'preserve_axes':report['preserve_axes'],'rewrite_axes':report['rewrite_axes'],'instruction':instruction,
      'reference_policy':('KEEP_AND_ABLATE_SUSPECTS' if strategy=='REFERENCE_ABLATION' else 'MINIMAL_ROLE_COMPLETE'),
      'requires_fresh_executor_context': strategy=='RESET_EXECUTION_CONTEXT',
      'experiments':report.get('experiments',[])
    }
    atomic_json(ad/'reconstruction-plan.json',plan); return plan
