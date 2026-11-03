# NarrativeCourier — 현재 구현

2026-10-03 storyboard/animatic의 공개 실행을 NVC-owned Rust `crates/authoring`으로 전환했다.
Native debug/release CLI build, authoring unit 6개와 실제 Completed STS lifecycle 10개 테스트가 통과했다.
동일 합성 project에서 기존 Python 8+4 명령·base revision diff·review bytes·bundle의 six text/hash와
native 결과가 일치했다. 기존 Python으로 만든 [golden bytes](../crates/authoring/tests/fixtures/)를
Rust schema·hash·serialization 회귀 검사로 보존했다. 실제 작품·모델 실행과 Human adoption은 검증 범위 밖이다.
Authoring의 ordered bytes는 private DOM으로 격리하고 serde_json preserve_order Cargo feature는 사용하지 않는다.
최종 `verify-all.sh --require-rust`는 contract/source 60+118, STS audit 14, blueprint 24,
UI checks와 workspace check/test(93 Rust tests)/fmt/Clippy를 통과했다. `--renderers` 추가 gate는
선택 도구 `magick` 부재로 FAIL이며 이 결과를 PASS로 전환하지 않는다. Source manifest는 이번 source를 재생성·검증했다.
정밀 재검토에서 취소 tombstone 이후 후보/승인 export·bundle·animatic 사용을 차단했고, 기존
Human review bytes를 그대로 보존하는 회귀 검사를 추가했다. `verify-all.sh --require-rust`를
격리 Python 3.12/jsonschema 개발 환경에서 다시 실행해 PASS했다. 이 개발 환경은 native
Story 배포 실행의 전제조건이 아니다. `--renderers`는 여전히 `magick unavailable`로 FAIL이다.
후속 R05 검토에서는 `narrative/<project>` leaf 재선택을 제거했다. Story adapter와 Story 파일 쓰기는 선택한 project FD를 공유하고, STS 공개 `--project-root DIR`은 process directory handle과 기존 logical PROJECT를 결속한다. `state.project_name`이 다르면 dispatch를 거절한다. Source/task/gate의 기존 bytes·hash profile은 유지한다. Import 생성은 별도 namespace 경로이며 `--project-root`와의 병용을 거절한다. 실제 native submit 직전 project leaf를 교체하는 회귀에서 원래 inode만 수정되고 대체 디렉터리는 비어 있음을 확인했다. 기존 task token 일치·symlink root 거절·잘못된 logical ID 거절도 확인했다. 기록된 `verify-all.sh --require-rust` 실행은 96 Rust tests와 check/fmt/Clippy 및 기존 Python·schema·문서·UI gate를 통과했다. Story native scenario는 81 CLI calls·19 tasks·10 합성 gates·exact nine-file package와 실제 Chromium/FFmpeg animatic을 interpreter 없는 PATH에서 다시 통과했다. 실제 사용자 작품·모델·Human adoption은 실행하지 않았다.

위 96-test gate와 native scenario는 2026-10-03 결과로 기록되어 있다. 해당 실행의 임시 `/tmp` logs는 현재 작업공간에 남아 있지 않으며, 이번 docs-only 작업에서 gate를 재현하지 않았다. `ANALYSIS.md`의 이후 source-only 항목은 이 기록된 gate 범위에 포함되지 않는다.
Story integration의 제품 계약은 [SPEC](../../integration/docs/SPEC.md), 현재 실행 근거와 qualification은 [구현 현황](../../integration/docs/IMPLEMENTATION_STATUS.md)에 둔다.

## 현재 연결

| 범위 | 실제 owner·근거 | 현재 의미 |
|---|---|---|
| STS | [workflow](../skills/sts/workflow.json)·[engine](../crates/sts/src/engine/) | Domain 4.3.0, exact source·typed coverage·task/Human gate·Completed release |
| 공개 CLI | [binary](../src-tauri/src/bin/narrative-courier-cli.rs) | 명령/옵션 수용, root 해석 뒤 Engine으로 위임; gate decide는 approve/revise/pause |
| Release | [bundle owner](../crates/sts/src/engine/release/bundle.rs) | 현재 state·package·scene-map·source raw/canonical binding; Completed resume의 누락 파생 복구 |
| Delivery 계약 | [contracts facade](../crates/contracts/src/lib.rs) | 다섯 canonical DTO와 목적별 producer artifact의 독립 invariant |
| Delivery 실행 | [Delivery facade](../crates/delivery/src/lib.rs) | Exact PRESERVE planning·SCN 자산·backend routing·output-local reuse·별도 review |
| Runtime | [runtime facade](../crates/runtime/src/lib.rs) | Domain-neutral path/JCS/hash/token/store와 bounded Unix process/capture |
| 텍스트 콘티 | [native storyboard](../crates/authoring/src/storyboard.rs) | Current source/base·선택 씬·candidate/diff·cancel·exact review·full bundle |
| Animatic | [native animatic](../crates/authoring/src/animatic.rs) | 승인 board·frame/layer/pose/caption/asset 요구·별도 plan review와 packet export |
| 어댑터 | crates/app-config·crates/mcp·src-tauri/ui | 명시 root, typed ingress·scoped UI 결과; 새 domain authority 없음 |

Owner-specific source/caller map은 [ANALYSIS](../ANALYSIS.md), product contract와 producer/consumer pin은 [SPEC](SPEC.md), state/effect 경계는 [ARCHITECTURE](ARCHITECTURE.md)에 둔다.

## 남은 한계

실제 모델의 충실도·원고 품질·Human adoption, Tauri/renderer/provider·배포 binary와
다른 host는 별도 qualification이다. Runtime의 8 MiB capture monitoring은 poll 사이 overshoot가
가능하고 kernel disk quota가 아니다. Escaped sessions·PID reuse·pre-exec latency·OS fault 및
대용량·동시성의 실제 관측은 제한된다. 미지원 Delivery 의미를 근사 성공으로 처리하지 않는다.

Animatic은 NVC 연출 packet의 source/coverage 계약이며 PNG 생성·검토와 합성·최종 작품 release를
소유하지 않는다. 실제품/선택 확장의 남은 범위는 [PLAN](PLAN.md)에 둔다.

## 과거 실행 근거

[2026-10-01 snapshot](../../docs/research/2026-10-01-live-use-revalidation-current-tree.md)은 NVC 73 Rust tests와
당시 512-entry manifest의 통과를 기록한다. [2026-10-02 animatic 기록](../../docs/research/2026-10-02-animatic-implementation.md)은
NVC/SAC public CLI와 실제 SMC Chrome/FFmpeg를 통한 합성 candidate 경로를 기록한다.
그 뒤의 소스·문서 상태에 이 PASS를 자동 적용하지 않는다. 실제 사용자 작품·Human 승인은 별개다.
