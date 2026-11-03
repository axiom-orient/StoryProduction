# 10. Research & External Components

## 선택 원칙

`core contract 변경 없이 adapter로 쓸 수 있는가` → 우선 재사용.  
`특정 delivery만 필요한가` → lazy dependency.  
`아직 실험적이나 구조가 단순한가` → qualification fixture 먼저.

## Current decisions

- **agent-browser** — browser CLI/capture/QA 1차 qualification 후보. Apache-2.0. screenshot/PDF/snapshot/video/local-file workflow를 command adapter로 감싸기 쉽다.
- **Playwright** — CI visual regression과 browser capture의 안정적 대안. `toHaveScreenshot` 계열 회귀에 특히 유용.
- **WebMotion** — browser-native deterministic video 실험 후보. DOM/Canvas + WebCodecs MP4, frame-pure model이 목표 구조와 잘 맞지만 maturity/Chromium capability를 먼저 검증.
- **HyperFrames** — optional HTML→video backend; 예시일 뿐 core가 아님.
- **Motion Canvas** — vector explainer/voice-over timed motion specialist.
- **Revideo** — editable/programmatic video product/API가 실제 필요해질 때.
- **Paged.js** — print-quality pagination이 browser print보다 필요할 때.
- **Cytoscape.js** — interactive story/dependency graph가 필요할 때.
- **dotLottie** — shared reusable vector/state-machine asset.
- **PixiJS/Three.js** — 2D/3D capability 요청이 실제 있을 때.
- **OpenTimelineIO** — NLE handoff only.

## 금지

- 위 라이브러리를 한꺼번에 설치.
- 특정 라이브러리 schema를 canonical DirectorPlan으로 승격.
- 도구 부재 시 silent format downgrade.

세부 URL/결정은 `../evidence/RESEARCH_SOURCES.json`이 기계 기록이다.
