# SceneMotionCompiler Agent Contract

## Project boundary
- This file applies to the scene-motion-compiler project root.
- Do not scan or modify sibling projects without an explicit request.
- Read this file, README.md, docs/PRODUCT_CONTRACT.md, docs/PLAN.md before changing code.

## Non-negotiable rules
- Human approval 4종(contract/visual/IR/release)은 어떤 경로로도 자동화 금지.
- 두 권위 모델 위반 금지: 이미지 픽셀이 ScreenContract 문구를 덮지 않고,
  NarrativeAuthority가 없는 셀은 시작하지 않는다.
- Silent fallback / fake success / 조용한 성공 위장 금지.
- evaluation-report.schema.json 편집 금지(미연결 Target 보존).
- Playwright/Puppeteer/harness 도입 금지(locked Chromium CDP만 사용).
- CI/GitHub Actions 생성 금지.
- 소스 변경 후 SOURCE_MANIFEST 재생성 + verify:manifest PASS 필수.

## Verification
- 변경 후 반드시 `npm test` 전체 통과 확인.
- provider 경계 외 테스트 double 금지.
- 실패 시 RCA(재현→관찰→가설→검증→최소 수정→재실행) 수행 후 보고.
