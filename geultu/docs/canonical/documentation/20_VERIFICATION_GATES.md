# 20. 검증 게이트 (Verification Gates)

모든 완료 주장은 아래 게이트의 실제 실행 결과에 귀속된다. 아래 명령은 저장소
루트에서 실행하며, 모든 경로는 저장소 루트 기준이다. 전체 공식 게이트는
`./scripts/verify.sh`다. 기대 출력을 함께 명시한다.

## G1. 코드 게이트 (오프라인)

```bash
cargo fmt --all -- --check                 # 기대: 출력 없음, exit 0
cargo clippy --workspace --all-targets --all-features --offline --target-dir ./target -- -D warnings
                                             # 기대: warning 없음, exit 0
cargo test --workspace --all-features --offline --target-dir ./target
                                             # 기대: 모든 "test result: ok", 실패 0
cargo build --workspace --release --offline --target-dir ./target
                                             # 기대: release build 완료, exit 0
```

## G2. 패키지 무결성 게이트

```bash
cd docs/canonical && shasum -a 256 -c manifests/SHA256SUMS --quiet
# 기대: 출력 없음, exit 0 (패키지 내 모든 파일이 매니페스트와 일치)
```

## G3. 데이터 자산 게이트

```bash
(cd assets/craft-packs-scriptforge-v4   && shasum -a 256 -c SHA256SUMS --quiet)
(cd assets/refinement-packs-language-v1 && shasum -a 256 -c SHA256SUMS --quiet)
(cd assets/nvc-upstream-reference       && shasum -a 256 -c SHA256SUMS --quiet)
# 각각 기대: exit 0
```

## G4. 통합 검증기 (verify.py)

```bash
python3 docs/canonical/tools/verify.py
# 현재 기준 기대: {"pass": 146, "fail": 0}, exit 0
# 수행 내용:
#   - 필수 파일 존재(00~20 문서·핵심 계약·카탈로그)
#   - 전체 JSON 파싱
#   - 스키마 레지스트리: id 유일성, 파일 존재, 버전=VERSION 일치
#   - T33 계약 성질(flat/closed/two-inputs/score-number) + 픽스처 판정
#   - commit/receipt/state 스키마 핵심 성질
#   - 문서 내 JSON 스키마 삽입 금지(계약 값 복제 금지)
#   - 자산 실측: assets/* 의 SHA256SUMS 재계산 대조 + 19장 집계 해시 재계산 대조
#   - manifests/SHA256SUMS 무결성
# 산출: evidence/verification-report.json (매 실행 갱신)
```

## G5. 계약 준종 게이트 (구현 진행에 따라 확장)

TARGET 스키마를 Rust 런타임이 충족하는지의 확인은 각 implementation slice마다 정의된다.
원칙: 스키마 검증 가능한 형태(픽스처 입출력)로 증명하며, "코드가 그렇다"는 주장만으로
충분하지 않다. 미구현 영역과 남은 qualification은 [Geultu Current](../../IMPLEMENTATION_STATUS.md)와 [PLAN](../../PLAN.md)에서 각각 확인한다. 18장은 2026-08-27 snapshot이다.

## 게이트 운영 규칙

- G1–G4 중 하나라도 실패하면 그 상태를 "완료"라고 쓰지 않는다.
- verify.py가 새 검사를 추가하면 본 문서의 수행 내용 목록을 함께 갱신한다.
- 외부 provider 품질·벤치마크 등 실행 불가 환경의 항목은 `NOT_RUN_EXTERNAL`로 남기고
  fixture PASS로 대변하지 않는다.
