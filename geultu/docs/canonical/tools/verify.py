#!/usr/bin/env python3
"""Geultu canonical documentation package verifier v4.0.0

Checks: structure, JSON validity, schema registry consistency, contract
properties, fixture semantics, doc duplication guards, data-asset integrity
(file-level SHA256SUMS + directory aggregates), package manifest.
Writes evidence/verification-report.json and exits non-zero on any failure.
"""
from __future__ import annotations
import hashlib, json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]          # docs/canonical
REPO = Path(__file__).resolve().parents[3]          # repository root
PASS = []
FAIL = []


def check(name, cond, detail=''):
    (PASS if cond else FAIL).append({'name': name, 'detail': detail})


def load(rel):
    return json.loads((ROOT / rel).read_text(encoding='utf-8'))


# --- 1. required files -------------------------------------------------------
for rel in [
    'VERSION', 'README.md',
    'documentation/00_INDEX.md',
    'documentation/17_ADOPTION_BASELINE.md',
    'documentation/18_CURRENT_STATE.md',
    'documentation/19_DATA_ASSET_REGISTRY.md',
    'documentation/20_VERIFICATION_GATES.md',
    'contracts/schema-registry.json',
    'contracts/geultu-story-package.schema.json',
    'contracts/artifact-commit.schema.json',
    'contracts/validation-receipt.schema.json',
    'catalogs/authority.json',
    'catalogs/failure-codes.json',
    'fixtures/t33/valid-minimal.json',
    'evidence/asset-aggregates.json',
]:
    check('exists:' + rel, (ROOT / rel).is_file())

# numbered documentation completeness 01..16
missing_docs = [f'{i:02d}_' for i in range(1, 17)
                if not list((ROOT / 'documentation').glob(f'{i:02d}_*.md'))]
check('docs_01_to_16_present', not missing_docs, ','.join(missing_docs))

# --- 2. all JSON parses ------------------------------------------------------
for p in sorted(ROOT.rglob('*.json')):
    try:
        json.loads(p.read_text(encoding='utf-8'))
        check('json:' + str(p.relative_to(ROOT)), True)
    except Exception as e:
        check('json:' + str(p.relative_to(ROOT)), False, str(e))

# --- 3. version consistency --------------------------------------------------
version = (ROOT / 'VERSION').read_text(encoding='utf-8').strip()
reg = load('contracts/schema-registry.json')
check('registry_version_matches_package', reg.get('version') == version,
      f"registry={reg.get('version')} package={version}")

ids = [x['id'] for x in reg['schemas']]
check('schema_ids_unique', len(ids) == len(set(ids)))
check('schema_files_exist', all((ROOT / 'contracts' / x['file']).is_file() for x in reg['schemas']))

# --- 4. key contract properties ---------------------------------------------
story = load('contracts/geultu-story-package.schema.json')
check('story_package_flat', 'body' not in story['properties'] and 'artifact_ref' not in story['properties'])
check('story_package_schema_id', story['properties']['schema_id'].get('const') == 'geultu.dev/geultu-story-package/v1')
check('story_package_closed', story.get('additionalProperties') is False)
check('story_package_two_inputs', set(story['properties']['source']['required']) == {'text_master', 'export_spec'})
check('story_package_score_number', story['properties']['scenes']['items']['properties']['visual_priority']['type'] == 'number')
check('legacy_story_package_schema_absent', not (ROOT / 'contracts/t33-narrative-package.schema.json').exists())
story_targets = [entry for entry in reg['schemas'] if entry.get('id') == 'https://geultu.dev/schema/geultu-story-package/v1']
check('story_package_single_target_schema', len(story_targets) == 1 and story_targets[0].get('file') == 'geultu-story-package.schema.json')
check('story_package_envelope_target_absent', not any('geultu_story_package' in entry.get('file', '') for entry in reg['schemas']))
check('story_package_envelope_file_absent', not (ROOT / 'contracts/artifacts/geultu_story_package.v2.schema.json').exists())

commit = load('contracts/artifact-commit.schema.json')
check('named_output_map', commit['properties']['outputs']['type'] == 'object'
      and 'additionalProperties' in commit['properties']['outputs'])
