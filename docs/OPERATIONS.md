# StoryProduction — 로컬 제작 계약 사용

현재 주 제작 작업은 [원문→텍스트 제작 운영](TEXT_PRODUCTION.md)을 따른다.
이 문서는 기존 cinematic/이미지/영상 확장의 공개 계약 사용법이며 텍스트 흐름의
선행 조건이 아니다. 승인된 텍스트 콘티로 움직이는 합성 후보를 만들 때는
[animatic 운영](ANIMATIC_PRODUCTION.md)을 따른다. 두 경로의 plan·승인·release 의미는 다르다.

독자: 실제 프로젝트 파일과 승인 권한을 가진 operator 및 CLI adapter 개발자. 정체성은 [IDENTITY](IDENTITY_AND_EVOLUTION.md), 실제 지원 범위는 [Current](IMPLEMENTATION_STATUS.md), 남은 작업은 [PLAN](PLAN.md)을 따른다. 아래 명령은 repository root에서 실행한다. `PROJECT`는 **작업하려는 실제 프로젝트의 절대 경로를 실행 시 지정**한다. 예제 경로는 프로젝트 내부 상대 경로이며 입력 파일의 실제 위치와 맞춰야 한다.

## 사전 조건과 승인 경계

Node 22+, Python 3/jsonschema는 projection/cinematic/reference 후보 경로에 필요하다. 실제 clip 검사에는 ffprobe가 필요하다. 기존 native NVC/geultu/IPC, locked browser/custody/macOS Studio는 각 제품의 toolchain/권한 조건을 별도로 충족해야 한다. 모델·이미지·영상 provider가 이 패키지에 자동 설치되거나 연결되는 것은 아니다.

Completed NVC release-bundle과 그 bundle이 참조하는 원본 state/package/scene-map 파일을 그대로 유지한다. 잘린 bundle, draft screenplay, 임의 SceneGraph를 release로 대체하지 않는다. 가짜 사람 승인과 synthetic pixels를 실제 제작의 검토 evidence로 사용하지 않는다.

```bash
python3 -m pip install -r NarrativeCourier/tools/requirements-cinematic.txt
python3 NarrativeCourier/tools/cinematic.py --help
node SceneMotionCompiler/packages/cli/src/main.js --help
```

## 1. continuity 추출 → 명시적 shot draft → 사람 승인

```bash
python3 NarrativeCourier/tools/cinematic.py continuity \
  --project-root "$PROJECT" --release-bundle release-bundle.json \
  --output cinematic/continuity.json

python3 NarrativeCourier/tools/cinematic.py plan \
  --project-root "$PROJECT" --release-bundle release-bundle.json \
  --continuity cinematic/continuity.json --draft cinematic/plan-draft.json \
  --output cinematic/plan.json
```

`plan-draft.json`은 [CinematicDirectorPlan schema](../NarrativeCourier/contracts/cinematic-director-plan-v1.schema.json)의 내용 중 `contentHash`를 제외한 명시적 draft다. 실제 narrative와 continuity hash, scene 순서, 각 shot의 duration/camera/semanticRefs/dialogueRefs/requiredEntities/referencePolicy를 작성한다. 누락된 draft를 1 scene/1 shot/4초로 보충하지 않는다. 기존 delivery `DirectorPlan`을 이 입력으로 전달하지 않는다.

사람이 봉인된 plan을 검토한 뒤, 확인한 `contentHash`와 자신의 reviewer 식별자를 명시한다. shell에 hash를 자동 읽어 승인 명령에 바로 전달하는 것을 검토로 간주하지 않는다.

```bash
python3 NarrativeCourier/tools/cinematic.py approve \
  --project-root "$PROJECT" --release-bundle release-bundle.json \
  --continuity cinematic/continuity.json --plan cinematic/plan.json \
  --expected-plan-hash "$REVIEWED_PLAN_HASH" --reviewer "$REVIEWER" \
  --output cinematic/approval.json
```

출력은 immutable이다. 같은 승인 재요청은 같은 결과를 재사용하지만 다른 내용으로 기존 파일을 덮어쓰지 않는다. 수정은 새 파일/revision과 새 검토다. reviewer 값은 operator claim이며 서명/로그인 인증을 제공하지 않는다.

## 2. 승인 bundle을 SceneGraph v3로 투영

```bash
node SceneMotionCompiler/packages/cli/src/main.js project-narrative-bundle \
  --project "$PROJECT" --bundle release-bundle.json \
  --continuity cinematic/continuity.json \
  --director-plan cinematic/plan.json --director-approval cinematic/approval.json
```

결과 receipt는 `projection/narrative-scene-projection.v3.json`이며 실제 staged graph 경로와 raw source capsule을 참조/보존한다. scene 수와 shot 수는 다르다. 이전 v2 projection을 rename해서 사용하지 않는다. 같은 raw narrative라도 plan/approval이 바뀌면 downstream source binding을 다시 만들어야 한다.

기존 HTML 경로는 SAC scene-release prepare/approve/publish → `handoff/scene-motion-asset-handoff.v2.json` → `compile-scene-html` → locked evidence → Studio approval/custody다. [SMC 경계](../SceneMotionCompiler/docs/BOUNDARY_NARRATIVE_COURIER.md) 및 [SAC 운영](../SceneAssetCompiler/docs/OPERATIONS.md)을 따른다. 새 video 후보 경로는 이 HTML release의 자동 연장선이 아니다.

## 3. 실제 reference/keyframe을 SAC에서 검토·봉인

