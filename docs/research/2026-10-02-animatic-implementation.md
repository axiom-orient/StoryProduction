# Animatic 구현·문제 해결 기록

2026-10-02. 이전 연구의 첫 실행 경로와 2.5D layer/pose/caption 표현을 구현한다.
테스트 재료를 사용자 작품이나 실제 Human 승인으로 제출하지 않는다.
운영 정본은 [ANIMATIC_PRODUCTION](../ANIMATIC_PRODUCTION.md)이다.

## Root cause

text board v1에는 출처·검토는 있지만 프레임 길이와 레이어가 없다. 기존 cinematic
plan은 shot별 의미 배정 계약이라 text board의 panel/sourceRef와 다르다.
wall-clock CSS 캡처는 정확한 seek/frame 원장이 아니고 기존 clip probe는
MOV/MP4 ingress다. 서로 다른 owner를 합치거나 가짜 projection으로 우회하지 않았다.

## Changes

| Owner | 실제 입력 → 결과 |
|---|---|
| NVC | 현재 STS/승인 board → pure plan validation → immutable candidate → 별도 review → 원문 자막 packet |
| SAC | pinned packet + 실제 PNG → 기존 구조/alpha inspector → immutable pack → 별도 review |
| SMC | packet/pack/reviews → exclusive attempt → exact-frame Canvas → Chrome PNG → FFmpeg/ffprobe → 후보 receipt |
| integration | public owner CLI, 렌더 전후 live source 확인, byte-equal schema registry |

컷 순서·기간, camera/layer transform, opacity, rotation, depth offset, pose swap,
원문 자막을 명시적으로 표현한다. 합성 원본에 PNG·locked font를 포함한다.
seek는 재생을 중단하고 reduced motion은 연속 transform을 고정한다. 자막이
화면에 맞지 않으면 생략 대신 실패한다. audio는 none이며 soundNotes를 보존한다.

candidate/cancelled/failed/outcome_unknown은 closed result schema의 서로 다른
경우다. candidate에만 output hashes·관측 frame count/canvas/fps가 있다.
finalApproval은 false만 허용한다. request와 terminal receipt는 불변 파일이고
중복 실행·late worker·취소는 같은 atomic terminal publication으로 정렬된다.

## 발견한 문제와 해결

| 문제 | 관찰/원인 | 구현한 해결 |
|---|---|---|
| 새 schema 후 기존 ingress 연쇄 실패 | closed loader가 animatic 디렉터리를 미분류로 거절 | 별도 closed schema set 선언, 기존 active set에는 허용 directory만 연결 |
| PromptGen nonzero exit가 다른 오류로 기록됨 | 리팩터링 후 `_ExecutionFailure` import 누락 | defining error owner에서 import; unknown/non-retryable 의미 보존 |
| decoder가 중복 JSON decision을 지움 | JSON.parse가 중복 object key를 덮어씀 | grammar/UTF-8·decoded key 중복 검사 뒤 contract validation |
| 명시 font 설정이 무시됨 | Korean discovery가 configured path보다 앞에 삽입됨 | 설정을 먼저 검증·선택; 잘못된 명시 설정은 fallback 없이 실패 |
| 구조화된 owner refusal이 unknown으로 기록됨 | 빈 stdout의 parsing 오류가 stderr 판정을 건너뜀 | 두 channel을 독립 decode; 실제 not_applied 의미 보존 |
| producer receipt에 integration metadata를 덧붙임 | self-hash와 closed producer schema가 깨짐 | 별도 Story observation에 원래 render를 보존; current/changed/unobserved를 구분 |
| packet에 plan/review schema 복제 | producer schema를 다시 inline해 수정 지점 증가 | producer `$ref`와 closed local Registry; HTTP lookup 없음 |
| 취소 후 late commit·인코딩 지속 | 별도 mutable flag와 commit의 경쟁 | 첫 terminal receipt 승리; frame boundary/encoding polling·abort |
| 강제 종료를 running으로 오인 | PID 재사용 가능 | OS start identity 비교; unknown recover는 재실행하지 않음 |
| material output durability | FFmpeg close만으로 disk persistence 보장 부족 | 영상 fsync, immutable link, terminal directory fsync |
| publication 오류를 이미 존재하는 파일로 성공 취급 | post-link fsync 오류와 terminal race를 혼동할 수 있음 | 실제 immutable conflict만 재관측; 다른 publication/cleanup 실패는 unknown으로 유지 |
| 100ms process gate의 full parallel timeout | Node/Chrome subprocess contention; 단독 경로 정상 | full test file concurrency 1; timeout·assertions 보존 |

