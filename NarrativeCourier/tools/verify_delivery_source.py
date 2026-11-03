#!/usr/bin/env python3
from pathlib import Path
import json, re, sys
import digest as _digest
ROOT=Path(__file__).resolve().parents[1]
passes=[]; errors=[]
def ok(x): passes.append(x); print('PASS',x)
def fail(x): errors.append(x); print('FAIL',x)
def require(cond, good, bad=None): ok(good) if cond else fail(bad or good)
def flat(s): return ''.join(s.split())  # whitespace-insensitive matching; keeps gates robust to rustfmt line wrapping without weakening any needle.
# Private module extraction keeps each static gate scoped to its original domain owner.
def source_bundle(*paths): return '\n'.join((ROOT/path).read_text() for path in paths)

# Canonical contract/protocol bytes are the implementation boundary, not a second schema fork.
for group,names in {
  # The package contract intentionally moved to v2.  The remaining delivery
  # contracts are still mirrored by the historical Blueprint v1.2 source
  # boundary and therefore remain byte-for-byte checked here.
  'contracts':['delivery-request.schema.json','director-plan.schema.json','asset-index.schema.json','delivery-set.schema.json'],
  'protocol':['agent-executor-request.schema.json','agent-executor-response.schema.json','media-backend-request.schema.json','media-backend-response.schema.json'],
}.items():
    for name in names:
        a=ROOT/group/name; b=ROOT/'docs/blueprint'/group/name
        require(a.is_file() and b.is_file() and a.read_bytes()==b.read_bytes(), f'{group}/{name} matches Blueprint v1.2')

# NarrativePackage is a breaking v2 contract: it is no longer compared with
# the retired Blueprint v1.2 mirror.  Keep the new boundary explicit so a
# future accidental downgrade cannot pass this source gate.
package_schema=json.loads((ROOT/'contracts/narrative-package.schema.json').read_text())
require(package_schema.get('$id') == 'https://narrative-courier.local/contracts/narrative-package/v2.schema.json',
        'NarrativePackage uses the v2 schema id')
require(package_schema.get('title') == 'NarrativePackage v2' and package_schema.get('properties', {}).get('schema_version', {}).get('const') == 2,
        'NarrativePackage declares schema version 2')
require('scene_map' in package_schema.get('required', []),
        'NarrativePackage v2 requires a structured scene_map')
scene_map_schema=json.loads((ROOT/'contracts/narrative-scene-map-v2.schema.json').read_text())
require(scene_map_schema.get('$id') == 'https://narrative-courier.local/contracts/narrative-scene-map/v2.schema.json'
        and scene_map_schema.get('title') == 'NarrativeSceneMap v2'
        and scene_map_schema.get('properties', {}).get('schema_version', {}).get('const') == 2,
        'NarrativeSceneMap uses the v2 schema identity')
require(scene_map_schema.get('properties', {}).get('map_id', {}).get('pattern') == r'^nsm2_[0-9a-f]{64}$'
        and 'semantic_manifest' in scene_map_schema.get('required', []),
        'NarrativeSceneMap v2 requires nsm2 identity and semantic_manifest')
bundle_schema=json.loads((ROOT/'contracts/release-bundle.schema.json').read_text())
for name, schema in [('NarrativePackage', package_schema), ('NvcReleaseBundle', bundle_schema)]:
    scene_map_ref = schema.get('properties', {}).get('scene_map', {})
    scene_map_def = schema.get('$defs', {}).get('scene_map_artifact_ref', {})
    require(scene_map_ref.get('$ref') == '#/$defs/scene_map_artifact_ref'
            and scene_map_def.get('properties', {}).get('kind', {}).get('const') == 'narrative_scene_map_v2',
            f'{name} scene_map artifact kind is narrative_scene_map_v2')
release_package_ref = bundle_schema.get('properties', {}).get('narrative_package', {})
release_package_def = bundle_schema.get('$defs', {}).get('narrative_package_artifact_ref', {})
release_state_def = bundle_schema.get('$defs', {}).get('completed_state_artifact_ref', {})
require(release_package_ref.get('$ref') == '#/$defs/narrative_package_artifact_ref'
        and release_package_def.get('properties', {}).get('kind', {}).get('const') == 'narrative_package_v2'
        and release_package_def.get('properties', {}).get('path', {}).get('const') == 'narrative-package.json',
        'NvcReleaseBundle narrative_package is the canonical v2 artifact path and kind')
