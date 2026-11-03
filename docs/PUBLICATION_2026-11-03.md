# 2026-11-03 공개 배포와 개발 검증

## 공개 소스

- [StoryProduction](https://github.com/axiom-orient/StoryProduction): 최초 공개 main `dff2b01e3ac6975fbd353a95646252ac9a9aec0d`, 6,143개 파일.
- [LayoutSheet](https://github.com/axiom-orient/LayoutSheet): main `db05024e285e763d1b0162f421dbfa37c686fb58`, 80개 파일.

익명 GitHub API로 public 접근, main SHA와 재귀 파일 트리의 로컬 일치를 확인했다. 두 저장소 모두 push 전에 GitHub Actions를 비활성화했다. StoryProduction 개발 변경은 최초 공개 이후 main에 반영한다.

기존 소스는 Git 이력·remote가 없었다. StoryProduction과 보관 `StoryProduction.zip` 사이의 문서 8개 차이를 보존했으며, Downloads의 LayoutSheet 사본을 독립 작업 루트로 복사했다. 원본 두 폴더의 사전 사본과 파일별 해시를 공개 저장소 밖에 보존했다.

## 공개 경계

`PUBLIC_DISTRIBUTION.md`와 기존 LICENSE·NOTICE를 유지한다. 개인 학습용 외부 screenwriting 자료와 귀속이 불명확한 ScriptForge craft pack은 Git에서 제외하고 로컬에 남겼다. geultu의 reserved corpus/current profile은 현재 입력에 없으며 향후에도 공개 대상에서 제외한다. MIT baseline craft pack과 핵심 실행 코드는 보존했다.

공개 커밋 대상과 ZIP 내부를 포함한 자격 증명 패턴 검사에서 실제 시크릿을 발견하지 않았다. 원래 Git 이력이 없어 과거 커밋을 함께 공개하지 않았다. 전체 제품에 새 blanket license를 임의 부여하지 않았다. 특히 LayoutSheet의 제3자 MIT 고지는 제품 전체의 신규 라이선스 선언이 아니다.

## 구현

- Cinematic storyboard의 인원수·위치를 패널별로 검증한다. receipt는 전체 등장 수와 전체 이미지 좌표를 기록한다. 3×1·2×2·3×2의 정상·누락·중복 배치와 위치 위반을 검사한다.
- Codex의 실제 exact imagegen skill `cat` 읽기를 허용하며, 다른 파일·셸 치환·명령 연결은 계속 거절한다.
- Story CLI가 Codex binary/home을 전달한다. capability와 typed task_mode·reference 계약의 불일치는 외부 호출 전에 거절한다. API edit은 기존 OPENAI_API_KEY 인증을 유지한다.
- 현재 NVC export에 결속된 이미지 결과를 `animatic asset-draft`로 기존 SAC draft에 전달한다. source·coverage·크기·hash·길이·provenance를 확인하고 발행 전에 입력과 NVC packet을 재관찰한다. 사람 승인과 SAC/SMC authority를 대신하지 않는다.
- LayoutSheet는 독립 선택 도구다. 필수 엔진이나 새 framework를 추가하지 않았다. 운영은 [promptGen 인계 문서](PROMPTGEN_ASSET_HANDOFF.md)를 따른다.

## 실행한 검증

| 검증 | 관찰 결과 |
|---|---|
| IPC workspace 전체 Rust tests, strict Clippy, release build | PASS |
| Codex adapter tests | 43 PASS. Provider seam과 decoder 회귀이며 실제 이미지 품질 증거와 구분 |
| Story native local gate | PASS. Owner builds, pins, Clippy와 native text/media 테스트. Fixture pixels·simulated human gates를 사용한 기계 검증 |
| LayoutSheet unit tests / production build | 185 PASS / PASS |
| LayoutSheet npm audit | production dependency 0건. 전체 개발 의존성 포함 시 moderate 1·high 2건. 자동 upgrade하지 않음 |
| IPC 전체 verify.sh | FAIL: authentic historical image-evidence.json 없음. 새 결과로 과거 evidence를 대체하지 않음 |
| SAC 전체 release gate | 원본에서는 Finder metadata로 FAIL. metadata 없는 보존 사본에서는 JSONSCHEMA_REQUIRED_FOR_RELEASE로 FAIL. PSD G09 원본 provenance 공백도 기존 기록에 남아 있음 |

## 실제 단일 장면과 미검증

새 단일 장면 「첫 열차 전」을 실제 `storyctl text create → next → authoring.submit → authoring.candidate`로 저장했다. 이야기 후보 해시는 `87219e0e9f0e85f7d43cae620e419f5246cbda79dd70c6be8aa1aad3a5689ccd`다. 사람의 채택은 기록하지 않았다. 기존 text 운영 계약에 따라 실제 사용자 검토를 기다린다.

독립된 실제 이미지 provider 시도도 수행했다. 모형 픽셀을 성공 결과로 사용하지 않았다.

| 시도 | 관찰·terminal 상태 |
|---|---|
| take-01 | 정상 skill cat 읽기를 기존 정책이 거절. outcome_unknown, 미채택. 관찰된 정책 오류를 수정 |
| take-02 | 검증 요청의 마지막 샷 요구가 서로 충돌하여 추가 수정을 중단. 로컬 취소 요청을 기록하고 external effect는 outcome_unknown으로 보존 |
| take-03 | 실제 PNG에서 인물은 3명으로 측정됐지만 요구한 가로 3열 대신 세로 3행을 생성. 4회 candidate가 geometry gate를 통과하지 못해 CODEX_FIDELITY_FAILED. Story terminal은 outcome_unknown, capability-result.artifacts는 빈 배열 |

잘못 쌓인 패널을 유지하지 않도록 geometry repair 지시에 정확한 행·열 수를 명시하는 회귀도 추가했다. 이 마지막 repair 문구 보완 뒤 추가 유료 생성은 실행하지 않았다.

**실제 작품의 전체 E2E는 미완료다.** 사용자 이야기 채택 이후의 NVC 작업·사람 검토·SAC 시각 승인·SMC 최종 실제 작품 렌더는 NOT_RUN이다. API edit 실호출은 OPENAI_API_KEY가 없어 NOT_RUN이며, LayoutSheet의 선택적 .layoutpack 교환도 실검증하지 않았다. 기존 animatic lane의 결과는 finalApproval:false인 후보이며, 최종 release adoption은 별도 미구현 범위다.

로컬 테스트 성공, 실제 provider 호출, 사람 승인, 공개 소스 배포를 서로 바꾸어 완료 판정하지 않는다.
