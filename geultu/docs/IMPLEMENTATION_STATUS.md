# 글투 — 현재 구현

이 문서는 2026-10-06에 읽은 source·public tool·persistence snapshot과 날짜별 실행 근거를 구분해 기록한다. 최신 code path 설명은 이 snapshot의 source inspection이고, gate 결과는 아래에 적힌 해당 날짜·범위에만 적용된다.

## 현재 연결

| 범위 | 실제 진입점·owner | 현재 의미 |
|---|---|---|
| 집필 | [authoring API](../crates/geultu/src/authoring/api/tools.rs)·mutation/read | 원문·브리프·Story/Synopsis task·후보·별도 채택·history/restore·handoff |
| 최신성 | [freshness.rs](../crates/geultu/src/authoring/freshness.rs) | 한 immutable Project 요청 안에서 guidance/document hash와 blocker 재사용; persistent cache 없음 |
| 저장 | [journal.rs](../crates/geultu/src/store/journal.rs)·[recovery.rs](../crates/geultu/src/store/recovery.rs) | Command event 묶음 stage/replace, 동일 command 응답 전 snapshot/index 복구 |
| 인계 | [handoff.rs](../crates/geultu/src/store/handoff.rs) | Adopted synopsis의 동일 검토 intent 재개·public receipt 마지막 발행; whole-project export는 별도 |
| 공개 도구 | [catalog](../crates/geultu/src/application/tools.rs)·CLI/MCP/Studio | 단일 tool/handler/human-only 등록; MCP의 사람 결정 실행 거절 |
| 독립 기능 | domain/review/voice/refinement/generation/visual | 기존 공개 기능·producer 계약·provider별 effect 유지 |

모듈·caller 세부는 [ANALYSIS](../ANALYSIS.md), 계약은 [SPEC](SPEC.md),
흐름은 [ARCHITECTURE](ARCHITECTURE.md)에 둔다. 현재 persisted schema는 v2이며 v1을 자동 변환하지 않는다.

## 남은 한계

Journal 저장은 전체 prefix에 비례하는 추가 I/O가 있다. 과거 손상·부분 command와
no-op command ID의 snapshot 소실은 자동 복구하지 않는다. Whole-project handoff의 고정 pair,
refinement 두 transaction의 crash gap과 직접 자식 밖 descendant는 별도 범위다.
실제 writer/provider·사용자 원고·사람 승인·다른 host 및 배포 권리는 미검증이다. Studio/browser는 아래 local scratch 범위만 검증했다.
후속 결정·qualification은 [PLAN](PLAN.md), 외부 자료 조건은 [LICENSING](LICENSING.md)을 따른다.

## 2026-10-05 Studio writing and review workspace

Studio의 기본 작업영역은 작품·장면 탐색 rail과 선택한 장면의 원고 편집기다. 작품·장면 생성과 작품 정보 수정은 필요할 때 열고, 글투 참고값과 읽기 흐름 검토·시각화·내보내기는 보조 패널에 둔다. 편집기에는 명시적 저장과 `저장됨`·`저장되지 않음`·`저장 중`·`충돌`·`실패`·`결과 확인 필요` 상태를 함께 둔다. 글투 관찰은 선택 기능이며 원문은 요청 범위에서만 처리한다. 저장되지 않은 변경은 장면 이동, 새 작업·장면, 새로고침과 저장 원고를 사용하는 작업을 안내와 함께 막고 입력을 보존한다.

글투 프로필이 없어도 최신 원고를 기준으로 수정안 준비를 할 수 있다. 프로필 기반 비교는 의존성 안내와 함께 비활성화된다. 후보 준비·비교는 승인과 별개다. 대기 중에는 저장 원고와 후보, 출처, reviewer 및 subject ID/revision/digest를 확인한다. 원고/장면 revision이 바뀐 낡은 후보는 현재 원고와 분리해 표시하고 승인을 잠근다. 거절은 로컬 reviewer attribution을 요구한다. 정확한 `changesetRejected` 영수증을 확인한 뒤 Project와 Scene을 다시 읽으며, 후보는 producer 규칙에 따라 계속 pending일 수 있다.