release_scene_map_def = bundle_schema.get('$defs', {}).get('scene_map_artifact_ref', {})
require(release_scene_map_def.get('properties', {}).get('path', {}).get('const') == 'output/scene-map.json',
        'NvcReleaseBundle scene_map is the canonical v2 artifact path')
require(release_state_def.get('properties', {}).get('path', {}).get('const') == 'state.json',
        'NvcReleaseBundle completed_state is the canonical artifact path')
require(bundle_schema.get('properties', {}).get('bundle_id', {}).get('pattern') == r'^nvcb_[0-9a-f]{64}$',
        'NvcReleaseBundle bundle_id declares the nvcb hash identity')

# ReviewedSynopsisHandoff is produced by geultu and consumed by NVC. Keep the
# producer schema as the one source of truth: a separately edited consumer
# copy is a contract fork, even when its JSON semantics appear equivalent.
reviewed_schema=ROOT/'contracts/reviewed-synopsis-handoff.schema.json'
producer_reviewed_schema=ROOT.parent/'geultu/docs/canonical/contracts/reviewed-synopsis-handoff.schema.json'
producer_pin='ec0f35450e38d8bcabb116e49e37ce69efe38a020a4ef921bb1a1458f8335e2b'
reviewed_schema_bytes=reviewed_schema.read_bytes() if reviewed_schema.is_file() else None
producer_reviewed_schema_bytes=producer_reviewed_schema.read_bytes() if producer_reviewed_schema.is_file() else None
require(reviewed_schema_bytes is not None and producer_reviewed_schema_bytes is not None
        and reviewed_schema_bytes==producer_reviewed_schema_bytes,
        'ReviewedSynopsisHandoff schema is byte-identical to geultu producer')
if reviewed_schema_bytes is not None:
    require(_digest.sha256_bytes(reviewed_schema_bytes)==producer_pin,
            f'ReviewedSynopsisHandoff schema matches producer pin {producer_pin}')

workspace=(ROOT/'Cargo.toml').read_text()
for crate in ['crates/runtime','crates/contracts','crates/sts','crates/delivery','src-tauri']:
    require(crate in workspace, f'workspace includes {crate}')

contract_source_root=ROOT/'crates/contracts/src'
contract_files=sorted(contract_source_root.rglob('*.rs'))
contracts='\n'.join(path.read_text() for path in contract_files)
contract_facade=(contract_source_root/'lib.rs').read_text()
for dto in ['NarrativePackage','DeliveryRequest','DirectorPlan','AssetIndex','DeliverySet']:
    owners=[path for path in contract_files if re.search(rf'pub struct {dto}\b',path.read_text())]
    exported=re.search(rf'pub use\s+[^;]*\b{dto}\b',contract_facade,re.S) is not None
    require(len(owners)==1 and exported,f'canonical DTO {dto} has one shared owner and crate-root export')
require('validate_delivery_set' in contracts and 'validate_director_plan' in contracts,'cross-field/cross-document invariant validators are executable')
require('DeliverySet status must equal aggregated delivery status' in contracts,'DeliverySet status aggregation is exact, not only false-PASS guarded')
require('checked_mul' in contracts and 'aspect_ratio overflows validation domain' in contracts,'aspect-ratio validation rejects arithmetic overflow')
require('NARRATIVE_SCENE_MAP_SCHEMA_VERSION: u32 = 2' in contracts
        and 'semantic_manifest_hash mismatch' in contracts
        and 'map_id hash mismatch' in contracts
        and 'bundle_id is not bound to its body' in contracts,
        'Rust v2 SceneMap and release validators enforce hash identities')

sts=source_bundle('crates/sts/src/engine.rs', 'crates/sts/src/engine/release.rs',
                  'crates/sts/src/engine/release/bundle.rs',
                  'crates/sts/src/engine/release/narrative_package.rs',
                  'crates/sts/src/engine/release/scene_map.rs',
                  'crates/sts/src/engine/workflow/decisions.rs',
                  'crates/sts/src/engine/workflow/resume.rs')
