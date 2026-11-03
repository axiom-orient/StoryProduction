# promptgen-codex

`promptgen-codex`는 valid image compilation을 Codex image generation으로 실행하고, 생성 artifact를 검증해 destination에 게시하는 adapter crate다.

## Does

- bounded `codex --version`과 `codex exec --json` lifecycle
- exact image-skill read만 허용하는 command identity 검증
- focused subject-count 좌표 열거와 전체 visual hard-constraint gate
- 실패 criterion 하나만 교정하는 최대 4개 후보 재검증
- PNG signature, CRC, DEFLATE, dimensions, SHA-256 검증
- overwrite rollback을 포함한 transactional publication

## Does not do

- prompt 의미 해석 또는 invalid request 보정
- 4개 후보를 넘는 무제한 재시도 또는 model downgrade
- HTTP 또는 CLI argument parsing

## Public surface

`CodexRunConfig`, `CodexExecutionReceipt`, `CodexExecutionError`, `execute_image_generation`.

## Verification

```bash
cargo test -p promptgen-codex --locked --offline
```
