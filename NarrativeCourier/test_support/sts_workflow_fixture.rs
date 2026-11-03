pub const SYNOPSIS: &str =
    "민준은 범인이 아니다. 지우는 민준을 의심하지만 마지막에 진실을 확인하고 혼자 떠난다.";

pub fn artifact_for(node: &str) -> &'static str {
    match node {
        "step_00" => concat!(
            "# Source IR\n\n## SRC Assertions\n",
            "SRC-001 — 민준은 범인이 아니다.; polarity=NEGATE; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-001\n",
            "SRC-002 — 지우는 민준을 의심한다.; polarity=AFFIRM; epistemic=FACT; materiality=SUPPORT; protected=yes; evidence=SPAN-002\n",
            "SRC-003 — 지우는 마지막에 진실을 확인한다.; polarity=AFFIRM; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-003\n",
            "SRC-004 — 지우는 혼자 떠난다.; polarity=AFFIRM; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-004\n\n",
            "## REL Relations\nREL-001 — 진실 확인 BEFORE 지우의 출발; relation=BEFORE; materiality=CORE; evidence=SPAN-005\n\n",
            "## Source Coverage Audit\n",
            "| Source span | Exact source text | SRC/REL ID | Unmapped reason |\n",
            "|---|---|---|---|\n",
            "| SPAN-001 | 민준은 범인이 아니다. | SRC-001 | — |\n",
            "| SPAN-002 | 지우는 민준을 의심하지만 | SRC-002 | — |\n",
            "| SPAN-003 | 마지막에 진실을 확인하고 | SRC-003 | — |\n",
            "| SPAN-004 | 혼자 떠난다. | SRC-004 | — |\n",
            "| SPAN-005 | 진실을 확인하고 혼자 떠난다. | REL-001 | — |\n\n",
            "## GAP Open Adaptation Space\nGAP-001: 구체 장소\n\n",
            "## CON Authority Conflicts\n없음\n\n## Ending Contract\n",
            "지우가 진실을 확인하고 혼자 떠난다.\n\n",
            "## Adaptation Preflight\nSource form: prose synopsis. Externalization load: MODERATE.\n",
            "Target scale: SHORT-FORM. Coverage: FULL. Semantic risk: MODERATE.\n",
            "Adaptation Pressure: MODERATE — invention authority stays UNRESOLVED until config supplies duration/format.\n",
            "## Fidelity Risks\n의심과 사실을 합치지 않고 확인과 출발의 순서를 지킨다.\n"
        ),
        "step_02" => "## Fabula\n사건\n\n## Syuzhet\n제시 순서\n\n## Information Ledger\n상태\n\n## Transformation Ledger\nPRESERVE\n",
        "step_06" => concat!(
            "SCN-001. INT. 방 - 밤\n",
            "- Necessary function: 지우의 의심이 기록 확인으로 전환되고 혼자 떠나는 결말을 수행한다\n",
            "- Cinematic value: 같은 기록이 의심과 확인을 바꾼다\n",
            "- Required preconditions: 지우는 민준을 의심한다\n",
            "- Observable action: 지우가 기록을 확인하고 출발한다\n",
            "- Dominant turn: 기록이 민준의 범행을 부정한다\n",
            "- Required postconditions: 지우는 진실을 알고 혼자 떠난다\n",
            "- Required props: 기록\n",
            "- Downstream dependencies: none\n",
            "- Risk: 의심과 사실을 합치지 않는다\n"
        ),
        "step_07_part1" | "step_07_part2" | "step_07_part3" | "step_07_part4" => concat!(
            "SCN-001. INT. 방 - 밤\n지우는 민준을 의심하지만 기록을 확인한다.\n\n",
            "## [TRANSFER CAPSULE]\n- last SCN: SCN-001\n- knowledge: 기록은 민준의 범행을 부정한다\n"
        ),
        "step_07_full" | "step_08_apply" => concat!(
            "SCN-001. INT. 방 - 밤\n",
            "지우는 민준을 의심하지만 기록을 확인한다. 그 기록은 민준이 범인이 아님을 보여준다. ",
            "지우는 진실을 확인하고 혼자 떠난다.\n"
        ),
        "step_08_reverse" => concat!(
            "## OBS Assertions\n",
            "- OBS-001 — 기록은 민준이 범인이 아님을 보여준다. evidence=SCN-001\n",
            "- OBS-002 — 지우는 민준을 의심한다. evidence=SCN-001\n",
            "- OBS-003 — 지우는 마지막에 기록으로 진실을 확인한다. evidence=SCN-001\n",
            "- OBS-004 — 지우는 혼자 떠난다. evidence=SCN-001\n\n",
            "## OREL Relations\n",
            "- OREL-001 — 진실 확인이 지우의 출발보다 BEFORE다. evidence=SCN-001\n"
        ),
        "step_08_verify" => concat!(
            "# Verification\n",
            "| Source ID | State | Recovered Screenplay IR IDs | Screenplay scene evidence | Rationale |\n",
            "|---|---|---|---|---|\n",
            "| SRC-001 | DRAMATIZED | OBS-001 | SCN-001: 기록이 민준의 무죄를 보여준다 | 핵심 주장을 대본에서 복원한다. |\n",
            "| SRC-002 | PRESERVED | OBS-002 | SCN-001: 지우가 민준을 의심한다 | 의심의 주체와 대상을 보존한다. |\n",
            "| SRC-003 | DRAMATIZED | OBS-003 | SCN-001: 지우가 기록으로 진실을 확인한다 | 확인의 행위자와 결과를 보존한다. |\n",
            "| SRC-004 | PRESERVED | OBS-004 | SCN-001: 지우가 혼자 떠난다 | 결말의 주체를 보존한다. |\n",
            "| REL-001 | PRESERVED | OREL-001 | SCN-001: 확인 뒤 지우가 혼자 떠난다 | 두 사건의 순서를 보존한다. |\n\n",
            "STS_STATUS: READY\n"
        ),
        _ => "# Artifact\n\nverified\n",
    }
}