final_block=sts[sts.index('if node.id == "final_release"'):sts.index('let text = feedback')]
require(final_block.index('self.publish_narrative_package') < final_block.index('rec.status = NodeStatus::Completed'),'NarrativePackage publishes before final Completed commit')
require('verify_narrative_package_at(&project, &state, &manifest)' in sts,'doctor/release path verifies NarrativePackage freshness')
require('source_snapshot_sha256: hash_jcs(snapshot)?' in sts,'NarrativePackage binds canonical JCS source snapshot hash')
scene_map_source=source_bundle('crates/sts/src/scene_map.rs',
                               'crates/sts/src/scene_map/screenplay.rs',
                               'crates/sts/src/scene_map/blueprint.rs',
                               'crates/sts/src/scene_map/semantic_manifest.rs')
require('narrative_scene_map_v2' in sts and 'nsm2_' in scene_map_source,
        'STS release producer emits the v2 scene-map artifact kind and identity')

planner=source_bundle('crates/delivery/src/planner.rs',
                      'crates/delivery/src/planner/scene.rs',
                      'crates/delivery/src/planner/plan.rs',
                      'crates/delivery/src/planner/plan/request_policy.rs',
                      'crates/delivery/src/planner/plan/timing.rs',
                      'crates/delivery/src/planner/preflight.rs')
require('Coverage::Summary | Coverage::Mood' in planner and 'UNSUPPORTED_CAPABILITY' in planner,'planner rejects semantic reinterpretation capabilities it does not own')
require('validate_supported_request' in planner and 'free-form delivery constraints require a reviewed DirectorIR planner' in planner and 'cannot prove non-SCN must_include_ref' in planner,'planner rejects request semantics it cannot honor')
require('ProjectionOperation::Preserve' in planner and 'unit.content.as_deref() != Some(*canonical)' in planner,'semantic preflight enforces exact SCN PRESERVE content')
require('does not preserve the requested SCN sequence' in planner,'FULL projection checks exact scene coverage/order')
require('request.asset_inputs' in planner and 'GenerationMode::User | GenerationMode::Derived' in planner and 'asset_refs: asset_requests.iter()' in planner,'USER/DERIVED asset inputs are promoted into reachable DirectorPlan asset requests')

backend=source_bundle('crates/delivery/src/backend.rs',
                      'crates/delivery/src/backend/types.rs',
                      'crates/delivery/src/backend/discovery.rs',
                      'crates/delivery/src/backend/routing.rs')
delivery_facade=(ROOT/'crates/delivery/src/lib.rs').read_text()
require('BackendLimits' in delivery_facade,'public backend descriptor field types are reachable from the crate root')
require('pub fn describe_backends' in backend and 'pub fn route_output' in backend and 'pub fingerprint: String' in backend and 'pub limits: BackendLimits' in backend,'capability routing is descriptor-driven and DESCRIBE-aligned')
require('AMBIGUOUS_BACKEND' in backend and 'output.backend_id.as_deref()' in backend,'routing rejects ambiguity and uses the request-owned backend pin')
require('width_multiple: Some(2)' in backend and 'supports(output, descriptor)' in backend,'routing enforces backend limits before render')
require('#[cfg(unix)]' in backend and 'process-tree cancellation' in backend,'external capabilities are not advertised without process-tree lifecycle support')
require('CapabilityRegistry' not in backend,'no unnecessary capability registry layer')
delivery_run=(ROOT/'crates/delivery/src/engine/run.rs').read_text()
require('route_output(output, &descriptors)' in delivery_run,'delivery routing has one backend pin source and no compatibility pin argument')

delivery_engine_root=ROOT/'crates/delivery/src/engine'
engine=source_bundle(
    'crates/delivery/src/engine.rs',
    *(path.relative_to(ROOT) for path in sorted(delivery_engine_root.rglob('*.rs'))),
)
pre=engine.index('semantic_preflight')
require(pre < engine.index('materialize_assets',pre) and pre < engine.index('render_output',pre),'semantic BLOCK occurs before materialize/render side effects')
require('index_id: format!("INDEX-{plan_sha}")' in engine,'AssetIndex identity is deterministic')
require('admit_materialized_index(project, &asset_index, &plan)?' in engine and 'read_materialized_image(project, item)?' in engine and 'item.size_bytes != bytes.len() as u64' in engine,'AssetIndex persisted files are hash/size revalidated')
require('existing immutable blob differs; explicit recovery is required' in engine and 'publish_blob(project, &rel, &bytes, &actual)?' in engine,'corrupted immutable asset blobs fail explicitly instead of implicit repair')
require('verify_package_files(project, &canonical)' in engine
        and 'if verify_released_package(project, package)? != package_sha' in engine
        and 'STALE_RESULT: STS release changed during delivery run' in engine,
        'delivery rejects stale/tampered narrative before canonical result publish')
