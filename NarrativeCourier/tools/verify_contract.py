#!/usr/bin/env python3
from __future__ import annotations
from pathlib import Path
import sys
sys.dont_write_bytecode = True
import json, re, subprocess
from tauri_wiring import inspect_tauri_wiring

ROOT=Path(__file__).resolve().parents[1]
SKILL=ROOT/'skills/sts'
errors=[]; passes=[]; warnings=[]

def ok(msg): passes.append(msg); print('PASS',msg)
def fail(msg): errors.append(msg); print('FAIL',msg)
def load(p): return json.loads(p.read_text(encoding='utf-8'))
def exists(rel): return (SKILL/rel).is_file()

# 1 parse all JSON
for p in ROOT.rglob('*.json'):
    try: load(p)
    except Exception as e: fail(f'JSON parse {p.relative_to(ROOT)}: {e}')
if not errors: ok('all repository JSON parse')

workflow=load(SKILL/'workflow.json'); schema=load(SKILL/'schemas/workflow.schema.json')
try:
    import jsonschema
    jsonschema.Draft202012Validator(schema).validate(workflow); ok('workflow Draft 2020-12 schema')
except Exception as e: fail(f'workflow schema validation: {e}')

try:
    import jsonschema
    for schema_path in sorted((SKILL/'schemas').glob('*.json')):
        jsonschema.Draft202012Validator.check_schema(load(schema_path))
    ok('all STS JSON Schemas are valid Draft 2020-12 schemas')
except Exception as e: fail(f'STS schema meta-validation: {e}')

if workflow.get('schema_version')==3 and workflow.get('domain_id')=='sts' and workflow.get('domain_version')=='4.3.0': ok('STS generation domain identity')
else: fail('STS generation domain identity/version mismatch')

nodes=workflow['nodes']; ids=[n['id'] for n in nodes]
if len(ids)==len(set(ids)): ok('unique generation node ids')
else: fail('duplicate generation node ids')
stage_ids={s['id'] for s in workflow['stages']}
if all(n.get('stage') in stage_ids for n in nodes): ok('all nodes use declared stages')
else: fail('unknown stage reference')

# Reachability and ordered I/O
available={'config.yaml','@synopsis','@references'}; outputs=set(); used_prompts=set(); used_principles=set(); seen_agents=set()
for n in nodes:
    if n['kind']=='agent':
        for key in ('system_prompt','task_prompt'):
            rel=n.get(key,''); used_prompts.add(rel)
            if not rel.startswith('prompts/') or not exists(rel): fail(f"{n['id']}: missing prompt {rel}")
        principles=n.get('principles',[])
        if len(principles)>6: fail(f"{n['id']}: more than six principles")
        for rel in principles:
            used_principles.add(rel)
            if not rel.startswith('principles/') or not exists(rel): fail(f"{n['id']}: missing principle {rel}")
        for token in n.get('inputs',[]):
            raw=token[1:] if token.startswith('?') else token
            if raw not in available and not raw.startswith('input/human_feedback/'):
                fail(f"{n['id']}: future/unproduced input {raw}")
        out=n.get('output')
        if not out or out in outputs: fail(f"{n['id']}: invalid/duplicate output {out}")
        else: outputs.add(out); available.add(out)
        seen_agents.add(n['id'])
    else:
        src=n.get('source')
        if src not in available: fail(f"{n['id']}: Human source not produced earlier {src}")
        rn=n.get('revision_node')
        if rn and rn not in seen_agents: fail(f"{n['id']}: revision_node is not earlier Agent {rn}")
if not any('future/unproduced' in e or 'missing prompt' in e or 'missing principle' in e or 'Human source' in e for e in errors): ok('ordered generation dependency/reachability')

prompt_files={str(p.relative_to(SKILL)).replace('\\','/') for p in (SKILL/'prompts').rglob('*.md')}
principle_files={str(p.relative_to(SKILL)).replace('\\','/') for p in (SKILL/'principles').rglob('*.md')}
# audit resources are additional executable reachability
aw=load(SKILL/'audit_workflow.json')
for st in aw['stages']:
    used_prompts.update([st['system_prompt'],st['task_prompt']]); used_principles.update(st.get('principles',[]))