Save는 요청 당시 project/scene/revision/text를 owner에 기록하고 상태를 다시 읽는다. 요청 후 입력과 실패한 입력은 편집기에 남고, 미확인 결과를 성공으로 표시하거나 요청을 자동 반복하지 않는다. 승인 경로는 호출 전 owner의 `candidate.subject`를 확인하고 호출 뒤 Project와 Scene을 다시 읽어 저장된 결정 상태를 검증한다. 결과가 `outcome_unknown`이면 원래 미확인 요청의 target·operation·summary와 최신 관찰을 별도로 보존한다. 다른 최신 owner 상태를 여는 public query로 새 context에 진입할 수 있고, 원 요청은 자동 재시도하지 않는다. 이 P3 복구 동작은 이번 검증에서 source inspection만 했으며 실제 unknown/readback 경로는 실행하지 않았다.

2026-10-05 frozen UI의 Aside 재검토에서 served `/`, `/style.css`, `/app.js`와 Studio binary의 frozen hashes 일치를 확인했다. 1425×900 scratch에서 프로필 없는 수동 수정안 준비가 가능하고 비교는 안내와 함께 비활성화됨을 확인했다. 합성 장면 revision을 올린 뒤 복원한 대기 후보는 현재 원고와 분리되고 승인은 잠겼으며, 거절은 로컬 reviewer attribution이 있을 때만 제공됐다. 실제 승인·거절은 제출하지 않았다. 앞선 10/05 pre-final scratch에서 확인한 기본 CRUD·저장·읽기 흐름 등은 그 source revision에 한정된 evidence다. 실제 사용자 원고·production human approval·provider 호출은 실행하지 않았다.

이번 frozen UI에서 320–520px viewport (scratch는 1425×900), Save D→E timing, save-unknown/readback failure injection, P3 unknown 이후 UI recovery, 실제 UI 승인 후 readback, VoiceOver/IME는 **NOT_RUN**이다. 이전 pre-final synthetic scratch의 D→E 및 approval response/readback recovery도 그 revision의 근거이며 이번 final source에서 재실측하지 않았다.

## 2026-10-05 official owner gate — final

Frozen UI identity: HTML `c56e675b7ba3c7ba11542100614cea0874a3438f57e66b9786848d385a47f78d`, CSS `0d738b74a31e492e2c507deb9fd72c425b0d1fcc8dbe87b89102a6a15da8aecb`, JS `6413d8a8977b6eb6a979048da4b0a7b572d86d6d5a2e50b58cc4099d3931f078`, `studio-smoke.py` `06e2eca797d6b8bad839105f1ab616abd7bc5ac07eaa963761b0278295b08d9c`, Studio binary `f7c47b15c4b2bd6f6037c117552e79de1d926c5f2856bf585e677f32b376efbb`. Geultu repo root에서 이 identity에 `RUSTUP_TOOLCHAIN=1.97.1 ./scripts/verify.sh` **PASS (exit 0)**: rustfmt, Clippy `-D warnings`, workspace tests (98 passed, 0 failed), release build, canonical 및 3 asset SHA256SUMS, `verify.py` (146 pass, 0 fail), Node syntax, CLI/MCP smoke가 통과했다. Studio smoke는 UI semantic/static guards와 HTTP/public owner fixture에서 IPv4/IPv6 loopback, unauthenticated rejection, profile 없는 manual candidate preparation, stale source approval rejection, exact changeset rejection 뒤 Project/Scene reread 및 `changesetRejected` 상태에서 candidate pending 범위와 더 새 scene 보존을 확인했다. 임시 workspace/process groups를 정리했다. 이것은 browser interaction 결과가 아니며 실제 UI 관찰은 위의 제한된 Aside 재검토 범위다.

## 2026-10-06 official owner gate — final

이 검증의 frozen source set은 31개 파일이며 composite SHA-256은
`3b97369de5d89dc5bc1c496b5993248fa58e73cb9a7483dc8985d151eba0c38a`다. 그 목록과 개별 해시는
검증 당시 임시 기록 `/tmp/geultu-source-set-2026-10-06.sha256`에 있다. 최종 Studio 자산은
HTML `2920e7af5b366b738a0a2a202cd0fdf44901b3011a92f6af5abdcdee5dd8598e`, CSS
`0d738b74a31e492e2c507deb9fd72c425b0d1fcc8dbe87b89102a6a15da8aecb`, JS
`c77dbd10e80596b3934e401df3cc85068072998e11d2297603b0f033db1d61d6`다.