require('claim_publish_ownership' in engine and 'fs::rename(&active, &cancel)' in engine and 'fs::rename(&active, &publishing)' in engine,'cancel and final publish are linearized by one atomic lease race')
require('pub struct DeliveryRecord' in engine,'request identity stays transport-only instead of expanding DeliverySet contract')
require('verify_released_package' in engine and 'supplied package is not the current canonical release' in engine,'delivery consumes an explicit released NarrativePackage and verifies canonical identity')
require('load_contract_set' in engine and 'persisted delivery contract chain' in engine,'saved DeliverySet is revalidated against request/plan/AssetIndex hash chain')
require('request_id cannot identify different request content' in engine and 'return Self::load(project, package, &request.request_id)' in engine,'delivery request_id is immutable and completed identical runs are idempotent')
runtime_paths=(ROOT/'crates/runtime/src/paths.rs').read_text()
require('pub fn validate_path_segment' in runtime_paths
        and 'fn is_valid_ascii_path_segment' in runtime_paths
        and '!is_valid_ascii_path_segment(value)' in runtime_paths,
        'path-segment invariant is owned by Runtime paths')
runtime_store=source_bundle('crates/runtime/src/store.rs',
                            *(path.relative_to(ROOT) for path in sorted((ROOT/'crates/runtime/src/store').rglob('*.rs'))))
require(flat('pub fn load(project: &Path, package: &NarrativePackage') in flat(engine)
        and flat('let package_sha = verify_released_package(project, package)?; let set = load_contract_set(project, package, request_id)?; if set.narrative_package_sha256 != package_sha') in flat(engine)
        and 'verify_delivery_artifacts(project, &set)?' in engine
        and 'for delivery in &set.deliveries' in engine,
        'delivery load rechecks current release identity and published artifact hashes')
require('project_owned_path(project, "narrative-package.json")?' in engine
        and 'format!("visual/requests/{request_id}")' in engine
        and 'delivery-request.json' in engine,
        'delivery persistence stays outside STS output authority')
require('visual/assets/blobs/' in engine,'materialized assets publish under canonical visual asset boundary')
require('output/deliveries' not in engine and 'output/narrative-package.json' not in engine,'legacy delivery paths are absent')
require('pub fn list(project: &Path, package: &NarrativePackage)' in engine and 'set.narrative_package_sha256 != package_sha' in engine,'delivery list is scoped to the current released NarrativePackage')

render=source_bundle('crates/delivery/src/render.rs',
                     'crates/delivery/src/render/cache.rs',
                     'crates/delivery/src/render/document.rs',
                     'crates/delivery/src/render/document/html.rs',
                     'crates/delivery/src/render/document/svg.rs',
                     'crates/delivery/src/render/document/graph.rs',
                     'crates/delivery/src/render/process.rs',
                     'crates/delivery/src/render/verification.rs')
require('match (backend.backend_id.as_str(), output.format)' in render,'execution is bound to the selected backend identity and output format')
require('.output()' not in backend and '.output()' not in render and 'run_piped_command_capture' in backend and 'run_piped_command_capture' in render,'delivery external probes share Runtime timeout/cancel ownership')
require('backend_fingerprint' in render and 'BuildKey' in render,'render cache key binds backend fingerprint')
require('visual/deliveries/{}/' in render and 'output/deliveries' not in render,'render artifacts publish under canonical visual delivery boundary')
require('graph_json' in render and '"kind":"sequence"' in render,'dependency-free graph projection is implemented')
require('ffprobe_video' in render and 'generated frame count differs from plan' in render,'video verifies exact frame count')
require('raster/video observable semantics require human preview' in render,'raster/video cannot falsely semantic-PASS')
require('atomic_publish_file(&staged, &target)' in render and 'pub fn atomic_publish_file' in runtime_store,'Runtime owns canonical render artifact publication')
require('technical_verify(&target, output, control)?' in render and 'CACHE_ARTIFACT_INVALID' in render and 'relevant_asset_keys' in render and 'fs::remove_file(&target)' not in render,'output-local build key safely reuses only revalidated content-addressed artifacts')
require('exact_html_semantics' in render and 'exact_graph_semantics' in render and 'does not exactly preserve planned' in render, 'HTML/JSON semantic PASS is proven from artifact bytes')
require('unit-{:06}' in render,'renderer temp paths do not trust contract unit_id as filesystem path')
require('StageCleanup' in render and 'own_stage(stage.clone())' in render and 'RENDER_CLEANUP_FAILED' in render,'render staging is cleaned on success, failure, and cancel paths')