receipt = load('contracts/validation-receipt.schema.json')
check('program_receipt_outcome', set(receipt['properties']['outcome']['enum']) == {'PASS', 'FAIL'})
state = load('contracts/text-run-state.schema.json')
check('state_budgets', set(state['properties']['budgets']['required']) == {'revision_remaining', 'pivot_remaining'})
check('state_frozen_inputs', set(state['properties']['frozen_inputs']['required']) == {'synopsis', 'style', 'canon', 'genre'})

# --- 5. fixture semantics ----------------------------------------------------
valid = load('fixtures/t33/valid-minimal.json')
check('fixture_flat', 'body' not in valid and valid.get('schema_id') == 'geultu.dev/geultu-story-package/v1')
legacy = load('fixtures/t33/invalid-legacy-envelope.json')
check('fixture_legacy_is_invalid_shape', 'body' in legacy and legacy.get('schema_id') != 'geultu.dev/geultu-story-package/v1')
caller = load('fixtures/t33/invalid-caller-pass.json')
check('fixture_caller_pass_is_extra', 'passed' in caller and 'passed' not in story['properties'])
score = load('fixtures/t33/invalid-score-out-of-range.json')
check('fixture_score_out_of_range', score['scenes'][0]['visual_priority'] > 1)

# --- 6. markdown duplication guard ------------------------------------------
for p in sorted((ROOT / 'documentation').glob('*.md')):
    txt = p.read_text(encoding='utf-8')
    check('doc_no_embedded_json_schema:' + p.name,
          '"$schema"' not in txt and '"additionalProperties"' not in txt)

# --- 7. data assets ----------------------------------------------------------
agg_doc = load('evidence/asset-aggregates.json')


def dir_aggregate(d: Path) -> tuple[int, str]:
    lines = []
    count = 0
    for f in sorted(x for x in d.rglob('*') if x.is_file() and x.name != 'SHA256SUMS'):
        digest = hashlib.sha256(f.read_bytes()).hexdigest()
        lines.append(f'{digest}  {f.relative_to(d)}')
        count += 1
    blob = '\n'.join(lines) + ('\n' if lines else '')
    return count, hashlib.sha256(blob.encode()).hexdigest()


for entry in agg_doc.get('assets', []):
    name = entry['name']
    adir = REPO / entry['path']
    if not adir.is_dir():
        check('asset_dir:' + name, False, f'missing {adir}')
        continue
    check('asset_dir:' + name, True)
    actual_count, actual_agg = dir_aggregate(adir)
    check('asset_file_count:' + name, actual_count == entry['file_count'],
          f'declared={entry["file_count"]} actual={actual_count}')
    check('asset_aggregate:' + name, actual_agg == entry['aggregate_sha256'],
          f'declared={entry["aggregate_sha256"][:16]}... actual={actual_agg[:16]}...')
    sums = adir / 'SHA256SUMS'
    ok, bad = True, []
    for line in sums.read_text(encoding='utf-8').splitlines():
        if not line.strip():
            continue
        digest, rel = line.split('  ', 1)
        f = adir / rel
        actual = hashlib.sha256(f.read_bytes()).hexdigest() if f.is_file() else None
        if actual != digest:
            ok = False
            bad.append(rel)
    check('asset_sums:' + name, ok, ','.join(bad[:5]))

# --- 8. package manifest ------------------------------------------------------
manifest = ROOT / 'manifests' / 'SHA256SUMS'
if manifest.exists():
    ok, details = True, []
    for line in manifest.read_text(encoding='utf-8').splitlines():
        if not line.strip():
            continue
        digest, rel = line.split('  ', 1)
        # The report is generated by this verifier. Including it in the
        # manifest check would create a self-referential hash that can never
        # remain stable after the report is rewritten.
        if rel == 'evidence/verification-report.json':
            continue
        p = ROOT / rel
        actual = hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else None
        if actual != digest:
            ok = False
            details.append(rel)
    check('manifest_integrity', ok, ','.join(details[:5]))
else:
    check('manifest_integrity', False, 'manifests/SHA256SUMS missing')

report = {'schema_id': 'geultu.dev/verification-report/v1', 'package_version': version,
          'pass_count': len(PASS), 'fail_count': len(FAIL),
          'passes': PASS, 'failures': FAIL}
(ROOT / 'evidence' / 'verification-report.json').write_text(
    json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + '\n', encoding='utf-8')
print(json.dumps({'pass': len(PASS), 'fail': len(FAIL)}, ensure_ascii=False))
sys.exit(1 if FAIL else 0)
