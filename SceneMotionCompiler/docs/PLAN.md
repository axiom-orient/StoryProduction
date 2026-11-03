# SceneMotionCompiler — 남은 PLAN

현재 구현과 기계 실행 증거는 [Current](IMPLEMENTATION_STATUS.md), 확정 계약은 [SPEC](SPEC.md)에 둔다.
아래 표는 해당 검증으로 완료가 확인되지 않은 결정과 qualification을 기록한다.

| 남은 범위 | 현재 계약 | 필요한 후속 결정·qualification |
|---|---|---|
| 최종 영상 release | editorial/animatic은 candidate, 기존 HTML custody와 별도 | 영상 subject·source/approval/QC/media retention·adoption·CAS/reconcile의 명시 계약 |
| Native/Studio 배포 | Darwin/arm64 descriptor/lock/helper·Studio Human approval | signed packaged helper·Node lifecycle·실제 Human session·독립 앱 서명/공증/Node bundle |
| 실제 generation/QC/repair | provider-neutral request·clip ingest·관찰·selective repair 범위 | 실제 reference/first-last-frame·provider unknown 조정·실품질·사람 판단 |
| Editorial/audio/interchange | frame timeline·제한 OTIO JSON, audio export 차단 | 실제 audio probe/sync/mix·transition/caption·NLE roundtrip·portable media/relink |
| Animatic 실사용 | approved packet/PNG·exact-frame candidate·cancel/recover·bounded policy | 실제 작품/imagegen 자산·Human 검토·최대 부하·다른 host·접근성·OS/disk faults |
| Storage·process 복구 | immutable publication·partial effect·worker identity·unknown no auto retry | HTML/provenance pair·multiwriter·hostile ancestor·transport cancel·orphan 조사/정리 범위 |

[전체 분석](../ANALYSIS.md)·[production](../packages/production/ANALYSIS.md)·[상위 PLAN](../../docs/PLAN.md)은
각자의 scope를 소유한다. 미지원 기능이나 실행하지 않은 qualification을 문서로 완료 처리하지 않는다.