ui=(ROOT/'ui/app.js').read_text()
# Delivery commands live in their registered adapter module, not in the
# composition root. Read both owners so the guard follows the current module
# boundary instead of requiring command bodies to stay in lib.rs.
tauri=source_bundle('src-tauri/src/lib.rs', 'src-tauri/src/commands/delivery.rs')
cli=source_bundle('src-tauri/src/bin/narrative-courier-cli.rs',
                  'src-tauri/src/bin/commands.rs',
                  'src-tauri/src/bin/commands/arguments.rs',
                  'src-tauri/src/bin/commands/sts.rs',
                  'src-tauri/src/bin/commands/delivery.rs')
for cmd in ['delivery_context','run_delivery','cancel_delivery','review_delivery','list_deliveries','read_delivery_preview']:
    require(cmd in tauri,f'Tauri exposes {cmd}')
require('data-route="deliveries"' in (ROOT/'ui/index.html').read_text(),'UI has one Deliveries surface')
require('frame.setAttribute("sandbox","")' in ui,'HTML preview is sandboxed')
require('request_id' in ui and 'fn read_delivery_preview(' in tauri
        and 'request_id: String' in tauri
        and 'DeliveryEngine::read_artifact_bytes' in tauri,
        'UI preview uses transport request identity, not cache-path inference')
require(tauri.count('engine.narrative_package(&name)') >= 4 and cli.count('engine.narrative_package(name)') >= 3,'official Tauri/CLI delivery run/read/list paths require STS Completed release API')
require('deliver-cancel' in cli and 'deliver-context' in cli and 'deliver-review' in cli,'CLI exposes delivery inspect/run/review/cancel boundary')
runtime_process=source_bundle('crates/runtime/src/process.rs',
                              *(path.relative_to(ROOT) for path in sorted((ROOT/'crates/runtime/src/process').rglob('*.rs'))))
codex_paths=[ROOT/'crates/sts/src/codex.rs']+sorted((ROOT/'crates/sts/src/codex').rglob('*.rs'))
codex='\n'.join(path.read_text(encoding='utf-8') for path in codex_paths)
require('libc::O_NONBLOCK' in runtime_process and 'started.elapsed() >= timeout' in runtime_process and 'pipe.write(&stdin_bytes[delivered..end])' in runtime_process,'nonblocking stdin delivery shares the child timeout/cancel loop')
require('.output()' not in codex and 'codex_probe' in codex and 'run_piped_command_capture' in codex,'Codex readiness probes share bounded Runtime process lifecycle')
require('pub fn review(' in engine and 'Human visual review rejected the artifact' in engine,'NEEDS_REVIEW has a minimal Human approve/reject lifecycle')
require('read_materialized_image(project, item)?' in render and 'data-asset=' in render and all(m in render for m in ['image/png','image/jpeg','image/webp']),'safe raster assets are user-visible without a second visual planner')
vertical=(ROOT/'crates/delivery/tests/vertical.rs').read_text()
require('ASSET-REF' in vertical and 'DeliveryEngine::review' in vertical and 'data:image/png;base64,' in vertical,'delivery vertical regression covers asset materialize/reuse/review')
runtime_tests=(ROOT/'crates/runtime/tests/primitives.rs').read_text()
require('piped_command_timeout_covers_blocked_stdin_delivery' in runtime_tests,'Runtime regression covers blocked-stdin timeout')

# JSON schemas must all parse.
for p in list((ROOT/'contracts').glob('*.json'))+list((ROOT/'protocol').glob('*.json')):
    try: json.loads(p.read_text()); ok(f'JSON parses {p.relative_to(ROOT)}')
    except Exception as e: fail(f'JSON invalid {p.relative_to(ROOT)}: {e}')

print(f'\nRESULT: {len(passes)} PASS / {len(errors)} ERROR')
sys.exit(1 if errors else 0)
