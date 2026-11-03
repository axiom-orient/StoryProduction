# 글투 — 남은 qualification과 Target

이 문서는 남은 durability·host·실사용 qualification을 추적한다. 공개 API와 동작은 [SPEC](SPEC.md), owner와 실행 흐름은 [ARCHITECTURE](ARCHITECTURE.md), 실제 source snapshot과 날짜가 붙은 gate 결과는 [Current](IMPLEMENTATION_STATUS.md)가 소유한다. 계획 항목이나 과거 gate 결과만으로 완료를 판정하지 않는다.

| 남은 범위 | 현재 소스·계약 | 필요한 후속 작업 |
|---|---|---|
| durability failure qualification | command event 묶음 발행·create/transaction projection 복구·authoring freshness를 구현; 2026-10-05 공식 owner gate 결과는 [Current](IMPLEMENTATION_STATUS.md)에 기록 | fsync/rename/snapshot/index failure injection, 동시 writer, 중단 뒤 복구·재개 관측 |
| 과거 손상/no-op 복구 | 손상 journal·부분 command·snapshot에서 잃은 no-op command ID를 자동 추정하지 않음 | 실제 데이터의 명시적 복구 정책 결정. 기존 데이터 자동 변환 금지 |
| whole-project handoff | 고정 두 파일을 별도 교체. 주 authoring handoff는 intent/body/public receipt로 재개 | 고정 pair transaction·역사 보존 정책 결정 및 각 경로의 crash qualification |
| refinement | 두 transaction과 pending artifact 판정을 사용 | 중단 시 두 단계 사이의 effect·복구 관측 |
| 외부 writer·호스트 | 직접 자식 cleanup·bounded JSON protocol | 실제 provider·descendant·중단·대용량 영향과 비POSIX 정책 검토 |
| 실제 사용자·배포 | 2026-10-05 local CLI/MCP/Studio gate와 synthetic Studio scratch는 [Current](IMPLEMENTATION_STATUS.md)에 기록; actor는 attribution이고 voice는 제한된 관찰 | 실제 원고 품질·사람의 검토와 채택·실제 browser navigation 취소·외부 provider 및 production host·배포와 라이선스 qualification |

각 행은 아직 필요한 관측·결정 범위다. 실행 결과가 추가되면 날짜와 근거를 [Current](IMPLEMENTATION_STATUS.md)에 기록하고, 공개 계약 변경은 [SPEC](SPEC.md), 배포 조건은 [LICENSING](LICENSING.md)에서 다룬다.