if used_prompts==prompt_files: ok('no orphan executable prompts')
else: fail(f'orphan/missing prompt refs orphan={sorted(prompt_files-used_prompts)} missing={sorted(used_prompts-prompt_files)}')
if used_principles==principle_files: ok('all principles are intentionally routed')
else: fail(f'principle reachability mismatch orphan={sorted(principle_files-used_principles)} missing={sorted(used_principles-principle_files)}')

# Critical generation topology
pos={v:i for i,v in enumerate(ids)}
def ordered(seq): return all(x in pos for x in seq) and [pos[x] for x in seq]==sorted(pos[x] for x in seq)
if ordered(['step_00','source_approval','step_01','step_02']): ok('Source IR Human authority boundary')
else: fail('Source IR approval topology broken')
if nodes[pos['source_approval']].get('revision_node')=='step_00': ok('Source IR revision returns to extractor')
else: fail('Source IR revision target wrong')
parts=[]
for i in range(1,5): parts += [f'step_07_part{i}',f'step_07_part{i}_review']
if ordered(parts+['step_07_full']): ok('neutral four-part drafting with Human boundaries')
else: fail('PART drafting topology broken')
# no act semantics in canonical workflow/prompts except research/provenance
exec_text='\n'.join((SKILL/p).read_text(encoding='utf-8') for p in sorted(used_prompts|used_principles)) + json.dumps(workflow)
if not re.search(r'\bact\s*(?:1|2a|2b|3)\b|step_07_act',exec_text,re.I): ok('no fixed act topology in execution contract')
else: fail('legacy fixed act topology leaked into execution')
rev=nodes[pos['step_08_reverse']]; ver=nodes[pos['step_08_verify']]; final=nodes[pos['final_release']]
if rev.get('inputs')==['output/final_screenplay.md']: ok('source-blind final reverse reconstruction')
else: fail(f"step_08_reverse leaked inputs: {rev.get('inputs')}")
if ver.get('inputs')==['output/step_00_source_ir.md','output/step_08_recovered_screenplay_ir.md']: ok('independent Source IR↔Screenplay IR comparison')
else: fail(f"step_08_verify answer-key leakage: {ver.get('inputs')}")
if ordered(['step_08_diagnosis','step_08_scope','step_08_apply','step_08_reverse','step_08_verify','final_release']): ok('Critic→Human→Writer→blind reconstruct→verify→Human chain')
else: fail('STEP 8 chain broken')
if final.get('source')=='output/step_08_verification.md' and final.get('revision_node')=='step_08_diagnosis': ok('final release can reopen repair cycle')
else: fail('final release repair boundary broken')
if 'STS_STATUS:' in ver.get('validation',{}).get('required_markers',[]): ok('final verification status marker required')
else: fail('final verification status marker not required')

# Source/scene contracts encoded in prompts
s0=(SKILL/'prompts/analyst/step_00_source_graph.md').read_text(); s6=(SKILL/'prompts/writer/step_06_scene_blueprint.md').read_text(); sv=(SKILL/'prompts/critic/step_08_verify.md').read_text()
for marker in ['SRC-','REL-','GAP','CON','AFFIRM|NEGATE','FACT|BELIEF|CLAIM|UNKNOWN','Ending Contract','Adaptation Preflight','WORLD_RULE','acquisition']:
    if marker not in s0: fail(f'Source IR missing {marker}')
if not any(e.startswith('Source IR missing') for e in errors): ok('typed Source IR contract')
for marker in ['Required preconditions','Required postconditions','Downstream dependencies','LOW|WATCH|HIGH','SCN-','Necessary function','Cinematic value','acquisition path']:
    if marker not in s6: fail(f'Scene IR missing {marker}')
if not any(e.startswith('Scene IR missing') for e in errors): ok('stateful Scene Transition contract')
for marker in ['PRESERVED','DRIFTED','UNCERTAIN','STS_STATUS: READY','STS_STATUS: BLOCKED']:
    if marker not in sv: fail(f'verifier missing {marker}')
if not any(e.startswith('verifier missing') for e in errors): ok('round-trip fidelity verdict contract')

