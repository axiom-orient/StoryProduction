# Source IR

## SRC Assertions
SRC-001 — 지우는 폐역 기록실에서 원본 근무표를 찾는다.; polarity=AFFIRM; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-003
SRC-002 — 원본 근무표는 민준의 9시 10분 퇴근을 기록한다.; polarity=AFFIRM; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-004
SRC-003 — 9시 40분은 화재 신고 시각이다.; polarity=AFFIRM; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-005
SRC-004 — 지우는 근무표를 원본대로 바로잡는다.; polarity=AFFIRM; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-006
SRC-005 — 지우는 봉투와 기록을 식탁에 둔다.; polarity=AFFIRM; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-007
SRC-006 — 지우는 어머니가 깨어나기 전에 역 문을 잠근다.; polarity=AFFIRM; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-008
SRC-007 — 지우는 첫 열차 전에 혼자 떠난다.; polarity=AFFIRM; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-009

## REL Relations
REL-001 — 근무표 정정과 기록을 남김 BEFORE 혼자 떠남; relation=BEFORE; materiality=CORE; evidence=SPAN-010

## Source Coverage Audit
| Source span | Exact source text | SRC/REL ID | Unmapped reason |
|---|---|---|---|
| SPAN-001 | # 통합 E2E 이야기 | — | MARKDOWN_HEADING |
| SPAN-002 | ## 시놉시스 | — | MARKDOWN_HEADING |
| SPAN-003 | 폐역 기록실에서 지우는 원본 근무표를 찾는다. | SRC-001 | — |
| SPAN-004 | 원본에는 민준이 9시 10분에 역을 떠났다고 적혀 있고, | SRC-002 | — |
| SPAN-005 | 화재 신고 기록에는 9시 40분이 신고 시각으로 남아 있다. | SRC-003 | — |
| SPAN-006 | 지우는 근무표를 원본대로 바로잡고 | SRC-004 | — |
| SPAN-007 | 봉투와 기록을 식탁에 둔다. | SRC-005 | — |
| SPAN-008 | 어머니가 깨어나기 전에 역 문을 잠그고 | SRC-006 | — |
| SPAN-009 | 첫 열차가 오기 전에 혼자 떠난다. | SRC-007 | — |
| SPAN-010 | 지우는 근무표를 원본대로 바로잡고 봉투와 기록을 식탁에 둔다. 어머니가 깨어나기 전에 역 문을 잠그고 첫 열차가 오기 전에 혼자 떠난다. | REL-001 | — |

## GAP Open Adaptation Space
GAP-001: 구체 장소

## CON Authority Conflicts
없음

## Ending Contract
지우가 기록을 대조·정정하고 자료를 남긴 뒤 첫 열차 전에 혼자 떠난다.

## Adaptation Preflight
Source form: prose synopsis. Externalization load: MODERATE.
Target scale: SHORT-FORM. Coverage: FULL. Semantic risk: MODERATE.
Adaptation Pressure: MODERATE — invention authority stays UNRESOLVED until config supplies duration/format.
## Fidelity Risks
9시 40분을 발화 시각으로 바꾸지 않고, 출발 시점을 첫 열차 전으로 유지한다.
