# 05. Assets & Renderers

## Shared AssetIndex

동일 canonical `asset_id`를 HTML/storyboard/video/deck이 공유한다. provider/backend provenance는 asset item에 기록한다.

## MediaBackend capability authority

모든 backend는 `DESCRIBE` protocol operation을 지원해야 한다. Descriptor의 `operations[]`에는 실행 capability인 `MATERIALIZE`와 `RENDER`만 기록한다.

- asset-only backend: `operations=[MATERIALIZE]`
- render-only backend: `operations=[RENDER]`
- combined backend: 둘 다

`DESCRIBE`를 별도 CapabilityRegistry로 복제하지 않는다.

## Protocol payload

- `MATERIALIZE`: exact DirectorPlan artifact ref + `asset_request_id` + isolated workspace.
- `RENDER`: exact DirectorPlan ref + AssetIndex ref + `output_request_id` + isolated workspace.

Backend는 canonical project path에 직접 publish하지 않는다.

## Renderer rule

Renderer는 `content`, layout/timing/style instruction, resolved assets만 실행한다. NarrativePackage를 다시 읽거나 missing meaning을 추론하지 않는다.

## Dependency rule

외부 렌더러/브라우저/영상 라이브러리는 실제 delivery가 요구할 때 adapter 안에만 추가한다. Core dependency로 선제 도입하지 않는다.

## Current safe USER/DERIVED path

Human/App may provide optional provenance-bound USER/DERIVED asset inputs. The deterministic planner associates them only with explicit `SCN-*` refs; it does not infer relationships. AssetIndex copies verified bytes into content-addressed shared blobs. Current HTML/PDF adapters may embed resolved PNG/JPEG/WEBP bytes; active/vector content is not auto-executed.