```bash
python3 SceneAssetCompiler/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py \
  visual-reference prepare "$PROJECT" --draft visual/draft.json --out visual/pack.json
python3 SceneAssetCompiler/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py \
  visual-reference approve "$PROJECT" --pack visual/pack.json \
  --reviewer "$REVIEWER" --expected-pack-hash "$REVIEWED_PACK_HASH" --out visual/approval.json
python3 SceneAssetCompiler/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py \
  visual-reference index "$PROJECT" --pack visual/pack.json --approval visual/approval.json
```

Draft에는 실제 `entityReferences`와 `storyboardFrames`를 넣는다. 필드는 [pack schema](../SceneAssetCompiler/docs/contracts/visual-reference-pack-v1.schema.json)의 해당 배열 정의를 따른다. 이미지가 존재해야 하며 실제 path/MIME/hash가 일치해야 한다. 이 CLI는 이미지 생성기가 아니다. `index` 출력은 승인 pack으로부터 재생성 가능한 조회 결과이며 별도 truth database가 아니다. unresolved visual 사실을 NVC bible에 몰래 확정하지 않는다.

## 4. motion → clip → QC → editorial 후보

모든 단계는 동일 command에 서로 다른 **typed request 파일**을 전달한다. exact shape는 [production schema](../SceneMotionCompiler/contracts/schemas/production/production-v1.schema.json)가 정본이다. 허용되지 않은 field는 거절한다.

```bash
node SceneMotionCompiler/packages/cli/src/main.js production-prepare \
  --project "$PROJECT" --request production/request.json --out production/result.json
```

| request.kind | 필수 입력의 의미 | 결과·멈춤 조건 |
|---|---|---|
| `MotionRequestDraft` | `sources`의 projectionReceiptPath/visualReferencePackPath/visualReferenceApprovalPath, shotId, width/height, providerTransferAllowed | source-bound ShotMotionRequest. 외부 생성은 호출하지 않음 |
| `ClipIngestDraft` | requestPath, providerReceiptPath, media path/sha256 또는 실패 상태의 null | 실제 ffprobe 검사와 GeneratedClipCandidate. 영수증 주장과 bytes가 다르면 거절 |
| `ClipReviewDraft` | candidatePath, reviewEvidencePath 또는 null | ShotQCReport. visualContinuity/motion/narrative 미검토는 NOT_RUN/BLOCKED |
| `EditorialDraft` | projectionReceiptPath, timelineId, 정확한 shot 순서의 clips(candidatePath/qcPath/sourceStartFrame), audioPlan, captions | 현재 source/QC/trim/transition에 bound된 CANDIDATE timeline |
| `OTIOExportDraft` | timelinePath | 제한된 OTIO JSON. audio/미지원 transition은 명시 오류 |

provider receipt는 실제 외부 실행의 requestHash/provider/job/transfer/outcome/media hash를 기록해야 한다. 이 파일을 직접 작성했다는 사실만으로 provider 실행 authenticity가 증명되지는 않는다. 생성 provider 연결·권한 확인은 caller의 책임이다. `providerTransferAllowed=false`인데 전송했다고 기록한 결과는 채택하지 않는다.

실제 사람이 clip을 보거나 별도 evaluator가 관찰한 결과만 ShotReviewEvidence에 기록한다. model review는 사람 approval이 아니다. QC PASS 뒤에도 timeline은 candidate이며 final approval을 생성하지 않는다. FAILED는 수정 대상이 될 수 있지만 CANCELLED는 새 실행 권한을, OUTCOME_UNKNOWN은 이전 job 결과 조정을 먼저 요구한다. repairs는 권고 분류이지 자동 재생성 실행기가 아니다.

## 5. 보관·재개·외부 편집 주의

원본 JSON capsule은 raw 승인 근거를 남기지만 모든 media bytes를 자동 보관하는 release store가 아니다. 입력/미디어를 삭제하면 downstream 재검증이 실패할 수 있다. 같은 content 재사용과 새로운 source revision을 구분한다. partial publication을 성공으로 숨기거나 output 디렉터리를 통째로 자동 삭제하지 않는다.

OTIO JSON은 현재 머신의 검증된 `file://` URI를 사용한다. 외부 머신으로 복사할 때는 media relink/portable package 정책이 필요하다. 공식 OTIO roundtrip·실제 NLE·audio sync/mix·자막 burn-in·최종 MP4 export는 구현/검증 완료로 간주하지 않는다. full transport cancellation과 hostile filesystem isolation 역시 별도 qualification 대상이다.

## 로컬 검사

```bash
integration/target/release/storyctl verify-contracts
integration/scripts/verify-local.sh
python3 NarrativeCourier/tools/verify_contract.py
python3 NarrativeCourier/tools/verify_delivery_source.py
(cd SceneMotionCompiler && node --test tests/cinematic-production.test.js tests/visual-reference-boundary.test.js tests/production-editorial.test.js)
STORY_REQUIRE_OWNER_TESTS=1 cargo test --manifest-path integration/Cargo.toml --test native_text
```

integration unittest의 import 경로가 없는 환경에서는 `PYTHONPATH=integration/src`를 설정한다. 마지막 native E2E는 Rust/native binary 조건을 요구한다. fixture provider·synthetic clip의 검사는 wiring/integrity 증거이며 작품 품질이나 사람 승인 완료의 증거가 아니다. 전체 검사 실패·NOT_RUN은 [Current](IMPLEMENTATION_STATUS.md)에 구분한다.