producer timing, renderer resource policy, configured executable, 사용자 motion
setting, observed state를 구분했다. frame storage도 bounded이고 실패한 부분 파일을
성공으로 승격하지 않는다. 범용 image engine이나 새 provider fallback은 없다.

## Removed / Integrated / Kept

- Removed: 새 packet의 중복 inline plan/review 정의. 기존 기능 파일 삭제 없음.
- Integrated: NVC source/lock/seal/publication, SAC PNG inspector/immutable files,
  SMC locked CDP, integration bounded transport와 registry.
- Kept: 독립 authority, text board v1, cinematic/visual/production optional APIs,
  MP4/MOV clip ingress, Human release gate, archival evaluation schema.

## 연구 근거와 결정

renderer 선택은 [2026-10-01 조사](2026-10-01-animated-storyboards.md)를 따른다.
기존 locked CDP를 확장했다. Chrome의
[Runtime promise evaluation](https://chromedevtools.github.io/devtools-protocol/tot/Runtime/)과
[Page PNG capture](https://chromedevtools.github.io/devtools-protocol/tot/Page/)를 사용했다.

PNG 입력의 image2 framerate와 VP9 품질을 명시했다.
[FFmpeg 실행 문서](https://ffmpeg.org/ffmpeg.html),
[libvpx encoder 문서](https://ffmpeg.org/ffmpeg-codecs.html),
[image2/Matroska/WebM 문서](https://ffmpeg.org/ffmpeg-formats.html)를 참고했다.
실제 파일은 ffprobe로 관측한다.

schema 중복 제거에는 jsonschema의
[in-memory referencing Registry](https://python-jsonschema.readthedocs.io/en/stable/referencing/)
방식을 쓴다. producer 소유권을 유지하며 URI를 HTTP 주소로 접근하지 않는다.
[Agent UX Guide](</Users/ax/Documents/Agent-UX-Guide/Agent-UX-Guide.md>)의
후보·현재 상태·허용 실행·확인된 결과·복구 구분을 CLI/artifact에 적용했다.

## Verification

최종 execution snapshot은 아래에 갱신한다. 이전 통과를 후속 수정에 재사용하지
않는다. 재료는 gradient PNG/alpha square와 `SIMULATED_TEST_ONLY` review다.
실제 imagegen 작품이나 사람 승인으로 판정하지 않는다.

실행 범위: SMC full npm test, integration suite, schema pins, JS syntax,
Python compile, producer/consumer manifests. full suite의 실제 Chrome/FFmpeg
경로에서 원문 변경·wrong hash·PNG/output 변조·취소·SIGKILL·복구·동일 attempt
재호출·known renderer failure를 관찰한다.

최종 execution snapshot:

| 실행 | 관측 결과 |
|---|---|
| SMC full `npm test` | PASS, 224/224, skip 0. bundled pnpm dlx npm@11로 실제 package test script 실행 |
| integration unittest discover | PASS, 34/34 |
| NVC/SAC public CLI → SMC real Chrome/FFmpeg | PASS, full suite의 animatic 4개. source/hash/변조/취소/강제 중단/복구/known failure 포함 |
| producer/consumer registry | PASS, 30 contracts; 기존 pins 및 새 schema의 byte equality |
| 실제 receipt의 Story observation 3 cases | PASS. producer에 추가 필드를 붙이는 형태는 schema가 거절 |
| SMC schema-backed examples | PASS |
| JS syntax | PASS, 150 files + 최종 변경 module 재검사 |
| 변경 Python compile | PASS |
| NVC source manifest | PASS, 542 files, missing/extra/hash mismatch 없음 |
| SAC skill/docs/release manifests | PASS, 각각 126/101/253 files |
| SMC verify:manifest | PASS, 255 files, mismatch 없음 |
| 이전 저장 manifest와 경로 비교 | NVC/SAC/SMC 모두 removedPaths=[] |

SMC source tree: `sha256:ae1243bdf347ab4c53ac17b0eb227c4adbb07c81508423fc2cc7265efffd21c5`.
Node 24.19.0, jsonschema 4.26.0, Chrome 154.0.8037.93, FFmpeg/ffprobe 9.0.1,
Darwin/arm64에서 실행했다. missing Python dependency는 임시 venv에 설치했고
사용자 global runtime이나 프로젝트 dependency graph를 변경하지 않았다.

[기계 검증 WebM](</var/folders/mv/sr12hssx0yg2dl2w4_3rxs980000gn/T/smc-nvc-release-26W3ce/animatic/renders/runtime/preview.webm>) ·
[합성 원본](</var/folders/mv/sr12hssx0yg2dl2w4_3rxs980000gn/T/smc-nvc-release-26W3ce/animatic/renders/runtime/composition.html>) ·
[receipt](</var/folders/mv/sr12hssx0yg2dl2w4_3rxs980000gn/T/smc-nvc-release-26W3ce/animatic/renders/runtime/result.json>).
24 frames, 12 fps, 640×360, audio none. 테스트 PNG와 simulated review이므로
미술 품질·실제 사용자 승인·실제 작품 완료의 근거가 아니다. 이 임시 fixture는
보존했으며 실제 프로젝트의 산출물 위치를 대신하지 않는다.

## UNKNOWN / 후속 범위

| 항목 | 현재 판단과 다음 작업 |
|---|---|
| 실제 작품의 재미·글투·인물 일관성 | NOT_RUN. 사용자 원고·실제 imagegen 자산으로 후보를 만들고 사람이 재생·검토 |
| 실제 Human approval·최종 작품 release | NOT_RUN. 테스트 문자열은 인증/승인 근거가 아니며 기존 gate를 우회하지 않음 |
| 그림 생성 executor qualification | NOT_RUN. 제공 PNG 검사/합성이며 imagegen/provider 자체는 실행하지 않음 |
| 제품 화면의 layer/timing 직접 편집 | 별도 host 작업. 저장소 AGENTS가 제품 화면 코드를 금지하므로 JSON revision/media API 구현 |
| 음원 믹스·rig·shader·3D·I2V·투명 WebM | Target/선택 확장. opaque WebM·soundNotes 보존을 이들의 완료로 보고하지 않음 |
| 최대 부하·다른 OS·VoiceOver | NOT_RUN. 정책 상한·코드 접근성은 실사용 qualification이 아님 |
| 강제 kill 후 orphan 자동 정리 | UNKNOWN이면 조사 필요. 다른 process를 임의 kill하지 않음; transport는 소유 group 정리 |
| 물리 disk/OS fault와 인코딩 중 취소의 실환경 qualification | NOT_RUN. 실행 중 취소의 terminal 경합은 검사했으며 polling/abort·fsync 오류 분리는 구현; 모든 OS fault를 실행 재현하지 않음 |
| 전체 working-tree diff | root는 Git repository가 아님. pins·current manifests·existing test surfaces 확인; 과거 uncommitted provenance 미확정 |

현재 결과는 실행 가능한 candidate pipeline이다. 후속 계획을 완료라고 보고하지 않는다.
