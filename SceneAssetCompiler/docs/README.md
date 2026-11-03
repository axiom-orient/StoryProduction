# SceneAssetCompiler 문서

[정체성](IDENTITY_AND_EVOLUTION.md) · [SPEC](SPEC.md) · [ARCHITECTURE](ARCHITECTURE.md) · [Current](IMPLEMENTATION_STATUS.md) · [PLAN](PLAN.md) · [ANALYSIS](../ANALYSIS.md)

[CHANGELOG](../CHANGELOG.md): versioned product history and unreleased contract changes.

[OPERATIONS](OPERATIONS.md): 사용자/운영자의 실행·실패·복구 절차. [RELEASE_GATES](RELEASE_GATES.md): source/외부 tool/실PSD/bundle 검증 의무. [RESEARCH](RESEARCH.md): 기존 설계 근거와 대안, 현재 provider 성공 증거는 아님.

일반 asset workflow의 직접 entrypoint는 [canonical Python CLI](../.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py)다.
선택 animatic의 packet·PNG pack·review·렌더 호출은 [상위 운영 문서](../../docs/ANIMATIC_PRODUCTION.md),
직접 실행기는 [Rust sac-animatic](../native/src/main.rs)을 따른다.

실행 원본은 [active skill](../.agents/skills/scene-asset-compiler/SKILL.md)과 같은 경로의 scripts/schemas/templates다. 문서 트리에는 skill 사본을 두지 않고 이 정본으로 연결한다. [contracts](contracts/)는 consumer 경계이며 별도 runtime owner가 아니다. 포함되지 않은 과거 field-test/Standalone archive 링크를 현재 증거로 사용하지 않는다.