Geultu 루트의 `RUSTUP_TOOLCHAIN=1.97.1 ./scripts/verify.sh`는 **PASS (exit 0)** 했다.
Rust workspace 105개 테스트, rustfmt, offline all-target/all-feature Clippy `-D warnings`,
release build, canonical verification (150 pass, 0 fail), Node syntax, CLI/MCP smoke와 Studio
synthetic HTTP smoke가 통과했다. HTTP smoke는 optional voice profile 없이 후보를 준비하는 흐름과
현재 원고와 맞지 않는 subject의 승인이 거부되는 범위를 확인했다. 네 craft pack의 checksum
gate도 통과했으며 새 `story-baseline` source archive SHA-256은
`05483ac6dda22937ec65ed901dc4807d007016a56d4f8e1ed86dcb721b18fe0f`다.

최종 source의 실제 Aside/browser 조작은 OS 잠금 때문에 **NOT_RUN**이다. 따라서 이 gate와 HTTP
fixture만으로 실제 작가의 상호작용, 사람의 승인·채택, 사용자 원고, 유료 provider 또는 배포
자격을 입증하지 않는다. 이전 UI 재검토는 2026-10-05 pre-final source에 대한 별도 이력이며,
최종 2026-10-06 browser 결과로 해석하지 않는다.

## 2026-10-06 source follow-up — Studio smoke only

Full owner gate 뒤에 알려진 effect-free 수정안 준비 conflict의 복구를 보완했다. 같은 작업과
장면이 선택되어 있고 원고 편집기가 아직 요청 기준의 저장 원문과 같을 때만 일관된 Project·Scene
관찰로 원고를 갱신한다. 수정안 입력은 유지한다. 관찰에 실패하면 원고를 덮지 않고 저장 상태를
`최신 원고 확인 필요`로 표시하며 기존 상단 새로고침을 명시적 최신 원고 확인으로 사용한다.
작업 지시·비교 결과는 비우고 prepare 요청을 반복하지 않는다.

Follow-up `app.js` SHA-256은
`b198a9409d09673f07103bb1d6c7ecabf2bd61028506452275575a93ddc7c406`이며, 이 source로 다시 빌드한
Studio binary SHA-256은 `299e93fe65be365a820adec3b339db815663703f31a6d8c2ce284410f52ee4e6`이다.
`node --check geultu/adapters/geultu-studio/web/app.js`와 `python3 scripts/studio-smoke.py`는
**PASS**했다. Smoke는 HTML/정적 workflow와 HTTP 기반 synthetic flow, voice profile 없는 후보
검토, 오래된 subject의 승인 거부, IPv4/IPv6·인증 및 임시 process 정리를 확인했다. 이 follow-up에
대해 `./scripts/verify.sh` 전체 gate와 실제 browser interaction은 다시 실행하지 않았으므로
이전 105-test/150-check gate 결과를 이 새 source hash의 결과로 적용하지 않는다. 실제 Aside
browser 조작은 계속 **NOT_RUN**이다.

## 과거 실행 근거

2026-10-01 기록은 Geultu 96 Rust tests 통과와 official gate의 rustfmt 실패를 함께 남긴다.
2026-09-30/24 기록의 통과는 해당 당시 graph와 합성 actor에 한정한다. 합성 작품 fidelity 실패와
실제 사용자 승인 미실행을 보존한다. 아래 기록을 현재 소스의 통과 결과로 적용하지 않는다.

<details>
<summary>과거 source snapshot의 실행·변경 기록</summary>

## 2026-10-01 formatting cleanup — source-only

The recorded 2026-10-01 run passed 96 Rust tests, while `bash scripts/verify.sh` stopped at rustfmt
differences in `store/history.rs`, `store/journal.rs`, and `store/recovery.rs`. Those import-order and
line-wrap differences were then manually aligned with rustfmt's reported layout. Tests, build, fmt,
Clippy, CLI/MCP/Studio smoke, and runtime checks after the source edit are **NOT_RUN**. The earlier
verification outcome is recorded in the [root revalidation report](../../docs/research/2026-10-01-live-use-revalidation-current-tree.md).