# Audit topology/schemas
if aw.get('schema_version')==1 and aw.get('domain_id')=='sts-audit' and aw.get('domain_version')=='4.3.0': ok('audit domain identity')
else: fail('audit domain identity mismatch')
if [x['id'] for x in aw['stages']]==['source','screenplay','compare']: ok('audit three-stage topology')
else: fail('audit stage order')
if 'screenplay' in aw['stages'][0].get('blind_to',[]) and 'synopsis' in aw['stages'][1].get('blind_to',[]) and {'raw_synopsis','raw_screenplay'} <= set(aw['stages'][2].get('blind_to',[])): ok('audit evidence-path isolation')
else: fail('audit blind boundaries missing')
for rel in ['schemas/audit-source-ir.schema.json','schemas/audit-screenplay-ir.schema.json','schemas/audit-result.schema.json']:
    try:
        sc=load(SKILL/rel)
        if sc.get('$schema')!='https://json-schema.org/draft/2020-12/schema': fail(f'{rel}: wrong schema draft')
    except Exception as e: fail(f'{rel}: {e}')
if not any('wrong schema draft' in e for e in errors): ok('audit schemas Draft 2020-12')
res_schema=load(SKILL/'schemas/audit-result.schema.json')
res_txt=json.dumps(res_schema)
for x in ['SOURCE_ASSERTIONS','SOURCE_RELATIONS','ENDING_OUTCOME','ENDING_MEANING','ACTOR_OWNERSHIP','KNOWLEDGE_EPISTEMIC','FORM_FUNCTION','TRANSFORMATION_INVENTION','CAUSALITY','KNOWLEDGE_STATE','CHARACTER_RELATIONSHIP_STATE','PHYSICAL_SPATIAL_STATE','SCENE_DEPENDENCIES','PLAYABILITY','SCENE_ECONOMY']:
    if x not in res_txt: fail(f'audit result schema missing fixed dimension {x}')
if not any('fixed dimension' in e for e in errors): ok('fixed dual-axis audit dimensions')
src_schema=load(SKILL/'schemas/audit-source-ir.schema.json')
src_txt=json.dumps(src_schema)
for x in ['source_form','semantic_reframe','residue','world_rule','acquisition','trigger','reversibility']:
    if x not in src_txt: fail(f'audit source schema missing field {x}')
if not any('audit source schema missing' in e for e in errors): ok('audit Source IR carries rule/knowledge/form/ending semantics')
sp_schema=load(SKILL/'schemas/audit-screenplay-ir.schema.json')
sp_txt=json.dumps(sp_schema)
for x in ['form_functions','semantic_reframe','residue','world_rule','acquisition']:
    if x not in sp_txt: fail(f'audit screenplay schema missing field {x}')
if not any('audit screenplay schema missing' in e for e in errors): ok('audit Screenplay IR recovers rule/knowledge/form/ending semantics')


config_schema=load(SKILL/'schemas/project-config.schema.json')
source_formats=set(config_schema['properties']['source_format']['enum'])
engine_source_formats=set(re.findall(r'"([a-z_]+)"', re.search(r'const SOURCE_FORMATS: &\[&str\] = &\[(.*?)\];', (ROOT/'crates/sts/src/engine.rs').read_text(), re.S).group(1)))
if source_formats == engine_source_formats: ok('project-config source_format schema matches Runtime')
else: fail(f'project-config/runtime source_format drift schema={sorted(source_formats)} runtime={sorted(engine_source_formats)}')

# Eval coverage
cases=load(SKILL/'evals/cases.json')['cases']; acases=load(SKILL/'evals/audit-cases.json')['cases']
need=['ending','causal','belief','negation','nonlinear','connective','actor','state']
if all(any(k in c['id'] for c in cases) for k in need): ok('generation adversarial eval coverage')
else: fail('generation adversarial eval cases incomplete')
if any(c['id']=='one-to-many' for c in acases) and any(c['id']=='many-to-one' for c in acases) and any(c['id']=='source-ambiguity' for c in acases): ok('audit adaptation-alignment adversarial coverage')
else: fail('audit alignment cases incomplete')

