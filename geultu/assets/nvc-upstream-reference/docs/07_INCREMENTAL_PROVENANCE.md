# 07. Incremental Build & Provenance

## Content-addressed artifact

권장:

```text
JCS canonical JSON (RFC 8785)
       ↓
SHA-256
       ↓
artifact_id
```

runtime timestamp는 artifact content hash에서 제외하고 RunRecord에 둔다.

## Invalidation

- canonical narrative 변경 → S01 이후 invalidate
- MeaningSpec 변경 → S02 이후 semantic-dependent artifacts invalidate
- DirectorIR 변경 → S03 이후 invalidate, semantic IR reuse
- 한 shot 변경 → 해당 shot assets/render/timeline/eval만 invalidate
- 한 asset 변경 → `consumers[]` shot만 invalidate

## Provenance

`parent_ids`로 내부 lineage를 완전히 보존한다.

C2PA는 외부 배포 시 provenance를 전달하기 위한 선택적 packaging layer다. **semantic correctness 검증을 대체하지 않는다.**

## Event transport

분산 worker로 갈 때 CloudEvents를 transport envelope로 사용할 수 있으나 domain semantics는 NVC schema가 소유한다.
