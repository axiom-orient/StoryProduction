# SceneMotionCompiler contracts — 분석

## Verdict·책임 경계

이 폴더는 SMC의 닫힌 schema set과 Track B 계약 판정을 소유한다. 영속 상태, Studio 서버 수명, 사람의 승인 행위는 소유하지 않는다. Producer schema의 원본은 각 producer 경로에 남고, SMC registry는 등록된 consumer schema와 Track B runtime rule만 읽는다.

| 모듈 | owner | 경계 |
|---|---|---|
| `src/closed-schema-set.js`, `src/trackb-registry.js` | schema admission | 명시적으로 분류된 schema만 등록하고 archival·unclassified lookup을 거부한다. |
| `src/trackb-artifact-schemas.js` | upstream schema binding | NVC projection/SceneGraph와 SAC handoff를 해당 owner registry로 검증한다. Producer schema를 복사해 authoritative로 취급하지 않는다. |
| `src/trackb-contract-primitives.js` | Track B-local values and identity | ID/path/date constraints, the Track B input/error wrapper `hashTrackBRawBytes`, canonical hash domains, and contract error/result helpers. Raw digest calculation delegates to SMC `core/canonical.js`; raw-byte equality remains at the caller, and no named projection/artifact equality wrappers remain. It is not a generic shared core. |
| `src/trackb-command-contract.js` | transport contract | CLI command input/result, application envelope, status projection, Studio execute body와 loopback contract를 소유한다. Studio는 승인 입력의 transport 경계만 검증한다. |
| `src/trackb-semantic-manifest-contract.js` | semantic evidence contract | HTML semantic-manifest anchor/payload parser, object hash/occurrence 무결성 validator, authority occurrence bijection, 결정적 meaning-qualification identity를 소유한다. Stable `trackb-contract.js`가 parser와 object validator도 공개한다. Narrative/approval state는 소유하지 않는다. |
| `src/trackb-artifact-contract.js` | stable artifact-contract facade | 기존 validator export path를 보존하고 per-artifact owner modules를 재수출한다. 자체 state나 relation policy를 추가하지 않는다. |
| `src/trackb-artifact-contract/{schema,authority,candidate,release,projection}.js` | artifact validators | schema lookup, NarrativeAuthority/measured evidence, release candidate, human-issued approval/delivery/pointer, projection receipt shape를 각각 소유한다. |
| `src/trackb-artifact-contract/bindings.js` | cross-artifact relation owner | source/authority/projection/SceneGraph/SAC handoff/HTML/provenance/evidence raw-byte and hash bindings를 단독 소유한다. |
| `src/trackb-contract.js` | stable contract facade | Producer/consumer callers use the canonical owner exports. Duplicate `verifyTrackBProjectionReceiptHash`/`verifyTrackBArtifactHash` aliases were removed; state/relation policy is not owned here. |
| `src/trackb-navigation-guard.js` | trusted navigation identity | measured evidence가 결속하는 deterministic guard mechanism/source hash를 제공한다. |

## 공개 surface와 실제 흐름

`packages/contracts/src/index.js` aggregate barrel was removed: the root package is private, no
separate contracts package manifest exists, and all production consumers already import the
Track B facade or defining schema owners directly. `trackb-contract.js` remains the stable Track B
contract facade; this is not a producer-schema or cross-component integration entrypoint.

일반 흐름은 `closed schema registry → producer DTO 검증 → Track B primitive/identity 검사 → semantic-manifest qualification → artifact relation validator → result/errors`다. Artifact contract은 원본 raw bytes와 parsed value의 일치, authority와 projection lineage, SAC handoff, HTML provenance와 measured evidence를 함께 확인한다. Candidate validation은 승인 생성이 아니며, Studio approval DTO의 승인 권한과 durable session은 Studio host가 소유한다.

## 분리 기준과 보존 범위

Transport shape와 semantic-manifest 해석을 persistent artifact/release relation에서 분리했다. HTML anchor/payload parser와 semantic-manifest object 무결성 검사는 contracts의 단일 owner이며 application과 browser-renderer HTML consumers는 `trackb-contract.js`를 통해 parser를 직접 호출한다. Composer는 HTML emission과 SceneGraph precondition만 소유한다. Track B artifact-facing DTO rules are split by their artifact owner, while `bindings.js` remains the one cross-artifact authority that confirms source/approval/hash lineage. `trackb-artifact-contract.js` and `trackb-contract.js` retain their current owner facades. `contracts/schemas/`의 producer schema, `integration/contracts/registry.json`의 pin, human approval 의미, candidate-vs-approval 경계는 바꾸지 않는다.

이번 구조 정리 뒤에는 tests/build/lint/manifest verifier를 실행하지 않는다. SMC `SOURCE_MANIFEST.json` 생성 metadata만 갱신했으며 검증 상태는 **NOT_RUN**이다.