# Field-study evidence is quarantined from runtime prompts
bench=ROOT/'benchmarks/short-story-154'
if (bench/'story-field-ledger.csv').is_file() and (bench/'summary.json').is_file() and (bench/'FIELD_LEDGER.md').is_file() and (bench/'corpus-manifest.json').is_file(): ok('154-story field evidence present')
else: fail('154-story field evidence missing')
if not any('benchmarks/short-story-154' in str(p) for p in used_prompts|used_principles): ok('field corpus evidence quarantined from runtime routing')
else: fail('field corpus evidence leaked into runtime routing')
try:
    import csv
    rows=list(csv.DictReader((bench/'story-field-ledger.csv').open(encoding='utf-8')))
    ids=[row['story_id'] for row in rows]
    expected=[f'{i:03d}' for i in range(1,155)]
    if ids==expected and len({row['sha256'] for row in rows})==154: ok('154-story field ledger complete and individually hashed')
    else: fail('154-story field ledger IDs/hashes incomplete')
    corpus_manifest=json.loads((bench/'corpus-manifest.json').read_text(encoding='utf-8'))
    corpus_rows=corpus_manifest.get('stories',[])
    corpus_map={x.get('story_id'):x.get('sha256') for x in corpus_rows}
    ledger_map={row['story_id']:row['sha256'] for row in rows}
    if corpus_manifest.get('story_count')==154 and corpus_map==ledger_map: ok('field ledger hashes bind to canonical 154-story corpus manifest')
    else: fail('field ledger/corpus manifest mismatch')
    cards=sorted((bench/'preflight').glob('[0-9][0-9][0-9].md')) if (bench/'preflight').is_dir() else []
    card_ids=[p.stem for p in cards]
    if card_ids==expected and (bench/'PREFLIGHT_INDEX.md').is_file(): ok('154-story individual STS preflight cards complete')
    else: fail('154-story individual STS preflight cards incomplete')
    card_contract=all('Adaptation Pressure: **UNRESOLVED until target duration/format is supplied.**' in p.read_text(encoding='utf-8') and '## Claim boundary' in p.read_text(encoding='utf-8') for p in cards)
    if card_contract and (bench/'VALIDITY.md').is_file(): ok('field cards preserve unresolved target scale and claim boundary')
    else: fail('field cards/validity boundary incomplete')
    summary=load(bench/'summary.json')
    if summary.get('stories')==154 and summary.get('high_severity_total')==16 and summary.get('high_severity_sts_matched')==16: ok('field-study aggregate oracle is internally consistent')
    else: fail('field-study aggregate oracle mismatch')
except Exception as e:
    fail(f'field-study ledger validation: {e}')


# Filmmaker/provenance isolation
names=['Nolan','Bong','Hitchcock','Park Chan','Kaufman','Lumet']
execution_files=[p for p in (SKILL/'prompts').rglob('*.md')]+[p for p in (SKILL/'principles').rglob('*.md')]
leaks=[]
for p in execution_files:
    t=p.read_text(encoding='utf-8')
    if any(name.lower() in t.lower() for name in names): leaks.append(str(p.relative_to(ROOT)))
if not leaks: ok('filmmaker research isolated from execution')
else: fail(f'filmmaker names leaked into execution: {leaks}')
heur=(SKILL/'principles/heuristics.md').read_text()
if 'Never Correctness Gates' in heur and 'three-act' in heur: ok('formula heuristics explicitly subordinate')
else: fail('heuristic boundary unclear')

