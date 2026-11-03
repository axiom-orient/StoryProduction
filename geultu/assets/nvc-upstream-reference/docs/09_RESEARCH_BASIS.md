# 09. Research Basis

2026-08-19 기준. 연구를 그대로 제품 법칙으로 복제하지 않고, 각 근거가 어떤 설계 결정을 지지하는지 제한해서 사용한다.

## Narrative development separation

- **A Decomposed Approach to Well-Crafted Screenwriting with LLMs** — outline→rich narrative prose와 prose→screenplay formatting을 분리. NDS 내부의 단계 분리 및 NDS/NVC 책임 분리에 참고.
  - https://arxiv.org/abs/2510.23163
- **Narrative Keyframing for Generative Creative Writing** — planning constraints와 생성 prose 사이에 명시적 control points. synopsis/outline을 planning artifact로 보는 근거.
  - https://arxiv.org/abs/2608.10337

## Long-form narrative state

- **STAGE: A Full-Screenplay Benchmark for Reasoning over Evolving Narrative Structures** — 장편 screenplay에서 character/event/relation/knowledge state를 provenance-linked backbone으로 추적.
  - https://arxiv.org/abs/2601.08510
- **Character-Grounded Multi-Agent Story Generation / Atlas** — graph 기반 world-state consistency 검사.
  - https://arxiv.org/abs/2607.00918

## Storyboard / multi-shot continuity

- **STAGE: Storyboard-Anchored Generation for Cinematic Multi-shot Narrative** — shot별 start/end anchor pair와 multi-shot memory.
  - https://arxiv.org/abs/2512.12372
- **DreamShot** — context-aware storyboard와 multi-reference role identity conditioning.
  - https://arxiv.org/abs/2604.17195
- **StoryBlender** — global canonical assets와 shot-specific variables 분리, continuity memory graph, editable 3D grounding.
  - https://arxiv.org/abs/2604.03315
- **Continuity-Aware Narratives via Visual Agentic Storyboarding** — character/environment/scene transition consistency를 explicit concern으로 처리.
  - https://arxiv.org/abs/2604.13452


## Semantic trajectory / diagnostic evaluation

- **KathaTrace: Diagnosing Semantic Trajectory Collapse in Generated Visual Narratives** — 장면이 개별적으로 자연스럽고 인물/배경이 일관돼도 장면 사이의 원인·의도·결과 의미가 이미지에서 복구되지 않을 수 있음을 별도 실패로 정의. `transition_obligations`와 recoverability gate의 직접 근거.
  - https://arxiv.org/abs/2607.01312
- **DirectorBench** — long-form video 품질을 단일 aggregate score 대신 script/visual/audio/cross-modal/stability의 checkpoint-level diagnosis로 평가. stage별 hard/soft gate 분리의 근거.
  - https://arxiv.org/abs/2605.30090
- **Stable Cinemetrics** — professional video controls를 Setup, Event, Lighting, Camera의 계층적 taxonomy로 구조화. ShotIR의 cinematic control decomposition 근거.
  - https://arxiv.org/abs/2509.26555

## Compiler architecture

- **MLIR Developer Guide / Pass Management** — valid IR input → transformation pass → valid IR output, pass별 verifier. NVC의 StageContract/IR verifier 구조에 직접 대응.
  - https://mlir.llvm.org/getting_started/DeveloperGuide/
  - https://mlir.llvm.org/docs/PassManagement/

## Machine contracts / provenance

- JSON Schema Draft 2020-12: https://json-schema.org/draft/2020-12
- RFC 8785 JCS: https://datatracker.ietf.org/doc/rfc8785/
- CloudEvents: https://github.com/cloudevents/spec
- C2PA 2.4: https://spec.c2pa.org/specifications/specifications/2.4/index.html

## 적용 경계

서사인지, 영화 언어인지, 생성 모델 품질인지, 소프트웨어 계약인지 평가 대상이 다르다. 하나의 “이론”으로 전부 설명하지 않는다. 각 이론은 자기 계층에서만 사용한다.
