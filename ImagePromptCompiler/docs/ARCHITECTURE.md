# ImagePromptCompiler — 아키텍처

## 경계·책임·의존 방향

`typed ImagePromptRequest → promptgen-core validate/interview/render → ImagePromptCompilation → optional effect → verified artifact/receipt`가 기본 흐름이다.

| 단위 | owner | 공개 소비자 |
|---|---|---|
| `promptgen-core` | typed request, catalog/profile constraints, interview, deterministic prompt, LUT domain | 모든 transport/provider |
| `promptgen-cli` | 명령·mode·result/image 경로와 boundary-specific 오류 | 사람·Story public binary adapter |
| `promptgen-mcp` | tool contract·stdio/HTTP transport·compile/interview 결과 | agent; image provider 실행을 임의 노출하지 않음 |
| `promptgen-web` | loopback server, transient interview/generation lease, artifact serving, UI presentation | browser |
| `promptgen-codex` | 선택 Codex generation, event/thread/call artifact binding, fidelity/제한 repair | CLI/Web |
| `promptgen-openai` | 2.6 native GIF generate·단일 base-reference edit, HTTP/PNG/GIF/receipt | CLI |

profile/category 정책은 core에만 둔다. effect adapter는 core를 다시 호출해 전달된 request와 compilation의 prompt가 같은지 확인해야 한다. 이는 중복 정책 구현이 아니라 신뢰 경계 검증이다.

## 상태·I/O·권한

Core의 image/profile validation·interview/inference·prompt section rendering은 정의 owner 모듈로
나뉜다. CLI는 하나의 `OutputOptions`로 result/force를 수용하고 path alias의 provider별
의미는 실제 command owner에 둔다. Codex/OpenAI buffered raw-byte SHA-256은 core artifact
primitive를 사용하며 streaming copy digest는 복사 effect에 남긴다.

순수 compiler는 별도 DB·job·승인 상태를 갖지 않는다. interview answers와 normalized decisions를 구별하고 사용자 답을 backend 추론으로 덮어쓰지 않는다. Web의 connection/generation guard는 임시 수명주기이고 provider receipt는 원문/scene/asset approval이 아니다.

provider binary/home/output/API-key는 Configuration, timeout/fidelity attempt ceiling은 Policy, protocol/profile/version은 Constant다. 파일 publication은 검증된 PNG/GIF와 binding receipt만 내보낸다. timeout이나 파일 존재만으로 성공을 결정하지 않는다. raw credentials/adapter stderr는 사용자용 UI 문구와 구분한다.

## 코어·선택·확장

prompt-only는 provider 없이 사용할 수 있어야 한다. Codex/OpenAI의 실제 transport 제한은 보존하고 reference를 전송하지 않았는데 적용했다고 주장하지 않는다. generated catalog/audit는 원본 generator에서 갱신한다. C6의 UI 이미지 profile은 독립 제품 화면 구현 pipeline이 아니다.

상세 [이미지 계약](IMAGE_PROMPT_SPEC.md), [MCP](MCP.md), [Studio UX](IMAGE_STUDIO_UX_SPEC.md), [시각·접근성](DESIGN.md), [runtime storage](RUNTIME_STORAGE.md)는 각각 독자와 의무가 달라 유지한다.