# Rust/static runtime contracts
engine_paths=[ROOT/'crates/sts/src/engine.rs']+sorted((ROOT/'crates/sts/src/engine').rglob('*.rs'))
engine='\n'.join(path.read_text(encoding='utf-8') for path in engine_paths)
model_paths=[ROOT/'crates/sts/src/model.rs']+sorted((ROOT/'crates/sts/src/model').glob('*.rs'))
model='\n'.join(path.read_text(encoding='utf-8') for path in model_paths)
docx=(ROOT/'crates/sts/src/docx.rs').read_text()
screenplay_text=(ROOT/'crates/sts/src/screenplay_text.rs').read_text()
codex_paths=[ROOT/'crates/sts/src/codex.rs']+sorted((ROOT/'crates/sts/src/codex').rglob('*.rs'))
codex='\n'.join(path.read_text(encoding='utf-8') for path in codex_paths)
if 'const SKILL_DIR: &str = "skills/sts"' in engine and 'manifest.schema_version != 3' in engine: ok('STS engine routing/schema over shared Runtime')
else: fail('Runtime skill routing/schema mismatch')
if 'pub principles: Vec<String>' in model and 'for path in &node.principles' in engine and '# ROUTED PRINCIPLES' in engine: ok('bounded principles are runtime-addressable')
else: fail('Runtime principle routing incomplete')
if 'final release requires exactly one STS_STATUS: READY' in engine: ok('BLOCKED final cannot be approved')
else: fail('Runtime does not enforce READY release')
if 'strip_prefix("SCN-")' in screenplay_text and 'parse(markdown)' in docx and 'Token::Scene(text) => ("SceneHeading", text)' in docx: ok('canonical SCN→DOCX contract')
else: fail('DOCX SCN heading contract missing')
if 'STS_CODEX_TIMEOUT_SECS' in codex and 'STS_CODEX_BIN' in codex and 'S2S_CODEX_' not in codex: ok('STS Codex runtime environment contract')
else: fail('stale Codex env identity')

audit_tool=(ROOT/'tools/sts_audit.py').read_text()
if all(x in audit_tool for x in ['validate_ir_ids(source_ir, screenplay_ir)','validate_alignment_coverage(result, source_ir, screenplay_ir)','validate_verdict_coherence(result, source_ir)']): ok('audit LLM output is mechanically post-validated')
else: fail('audit mechanical post-validation wiring incomplete')
if 'gate_type.as_deref() == Some("feedback")' not in engine and 'optional_output' not in model: ok('legacy feedback gate removed')
else: fail('legacy feedback gate remains')

# Phase 1 runtime extraction
runtime_root=ROOT/'crates/runtime'
sts_root=ROOT/'crates/sts'
workspace=(ROOT/'Cargo.toml').read_text()
if all(x in workspace for x in ['"crates/runtime"','"crates/sts"','"src-tauri"']) and 'workflow-core' not in workspace:
    ok('workspace uses shared runtime + STS domain crates')
else: fail('workspace runtime extraction incomplete')
if all(x in engine for x in ['narrative_courier_runtime::store::', 'RuntimePaths', 'atomic_write_text', 'sha256_file', 'validate_relative_path']) and 'const SKILL_DIR: &str = "skills/sts"' in engine:
    ok('STS composes domain-neutral Runtime primitives')
else: fail('STS does not compose shared Runtime primitives')
forbidden_runtime=[r'\bsts\b',r'\bsynopsis\b',r'\bscreenplay\b',r'\btreatment\b',r'\bcritic\b',r'\bwriter\b',r'\banalyst\b',r'\bstep_0[0-9]\b']
runtime_owned_forbidden=['struct Event','struct GenerationPolicy','struct DomainIdentity','struct FileFingerprint']
runtime_leaks=[]
for rp in runtime_root.rglob('*.rs'):
    text=rp.read_text(encoding='utf-8')
    for pat in forbidden_runtime:
        if re.search(pat,text,re.I): runtime_leaks.append(f"{rp.relative_to(ROOT)}:{pat}")
if not runtime_leaks: ok('shared Runtime source is STS-domain neutral')
else: fail(f'shared Runtime contains STS-domain terms: {runtime_leaks}')
if not (ROOT/'crates/workflow-core').exists() and not any('screenplay-workflow' in p.read_text(encoding='utf-8',errors='ignore') for p in [ROOT/'Cargo.toml',ROOT/'src-tauri/Cargo.toml']):
    ok('legacy workflow-core crate removed from trunk')
else: fail('legacy workflow-core crate/dependency remains')

# Runtime ownership: shared runtime must not own STS state/domain DTOs.
runtime_src='\n'.join(p.read_text(encoding='utf-8') for p in (ROOT/'crates/runtime/src').rglob('*.rs'))
for marker_name in runtime_owned_forbidden:
    if marker_name in runtime_src:
        fail('Runtime owns domain/state DTO: '+marker_name)
