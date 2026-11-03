# 18. 구현 현황 snapshot — Historical (2026-08-27)

> 이 문서는 2026-08-27 source snapshot의 측정과 격차를 보존한다. 현재 source·제품 범위·실행 qualification은 [Geultu Current](../../IMPLEMENTATION_STATUS.md)와 [저장소 루트의 `ANALYSIS.md`](../../../ANALYSIS.md)를 따른다. 아래 수치와 의미는 그 날짜의 기록이다.

기준 시점: **2026-08-27**. 아래 상태는 이 저장소 루트에서 같은 날 실행한 명령과
현재 source·tests·contracts를 기준으로 기록한다. 이 문서는 설계 목표가 아니라
측정 결과를 소유한다. 상태가 바뀌면 해당 명령을 다시
실행하고 이 문서의 날짜·결과·근거를 함께 갱신한다.

## 1. 증거 우선순위

판정 목적에 따라 정본을 분리한다.

1. **동작·필드·허용 값:** 실제 Rust source와 tests, 그리고 저장소 루트 기준
   `docs/canonical/contracts/*.schema.json`.
2. **현재 상태:** 가장 최근 날짜의 이 문서와 [저장소 루트의 `ANALYSIS.md`](../../../ANALYSIS.md)에 기록된 재현 가능한
   명령 결과. 둘이 다르면 설계 문서를 조정하지 말고 명령을 다시 실행한다.
3. **설계·backlog:** `documentation/01–16` 중 target으로 표시된 내용.
4. **역사 참고:** `documentation/21–29`; 26장은 `ARCHIVED / NON-AUTHORITATIVE`이며
   현재 계약 읽기 경로에서 제외한다.

문서 디렉터리에서 schema를 상대 경로로 가리킬 때는 `../contracts/<name>.schema.json`,
저장소 루트 기준 경로를 적을 때는 `docs/canonical/contracts/<name>.schema.json`을
사용한다. canonical root를 생략한 경로 표기는 사용하지 않는다.

## 2. 검증 명령 — 2026-08-27 측정

공식 저장소 gate는 `./scripts/verify.sh`다. 다음 구성 요소가 한 번의 실행에서
모두 성공했다.

| 명령/단계 | 결과 | 관찰 |
|---|---|---|
| `cargo fmt --all -- --check` | `PASS` | formatting 오류 없음 |
| `cargo clippy --workspace --all-targets --all-features --offline --target-dir ./target -- -D warnings` | `PASS` | warning으로 인한 실패 없음 |
| `cargo test --workspace --all-features --offline --target-dir ./target` | `PASS` | 82 tests passed, 0 failed |
| `cargo test --workspace --all-features --offline --target-dir ./target -- --list` | `PASS` | `listed_tests=82` |
| `cargo build --workspace --release --offline --target-dir ./target` | `PASS` | 5 workspace packages build |
| `cd docs/canonical && shasum -a 256 -c manifests/SHA256SUMS --quiet` | `PASS` | canonical package manifest 일치 |
| `assets/*/SHA256SUMS` 3종 대조 | `PASS` | 20 + 17 + 83 = 120 files |
| `python3 docs/canonical/tools/verify.py` | `PASS` | 146 checks passed, 0 failed |
| `node --check adapters/geultu-studio/web/app.js` | `PASS` | JavaScript syntax valid |
| `./scripts/smoke.sh` | `PASS` | CLI release smoke |
| `./scripts/mcp-smoke.py` | `PASS` | stdio MCP smoke |
| `python3 scripts/studio-smoke.py` | `PASS` | loopback, token, UI flow, cleanup |

검증 결과 report는 `docs/canonical/evidence/verification-report.json`에 기록된다.
이 report와 manifest는 위 명령의 산출물이며, 실제 source·tests·schema를 대신하지
않는다.

## 3. 현재 연결된 구현

| 기능 | 상태 | 실제 근거 |
|---|---|---|
| project/scene/fact/timeline/thread 전이 | `REACHABLE` | `crates/geultu/src/domain/command.rs`, `crates/geultu/src/domain/reducer.rs`, `crates/geultu/src/application/dispatch/narrative.rs`, `crates/geultu/tests/contract.rs` |
| story/synopsis authoring candidate lifecycle | `REACHABLE` | `crates/geultu/src/authoring/{mutation,validation,read,conformance}.rs`, `crates/geultu/src/authoring/api.rs` |
| journal/snapshot/lock/recovery/idempotency | `REACHABLE` | `crates/geultu/src/store.rs`, `crates/geultu/src/store/` (journal, file_io, lock, recovery, handoff, history), `crates/geultu/tests/contract.rs` 및 store 관련 tests |
| text-run budget admission/snapshot conformance | `REACHABLE` | `crates/geultu/src/admission/{budget,snapshot}.rs`, `crates/geultu/src/domain/command.rs`, `crates/geultu/src/store/recovery.rs` |
| review/proof/validation receipt/human decision | `REACHABLE` | `crates/geultu/src/review/{diagnostics,proof,text}.rs`, `crates/geultu/src/receipts/{subject,validation,decision,timestamp}.rs`, `crates/geultu/tests/receipts_admission.rs` |
| generation plan/provider/candidate 경계 | `REACHABLE` | `crates/geultu/src/generation.rs`, `crates/geultu/src/generation/registry.rs`, `crates/geultu/src/generation/hash.rs`, `adapters/geultu-writing`, `adapters/geultu-writer-process` |
| craft-pack discovery/verified loading/status | `REACHABLE` | `crates/geultu/src/craftpack/` and `crates/geultu/src/integrity/{parse,verify}.rs`, `crates/geultu/tests/craftpack_refinement.rs` |
| reviewed synopsis handoff | `REACHABLE` | `synopsis.export_reviewed_handoff`, `crates/geultu/src/contracts/reviewed_synopsis.rs`, `crates/geultu/tests/contracts.rs` |
| flat story package export | `REACHABLE` | `story.export`, `crates/geultu/src/story_package.rs`, `docs/canonical/contracts/geultu-story-package.schema.json` |
| visual package admission/state receipt | `REACHABLE` | `visual.admit_package`, `crates/geultu/src/visual_run.rs`, `crates/geultu/tests/visual_run.rs` |
| visual plan derivation | `REACHABLE` | `visual.plan`, `crates/geultu/src/visual.rs` |
| CLI/MCP/Studio adapters | `REACHABLE` | `crates/geultu-cli`, `adapters/geultu-studio`, smoke tests |

