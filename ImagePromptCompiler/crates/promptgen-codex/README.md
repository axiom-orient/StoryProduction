# promptgen-codex

`promptgen-codex`는 valid image compilation을 Codex image generation으로 실행하고, 생성 artifact를 검증해 destination에 게시하는 adapter crate다.

## Does

- bounded `codex --version`과 `codex exec --json` lifecycle
- exact image-skill read만 허용하는 command identity 검증
- focused subject-count 좌표 열거와 전체 visual hard-constraint gate
- 실패 criterion 하나만 교정하는 최대 4개 후보 재검증
- PNG signature, CRC, DEFLATE, dimensions, SHA-256 검증
- 같은 `codex-imagegen` 모드의 구독 GIF 생성·단일 base PNG 참조 편집
- GIF LZW, 프레임 수·지연·크기 검증과 원본 바이트 발행
- overwrite rollback을 포함한 transactional publication

## Does not do

- prompt 의미 해석 또는 invalid request 보정
- 4개 후보를 넘는 무제한 재시도 또는 model downgrade
- HTTP 또는 CLI argument parsing

## Public surface

`CodexRunConfig`, `CodexExecutionReceipt`, `CodexExecutionError`,
`execute_image_generation`, `execute_image_gif`.

## 구독 GIF 실행 버전

이 crate는 workspace 버전의 로컬 adapter이며, OpenAI SDK나 Codex 라이브러리를
내장하지 않는다. 유일한 library 의존성은 로컬 `promptgen-core`다. 이미지 실행
클라이언트는 `CodexRunConfig.codex_binary`로 선택한 외부 Codex CLI다.

공식 `0.168.0-alpha.7`에서 native `gif` 인자와
`image_generation_animation` feature를 확인했다. 같은 릴리스의 code-mode host와
exec-server도 함께 설치해야 한다. 기존 앱이나 인증 파일을 덮어쓸 필요는 없다.
`execute_image_gif`는 기존 CODEX_HOME의 `Logged in using ChatGPT`를 확인하고
API 키 환경변수를 제거한 뒤 두 image feature를 명시적으로 켠다.
단일 요청이며 자동 재시도나 PNG 대체는 없다.

GIF는 2.6 typed profile을 요구하지만 구독 tool의 실제 모델 ID는 노출되지 않는다.
receipt의 `requested_model`과 `provider_model=UNKNOWN`을 구분한다. 서버가 GIF를
허용하지 않으면 업그레이드만으로 해제되지 않는다. 확인된 HTTP 400 거부는 별도
실패로 보존하며, 그 밖의 중단·불확실 상태를 성공으로 바꾸지 않는다.

크기는 기본 `exact`다. 구독 tool이 출력 크기를 선택하도록 명시적으로 동의한
요청만 `output.size_policy=provider`를 사용할 수 있다. 원본 크기를 기록하고
자동 크기 변환을 하지 않는다. GIF 후보에는 별도 시각 검토가 필요하다.

ChatGPT 화면에서 받은 GIF는 [Story GIF intake](https://github.com/axiom-orient/StoryProduction/blob/main/docs/GIF_GENERATION.md)를
사용한다. 이 crate가 브라우저를 제어하거나 화면 생성을 API 성공으로 기록하지 않는다.

## Verification

```bash
cargo test -p promptgen-codex --locked --offline
```