sts_model=model
missing=[marker_name for marker_name in runtime_owned_forbidden if marker_name not in sts_model]
if missing:
    fail('STS state DTO ownership missing: '+str(missing))
else:
    ok('Runtime ownership boundary: STS state DTOs live in sts-workflow')

store_src='\n'.join(path.read_text(encoding='utf-8') for path in [ROOT/'crates/runtime/src/store.rs', *sorted((ROOT/'crates/runtime/src/store').rglob('*.rs'))])
process_src='\n'.join(path.read_text(encoding='utf-8') for path in [ROOT/'crates/runtime/src/process.rs', *sorted((ROOT/'crates/runtime/src/process').rglob('*.rs'))])
windows_publication_branches=re.findall(r'#\[cfg\(windows\)\]\s*\{(.*?)\}\s*#\[cfg\(not\(windows\)\)\]', store_src, re.S)
if len(windows_publication_branches)==2 and all('return Err' in branch and 'fs::' not in branch for branch in windows_publication_branches):
    ok('Windows atomic-write gap is explicit failure, not delete-then-rename')
else:
    fail('Windows atomic-write contract can silently lose canonical path')
if 'process-tree lifecycle is not implemented on this platform' in process_src and '#[cfg(not(unix))]' in process_src:
    ok('non-Unix process-tree contract fails explicitly')
else:
    fail('non-Unix process-tree lifecycle is silently weakened')

# CLI / Tauri / UI
cli=(ROOT/'src-tauri/src/bin/narrative-courier-cli.rs').read_text()
conf=(ROOT/'src-tauri/tauri.conf.json').read_text()
ui=(ROOT/'ui/app.js').read_text()
if 'name = "narrative-courier-cli"' in (ROOT/'src-tauri/Cargo.toml').read_text() and 'src/bin/narrative-courier-cli.rs' in (ROOT/'src-tauri/Cargo.toml').read_text(): ok('NarrativeCourier CLI identity')
else: fail('CLI cargo identity')
if '../skills/sts/' in conf: ok('Tauri STS resource bundle')
else: fail('Tauri skill resource missing')
for passed, message in inspect_tauri_wiring(ROOT, ui):
    (ok if passed else fail)(message)

# package version: compare declared values instead of pinning a historical release string.
import tomllib
versions={}
for rel in ['crates/runtime/Cargo.toml','crates/contracts/Cargo.toml','crates/sts/Cargo.toml','crates/delivery/Cargo.toml','src-tauri/Cargo.toml']:
    versions[rel]=tomllib.loads((ROOT/rel).read_text())['package']['version']
versions['package.json']=json.loads((ROOT/'package.json').read_text())['version']
versions['src-tauri/tauri.conf.json']=json.loads((ROOT/'src-tauri/tauri.conf.json').read_text())['version']
if len(set(versions.values())) == 1: ok(f"package version {next(iter(versions.values()))} synchronized")
else: fail(f'package version drift: {versions}')

# Legacy names only allowed in provenance/notice/changelog
allowed={'NOTICE.md','CHANGELOG.md','skills/sts/references/research-decisions.md','skills/sts/references/craft-provenance.md'}
legacy=[]
for p in ROOT.rglob('*'):
    if not p.is_file() or p.name=='MANIFEST.sha256': continue
    rel=str(p.relative_to(ROOT)).replace('\\','/')
    if rel in allowed: continue
    try:t=p.read_text(encoding='utf-8')
    except UnicodeDecodeError: continue
    old_skill = 'skills/' + 'synopsis' + '-to-' + 'scenario'
    old_short = r'\b' + 's' + '2' + 's' + r'\b'
    if old_skill in t or re.search(old_short,t,re.I): legacy.append(rel)
if not legacy: ok('legacy runtime/domain identifiers removed')
else: fail(f'legacy identifiers remain: {legacy}')

print(f"\nRESULT: {len(passes)} PASS / {len(warnings)} WARN / {len(errors)} ERROR")
if errors: sys.exit(1)
