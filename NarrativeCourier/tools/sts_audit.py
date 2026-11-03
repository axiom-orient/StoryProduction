#!/usr/bin/env python3
"""STS existing-screenplay auditor.

Three isolated model stages:
  synopsis -> Source IR
  screenplay -> source-blind Screenplay IR
  Source IR + Screenplay IR -> semantic comparison

The screenplay reconstruction stage never receives the synopsis/source IR.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

import digest as _digest
from sts_audit_inputs import capture_input
from sts_audit_results import (
    render_report,
    validate_alignment_coverage,
    validate_fixed_dimensions,
    validate_ir_ids,
    validate_result,
    validate_verdict_coherence,
)

ROOT = Path(__file__).resolve().parents[1]
SKILL = ROOT / "skills" / "sts"
AUDIT = SKILL / "audit_workflow.json"


def load_workflow() -> dict:
    data=json.loads(AUDIT.read_text(encoding="utf-8"))
    if data.get("domain_id") != "sts-audit" or len(data.get("stages",[])) != 3:
        raise ValueError("invalid audit workflow contract")
    ids=[s.get("id") for s in data["stages"]]
    if ids != ["source","screenplay","compare"]:
        raise ValueError(f"audit stages must be source, screenplay, compare; got {ids}")
    return data


def ref_path(rel: str) -> Path:
    p=(SKILL/rel).resolve()
    if SKILL.resolve() not in p.parents:
        raise ValueError(f"audit resource escapes skill root: {rel}")
    if not p.is_file(): raise FileNotFoundError(p)
    return p


def audit_bundle_hash(workflow: dict) -> str:
    rels={"audit_workflow.json"}
    for stage in workflow["stages"]:
        for key in ("system_prompt","task_prompt","schema"):
            rels.add(stage[key])
        rels.update(stage.get("principles",[]))
    rows=[]
    for rel in sorted(rels):
        p=AUDIT if rel == "audit_workflow.json" else ref_path(rel)
        rows.append({"path":rel,"sha256":_digest.sha256_file(p)})
    return _digest.sha256_bytes(json.dumps(rows,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode())


def build_prompt(stage: dict, payload: str) -> str:
    system=ref_path(stage["system_prompt"]).read_text(encoding="utf-8")
    task=ref_path(stage["task_prompt"]).read_text(encoding="utf-8")
    principles=[]
    for rel in stage.get("principles",[]):
        principles.append(f"## {rel}\n{ref_path(rel).read_text(encoding='utf-8')}")
    blind=", ".join(stage.get("blind_to",[])) or "none"
    return (
        "[STS AUDIT EXECUTION CONTRACT]\n"
        f"Stage: {stage['id']}\n"
        f"Blind to: {blind}\n"
        "Do not inspect files or infer unavailable inputs. Return only JSON matching the supplied schema.\n\n"
        f"# SYSTEM\n{system}\n\n# ROUTED PRINCIPLES\n"+"\n".join(principles)+
        f"\n\n# TASK\n{task}\n\n# INPUT\n{payload}\n"
    )


def codex_bin() -> str:
    return os.environ.get("STS_CODEX_BIN") or shutil.which("codex") or "codex"


def run_codex_json(stage: dict, prompt: str, work: Path) -> dict:
    schema=ref_path(stage["schema"])
    out=work/f"{stage['id']}.json"
    model=os.environ.get("STS_CODEX_MODEL","gpt-5.6-sol")
    reasoning=os.environ.get("STS_CODEX_REASONING","high")
    timeout=int(os.environ.get("STS_CODEX_TIMEOUT_SECS","1800"))
    cmd=[codex_bin(),"--strict-config","--ask-for-approval","never","exec","--ignore-rules","--ignore-user-config","--model",model,"--config",f'model_reasoning_effort="{reasoning}"',"--config","agents.enabled=false","--config",'web_search="disabled"',"--ephemeral","--sandbox","read-only","--skip-git-repo-check","--output-schema",str(schema),"--output-last-message",str(out),"-"]
    proc=subprocess.run(cmd,input=prompt,text=True,cwd=work,stdout=subprocess.DEVNULL,stderr=None,timeout=timeout)
    if proc.returncode != 0: raise RuntimeError(f"Codex stage {stage['id']} exited {proc.returncode}")
    data=json.loads(out.read_text(encoding="utf-8"))
    validate_result(data, json.loads(schema.read_text(encoding="utf-8")))
    return data


def main(argv: list[str] | None=None) -> int:
    ap=argparse.ArgumentParser(description="STS synopsis↔existing-screenplay fidelity/integrity audit")
    ap.add_argument("--synopsis",required=True,type=Path)
    ap.add_argument("--screenplay",required=True,type=Path)
    ap.add_argument("--out",required=True,type=Path)
    ap.add_argument("--dry-run",action="store_true",help="validate/extract/plan without Codex calls or output writes")
    args=ap.parse_args(argv)
    workflow=load_workflow()
    synopsis_input=capture_input(args.synopsis)
    screenplay_input=capture_input(args.screenplay)
    synopsis=synopsis_input.text
    screenplay=screenplay_input.text
    bundle=audit_bundle_hash(workflow)
    meta={
        "domain_id":workflow["domain_id"],
        "domain_version":workflow["domain_version"],
        "audit_bundle_sha256":bundle,
        "synopsis_sha256":synopsis_input.raw_sha256,
        "synopsis_text_sha256":synopsis_input.text_sha256,
        "synopsis_extractor":synopsis_input.extractor,
        "screenplay_sha256":screenplay_input.raw_sha256,
        "screenplay_text_sha256":screenplay_input.text_sha256,
        "screenplay_extractor":screenplay_input.extractor,
        "input_profile":"sts-audit.captured-input/2",
        "synopsis_format":args.synopsis.suffix.lower() or "<none>",
        "screenplay_format":args.screenplay.suffix.lower() or "<none>",
        "synopsis_chars":len(synopsis),
        "screenplay_chars":len(screenplay),
        "model":os.environ.get("STS_CODEX_MODEL","gpt-5.6-sol"),
        "reasoning":os.environ.get("STS_CODEX_REASONING","high"),
        "stages":[s["id"] for s in workflow["stages"]]
    }
    if args.dry_run:
        print(json.dumps({**meta,"dry_run":True},ensure_ascii=False,indent=2))
        return 0
    outdir=args.out
    outdir.mkdir(parents=True,exist_ok=False)
    try:
        with tempfile.TemporaryDirectory(prefix="sts-audit-") as td:
            work=Path(td)
            source_stage,screen_stage,compare_stage=workflow["stages"]
            source_ir=run_codex_json(source_stage,build_prompt(source_stage,synopsis),work)
            screenplay_ir=run_codex_json(screen_stage,build_prompt(screen_stage,screenplay),work)
            validate_ir_ids(source_ir, screenplay_ir)
            # Critical boundary: comparator receives only two IR JSON objects.
            compare_payload=json.dumps({"source_ir":source_ir,"screenplay_ir":screenplay_ir},ensure_ascii=False,indent=2)
            result=run_codex_json(compare_stage,build_prompt(compare_stage,compare_payload),work)
            validate_fixed_dimensions(result)
            validate_alignment_coverage(result, source_ir, screenplay_ir)
            validate_verdict_coherence(result, source_ir)
        (outdir/"01_source_ir.json").write_text(json.dumps(source_ir,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
        (outdir/"02_screenplay_ir.json").write_text(json.dumps(screenplay_ir,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
        (outdir/"audit.json").write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
        (outdir/"audit-meta.json").write_text(json.dumps(meta,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
        (outdir/"STS-AUDIT.md").write_text(render_report(result,meta),encoding="utf-8")
    except Exception:
        shutil.rmtree(outdir,ignore_errors=True)
        raise
    print(outdir)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
