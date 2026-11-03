# ADR 0003 — 승인된 cinematic intent와 실행 projection 분리

상태: **채택 — 첨부 재구현 가이드에 따른 작업본 계약 변경**. 기준일·외부 자료 확인일: **2026-09-22**.

## 맥락

원본 `SceneMotionCompiler/packages/adapters/src/narrative-courier-release.js`는 narrative scene마다 shot 하나와 4초를 생성했다. composer에도 시간 fallback과 추가 delay가 있었다. `NarrativeCourier/crates/delivery`의 기존 `DirectorPlan`은 exact semantic delivery 계획으로, 영화 연출의 shot decomposition과 의미가 다르다. 폴더/언어를 제품 또는 stage와 일치시키면 ownership이 다시 섞인다.

사용자가 첨부 가이드 기반 재구현을 요청했다. 가이드의 권장 내부 crate 분리는 예시이지 새 Rust workspace 세 개를 반드시 만들라는 실행 계약이 아니다. 기존 NVC AGENTS의 다섯 delivery DTO 규칙은 **기존 delivery domain**에서 보존한다. 새 cinematic 계약을 그 DTO에 병합하거나 해당 작업규칙을 수정하지 않는다.

## 결정

1. 다섯 제품과 integration 경계를 유지한다. Narrative·cinematic intent는 NVC, projection/motion/editorial mechanism은 SMC, visual candidate/approval은 SAC, optional image prompt/generation은 IPC가 소유한다.
2. `ContinuityBible`, `CinematicDirectorPlan`, `CinematicApproval`의 schema producer는 NVC다. 기존 delivery `DirectorPlan`은 그대로 둔다. 별도 manifest 없는 `NarrativeCourier/tools/cinematic.py`를 명시적인 operator authoring entry로 두며, agent draft는 사람이 검토하기 전 승인물이 아니다.
3. SceneGraph와 NarrativeSceneProjection은 v3로 내부 consumer를 함께 전환한다. 승인 plan의 shot 수·ID·순서·frame duration·camera·semantic occurrence를 정확히 투영한다. scene→implicit shot 및 4초 fallback, v2 projection compatibility wrapper를 유지하지 않는다. NarrativeSceneMap v2·SAC handoff v2·기존 custody version은 의미가 바뀌지 않은 독립 계약이므로 보존한다.
4. projection receipt에 scene-map/bible/plan/approval의 raw-byte capsule을 결합한다. hash만으로 잃어버린 승인 원문을 복원할 수 있다고 가정하지 않는다. 기존 HTML custody의 native 입력 ABI는 늘리지 않는다. media와 원본 전체의 release 보관 정책은 별도의 미결정 사항이다.
5. SAC visual reference 승인과 SMC motion/clip/QC/editorial은 명시적 파일 계약으로 연결한다. model/provider 출력은 candidate, QC는 evidence, release는 사람 권한이다. SMC의 추가 `production.prepare`는 후보 준비만 수행하고 생성 provider·승인 agent·전역 scheduler를 만들지 않는다.
6. 내부 editorial artifact가 source/approval binding을 소유하고 OpenTimelineIO는 선택적 interchange다. 연구 모델의 attention/memory 내부 구현을 core domain에 이식하지 않는다.

## 결과와 한계

새 외부 작업자가 cinematic Python CLI와 jsonschema 의존성을 설치해야 한다. 이는 native NVC CLI/Tauri/MCP에 이미 자동 연결됐다는 뜻이 아니다. 사람이 제공한 shot draft를 검사·봉인하는 현재 구현과, 고품질 연출 draft를 제안하는 미래 capability를 구분한다. 보유 bytes의 integrity와 operator reviewer 문자열은 인증된 인간 identity가 아니다.

외부 SceneGraph v2 consumer의 실제 존재·전환 권한은 UNKNOWN이다. 내부 fixture/consumer는 전환하지만 외부 consumer가 발견되면 버전 배포 결정을 다시 해야 한다. full video release, rich continuity, actual generation, audio mixing, OTIO/NLE qualification, native runtime은 [PLAN](../PLAN.md)이 소유한다. 문서로 완료 처리하지 않는다.

## 외부 연구 — repository evidence와 분리

