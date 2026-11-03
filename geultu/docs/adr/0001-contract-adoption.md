# ADR 0001 — core 계약·오프라인 의존성·확장 경계

- 상태: Accepted — 기존 IMPLEMENTATION_PLAN D1–D5와 embedded adoption baseline에서 채택한 결정의 통합. 이번 실행 검증 결과가 아니다.

## 맥락

글투는 canonical journal/도구를 유지하면서 schema/hash/receipt/craft/refinement 계약을 사용한다. 비표준 JSON digest, 온라인 schema resolution, 외부 서사 엔진 권한의 혼입을 피해야 한다.

## 결정

`serde_json_canonicalizer = "=0.3.2"`로 JCS를 처리하고 `jsonschema = "=0.50.1"`, `default-features = false` 및 사전 등록 schema registry를 사용한다. Cargo.lock과 vendor/.cargo의 offline closure를 보존한다. MCP는 기존 동기 stdio implementation과 고정 protocol을 유지하며 async framework 채택은 별도 결정이다.

craft/refinement packs는 ACTIVE_INPUT이며 `assets/nvc-upstream-reference`는 REFERENCE_ONLY다. reference 자료를 runtime execution으로 자동 연결하지 않는다. `Application::call`과 unsafe 금지·명시 실패 규칙을 유지한다.

## 결과·제약

vendor 크기가 줄어들지 않아도 reproducible/offline 입력을 임의 삭제하지 않는다. raw hash/JCS hash와 schema version의 의미를 바꾸려면 계약과 회귀가 함께 필요하다. 제3자 asset의 재배포 권한은 [LICENSING](../LICENSING.md)과 [embedded baseline](../canonical/documentation/17_ADOPTION_BASELINE.md)을 따른다. 이전 외부 벤치마크/Bowtie 통과 주장은 이번 Current 증거로 이월하지 않는다.
