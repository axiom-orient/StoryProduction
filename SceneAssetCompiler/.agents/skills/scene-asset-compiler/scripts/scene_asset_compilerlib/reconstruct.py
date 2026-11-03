from __future__ import annotations
from pathlib import Path
from .util import load_json
from .workspace import begin_attempt


def reconstruct(run_dir:Path, task_id:str, attempt_no:int):
    ad=run_dir/'tasks'/task_id/'attempts'/f'{attempt_no:04d}'
    plan=load_json(ad/'reconstruction-plan.json')
    if plan.get('task_id')!=task_id or plan.get('source_attempt')!=attempt_no: raise ValueError('RECONSTRUCTION_LINEAGE')
    repair={
      'schema_version':2,
      'source_attempt':attempt_no,
      'failure_codes':['DOCTOR_'+x.upper() for x in plan.get('rewrite_axes',[])],
      'strategy':plan['strategy'],
      'instruction':plan['instruction'],
      'preserve':[f'preserve axis: {x}' for x in plan.get('preserve_axes',[])],
      'edit_target':plan.get('edit_target'),
      'reference_policy':plan.get('reference_policy'),
      'experiments':plan.get('experiments',[]),
      'reconstruction_id':plan['reconstruction_id'],
      'fresh_executor_context_required':bool(plan.get('requires_fresh_executor_context',False))
    }
    return begin_attempt(run_dir,task_id,repair)
