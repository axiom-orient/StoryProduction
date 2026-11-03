# SceneAssetCompiler — 현재 구현

2026-10-03 animatic 공개 실행기를 SAC-owned Rust `sac-animatic`으로 이식했다.
`cargo test` 11개와 release build는 PASS다. 실제 이미지 provider·미술 품질·일반 SAC
release qualification은 이 합성 animatic 검증에 포함하지 않았다.

정밀 재검토에서 receipt 존재 조회도 held directory FD로 옮겼다. root 이름 교체 뒤에도
원본 receipt 조회·same-byte replay가 유지되는 회귀 검사와 fmt/Clippy/release build가 PASS다.
SAC의 integral-float bytes/contentHash는 그대로이며 SMC가 이 producer profile의 기존
approved receipt를 소비하는 실제 native CLI 검사도 PASS했다.

After the docs-only update, `PYTHONDONTWRITEBYTECODE=1 python3 tools/source_manifest.py generate`
and `verify` passed for the skill, docs, and release inventories. These metadata checks do not
qualify code behavior. Python/Rust suites, builds, live providers, real PSD postflight, and the full
release gate were not run after this documentation revision; the original G09 PSD evidence gap
remains unresolved.

## 현재 연결

| 범위 | 직접 읽은 근거·owner | 현재 의미 |
|---|---|---|
| 일반 workflow | [active CLI](../.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py) | Authority·tasks·attempts·execution·mechanical/visual review·selection·Human policy·AssetPack |
| Scene release | [asset release](../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/asset_release.py)·builder/validation/files | SMC projection binding·subjectHash 승인·handoff |
| Scene/PSD | scene_plan/units/locks·psd/validation/profile | GenerationUnit·layer/placeholder lock와 외부 PSD의 사후검사 |
| Packaging | [preservation owner](../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/packaging_preservation.py) | Exact attempt/substrate/PSD/render-back/comparison receipt·metrics 결속 |
| Provider | [facade](../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/scene_provider.py)·identity/preflight/attempt/protocol/process/inspection | Reference-free PNG 한 dispatch와 durable evidence, unknown no retry |
| Animatic assets | [Rust owner](../native/src/lib.rs) | NVC pinned packet·실제 PNG 구조/alpha/hash/coverage·candidate·별도 review |
| 배포 계약 | active skill·docs mirror·contracts·source manifests | Root provider 정책·producer/consumer pin·독립 integrity scope |

File publication·hash owner·provider preflight·packaging/visual evidence·scene contract의 책임을 정리했다.
자세한 module map·변경은 [ANALYSIS](../ANALYSIS.md), 계약은 [SPEC](SPEC.md),
흐름은 [ARCHITECTURE](ARCHITECTURE.md)에 둔다. 문서 mirror나 memory index가 새 runtime authority는 아니다.

Rust tests는 duplicate JSON key, integer canonical hash, PNG CRC·후행 bytes·decoded alpha,
asset coverage·symlink, exact review hash·immutable retry·pixel drift·동시 불변 발행을 확인했다. 기존
Story/NVC/SAC wire schema는 변경하지 않았다. 일반 asset generation의 Python runtime은
독립 owner 경로로 남으며 전체 SAC를 Rust로 전환했다고 표시하지 않는다.

제거 전 격리 Python 3.12/jsonschema 4.26과 native release를 대조해 submit contentHash,
pack 저장 bytes, inspection, 기존 review receipt 재사용, 잘못된 review hash와 PNG bytes
변경 거절을 확인했다(8 checks PASS). 이 대조는 합성 PNG·simulated review 입력을 사용했다.
SAC integer schema가 허용하는 `1.0` dimension 표현은 기존 compact bytes로 보존하고,
NVC 입력의 safe-integer-only canonical profile은 별도로 검사한다.

## 남은 한계

G09의 원본 PSD/provenance 공백은 해결되지 않았다. 합성 PNG·focused tests·metadata를 원본
release evidence로 대체하지 않는다. 실제 imagegen/Scene Provider/PerfectPixel·PSD/render-back·
미술 품질·사람 review·host crash/multiwriter·binary bundle과 다른 host qualification은 별도다.
Animatic의 PNG 검사와 합성 fixture 통과도 실제 그림 생성이나 Human final release 증거가 아니다.
후속 범위는 [PLAN](PLAN.md)·[RELEASE_GATES](RELEASE_GATES.md)를 따른다.

## 과거 실행 근거

[2026-10-02 animatic snapshot](../../docs/research/2026-10-02-animatic-implementation.md)은 SAC/NVC public CLI와
실제 SMC renderer의 합성 입력 경로 및 당시 manifests를 기록한다. 이전 focused 14 tests 통과와
G09/CLEAN_SCAN 실패는 아래 각 source snapshot의 범위다. 현재 문서 변경으로 gate가 통과했다고
주장하지 않으며 원본 logs·hash·실패 기록은 보존한다.

<details>
<summary>과거 source snapshot의 실행·변경 기록</summary>

