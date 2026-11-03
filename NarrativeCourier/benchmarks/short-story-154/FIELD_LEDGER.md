# STS 154-Story Field Ledger

This ledger is empirical regression evidence, not runtime prompt material.

## Aggregate

```json
{
  "corpus": "Korean Story Polish v4 001-154",
  "stories": 154,
  "prior_revised": 55,
  "prior_kept": 99,
  "direct_contract_revision_matches": 41,
  "adjacent_craft_revision_matches": 2,
  "out_of_scope_prose_only_revisions": 12,
  "high_severity_total": 16,
  "high_severity_sts_matched": 16,
  "medium_severity_total": 13,
  "medium_severity_sts_matched": 11,
  "risk_lens_counts": {
    "SEMANTIC_CODA": 98,
    "WORLD_RULE": 60,
    "PHYSICAL_INSTITUTIONAL": 24,
    "EPISTEMIC": 70,
    "FORM_FUNCTION": 23,
    "SPACE_STATE": 32,
    "SOURCE_FIDELITY": 32
  },
  "sts_value_counts": {
    "MEDIUM": 57,
    "HIGH": 69,
    "BASELINE": 28
  }
}
```

## Per-story application

| ID | Title | STS value | Risk lenses | Oracle | Primary screenplay obligation |
|---|---|---|---|---|---|
| 001 | 아주 옛날에는 사람에게 그림자가 없었더란다. | MEDIUM | SEMANTIC_CODA | KEEP/none | preserve ending meaning while externalizing prose interpretation cinematically |
| 002 | 잠을 대신 자는 사람 | HIGH | WORLD_RULE|PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA | REVISE/high (referent|world_rule|causality|ending) | world-rule scope/cost/exception consistency |
| 003 | 막내 | MEDIUM | EPISTEMIC | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 004 | 기억 보관 서비스 이용약관 | MEDIUM | WORLD_RULE|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | world-rule scope/cost/exception consistency |
| 005 | 83초 | HIGH | WORLD_RULE|PHYSICAL_INSTITUTIONAL|SPACE_STATE | REVISE/high (world_rule|physical_timing) | world-rule scope/cost/exception consistency |
| 006 | 엄마 회사 | HIGH | SOURCE_FIDELITY | REVISE/low (referent) | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 007 | 버리고 싶은 마음 접수처 | MEDIUM | WORLD_RULE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 008 | 목소리 보관 사기 | HIGH | EPISTEMIC|PHYSICAL_INSTITUTIONAL | REVISE/high (character_knowledge|continuity) | who knows/believes what, and how that knowledge is acquired |
| 009 | 기억하지 못하는 자장가 | MEDIUM | EPISTEMIC | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 010 | 면회 시간 | MEDIUM | EPISTEMIC|SEMANTIC_CODA | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 011 | 돌이 되는 왕 | MEDIUM | WORLD_RULE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 012 | 마지막 박자 | MEDIUM | SEMANTIC_CODA | KEEP/none | preserve ending meaning while externalizing prose interpretation cinematically |
| 013 | 은퇴한 용사의 귀환 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 014 | 할아버지가 칭찬하면 전기가 나가요 | MEDIUM | WORLD_RULE | KEEP/none | world-rule scope/cost/exception consistency |
| 015 | 딱 한 번 돌아갈 수 있는 하루 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 016 | 새벽 두 시의 라디오 | MEDIUM | FORM_FUNCTION | KEEP/none | preserve the narrative function of the source form, not its literal container |
| 017 | 하루 동안 돌려드립니다 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 018 | 먼저 가 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 019 | 내가 들게 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 020 | 실종신고 취소 | MEDIUM | EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | REVISE/low (naturalness) | who knows/believes what, and how that knowledge is acquired |
| 021 | 친구의 기준 | MEDIUM | EPISTEMIC|SEMANTIC_CODA | REVISE/low (naturalness) | who knows/believes what, and how that knowledge is acquired |
| 022 | 마중 나오는 사람 | MEDIUM | EPISTEMIC|SEMANTIC_CODA | REVISE/medium (dialogue|clarity) | who knows/believes what, and how that knowledge is acquired |
| 023 | 중고 시계 | HIGH | EPISTEMIC|PHYSICAL_INSTITUTIONAL | REVISE/high (continuity) | who knows/believes what, and how that knowledge is acquired |
| 024 | 아무도 앉지 않는 자리 | MEDIUM | SPACE_STATE|SEMANTIC_CODA | REVISE/low (naturalness) | convert setting into observable access/visibility/action constraints |
| 025 | 용서 대기번호 | HIGH | WORLD_RULE|SEMANTIC_CODA|FORM_FUNCTION | REVISE/low (referent) | world-rule scope/cost/exception consistency |
| 026 | 고양이가 보는 집 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 027 | 마지막 역 다음 | MEDIUM | WORLD_RULE|SPACE_STATE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 028 | 슈퍼 앞 의자 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 029 | 왕의 코는 거짓말을 못 한다 | HIGH | WORLD_RULE|EPISTEMIC | KEEP/none | world-rule scope/cost/exception consistency |
| 030 | 모르는 척해 주는 가게 | MEDIUM | WORLD_RULE|SPACE_STATE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 031 | 낮 근무와 밤 근무의 노트 | HIGH | SOURCE_FIDELITY | REVISE/low (referent) | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 032 | 다시 연결된 번호 | MEDIUM | EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 033 | 유리구슬 | MEDIUM | EPISTEMIC|SEMANTIC_CODA | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 034 | 두 형제와 빈 곡간 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 035 | 숟가락과 젓가락 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 036 | 달리기 싫어하는 사슴 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 037 | 종을 너무 자주 울린 아이 | HIGH | EPISTEMIC|SEMANTIC_CODA | REVISE/medium (causality|wording) | who knows/believes what, and how that knowledge is acquired |
| 038 | 달을 빌려 간 나무꾼 | MEDIUM | WORLD_RULE|SPACE_STATE | KEEP/none | world-rule scope/cost/exception consistency |
| 039 | 나무 아이의 첫 거짓말 | HIGH | WORLD_RULE|EPISTEMIC | KEEP/none | world-rule scope/cost/exception consistency |
| 040 | 자리 맡아 드립니다 | MEDIUM | SEMANTIC_CODA | KEEP/none | preserve ending meaning while externalizing prose interpretation cinematically |
| 041 | 흰 우산을 든 아이 | HIGH | WORLD_RULE|EPISTEMIC|SPACE_STATE | KEEP/none | world-rule scope/cost/exception consistency |
| 042 | 등대 안의 바다 | MEDIUM | WORLD_RULE|SPACE_STATE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 043 | 높은 가지의 감 | HIGH | EPISTEMIC | REVISE/medium (physical_action) | who knows/believes what, and how that knowledge is acquired |
| 044 | 문이 좁은 식당 | HIGH | PHYSICAL_INSTITUTIONAL|SPACE_STATE|SEMANTIC_CODA | REVISE/medium (ending|theme|clarity) | physical/institutional feasibility and state continuity |
| 045 | 마른 우물 | HIGH | EPISTEMIC|PHYSICAL_INSTITUTIONAL|SPACE_STATE|SEMANTIC_CODA | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 046 | 김치 끝 조각 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 047 | 은퇴자 밴드 | HIGH | SOURCE_FIDELITY | REVISE/low (referent) | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 048 | 새벽 해장국 | MEDIUM | SEMANTIC_CODA | KEEP/none | preserve ending meaning while externalizing prose interpretation cinematically |
| 049 | 국 한 그릇 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 050 | 십 분을 파는 시계방 | MEDIUM | WORLD_RULE|SEMANTIC_CODA | REVISE/low (line_edit) | world-rule scope/cost/exception consistency |
| 051 | 대신 고맙다고 전해 드립니다 | MEDIUM | EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 052 | 옥상에서 가장 가까운 곳 | HIGH | PHYSICAL_INSTITUTIONAL|SPACE_STATE|SEMANTIC_CODA | REVISE/high (world_rule|institutional_logic) | physical/institutional feasibility and state continuity |
| 053 | 유실물 보관 기간 | MEDIUM | EPISTEMIC | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 054 | 감정 반품센터 | MEDIUM | WORLD_RULE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 055 | 사진관의 다섯 번째 칸 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | REVISE/medium (interpretation|clarity) | world-rule scope/cost/exception consistency |
| 056 | 정원을 물려받은 사람 | MEDIUM | SEMANTIC_CODA | REVISE/medium (specificity|naturalness) | preserve ending meaning while externalizing prose interpretation cinematically |
| 057 | 아이들만 보이는 신호등 | HIGH | WORLD_RULE|EPISTEMIC|SPACE_STATE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 058 | 바리의 빈 병 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 059 | 두 자매의 신발 | HIGH | SOURCE_FIDELITY | REVISE/medium (referent|specificity) | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 060 | 마지막 배달 | MEDIUM | SPACE_STATE|SEMANTIC_CODA | REVISE/low (naturalness) | convert setting into observable access/visibility/action constraints |
| 061 | 별 하나 늦게 뜨는 이유 | HIGH | WORLD_RULE|PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA | REVISE/high (world_rule|causality) | world-rule scope/cost/exception consistency |
| 062 | 그림자가 먼저 늙는다 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 063 | 꿈 수선집 | HIGH | WORLD_RULE|SEMANTIC_CODA | REVISE/low (referent) | world-rule scope/cost/exception consistency |
| 064 | 후회 보험 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 065 | 일주일에 한 시간 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 066 | 명절 자리 배치 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 067 | 하루를 다시 사는 법 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | REVISE/high (character_knowledge|loop_invariant) | world-rule scope/cost/exception consistency |
| 068 | 부재중 전화 17통 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 069 | 세 가지 소원을 거절한 사람 | HIGH | WORLD_RULE|SEMANTIC_CODA | REVISE/low (ending|naturalness) | world-rule scope/cost/exception consistency |
| 070 | 숲의 재판 | MEDIUM | EPISTEMIC|SPACE_STATE|SEMANTIC_CODA | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 071 | 가까이 서지 못하는 여자 | HIGH | WORLD_RULE|EPISTEMIC|SPACE_STATE|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | world-rule scope/cost/exception consistency |
| 072 | 장난감 상자의 마지막 밤 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 073 | 늙은 사자의 마지막 포효 | MEDIUM | WORLD_RULE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 074 | 기억 창고의 작은 직원 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 075 | 엄마가 남긴 알람 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | world-rule scope/cost/exception consistency |
| 076 | 막차 기사 | HIGH | PHYSICAL_INSTITUTIONAL|SPACE_STATE|SEMANTIC_CODA | REVISE/high (institutional_logic|causality) | physical/institutional feasibility and state continuity |
| 077 | 자판기 커피 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 078 | 셈을 못하는 아이 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 079 | 3층 버튼 | MEDIUM | SPACE_STATE|SEMANTIC_CODA | KEEP/none | convert setting into observable access/visibility/action constraints |
| 080 | 거짓말이 자라는 화분 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 081 | 대장간 고양이 삼 년이면 불을 안다 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 082 | 늙은 독수리는 새끼였던 때를 잊는다 | BASELINE | SOURCE_FIDELITY | REVISE/low (line_edit) | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 083 | 곰 이야기하면 곰이 온다 | HIGH | WORLD_RULE|EPISTEMIC|PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA | REVISE/medium (causality|dialogue) | world-rule scope/cost/exception consistency |
| 084 | 메아리 장사꾼 | MEDIUM | WORLD_RULE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 085 | 까치가 물고 간 반지 | HIGH | EPISTEMIC|PHYSICAL_INSTITUTIONAL | REVISE/high (continuity) | who knows/believes what, and how that knowledge is acquired |
| 086 | 마지막 봉화 | MEDIUM | PHYSICAL_INSTITUTIONAL|SPACE_STATE | KEEP/none | physical/institutional feasibility and state continuity |
| 087 | 모래알로 벽 치기 | MEDIUM | PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA | REVISE/low (line_edit) | physical/institutional feasibility and state continuity |
| 088 | 낮에는 참새가 듣고 밤에는 와이파이가 듣는다 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 089 | 고양이 말을 알아듣는 여자 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | world-rule scope/cost/exception consistency |
| 090 | 사진 밖의 아버지 | MEDIUM | EPISTEMIC|SEMANTIC_CODA | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 091 | 해가 북쪽에서 뜬 날 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 092 | 나이를 맡기는 은행 | HIGH | WORLD_RULE|EPISTEMIC|PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA | REVISE/high (world_rule) | world-rule scope/cost/exception consistency |
| 093 | 결혼사진의 빈 의자 | MEDIUM | SPACE_STATE|SEMANTIC_CODA | KEEP/none | convert setting into observable access/visibility/action constraints |
| 094 | 바다에게 목소리를 빌린 소녀 | MEDIUM | WORLD_RULE | KEEP/none | world-rule scope/cost/exception consistency |
| 095 | 세 친구의 우물 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 096 | 참은 만큼 돌려드립니다 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 097 | 좋아요가 사라진 날 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | REVISE/low (referent) | world-rule scope/cost/exception consistency |
| 098 | 한 짝 장갑 | HIGH | PHYSICAL_INSTITUTIONAL | REVISE/medium (counting|word_choice) | physical/institutional feasibility and state continuity |
| 099 | 취하지 않는 술 | MEDIUM | WORLD_RULE|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | world-rule scope/cost/exception consistency |
| 100 | 패배한 장군의 독백 | MEDIUM | EPISTEMIC|FORM_FUNCTION | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 101 | 성문 앞의 무명 병사 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 102 | 걱정 보관소의 만기일 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | REVISE/medium (referent|ending) | world-rule scope/cost/exception consistency |
| 103 | 답장이 오지 않는 편지 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | REVISE/low (referent) | world-rule scope/cost/exception consistency |
| 104 | 입이 거친 세탁소 할머니 | HIGH | SOURCE_FIDELITY | REVISE/low (referent) | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 105 | 고양이가 모르는 하루 | MEDIUM | EPISTEMIC | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 106 | 강아지의 귀 | MEDIUM | EPISTEMIC | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 107 | 평범한 날의 영수증 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 108 | 콩 한 알 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 109 | 초 하나 | HIGH | EPISTEMIC|SEMANTIC_CODA | REVISE/medium (ending|invariant_review) | who knows/believes what, and how that knowledge is acquired |
| 110 | 후회 지우개 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | world-rule scope/cost/exception consistency |
| 111 | 급식실의 빈 접시 | HIGH | PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA | REVISE/high (physical_accounting|safety) | physical/institutional feasibility and state continuity |
| 112 | 수요일 밤의 토요일 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | world-rule scope/cost/exception consistency |
| 113 | 왕의 늙은 말 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 114 | 국가기밀문서-우체통 프로젝트 | HIGH | EPISTEMIC|PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 115 | 관리실 불빛 | MEDIUM | SPACE_STATE|SEMANTIC_CODA | KEEP/none | convert setting into observable access/visibility/action constraints |
| 116 | 호텔 수건 | MEDIUM | EPISTEMIC|SEMANTIC_CODA | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 117 | 복사기 옆의 머그컵 | MEDIUM | SPACE_STATE|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | preserve the narrative function of the source form, not its literal container |
| 118 | 이 부장의 낚시 | MEDIUM | EPISTEMIC | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 119 | 손바닥만 한 자국 | MEDIUM | SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | preserve the narrative function of the source form, not its literal container |
| 120 | 세 개의 코드 | BASELINE | SOURCE_FIDELITY | REVISE/low (collocation|naturalness) | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 121 | 수면선 | HIGH | EPISTEMIC|SEMANTIC_CODA | REVISE/low (referent|naturalness) | who knows/believes what, and how that knowledge is acquired |
| 122 | 막차의 빈 정류장 | MEDIUM | SPACE_STATE|SEMANTIC_CODA | KEEP/none | convert setting into observable access/visibility/action constraints |
| 123 | 등대지기의 노래 | HIGH | WORLD_RULE|EPISTEMIC|PHYSICAL_INSTITUTIONAL|SPACE_STATE|SEMANTIC_CODA | REVISE/high (physical_logic|causality) | world-rule scope/cost/exception consistency |
| 124 | 할머니가 웃으면 눈이 와요 | MEDIUM | WORLD_RULE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 125 | 일요일의 다리미 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 126 | 죽음에게 매일 편지를 쓴 소녀 | HIGH | WORLD_RULE|EPISTEMIC|PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA | REVISE/high (character_knowledge) | world-rule scope/cost/exception consistency |
| 127 | 약속의 벤치 | HIGH | SPACE_STATE|SEMANTIC_CODA | REVISE/low (referent) | convert setting into observable access/visibility/action constraints |
| 128 | 마지막 손님을 깎는 이발사 | MEDIUM | SPACE_STATE|SEMANTIC_CODA | KEEP/none | convert setting into observable access/visibility/action constraints |
| 129 | 내일에서 온 친구 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | REVISE/high (timeline|ending) | world-rule scope/cost/exception consistency |
| 130 | 화가의 빈 캔버스 | MEDIUM | SEMANTIC_CODA | KEEP/none | preserve ending meaning while externalizing prose interpretation cinematically |
| 131 | 할머니의 두 번째 이름 | HIGH | EPISTEMIC|SEMANTIC_CODA | REVISE/low (referent) | who knows/believes what, and how that knowledge is acquired |
| 132 | 수호자의 퇴근 | HIGH | SEMANTIC_CODA | REVISE/medium (referent|invariant_review) | preserve ending meaning while externalizing prose interpretation cinematically |
| 133 | 열지 않은 타임캡슐 | HIGH | EPISTEMIC|PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA | REVISE/high (timeline|referent) | who knows/believes what, and how that knowledge is acquired |
| 134 | 지옥의 리코더 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 135 | 보이지 않는 사람의 상담 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | world-rule scope/cost/exception consistency |
| 136 | 아버지의 스텝 | MEDIUM | EPISTEMIC | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 137 | 말이 바뀌는 가족 | BASELINE | SOURCE_FIDELITY | KEEP/none | preserve actor ownership, causality and ending while externalizing prose into behavior |
| 138 | 사람이 있는 편의점 | HIGH | PHYSICAL_INSTITUTIONAL|SPACE_STATE|SEMANTIC_CODA | REVISE/low (referent) | physical/institutional feasibility and state continuity |
| 139 | 맞은편 창문 | MEDIUM | EPISTEMIC|SPACE_STATE|SEMANTIC_CODA | REVISE/low (word_order) | who knows/believes what, and how that knowledge is acquired |
| 140 | 놓고 온 우산 | MEDIUM | SPACE_STATE|SEMANTIC_CODA | KEEP/none | convert setting into observable access/visibility/action constraints |
| 141 | 귀신들의 수선집 | MEDIUM | EPISTEMIC|SPACE_STATE|SEMANTIC_CODA | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 142 | 한 사람의 소원 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 143 | 화성에서 온 단체채팅 | MEDIUM | EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | who knows/believes what, and how that knowledge is acquired |
| 144 | 내 동생은 시간 여행자 | HIGH | WORLD_RULE|EPISTEMIC | KEEP/none | world-rule scope/cost/exception consistency |
| 145 | 칼국수 두 그릇 | BASELINE | SPACE_STATE | REVISE/low (line_edit) | convert setting into observable access/visibility/action constraints |
| 146 | 마지막 손그림 포스터 | HIGH | PHYSICAL_INSTITUTIONAL|SPACE_STATE|SEMANTIC_CODA | REVISE/high (physical_continuity) | physical/institutional feasibility and state continuity |
| 147 | 세 여신과 빈 사과 | HIGH | WORLD_RULE|PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA | REVISE/medium (referent|world_detail|ending) | world-rule scope/cost/exception consistency |
| 148 | 소개팅에서 미래 얼굴을 본 남자 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | world-rule scope/cost/exception consistency |
| 149 | 아들이 찾아온 날 | MEDIUM | EPISTEMIC|SEMANTIC_CODA | REVISE/low (naturalness) | who knows/believes what, and how that knowledge is acquired |
| 150 | 악마 인턴과 계약한 남자 | MEDIUM | WORLD_RULE|SPACE_STATE|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 151 | 가상도시의 꽃집 | HIGH | WORLD_RULE|EPISTEMIC|SPACE_STATE|SEMANTIC_CODA|FORM_FUNCTION | REVISE/low (referent) | world-rule scope/cost/exception consistency |
| 152 | 울어도 출근할 수 있는 안경 | HIGH | WORLD_RULE|EPISTEMIC|PHYSICAL_INSTITUTIONAL|SEMANTIC_CODA|FORM_FUNCTION | REVISE/low (particle|naturalness) | world-rule scope/cost/exception consistency |
| 153 | 혼인 전 열람실 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA | KEEP/none | world-rule scope/cost/exception consistency |
| 154 | 고맙다는 말을 못 듣는 남자 | HIGH | WORLD_RULE|EPISTEMIC|SEMANTIC_CODA|FORM_FUNCTION | KEEP/none | world-rule scope/cost/exception consistency |
