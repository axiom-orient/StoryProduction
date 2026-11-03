# StoryProduction — 교차 컴포넌트 계획과 owner 경로

각 제품의 데이터·결정·남은 구현은 해당 owner가 정본으로 관리한다. 이 문서는 교차 컴포넌트 범위와 다음 정보를 찾을 위치를 안내한다. owner별 backlog나 제품 계약을 여기서 복제하지 않는다.

## 범위 경계

`integration/`은 public entrypoint와 immutable contract로 owner를 연결한다. geultu/NVC/SAC/SMC/IPC의 domain state와 승인은 해당 제품에 남는다. Story integration gate의 계약·구조·실행 근거는 [SPEC](../integration/docs/SPEC.md), [아키텍처](../integration/docs/ARCHITECTURE.md), [구현 현황](../integration/docs/IMPLEMENTATION_STATUS.md)을 따른다.

현재 text와 media 사용은 [텍스트 운영](TEXT_PRODUCTION.md), [미디어 운영](OPERATIONS.md), [animatic 운영](ANIMATIC_PRODUCTION.md)에 있다. Story의 전체 authoring Target은 [정밀 작업 명세](STAGE_EXECUTION_SPEC.md)에 있다. Target은 현행 CLI의 workflow graph·호출 횟수·승인 절차를 정하지 않는다. 선택 animatic의 render 결과는 candidate이며 최종 작품 release 계약은 아직 없다.

Story 실행은 Python media generation, SMC HTML/JavaScript renderer, Chromium·FFmpeg 또는 owner별 개발 toolchain을 하나의 Rust 이관 범위로 만들지 않는다. 실제 원고·model/provider 품질·사람 채택과 production SEMI/XPC qualification은 [root Current](IMPLEMENTATION_STATUS.md)와 각 owner의 Current/PLAN에서 별도로 추적한다. **NOT_RUN은 미실행 qualification이지 integration의 현재 gate 결과가 아니다.**

## 핵심 작업 목록

| 교차 컴포넌트 질문 | 현재 경계 | 정본과 후속 owner |
|---|---|---|
| 작가 중심 개발 과정 | 5개 사용자 작업 구간과 18개 Target 단계. 이는 구현된 CLI 단계가 아니다. | [정밀 작업 명세](STAGE_EXECUTION_SPEC.md); 각 제품의 계약 변경은 producer와 consumer가 함께 담당 |
| 실제 작품과 품질 검증 | 2026-10-04 integration gate는 PASS. 사용자 원고, 실제 model/provider 품질, 사람의 채택은 별도 **NOT_RUN**. | [integration Current](../integration/docs/IMPLEMENTATION_STATUS.md)와 각 owner Current |
| SEMI 실사용 승인 경계 | native provider 테스트와 production Team trust, XPC, 사용자 동의·실제 변경 승인은 별도 qualification이다. | [provider 운영](../integration/macos-provider/README.md), [provider 분석·검증](../integration/macos-provider/ANALYSIS.md), [integration Current](../integration/docs/IMPLEMENTATION_STATUS.md) |
| animatic release | 편집용 HTML/VTT/WebM은 candidate다. 최종 작품 release adoption은 아직 정의·구현되지 않았다. | [제품 계약](SPEC.md), [animatic 도입 Target](ANIMATIC_DESIGN_SPEC.md), [animatic 운영](ANIMATIC_PRODUCTION.md) |

새 교차 owner 기능은 제품 요구가 정해진 뒤 producer가 계약을 소유하고 필요한 consumer와 registry pin을 함께 갱신한다. Producer schema 원본은 producer에 두고 consumer copy는 [registry](../integration/contracts/registry.json)의 byte equality를 유지한다. 이 계획 페이지는 독립적인 서사·승인 원장이나 새 구현 권한을 만들지 않는다.

## Owner별 실행 계획과 현재 근거

| Owner | 남은 구현·결정과 실행 근거 |
|---|---|
| geultu | [PLAN](../geultu/docs/PLAN.md) · [Current](../geultu/docs/IMPLEMENTATION_STATUS.md) |
| NarrativeCourier | [PLAN](../NarrativeCourier/docs/PLAN.md) · [Current](../NarrativeCourier/docs/IMPLEMENTATION_STATUS.md) |
| SceneAssetCompiler | [PLAN](../SceneAssetCompiler/docs/PLAN.md) · [Current](../SceneAssetCompiler/docs/IMPLEMENTATION_STATUS.md) |
| SceneMotionCompiler | [PLAN](../SceneMotionCompiler/docs/PLAN.md) · [Current](../SceneMotionCompiler/docs/IMPLEMENTATION_STATUS.md) |
| ImagePromptCompiler | [PLAN](../ImagePromptCompiler/docs/PLAN.md) · [Current](../ImagePromptCompiler/docs/IMPLEMENTATION_STATUS.md) |
| integration | [SPEC](../integration/docs/SPEC.md) · [Current](../integration/docs/IMPLEMENTATION_STATUS.md) |
