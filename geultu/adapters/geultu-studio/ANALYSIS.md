# 글투 Studio — 구현 분석

## Verdict·책임 경계

Loopback HTTP parser, bearer/origin boundary와 browser presentation을 소유하는 실행 adapter다. Canonical state와 commit은 Application/Workspace가 소유한다. 화면의 원고·수정안 buffer와 dirty state는 일시적인 presentation state이며 별도 저장소가 아니다. 상위 제품과 별도 제품으로 포장하지 않는다.

## Module ownership

`src/main.rs`는 process bootstrap, writer/Application composition, listener lifetime와 connection timeout을 조정한다. `src/config.rs`는 CLI configuration과 loopback-only bind policy, `src/assets.rs`는 HTML/JS/CSS embedding을 소유한다. `src/http.rs`는 bounded HTTP/1.1 framing과 response serialization, `src/security.rs`는 OS CSPRNG token, Host/Origin/bearer 검사, `src/routes.rs`는 static/API route와 `Application` 호출을 소유한다. 모듈 분리는 binary 내부 구현이며 공개 route set `/`, `/app.js`, `/style.css`, `/healthz`, `/api/tools`, `/api/tools/call`은 유지한다.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| 시작 | GeultuStudio main → bind | loopback 주소·workspace·writer config·OS CSPRNG | listener/token owner | local URL 또는 시작 오류 | `src/main.rs`, `src/config.rs`, `src/security.rs` |
| browser assets | TCP accept → HTTP parser/router | method/path/header limits | Studio transport·browser presentation | asset bytes/HTTP errors | `src/http.rs`, `src/routes.rs`, `src/assets.rs` |
| `/api/tools/call` | browser → authenticated route → `Application::call` | origin/bearer·Content-Length·tool JSON | Studio framing; core command owner | JSON result/typed error | `src/http.rs`, `src/security.rs`, `src/routes.rs` |
| 종료/connection failure | listener/stream lifecycle | read/write timeout·bounded request | transport | connection close; core commit는 별개 | `src/main.rs` |

## 실제 흐름과 browser 상태

`TCP bytes → http.rs framing → security.rs authority checks → routes.rs dispatch → Application → canonical transaction → http.rs JSON response`. Browser는 원고 buffer와 검토 presentation을 들고 있지만 canonical state authority는 갖지 않는다. 수정안은 `candidate.create → candidate.subject → changeset.create`로 대기 상태에 저장되고, 사람 결정만 별도 `changeset.approve` 또는 `changeset.reject`로 기록된다.

저장되지 않은 장면 원고·수정안·작품 정보는 작업/장면 이동, refresh, 새 작업/장면 생성과 저장 원고를 쓰는 작업을 막는다. `voice.compare`는 후보와 현재 글투 profile의 비교라 dirty source 초안만으로 막지 않는다. `beforeunload` guard는 미저장 입력이나 실행 중인 작업을 감지한다.

Save는 요청 시점의 project ID, scene ID, scene revision, text를 캡처해 `scene.set_text`에 revision CAS와 command ID를 보낸다. 응답 또는 오류 뒤 `project.get`과 `scene.get`으로 owner state를 다시 읽어 저장된 요청 내용, 다른 먼저 저장된 내용, 미반영, 결과 불명을 구분한다. 요청 뒤 입력한 글과 실패한 입력은 편집기에 남는다.

Approval 전에는 `candidate.subject`를 다시 읽어 표시된 ID/revision/digest와 같은지 확인한다. `changeset.approve` 호출 뒤에는 Project와 Scene을 다시 읽어 candidate identity/content/source revision, changeset·operation binding, 저장된 decision status와 Project/Scene 일관성을 확인한다. 불확실한 결과는 결정 controls를 잠그고 명시적인 재읽기를 요구하며 승인 command를 자동 반복하지 않는다. 재읽기 결과가 여전히 pending일 때는 current source/scene과 candidate를 확인하고 subject ID/revision/digest까지 일치해야 기존 review로 돌아간다. Approved/rejected 결과 복구는 owner가 저장한 candidate/change binding과 status를 확인한다. 현재 Scene이 이후 바뀌었다면 승인 기록과 최신 저장 원고를 구분해 표시한다.

## Current transport behavior

- `config.rs`는 typed default listener와 CLI help의 값을 맞추며 빈 workspace와 program 없는 writer args를 거절한다. Loopback-only·기존 workspace 기본값·명시적 provider 설정은 유지한다.
- `http.rs`는 header delimiter 뒤 같은 receive buffer에서 body를 읽고 header/body limits, duplicate/ambiguous framing 거절, `Connection: close`와 security headers를 적용한다.
- `routes.rs`는 malformed request/JSON에 400, Host/token/Origin 거절에 403을 반환하고 Application의 typed status/body를 전달한다. `ToolCall`의 unknown fields는 거절한다.
- `main.rs`는 bind 뒤 실제 listener address를 사용해 안내·검증한다. Port 0은 선택된 실제 port를 사용하고 socket timeout 설정 실패는 시작 오류다. 이미 시작한 response의 write 실패에는 diagnostic만 남긴다.
- `security.rs`는 listener와 같은 port의 loopback Host와 그 HTTP Origin을 확인한다. Null/HTTPS Origin, 다른 port Host 및 미등록 ToolCall field의 호환 경로는 지원하지 않는다. Origin이 없는 token-bearing local client는 허용하며 token은 OS CSPRNG로 만든다.

이 framing/auth 설정은 과거의 malformed request가 500으로 분류되거나, unsupported origin·Host·ToolCall fields·무효 launch 값이 받아들여지던 경로를 제거한다. API tool fields와 core commit/review policy는 owner 계약을 따른다. 응답 소실은 이미 commit된 작업의 취소나 not-applied 증거가 아니다.

## 의존성·finding

**GS1.** 중복/모호한 header, 추가 request-line token과 read-ahead body가 framing 해석을 흔들 수 있었다. Parser가 header/body 경계를 분리하지 않고 값을 엄격히 검증하지 않은 점이 원인이었다. 현재 구현은 delimiter 기반 header 크기, 중복 header·비숫자 Content-Length·body 초과·request-line 잔여를 거절한다. Auth/CSPRNG/loopback 경계는 유지한다.

## 실패·취소·재시도·복구와 검증 경계

HTTP server는 범용 proxy가 아니며 keepalive/pipelining 성공을 약속하지 않는다. Listener 요청 처리는 순차이고 long-running command가 다른 요청을 지연할 수 있다. 전체 browser async stale/close race, OS CSPRNG failure injection, production host와 실제 사용자 검토는 별도 미검증 범위다. `beforeunload` listener가 synthetic event에 취소 표시를 했지만, 현재 Aside navigation에서 실제 native confirmation/cancel 동작은 **UNVERIFIED**다.

2026-10-05 공식 Geultu gate의 HTTP/Studio smoke, source/current guard 확인, 두 장면 synthetic scratch 검증은 [제품 Current](../../docs/IMPLEMENTATION_STATUS.md)에 기록되어 있다. 이 증거는 synthetic actor·local fixture 범위다. 독립 binary이므로 PUBLIC_LIBRARY가 아니며 loopback reachability 자체를 사람 승인 qualification으로 해석하지 않는다.

[상위 ANALYSIS](../../ANALYSIS.md) · [제품 Current](../../docs/IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../docs/PLAN.md). PASS는 제품 Current에 표시한 검사·환경에만 적용한다.
