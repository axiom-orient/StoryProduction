# 02. Canonical Contracts

Durable cross-boundary contract는 **정확히 5개**다.

```text
Contract authority
= JSON Schema(shape/local constraints)
+ invariant validator(cross-field/cross-document semantics)
```

Schema 단독 PASS를 제품 contract PASS로 표현하지 않는다.

## NarrativePackage — STS owner

새 Narrative copy가 아니라 STS authority artifact의 **hash-bound manifest**다.

`source_snapshot_sha256`는 **STS final release가 실제 소비한 source input snapshot descriptor의 RFC 8785 JCS SHA-256**이다. Snapshot descriptor는 project-relative path와 각 파일의 raw SHA-256만 포함하며 파일 내용 자체를 복제하지 않는다.

## DeliveryRequest — Human/App owner

`intent + coverage + outputs + must_include_refs + constraints + optional asset_inputs`.

`asset_inputs[]`는 Human/App가 이미 가진 USER/DERIVED material의 project-relative path + raw SHA-256 provenance만 전달한다. 새 durable contract가 아니며 DirectorIR가 이를 `AssetRequest`로 승격한다.

- `outputs[].output_id`는 request 내에서 unique.
- `format`이 capability 요구를 결정한다.
- `backend_id`는 optional explicit pin.
- `MP4/WEBM`: `width,height,fps,duration_frames` 필수.
- `PNG/WEBP`: `width,height` 필수.

## DirectorPlan — DirectorIR owner

```text
visual_grammar
asset_requests[]
deliveries[].units[]
```

Projection operation:
`PRESERVE | COMPRESS | EXCERPT | REORDER | ABSTRACT | OMIT`

- 모든 unit은 `narrative_refs[]`를 가진다.
- `OMIT`이 아닌 unit은 renderer가 실행할 viewer-facing `content`를 가진다.
- `TIMED` delivery의 unit은 `duration_frames`를 가지며 합계가 DeliveryRequest와 일치해야 한다.
- `AssetRequest.source`는 `USER/DERIVED` 입력의 path + raw SHA-256 provenance를 고정한다.
- `asset_refs`는 동일 DirectorPlan의 `asset_requests[].asset_id`만 참조할 수 있다.

`director_bundle_sha256`는 **해당 DirectorPlan을 생성한 exact immutable DirectorIR planner bundle artifact의 raw SHA-256**이다. 구현은 하나의 release artifact를 명시적으로 선택해야 하며 여러 파일을 임의 순회해 암묵적으로 계산하지 않는다.

## AssetIndex — Runtime/P2 owner

공유 asset materialization의 canonical index.

- provider provenance는 asset item별 `backend_id + backend_fingerprint`.
- `source_request_id`는 DirectorPlan의 실제 AssetRequest를 가리킨다.
- mixed provider는 합법이다.
- canonical publish는 모든 required materialization 검증 후 atomic하게 수행한다.

## DeliverySet — P4 owner

Render가 일어난 뒤 각 output의 artifact/hash/backend/lineage와 `semantic + visual + technical` verification을 묶는다.

- pre-render semantic BLOCK은 DeliverySet을 만들지 않는다.
- delivery `PASS`는 semantic/technical `PASS`, visual `PASS|NOT_APPLICABLE`일 때만 가능하다.
- 최상위 `PASS`는 모든 required delivery가 `PASS`일 때만 가능하다.
- automatic visual qualification이 불가능한 `NEEDS_REVIEW` delivery는 Human preview decision이 기존 semantic/visual layer를 `PASS` 또는 `FAIL`로 확정한다. 별도 sixth durable review contract를 만들지 않는다.
- `PASS/NOT_APPLICABLE` layer는 findings가 없어야 한다.
- `WARN/FAIL` layer는 evidence finding이 있어야 한다.

## Hash rule

- canonical JSON: RFC 8785 JCS bytes SHA-256.
- file artifact: raw bytes SHA-256.
- hash 대상/bytes가 문서로 정의되지 않은 필드는 추가하지 않는다.
