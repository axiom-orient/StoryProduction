# promptgen-web

`promptgen-web`는 embedded Studio와 loopback-only HTTP API를 제공하는 library crate다. domain 규칙은 core에 위임하고, image execution은 Codex adapter에 위임한다.

## Does

- loopback bind와 HTTP request parsing
- Studio asset, interview, generate, catalog, LUT, artifact routes
- Host/Origin/client header/body-size 검증
- process-local single-generation guard

## Does not do

- 별도 domain validation·renderer·catalog 지식
- database, multi-user history, provider matrix
- command-line parsing

## Public surface

`WebConfig`, `WebError`, `serve`, `serve_listener`. 서버는 CLI의 `promptgen serve`로 시작한다.

## Verification

```bash
cargo test -p promptgen-web --locked --offline
```
