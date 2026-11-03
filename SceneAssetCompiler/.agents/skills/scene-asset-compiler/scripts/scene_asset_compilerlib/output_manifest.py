"""Persist deterministic output manifests for one completed generation attempt."""
from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path
import uuid

from .util import atomic_json, sha256_file


IMG_EXT={'.png':'image/png','.webp':'image/webp','.jpg':'image/jpeg','.jpeg':'image/jpeg','.psd':'image/vnd.adobe.photoshop'}

def artifact_role(p:Path):
    return 'PREVIEW' if '.preview.' in p.name.lower() else 'PRIMARY'

def manifest_outputs(run_dir:Path, task_id:str, attempt_no:int):
    ad=run_dir/'tasks'/task_id/'attempts'/f'{attempt_no:04d}'; outs=[]
    for p in sorted((ad/'outputs').iterdir()):
        if p.is_file(): outs.append({'artifact_id':f'ART-{uuid.uuid4()}','role':artifact_role(p),'path':str(Path('outputs')/p.name),'sha256':sha256_file(p),'mime':IMG_EXT.get(p.suffix.lower(),'application/octet-stream'),'size_bytes':p.stat().st_size})
    result={'schema_version':1,'task_id':task_id,'attempt_no':attempt_no,'created_at':datetime.now(timezone.utc).isoformat(),'artifacts':outs}
    atomic_json(ad/'result.json',result); return result