## 2026-09-30 실행 snapshot

`bash scripts/verify.sh` passed on that source snapshot: 96 Rust tests, fmt/Clippy, release build, 146
canonical checks, CLI smoke, MCP stdio smoke, and local Studio lifecycle smoke. The root public
owner-chain and `storyctl text` QA workspace also passed the Geultu authoring/handoff path. Candidate
reviews were fixture decisions, not real author approval. Full results are in the
[current-source validation report](../../docs/research/2026-09-30-real-use-validation-cancellation-hardening.md).

The synthetic screenplay's independent fidelity review failed to preserve the mother's belief and
the source's stated unknowns. Actual user manuscripts, human adoption, and external provider calls
remain **NOT_RUN**.

## 2026-09-30 recorded source snapshot

The isolated current-source `./scripts/verify.sh` passed 96 Rust tests, fmt/Clippy, release build,
and CLI/MCP/Studio local smoke. The root public authoring path created a 9-file package after 82
`storyctl text` calls; byte/hash integrity passed while independent synthetic story fidelity failed.
All author/review decisions in that fixture were simulated. The user's manuscript and actual human
adoption were not run. The later active-worktree results supersede this source snapshot and are in the
[root verification report](../../docs/research/2026-09-30-active-worktree-rerun.md).

## 2026-09-24 CLI adapter split — current-source verified

The last completed official `./scripts/verify.sh` passed the pre-GenerationRegistry-split CLI graph: 146 tests, fmt/clippy,
release build, CLI/MCP smoke, and isolated authenticated Studio HTTP project/scene flow. The CLI
adapter split's rustfmt import-order issue was corrected before this run. Current `storyctl text`
81-call package and the cross-owner fixture E2E are recorded in the
[root real-use report](../../docs/research/2026-09-24-real-use-validation.md). Fixture package
integrity passed; manual screenplay/storyboard fidelity failed. Real user-story review remains NOT_RUN.

## Authoring mutation owner 분리 — current-source verified

`authoring/mutation/mod.rs::apply`는 command-id fingerprint/idempotency, private next-state 복사,
action routing, successful command identity 기록을 맡는다. Canonical document Start/Edit/Restore,
brief/task lifecycle, candidate submit, human Review/adoption transition은 private
`mutation/{documents,tasks,candidates,review}.rs`로 나눴다. 각 handler는 같은 `State` clone을
바꾸고 최종 journal commit은 기존 `Workspace` 경계에서만 일어난다. public `authoring::apply`,
Action DTO, error branches와 transition bodies는 유지했다. 현재 geultu official gate의 Rust
build/tests/fmt/clippy와 authoring/public CLI flow에서 이 owner split을 검증했다.

## Application composition·policy·support split — current-source verified

`application.rs`는 공개 `Application` 구성·Workspace/registry 설정·tool facade를 남기고,
generation/craft-pack context는 `application/generation.rs`, refinement decision gate와 subject
binding은 `application/approval.rs`, 공통 parse/commit/id helpers는 `application/support.rs`로
분리했다. 이어 review command router를 receipt/candidate/changeset/run adapters로 나눠
`application/dispatch/review.rs`를 tool-name facade로 유지했다. `dispatch` caller와 stable
`Application`/`ToolDefinition` 경로는 유지한다. `generated_id`는 in-tree 외부 caller가 없는 내부 support helper로 좁혔다.
마지막으로 `application/requests.rs`는 flat facade가 되고, workspace/generation/narrative/voice/
review/visual/refinement DTO를 owner-group files로 분리했다. DTO fields, Serde attributes, and
existing `requests::Type` imports remain in the same Application-internal visibility scope.
Current-source `./scripts/verify.sh` passed 146 Rust tests, fmt/clippy, release build, CLI/MCP smoke,
and Studio loopback/authenticated tool flow.


## 입력 archive의 실행 결과

`./scripts/verify.sh` current-source gate PASS: 146 Rust tests, fmt/clippy, release build, CLI/MCP smoke,
and Studio loopback/authenticated project·scene smoke. Producer schema pin과 public story flow도
root integration verification에서 PASS했다. 외부 provider 품질, 실제 사용자 원고의 서사 충실도와
사람의 편집 검토는 **NOT_RUN**이다.

</details>