### 3.1 Reviewed synopsis handoff

`synopsis.export_reviewed_handoff`는 `project_id`, `actor_id`, `language`와 필수
`expected_source_revision`/`expected_synopsis_sha256`을 받는다. 먼저 `project.export_markdown`의
전체 원문 bytes를 검토하고 동일 revision/hash를 전달한다. Reason만 선택적이다. 이 경로는
adopted authoring synopsis만 발행하는 authoring handoff와 별개다. 이번 source 변경의 실행
검증은 **NOT_RUN**이다. 성공 시
project journal을 변경하지 않고 다음 두 파일을 만든다.

```text
.geultu/projects/<project_id>/handoff/synopsis.md
.geultu/projects/<project_id>/handoff/reviewed-synopsis-handoff.v1.json
```

JSON은 `docs/canonical/contracts/reviewed-synopsis-handoff.schema.json`의
`GeultuReviewedSynopsisHandoff v1`이고, approval은 동일한 source revision과
raw `synopsis.md` UTF-8 SHA-256에 결합된다. NarrativeCourier가 이 파일을 읽어
STS screenplay와 downstream release를 만드는 것은 geultu 외부 단계다.

### 3.2 Flat GeultuStoryPackage

`story.export`는 `project_id`, `language`, `logline`과 선택적인
`global_constraints`를 받아 `story_package_id`와 flat package를 응답한다. package
identity는 `package_id`를 제외한 payload의 JCS SHA-256에 `np_`를 붙여 만든다.
`source.text_master`와 `source.export_spec` 두 `ArtifactRef`가 lineage 시작점을
제공한다. 이 명령은 pure read라 journal·snapshot·canonical project를 바꾸지
않는다.

이 package는 geultu의 projection이다. NarrativeCourier release object의 대체물이
아니며, 외부 NVC 실행기를 호출하지 않는다.

### 3.3 Visual admission

`visual.admit_package`는 필수 inline flat package만 받는다. Bare `package_id`와 unknown
fields는 invalid input으로 거절한다. Package contract와 identity를 검사한 뒤 현재 Geultu
export projection 및 source references와 결속한다. Route와 initial READY run/PASS receipt를
검사하고 project revision CAS 아래 저널에 기록하며, 반환 전 실제 stored run/receipt가 요청과
일치하는지 확인한다. 실행 검증은 **NOT_RUN**이다. Admission receipt는 로컬 preflight 결과이고
별도 upstream approval, NVC execution, 이미지 생성·렌더·composition을 완료하지 않는다.

## 4. 아직 연결되지 않은 범위

| 범위 | 상태 | 의미 |
|---|---|---|
| 전체 T00–T33 stage executor와 artifact graph | `UNWIRED` | 설계 문서와 일부 local command는 있으나 전체 runtime graph는 없음 |
| NVC S00–S12 executor | `UNWIRED` | geultu는 package admission만 제공 |
| 이미지 provider, asset materialization, render/composition | `UNWIRED` | 이 저장소에 실행 surface 없음 |
| receipt와 route input의 cross-object binding | `PARTIAL` | shape/identity는 검사하지만 project-aware 전체 binding은 후속 작업 |
| 외부 process timeout/output cap/cancellation custody | `PARTIAL` | process adapter는 JSON 경계만 제공하며 운영 상한이 부족함 |
| geultu→NarrativeCourier→최종 visual end-to-end | `UNVERIFIED` | 이 저장소 테스트는 local contract/admission 범위 |
| 실제 LLM/provider 품질·장편 성능 benchmark | `UNVERIFIED` | 공식 gate가 외부 품질을 측정하지 않음 |
| 상용 배포 권리와 upstream license 최종 판정 | `UNKNOWN` | 저장소 코드만으로 결정할 수 없음 |

외부 provider를 실제로 호출했다는 과거 기록은 현재 공식 gate의 PASS 근거가
아니다. `./scripts/verify.sh`는 local/offline 검증과 smoke만 수행하므로, 외부
provider 품질은 별도 bounded test로 증명해야 한다.

## 5. Legacy·역사 자료

- 현재 runtime은 flat `GeultuStoryPackage v1`과 reviewed synopsis handoff를 사용한다.
  이전 envelope·이전 경로는 compatibility shim 없이 fail-closed 정책으로 다룬다.
- `docs/canonical/documentation/26_NVC_VISUAL_COMPILER_S00_S12.md`는 역사적 S00–S12
  참고 자료이며 현재 command, schema, 입력·출력 계약으로 사용하지 않는다.
- 구현 여부를 과거 archive의 단계 수나 명칭으로 추정하지 않는다. 최신 source·tests·
  contracts와 위 측정 명령만 현재 상태 근거다.
