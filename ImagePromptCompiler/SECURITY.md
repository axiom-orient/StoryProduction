# Security

## Trust model

promptGen은 다음 입력을 신뢰하지 않는다.

- 사용자 JSON과 guided answers
- HTTP headers/body/path
- Codex stdout/stderr JSONL
- `CODEX_HOME/generated_images` 파일
- destination 경로의 기존 상태

## 방어

### Domain/input

- JSON byte/depth 제한
- duplicate key와 unknown field 거부
- enum/range/cross-field invariant
- placeholder, slot token, ambiguous keyword, invalid safety tier 거부
- invalid compilation에서 prompt 미생성

### Web

- loopback bind만 허용
- Host와 optional Origin 검증
- POST JSON content type 요구
- `X-PromptGen-Client: web-v1` 요구
- duplicate header와 transfer-encoding 거부
- bounded body/header/read/write timeout
- 단일 process 내 generation mutex
- artifact filename, symlink, size 검증
- embedded local assets; external CDN 없음
- DOM output은 `textContent` 사용; `innerHTML` 없음

### MCP

- Streamable HTTP는 `rmcp 3.1.0` stateless mode만 사용하며 session store를 설치하지 않음
- protocol `2026-07-28`와 per-request metadata를 강제
- 기본 bind는 loopback; non-loopback은 bearer token과 explicit Host allowlist 없이는 시작하지 않음
- bearer token은 환경 변수 사용을 권장하고, 외부 노출 시 TLS reverse proxy가 필요
- MCP 도구는 pure compiler/catalog/LUT surface만 호출하며 Codex process와 artifact publication을 시작하지 않음

### Codex/artifact

- shell command string이 아닌 argument 배열
- version/exec timeout
- 단일 thread/event completion 요구
- forbidden non-image action 거부
- thread-scoped artifact authority
- stale/symlink/multiple/non-regular file 거부
- PNG signature/chunk/CRC/zlib/DEFLATE/Adler-32/scanline/dimension 검증
- SHA-256 receipt
- same-filesystem temporary publication과 overwrite rollback

### Source

- `unsafe_code = forbid`
- catalog 구조와 renderer의 정적 검증
- lockfile/offline build

## 비보장

- OpenAI/Codex service 가용성·정책·인증
- 모델이 의미·배치·정확 문구를 완벽히 재현함
- OCR/시각 평가
- Codex가 독립 daemon descendant를 남길 때 process-group 전체 종료
- 미검증 macOS/Windows runtime

## 보고

보안 이슈에는 재현 입력, 명령, 실제 관찰 결과, 영향 경로를 포함한다. token, credential, 개인 이미지, `CODEX_HOME` 인증 파일은 첨부하지 않는다.
