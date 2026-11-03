# SceneMotionCompiler — 아키텍처

## 경계·책임·의존 방향

| 단위 | 소유 | 비소유 |
|---|---|---|
| contracts/core | 닫힌 schema registry, canonical hash·invariant | provider·approval·UI 상태 |
| adapters | Completed NVC/approved plan 검증·v3 projection, SAC handoff admission | scene→shot/camera 발명 |
| composer | graph/verified assets의 deterministic HTML projection | narrative/shot approval |
| production | motion request, clip observations, QC evidence, editorial/OTIO 후보 | actual model runtime·Human release |
| production animatic | approved packet/PNG inputs·exclusive attempt·exact-frame 합성·WebM 관찰·terminal receipt | NVC 연출·SAC 자산 approval·final release |
| browser-renderer | pinned Chromium CDP·renderer evidence | 승인·최종 pointer |
| application | 단일 command registry와 stage dispatch | 별도 cinematic truth |
| store/Rust custody | release protocol client, descriptor I/O·fsync·exclusive publication·CAS | narrative/visual 평가 |
| CLI/MCP/Studio/launcher | framing, operator/human ingress, local host 수명 | 별도 domain state·숨은 approval |

`CLI/MCP/Studio → application → adapters/composer/production/store`다. contracts는 상위 runtime을 import하지 않는다. producer schema snapshot은 원본과 pin으로 결합하며 중복 policy engine으로 만들지 않는다. application은 custody publication을 pathname 쓰기로 우회하지 않는다.

## 실행·상태 모델

```text
NVC Completed + continuity + cinematic plan + human approval
  → verified SceneGraph v3 + projection receipt/capsules
  ├─ SAC scene-release → verified handoff → HTML/provenance
  │    → locked evidence → candidate → Studio approval → custody CAS
  └─ SAC visual reference/keyframe approval → ShotMotionRequest
       → external result admission → clip candidate → QC
       → editorial candidate → optional interchange
```

두 branch의 승인 subject는 같다고 가정하지 않는다. 새 영상 branch의 final release를 붙이려면 정확한 media/timeline/QC/approval closure를 별도로 정의해야 한다.

shot decision은 NVC plan에만 있다. receipt는 staged graph 후 마지막에 publish하며 원본 JSON capsule/hashed source와 관계를 보존한다. media candidate state는 성공/실패/취소/결과불명으로 나누고, QC evidence와 accepted candidate 및 final approval을 구분한다. visual memory는 SAC pack에서 파생한 index다.

## persistence·권한·동시성·복구

Animatic은 owner-local `engine/crates/smc-animatic`의 별도 media candidate 경로다.
Rust producer가 bounded input·worker claim·runtime lock·CDP·cancel/timeout·immutable result를 소유한다.
`animatic.js`는 기존 Node application에서 native binary를 호출하는 transport이며 실행 fallback은 없다.
`animatic-runtime.js`는 binary에 포함되는 deterministic frame/layer/pose drawing asset이며
Rust producer가 HTML과 VTT를 발행하며 개발 검증도 같은 drawing asset과 실제 VTT를 검사한다.
명시 owner-root의 resource는 embedded bytes에 pin한 뒤 사용한다. Chromium CDP를 사용해
seekFrame(n)마다 캡처하고 FFmpeg/ffprobe identity·실제 frame/codec/canvas/fps를 관찰한다.
입력 bytes를 효과 뒤 다시 읽고 candidate receipt를 발행한다. Integration의 live NVC source 관찰은
별도 Story envelope이며 SMC producer receipt를 다시 seal하지 않는다. 부분 파일이나 fsync/cleanup
오류는 성공 근거가 아니고 강제 종료 후 unknown은 자동 재시작하지 않는다.

Animatic의 root·attempt·frames·publication parent는 열린 directory FD를 보존하고
`openat`/`mkdirat`/`linkat` 및 no-follow regular-file 검사로 접근한다. root 이름의 교체가
실행 대상을 바꾸지 않는다. attempt ancestor가 교체되면 다른 디렉터리에 terminal receipt를
쓰지 않고 unknown을 반환한다. PNG frame bytes는 held files에서 제한된 크기로 읽어
FFmpeg `image2pipe` stdin으로 전달하고, encoded stdout은 독점 생성한 held media file에
연결한다. FFprobe도 그 held file을 stdin으로 읽는다. encoder/probe에 가변 project pathname이나
macOS에서 directory 경로로 사용할 수 없는 `/dev/fd/N/leaf` alias를 전달하지 않는다.

Producer hash는 `producer.rs`에서 각각의 계약으로 검증한다. NVC/SMC의 JCS와 SAC의
기존 compact UTF-8·integral-float profile을 통일하지 않는다. SAC dimension의 `1`과 `1.0`은
크기 비교에서 동등하지만 producer의 저장 bytes·contentHash는 바꾸지 않는다.

읽기는 path containment, symlink/special-file 거부, bounded bytes와 raw/canonical hash를 따른다. publisher는 자신의 temp만 정리하고 기존 다른 content를 자동 덮어쓰지 않는다. multi-file transaction이나 hostile filesystem isolation을 단일 rename만으로 보장하지 않는다. media source와 observation은 소비 시 재검증한다.

ffprobe는 shell 없이 local-file 제한으로 실행하고 timeout/output quota를 둔다. process permission·실제 provider transfer·receipt 진실성은 hash만으로 인증되지 않는다. external execution이 불확실하면 reconcile한 뒤 새 실행 여부를 결정한다.

final HTML release의 candidate/approval/pointer는 exact subject/CAS를 따르며, rename 이후 durable 여부를 알 수 없으면 reconcile-required다. browser global/DOM marker는 authority가 아니다. CDP binding/nonce/독립 event layer와 renderer-only evidence를 분리한다. macOS launcher는 프로젝트 선택/Node host 시작·재열기·종료만 소유하며 승인하지 않는다.

## 코어·선택 기능·확장 지점

[command registry](../packages/application/src/command-registry.js)가 transport 공개 범위·권한을 소유한다. production은 optional typed stage이며 Studio Human approval에 자동 연결하지 않는다. OTIO는 interchange이고 내부 StoryEditorialTimeline이 source/approval binding을 소유한다. Track A·ScreenContract·UI 복원은 이 제품에 추가하지 않는다. 외부 consumer 버전, provider, audio, video release 보관 정책의 미결정 사항은 [PLAN](PLAN.md)에 둔다.