아래는 **설계 원칙을 판단하기 위한 1차 자료**다. 논문·README의 성능 주장은 이 저장소의 결과가 아니다. 제시된 버전/문서는 확인한 범위이며 각 프로젝트의 최신 stable 버전이라고 일반화하지 않는다.

| 자료·확인 버전 | 채택 원칙 | 채택하지 않은 것·검증 경계 |
|---|---|---|
| [VideoDirectorGPT](https://arxiv.org/html/2309.15091v2), arXiv 2309.15091v2 | generation 전 scene/entity/layout 계획; directing과 generation 분리 | 논문의 모델 성능·공간 제어가 이 compiler에서 재현된다는 주장 |
| [StoryDiffusion](https://arxiv.org/abs/2405.01434), [원본 저장소](https://github.com/HVision-NKU/StoryDiffusion) | 긴 구간에서 동일 subject/reference를 지속적으로 전달 | 모델 내부 consistent attention·motion predictor를 orchestration 코드에 복제 |
| [VideoStudio](https://arxiv.org/abs/2401.01256), arXiv 2401.01256 | text에서 scene/reference와 영상 생성 단계를 나누는 접근 | 실제 video model 연결·생성 품질 PASS |
| [MovieDreamer](https://arxiv.org/abs/2407.16655), [원본 저장소](https://github.com/aim-uofa/MovieDreamer) | long-range identity/shot 간 조건 전달을 별도 관심사로 취급 | diffusion/autoregressive runtime 자체를 제품 책임으로 도입 |
| [FilmAgent](https://arxiv.org/abs/2501.12909), arXiv 2501.12909 | script/directing/평가의 역할을 구분 | 다중 agent 대화·자동 승인자를 필수 architecture로 도입 |
| [VBench](https://arxiv.org/abs/2311.17982), [원본 저장소](https://github.com/Vchitect/VBench); [VBench-2.0](https://arxiv.org/html/2503.21755v2), v2 2025-08-20 | 기술 media QC와 subject/motion/narrative 평가 축을 분리 | benchmark 점수를 사람 approval이나 release로 치환 |
| [Camera Artist](https://arxiv.org/html/2604.09195v1), v1 2026-04-10 | cinematography intent와 camera/shot 언어를 명시적 계획으로 보존 | 논문의 agent implementation·recursive generation을 기본 runtime으로 채택 |
| [DreamShot](https://arxiv.org/html/2604.17195v1), v1 2026-04-19 | storyboard/reference의 역할 구분과 shot 간 continuity 전달 | 모델 설치·실행 없이 해당 품질을 달성했다고 주장 |
| [CineWeaver](https://arxiv.org/html/2607.26529v1), v1 2026-07-29 | shot별 reference routing과 경계, 구간 간 anchor를 분리 | attention/RoPE/VAE 내부 기법을 provider-neutral 계약에 노출 |
| [OpenTimelineIO time ranges](https://opentimelineio.readthedocs.io/en/latest/tutorials/time-ranges.html), 확인 문서 0.19.0.dev1; [원본 JSON 예제](https://raw.githubusercontent.com/AcademySoftwareFoundation/OpenTimelineIO/main/tests/sample_data/clip_example.otio) | rational frame time, track/clip/transition, external media reference | OTIO를 StoryProduction domain SSOT로 채택; JSON 출력만으로 공식 roundtrip PASS 주장 |
| [ffprobe 공식 문서](https://ffmpeg.org/ffprobe.html), 실행 환경 7.1.5-0+deb13u1 | 실제 bytes의 fps/frame count/stream/dimensions 관찰 | 파일명·provider 주장만으로 media contract PASS 처리 |
| [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785) | canonical object key ordering와 raw/canonical hash 역할 구분 | 현재 정수 제한 구현을 모든 ECMAScript number를 지원하는 범용 JCS로 주장 |

선택 이유: shot과 reference를 명시적으로 전달하는 설계는 여러 연구에서 반복되는 공통 요소다. 따라서 provider 변경에도 유지할 수 있는 계약으로 채택한다. 특정 모델의 성공률이나 장편 품질은 실제 결과물이 있어야 평가할 수 있다.
