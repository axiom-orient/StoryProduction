# 14. Schema and Versioning Policy

## 정본

`contracts/schema-registry.json`은 schema id → 파일 → current status를 연결한다. 동일 schema id를 여러 파일이 소유할 수 없다.

## 정책

- clean-break target은 legacy reader/writer를 runtime에 포함하지 않는다.
- required/meaning/type 변경은 새 schema id major version이다.
- optional additive field도 `additionalProperties: false` 계약에서는 새 schema version이다.
- schema version과 artifact revision은 다른 개념이다.
- runtime이 생성하는 모든 artifact는 exact schema id를 선언하거나 kind registry가 exact schema를 결정한다.

## canonicalization

JCS는 object key ordering과 number serialization을 포함한 body digest 규칙이다. schema validation 전에 duplicate JSON key를 거부하고 UTF-8을 요구한다.

## 문서 중복 금지

필드 표, enum 전체, 정규식, required 목록을 Markdown에 복사하지 않는다. 문서는 schema 링크와 semantic invariant만 설명한다.