## 2026-09-30 latest current-source qualification

The latest active-tree run passed the publication-boundary suite (14/14), current skill/docs/release
manifests (125/100/247 entries), and public SMC→SAC→SMC owner-chain with fixture pixels and simulated
reviews. In the active checkout, `tools/verify_release.py` stopped at `CLEAN_SCAN` because retained
Python bytecode caches are present. A cache-free isolated copy then failed closed at
`G09_SOURCE_EVIDENCE_MISSING:tests/fixtures/legacy-psd/PROVENANCE.json`. No PSD or provenance was
recreated; actual imagegen, PSD postflight, and Human visual approval remain NOT_RUN. See the
[latest root report](../../docs/research/2026-09-30-real-use-validation-latest.md).
After this status edit, all three current manifests were regenerated and verified at 125/100/247
skill/docs/release entries; their hashes are owned by the corresponding manifest files.

## 2026-09-23 JSON ingress·projection snapshot 정리 (현재 소스 실행 확인)

모든 library JSON file/bytes ingress를 duplicate-key rejecting loader로 통합했다.
SMC projection importer는 parse/hash 확인에 사용한 receipt·graph bytes 그 자체를 SAC run에
게시하며, destination 경로에는 같은 bytes만 재사용하고 충돌 bytes는 덮지 않는다.
생성 source/document/release manifest는 갱신했다. 모듈 분리 뒤 14개 publication-boundary
tests와 SAC public CLI를 포함한 geultu→NVC→SMC→SAC→SMC E2E를 현재 소스에서 실행했다.
normal release 흐름은 통과했으며, source mutation race 전용 failure injection은 별도 미실행이다.

## 2026-09-23 strict JSON boundary 정리 (검증 범위 분리)

`scene_asset_compilerlib.util`에 bytes/file용 duplicate-key rejecting loader를 추가했다.
workspace RunRequest, 일반 상태·승인 파일, publish approval, SMC capsule을 같은 규칙으로
읽게 했다. provider component-lock 오류 접두사는 보존했다. valid artifact·schema·외부
DTO는 변경하지 않았다. 모듈 분리 뒤 publication-boundary suite와 전체 공개 owner-chain E2E는
통과했다. 중복 key가 있는 JSON을 각 ingress에서 거부하는 별도 adversarial case는 이번
실행에서 직접 구동하지 않아 **NOT_RUN**으로 남긴다.

## 검증

`python3 -m unittest discover -s tests -v`: **14 PASS**. 실제 filesystem에서 approval drift·정상 재승인/멱등 발행·선택 empty/hash·request policy drift·symlink·compile/begin write failure를 검사했다. byte fixtures는 실제 생성/PSD 품질 증거가 아니다. `tools/verify_release.py`: **FAIL** — G09에서 `tests/fixtures/legacy-psd/PROVENANCE.json`이 없다. G09 이전 경로를 통과했더라도 전체 gate PASS로 보고하지 않는다. skill/template 검사는 별도 실행 결과로 확인한다.

## 2026-09-23 latest module layout 실사용 재검증

Scene Provider protocol, RenderContract, task/review contracts, PNG/output manifest,
SceneProductionPlan/lock modules를 현재 layout에서 다시 확인했다. `python3 -m unittest
discover -s tests -v`는 14/14 PASS, `verify_skill.py`와 source manifests도 PASS다. Public
`scene-plan-check`는 canonical template에서 PASS했고 `scene-init`은 격리 `/private/tmp`
workspace에 13 units를 생성했다. Root `integration/scripts/verify-local.sh`와 current-source
geultu→NVC→SMC→SAC→SMC owner-chain E2E도 PASS했다.

E2E의 human labels, PNG/PSD bytes, asset review는 fixture 전용이며 actual Codex imagegen, Scene
Provider v2, PerfectPixel inspection, real PSD postflight와 사람의 visual approval은 **NOT_RUN**이다.
이 결과가 외부 image generation이나 PSD 품질을 인증하지 않는다. 현재 실행 경로·manifest hashes는
[root verification report](../../docs/research/2026-09-23-real-use-validation.md)에 기록했다.

## 2026-09-23 scene-release approval ownership repair — current source verified

The latest isolated current-source run exposed that the CLI's `approve_scene_asset_release` import
had been dropped from the lifecycle façade. A temporary re-export then exposed that approval's file
write had been placed in the read-only validation module without its dependencies. The approval
effect now lives in `asset_release.py`; `asset_release_validation.py` owns only prepared-subject reads
and validation. After the repair, all 14 publication-boundary tests, the 8 SMC scene-asset boundary
tests, the StoryProduction local gate, and the public geultu→NVC→SMC→SAC→SMC E2E passed. The E2E used
fixture pixels and simulated review labels; actual imagegen, Scene Provider, PerfectPixel, PSD
postflight, and human visual approval remain **NOT_RUN**. Exact evidence is in the
[current verification report](../../docs/research/2026-09-23-real-use-validation.md).

</details>
