# Studio Host 분석

## 판정과 경계

Studio host는 SMC Track B의 로컬 전송 어댑터다. loopback HTTP로 application command와 별도 human approval dispatcher를 제공하지만, 화면 상태나 요청 성공을 approval authority로 승격하지 않는다. 세션은 host 프로세스 메모리에서만 유효하며 durable approval/custody는 application과 Rust custody owner가 각각 소유한다.

| 모듈 | 소유 책임 | 넘기는 경계 |
|---|---|---|
| `src/server.js` | 서버 조립, method/origin/socket admission, API route 결정, execute body 제한·decode, malformed HTTP 분류, application/approval dispatch | application public entrypoint와 dispatcher |
| `src/project-root.js` | macOS launch에 필요한 실제 project directory와 published handoff file admission | `macos-launcher.js`; canonical root만 반환하고 server lifecycle은 소유하지 않음 |
| `src/launch-error.js` | project admission과 macOS launcher가 공유하는 typed error identity | launch owner들이 동일 `code`/error type 사용 |
| `src/review-sessions.js` | 일회성 review bearer 발급, token hash, cookie parsing, TTL·용량·record 정리 | server가 소유하는 process-local `Map`을 인자로 받음 |
| `src/project-artifacts.js` | dashboard 읽기와 정해진 `scene-output.html`의 no-follow 파일 접근, 응답 security headers | 요청 route에서 명시적으로 호출; 임의 프로젝트 파일 공개 없음 |
| `src/raw-request.js` | Node parser가 거절한 raw packet에서 origin/cookie marker와 header 신뢰 여부만 추출 | 이 결과만으로 승인하지 않으며 server가 loopback/session 정책을 계속 적용 |
| `src/request-body.js` | bounded body 수신, decimal `Content-Length` 상한 해석, oversized/error 응답 뒤 request drain·socket close 처리 | 상한은 SMC contract 값으로 server가 주입; HTTP 요청 authority는 갖지 않음 |
| `src/responses.js` | standard/raw-socket JSON serialization, generic/Track B error envelopes와 metadata projection | application metadata를 조회; authority 결정은 하지 않음 |
| `src/cli.js`, `src/macos-launcher.js` | CLI 및 macOS host lifecycle | 동일한 `createStudioServer` entrypoint |

`macos-launcher.js`는 project-root admission을 재수출하지 않고 `project-root.js`에서 직접
가져온다. Launcher는 server lifecycle과 operator identity만 조정한다.

`review-sessions.js`는 token 수명주기와 저장 record의 invariants를 한곳에 둔다. `macos-launcher.js`는 local operator identity를 구성한다. 둘은 digest 계산만 SMC core primitive에 위임하며 token/cookie/operator 의미와 lifecycle은 계속 각각의 owner가 소유한다. server route가 HTTP cookie/status를 만든다. `project-artifacts.js`는 파일 읽기와 filesystem identity 경계를 소유하지만 Track B session이나 project authority는 읽지 않는다. 이 분리는 Studio 전용이며 공용 이미지/프로젝트 코어를 만들지 않는다.

## 변경 상태

`server.js`의 review-session record lifecycle, project artifact access, Node parser error의 raw-header 분류, bounded request-body transport, JSON/raw-socket response serialization을 각각 전용 모듈로 옮겼다. public `createStudioServer` entrypoint, route, JSON envelope, cookie 속성, response headers, 허용 artifact path, no-follow open/close 동작, Track B admission 경로를 보존하도록 source를 재구성했다. Studio session TTL/one-shot consumption과 approval dispatcher는 기존 application authority 경계를 유지한다.

이 변경 뒤 build, test, lint, runtime, manifest verification은 **NOT_RUN**이다. Source manifest freshness verifier는 실행하지 않았다. Node HTTP framing, route admission, JSON command dispatch는 여전히 `server.js`가 함께 조정한다. 후속 분리 때는 malformed HTTP 분류가 Track B 인증 응답에 미치는 영향을 먼저 추적해야 한다.
