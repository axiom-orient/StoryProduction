# 08. T33 GeultuStoryPackage Canonical Contract

## 결론

T33의 유일한 Geultu public output은 `contracts/geultu-story-package.schema.json`을 만족하는 **flat GeultuStoryPackage v1 payload**다. 이 파일이 T33의 canonical schema이며, 같은 `geultu.dev/geultu-story-package/v1` 식별자에 다른 TARGET shape을 등록하지 않는다. Geultu provenance envelope를 public payload에 중첩하지 않는다. NarrativeCourier의 `NarrativePackage`는 이 계약의 출력이 아니며 NVC 저장소가 단독 소유한다.

이전 `artifacts/geultu_story_package.v2.schema.json` envelope는 이 canonical flat contract로 대체되어 제거했다. 해당 경로와 envelope shape은 현재 API가 아니다.

## 입력

- committed `TextMaster` ArtifactRef
- committed `GeultuStoryExportSpec` ArtifactRef
- 두 body의 canonical bytes
- project/run revision binding

## 출력

- `schema_id = geultu.dev/geultu-story-package/v1`
- deterministic `package_id`
- source refs
- NVC가 직접 소비하는 narrative fields

## package_id

1. payload에서 `package_id`만 제외한 identity projection을 만든다.
2. RFC 8785 JCS bytes를 계산한다.
3. SHA-256을 계산한다.
4. `np_` + lowercase hex digest를 package_id로 사용한다.

ArtifactRef.digest는 최종 payload 전체(package_id 포함)의 JCS SHA-256이다. 두 digest는 목적이 다르며 같은 값일 필요가 없다.

## legacy 거부

다음은 admission에서 fail-closed로 거부한다.

- `geultu.dev/geultu-story-package-envelope/*`
- 최상위 `body`, `artifact_ref`, `passed`
- schema에 없는 추가 필드
- input refs와 다른 source refs
- 재계산과 다른 package_id
- validator가 아닌 caller가 제출한 pass 주장

## provenance

creator, model, prompt, source snapshot, approval, validation receipts는 payload가 아니라 `ArtifactCommit`에 저장한다. NVC는 제작 provenance를 알아야 할 경우 commit API를 별도 조회한다.
