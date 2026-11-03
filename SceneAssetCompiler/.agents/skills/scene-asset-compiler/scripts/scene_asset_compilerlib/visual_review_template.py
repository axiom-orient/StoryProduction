"""Create a rejected-by-default human visual-review template."""
from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path

from .util import load_json, atomic_json
from .render_contract_validation import validate_render_contract


def visual_template(run_dir:Path,task_id:str,attempt_no:int):
    ad=run_dir/'tasks'/task_id/'attempts'/f'{attempt_no:04d}'; task=load_json(run_dir/'tasks'/task_id/'task.json')
    contract=task.get('render_contract')
    if contract is not None:
        validate_render_contract(contract)
    keys=['identity','style','narrative','composition','lighting_color','material','photorealism','structure','articulation','usability']
    if contract is not None:
        keys.append('genre_fidelity')
        if contract.get('wireframe_layout'):
            keys.append('layout_fidelity')
    out={'schema_version':3,'task_id':task_id,'attempt_no':attempt_no,'reviewer':'CODEX','created_at':datetime.now(timezone.utc).isoformat(),'status':'REJECTED','prompt_profile_snapshot':task.get('prompt_profile','PROJECT_STYLE'),'checks':{k:'NA' for k in keys},'evidence':{k:[] for k in keys},'findings':[],'acceptance_snapshot':task['acceptance'],'visual_brief_snapshot':task['visual_brief']}
    if contract is not None:
        out['render_contract_snapshot']=contract
        layout=contract.get('wireframe_layout')
        if layout:
            out['layout_evidence']={
                'source_sha256':'',
                'output_sha256':'',
                'slots':[
                    {
                        'slot_id':slot['slot_id'],
                        'observed_bounds':slot['bounds'],
                        'observed_rotation_degrees':0,
                        'observed_z_order':slot['z_order'],
                        'focal_crop_result':{
                            'observed_focal_point':slot['focal_point'],
                            'observed_crop_safe_inset':slot['crop_safe_inset'],
                            'verdict':'NA',
                        },
                        'label_guide_result':{
                            'source_labels':'NOT_APPLICABLE',
                            'output_labels':'NOT_APPLICABLE',
                            'source_guides':'NOT_APPLICABLE',
                            'output_guides':'NOT_APPLICABLE',
                            'verdict':'NA',
                        },
                        'tolerance_verdict':'NA',
                    }
                    for slot in sorted(layout['slots'],key=lambda item:item['slot_id'])
                ],
            }
    atomic_json(ad/'visual-review.json',out); return out
