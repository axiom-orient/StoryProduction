# 12. Call Graph & Ports

## External call graph

```text
NDS
 └─ emitNarrativePackage()
       ↓
NVC.accept(package)
 └─ S00
    └─ S01
       └─ S02
          └─ S03
             └─ S04
                └─ S05
                   └─ S06
                      └─ S07
                         └─ S08
                            └─ S09
                               └─ S10
                                  └─ S11
                                     ├─ PASS → S12
                                     └─ FAIL → RepairRouter(root_owner)
```

## Core ports

### `NarrativePackagePort`
- input: `narrative-package.schema.json`
- write owner: NDS
- read owner: S00+
- mutation: forbidden

### `ArtifactStorePort`
- put immutable artifact
- get by artifact_id
- list parents/consumers
- no in-place update

### `ExecutorPort`
- execute(StageContract, input artifact IDs)
- returns RunRecord + output artifacts/failures

### `CapabilityRegistryPort`
- query task/control/limit
- S02 uses feasibility
- S08 uses exact backend assignment

### `EvaluationPort`
- deterministic and VLM evaluations
- must return evidence and confidence

### `RepairRouterPort`
- input: Failure
- output: root stage + invalidation set
- never mutates artifacts itself

## Grep rule

구현이 생기면 CI에서 다음을 강제한다.

1. schema filename literal은 Registry 외 직접 참조 금지.
2. failure code literal은 FailureCatalog 외 직접 정의 금지.
3. stage ID literal은 StageRegistry 외 직접 정의 금지.
4. output kind producer는 하나만 존재.
5. public NVC input 함수는 `NarrativePackage` 하나만 받음.

이 패키지에는 실제 runtime source가 아직 없으므로 “실제 구현 call site grep”은 할 수 없다. 대신 `scripts/grep_contract_usage.sh`가 현재 문서/계약 패키지 전체의 schema/stage/failure reference를 정적 열거하며, 구현 단계에서 동일 검사를 source tree까지 확장해야 한다.
