# StoryProduction

원문을 바탕으로 사용자와 에이전트가 스토리·시놉시스·씬 설계·대본·텍스트 콘티를 함께 만드는 로컬 제작 시스템이다. 에이전트는 후보를 작성하고, 사용자는 실제 내용을 수정·검토·채택한다.

```text
원문·집필 브리프·글투
  → geultu 스토리·시놉시스 후보와 채택
  → NarrativeCourier STS 씬 설계·대본·Human release
  → 씬에 연결된 텍스트 콘티와 별도 검토
  → Markdown/JSON 제작 패키지
```

기본 경로는 이미지·영상 도구 없이 사용한다. 움직이는 콘티를 선택하면 승인된 텍스트 콘티에서 NVC 연출 계획, IPC/promptGen 이미지 후보와 SAC PNG 자산 검토, SMC 프레임 합성을 거쳐 편집 가능한 HTML·자막·VP9 WebM **후보**를 만든다. 이 후보의 최종 release 채택은 별도 미구현 범위다.

## 시작

기본 텍스트 흐름은 POSIX(macOS/Linux)의 native `storyctl`·geultu·NarrativeCourier 바이너리를 사용한다. 실행에 Python·pip·Cargo·Rustc가 필요하지 않다. 빌드는 [로컬 검증](integration/scripts/verify-local.sh)을 따르고, 실행 계약은 [integration SPEC](integration/docs/SPEC.md), owner/effect 경계는 [아키텍처](integration/docs/ARCHITECTURE.md), 최신 검증과 qualification은 [구현 현황](integration/docs/IMPLEMENTATION_STATUS.md)에 둔다. 기본 owner 경로는 각 제품의 `target/release`이며 `--geultu-bin`·`--nvc-bin`으로 바꿀 수 있다. 공통 옵션은 하위 명령 앞에 둔다.

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story create \
  --project my-story --title "내 이야기" --source /absolute/path/original.md --actor author

integration/target/release/storyctl text --workspace /absolute/path/my-story next
```

예제 경로를 실제 원문·작업 공간으로 바꾼다. `next`는 저장된 상태에서 작업 패킷이나 검토할 후보를 반환하며 task를 저장할 수 있다. 모델 선택·호출은 하지 않는다. 외부 작성 프로그램은 `run`으로 한 번씩 명시적으로 실행한다.

## 결과와 책임

| 결과 | 소유자 | 완료 의미 |
|---|---|---|
| 원문·스토리·시놉시스 | geultu | 원문·집필 자료에 결속된 본문과 별도 채택 이력 |
| 씬 설계·대본 | NarrativeCourier STS | 현재 source의 task/gate를 거친 Completed release |
| 텍스트 콘티 | NarrativeCourier | 현재 씬의 의미 참조와 별도 검토 |
| 제작 패키지 | integration | 원고 3개·NVC 산출물 6개와 마지막 `story-package.json` |
| 선택 animatic | NVC → SAC → SMC | 연출·자산 검토에 결속된 HTML/VTT/WebM 후보 |

생성 성공, 후보 저장, 사람의 채택, package의 파일 무결성을 구분한다. `complete`나 문체 점수는 작품 품질을 인증하지 않는다. 원문·상위 자료가 바뀌면 오래된 후보나 승인 결과를 그대로 재사용하지 않는다. 실패·취소·결과 불명은 자동 성공이나 재시도로 바꾸지 않는다.

## 문서

| 목적 | 문서 |
|---|---|
| 원문부터 패키지까지 사용 | [텍스트 제작 운영](docs/TEXT_PRODUCTION.md) |
| 움직이는 콘티 사용 | [Animatic 운영](docs/ANIMATIC_PRODUCTION.md) |
| promptGen 이미지와 SAC 자산 인계 | [인계·인증 운영](docs/PROMPTGEN_ASSET_HANDOFF.md) |
| 기존 cinematic·미디어 경로 사용 | [미디어 운영](docs/OPERATIONS.md) |
| 에이전트 집필·검토 절차 | [작업 지침](docs/AGENT_PLAYBOOK.md) |
| 제품 방향·계약·구조 | [정체성](docs/IDENTITY_AND_EVOLUTION.md) · [SPEC](docs/SPEC.md) · [ARCHITECTURE](docs/ARCHITECTURE.md) |
| 전체 작업 구간의 상세 Target | [정밀 작업 명세](docs/STAGE_EXECUTION_SPEC.md) |
| 구현·변경·실행 근거 | [현재 상태](docs/IMPLEMENTATION_STATUS.md) · [완료 범위와 남은 계획](docs/PLAN.md) |

컴포넌트별 진입점은 [geultu](geultu/README.md), [NarrativeCourier](NarrativeCourier/README.md), [integration](integration/README.md), [ImagePromptCompiler](ImagePromptCompiler/README.md), [SceneAssetCompiler](SceneAssetCompiler/README.md), [SceneMotionCompiler](SceneMotionCompiler/README.md)에 둔다. Producer schema와 consumer 사본의 연결은 [계약 registry](integration/contracts/registry.json)가 소유한다. 연구·후속 설계는 현재 실행 API와 구분하며 LICENSE/NOTICE는 각 소유자의 조건을 따른다.

공개 소스의 라이선스 범위와 제외 자료는 [공개 배포 경계](PUBLIC_DISTRIBUTION.md)를 따른다. LayoutSheet는 독립 저장소의 선택 도구이며 필수 실행 엔진으로 포함하지 않는다.

SEMI 연결은 [macOS provider](integration/macos-provider/README.md)를 따른다. 선택된 단일 workspace의 geultu 조회와 brief/task/candidate/review 변경을 좁게 제공한다. Candidate submit은 채택이 아니며 task preparation은 writer를 실행하지 않는다. 서명·XPC·실제 사용자 승인 qualification은 현재 상태 문서에서 별도로 다룬다.
