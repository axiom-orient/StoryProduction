# ImagePromptCompiler — 제품 계약

## 필수·선택 기능

typed image request, category/profile validation, 질문형 interview, deterministic prompt-only compilation, catalog/LUT 도구가 공개 core capability다. Codex generation, GPT Image 2.6 native GIF generation과 명시 단일 base-reference OpenAI edit, CLI/MCP/Studio는 각 허용 범위에서 사용한다. prompt-only는 provider 없이 동작해야 한다.

## 입력·출력·오류·전제조건

| 경계 | 입력·전제 | 출력·거절 조건 |
|---|---|---|
| compile/interview | typed image/subject/composition/color/text/reference/output/profile | deterministic prompt 또는 질문/diagnostics-only failure |
| catalog/profile | 여섯 outcome C1/C4/C5/C6/C10/C11와 category-bound profile | incompatible/unknown profile은 prompt 생성 안 함 |
| Codex generate | valid request와 같은 core compilation·실제 전송 가능한 references | verified artifact/receipt 또는 error; mismatch는 effect 전 거절 |
| OpenAI generate | GPT Image 2.6, generate·참조 없음·key·output | JSON `/v1/images/generations`; strict PNG 또는 native GIF+receipt |
| OpenAI edit | edit, 정확히1개 base index1 typed ref·실제 ref file·key·output | multipart `/v1/images/edits`; GPT Image 2 PNG 또는 2.6 PNG/GIF+receipt; mask/multiref는 지원하지 않음 |
| Studio/MCP | 같은 core request/질문 계약·transport security | UI/tool result; provider 불가를 이미지 성공으로 표시 안 함 |

## CLI 결과와 옵션

CLI `image --mode`는 `prompt-only`·`codex-imagegen`·`openai-image-edit`·`openai-image-generate`다.
JSON/text 결과 파일은 `--result FILE` 하나로 지정하고 이전 `--output` alias는 받지 않는다.
`--image-output`과 `serve --output-dir`은 각각 PNG/GIF와 Studio 저장 디렉터리의 별도 설정이다.
알 수 없는 mode·중복 옵션·input/result/image alias를 편의상 fallback하거나 성공으로 변환하지 않는다.

## 유지할 세부 profile

C4 `logo_identity`는 opaque PNG concept이며 SVG/vector/transparent final 납품을 보장하지 않는다. C4 `app_icon`은 generate-only, 한 개 centered typed subject, non-photo medium, no text, opaque1024² PNG master concept이다. purpose/metaphor·2–3 visual planes·32px silhouette 판단을 포함하며 platform package/rounded mask/editable layers를 약속하지 않는다.

C6 `app_web_ui`·`information_design`, C10 `character_pose`·`poomsae_pose`를 유지한다. textless UI 이미지는 구조·상태·관계만 표현하고 읽을 수 있는 가짜 copy/numerals/data/charts를 만들지 않는다. exact text는 별도 명시 경로다. `pose_transfer`는 composite profile이며 typed pose/subject references가 없으면 거절한다. 여섯 outcome은 profile이 추가된다고 자동 증가하지 않는다.

## 관찰 동작·intentional 원본

공개 compiler facade는 [core lib.rs](../crates/promptgen-core/src/lib.rs), CLI 수용은
[image options](../crates/promptgen-cli/src/options/image.rs)·[공통 output options](../crates/promptgen-cli/src/options/mod.rs),
provider effect는 각 Codex/OpenAI crate가 소유한다.

core constraints와 같은 prompt인지 effect 경계에서 검증한다. source/reference/receipt binding·실제 artifact validation 없이 생성 성공을 주장하지 않는다. output은 StoryProduction의 candidate이며 자산 또는 narrative approval이 아니다.

[IMAGE_PROMPT_SPEC](IMAGE_PROMPT_SPEC.md), [POSE_TRANSFER_PROMPT_SPEC](POSE_TRANSFER_PROMPT_SPEC.md), [CINEMATIC_STORYBOARD_PROFILE](CINEMATIC_STORYBOARD_PROFILE.md), [MULTI_IMAGE_CONSISTENCY](MULTI_IMAGE_CONSISTENCY.md), [MCP](MCP.md)를 원본으로 참조한다. 실제 provider endpoint/model 가용성·품질은 Current의 검증 범위와 분리한다.

GIF는 2.6 모델, 명시 frame_count 2–16, frame_duration_ms 20–2000(10ms 단위)을 요구한다. 전체 decoded pixels는 로컬 한도 16,777,216이다. 실제 LZW 프레임·크기·시간을 검증하며 PNG 오반환을 GIF로 전환해 성공 처리하지 않는다. 계정 GIF preview 권한과 실 API generation qualification은 별도다. [CLI 계약](../README.md).
