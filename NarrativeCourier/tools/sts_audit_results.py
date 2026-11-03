from __future__ import annotations

def validate_result(data: dict, schema: dict) -> None:
    try:
        import jsonschema  # type: ignore
        jsonschema.Draft202012Validator(schema).validate(data)
    except ImportError:
        for k in schema.get("required",[]):
            if k not in data: raise ValueError(f"schema required field missing: {k}")


def _unique_ids(items: list[dict], label: str) -> set[str]:
    ids=[str(item.get("id", "")) for item in items]
    if any(not value for value in ids):
        raise ValueError(f"{label} contains an empty id")
    if len(ids) != len(set(ids)):
        raise ValueError(f"{label} contains duplicate ids")
    return set(ids)


def validate_ir_ids(source_ir: dict, screenplay_ir: dict) -> None:
    _unique_ids(source_ir.get("assertions", []), "Source IR assertions")
    _unique_ids(source_ir.get("relations", []), "Source IR relations")
    _unique_ids(source_ir.get("gaps", []), "Source IR gaps")
    _unique_ids(source_ir.get("conflicts", []), "Source IR conflicts")
    _unique_ids(screenplay_ir.get("observed_assertions", []), "Screenplay IR assertions")
    _unique_ids(screenplay_ir.get("relations", []), "Screenplay IR relations")


def validate_alignment_coverage(result: dict, source_ir: dict, screenplay_ir: dict) -> None:
    source_assertions=_unique_ids(source_ir.get("assertions", []), "Source IR assertions")
    source_relations=_unique_ids(source_ir.get("relations", []), "Source IR relations")
    observed_assertions=_unique_ids(screenplay_ir.get("observed_assertions", []), "Screenplay IR assertions")
    observed_relations=_unique_ids(screenplay_ir.get("relations", []), "Screenplay IR relations")

    assertion_rows=result.get("assertion_alignment", [])
    relation_rows=result.get("relation_alignment", [])
    assertion_sources=[str(row.get("source_id", "")) for row in assertion_rows]
    relation_sources=[str(row.get("source_id", "")) for row in relation_rows]
    if len(assertion_sources) != len(set(assertion_sources)) or set(assertion_sources) != source_assertions:
        raise ValueError("assertion alignment must cover every Source IR assertion exactly once")
    if len(relation_sources) != len(set(relation_sources)) or set(relation_sources) != source_relations:
        raise ValueError("relation alignment must cover every Source IR relation exactly once")
    for row in assertion_rows:
        unknown=set(row.get("recovered_ids", [])) - observed_assertions
        if unknown:
            raise ValueError(f"assertion alignment references unknown Screenplay IR ids: {sorted(unknown)}")
    for row in relation_rows:
        unknown=set(row.get("recovered_ids", [])) - observed_relations
        if unknown:
            raise ValueError(f"relation alignment references unknown Screenplay IR ids: {sorted(unknown)}")


def validate_verdict_coherence(result: dict, source_ir: dict) -> None:
    dimensions=result["fidelity"]["dimensions"] + result["integrity"]["dimensions"]
    has_blocked=any(d["status"] == "BLOCKED" for d in dimensions)
    has_review=any(d["status"] == "REVIEW" for d in dimensions)
    has_critical=any(f["severity"] == "CRITICAL" for f in result.get("findings", []))

    source_assertions={item["id"]: item for item in source_ir.get("assertions", [])}
    source_relations={item["id"]: item for item in source_ir.get("relations", [])}
    hard_alignment=False
    uncertain=False
    for row in result.get("assertion_alignment", []):
        item=source_assertions[row["source_id"]]
        if row["status"] in {"OMITTED", "DRIFTED"} and (item.get("protected") or item.get("materiality") == "CORE"):
            hard_alignment=True
        uncertain = uncertain or row["status"] == "UNCERTAIN"
    for row in result.get("relation_alignment", []):
        item=source_relations[row["source_id"]]
        if row["status"] in {"OMITTED", "DRIFTED"} and item.get("materiality") == "CORE":
            hard_alignment=True
        uncertain = uncertain or row["status"] == "UNCERTAIN"

    if (has_blocked or has_critical or hard_alignment) and result["verdict"] != "BLOCKED":
        raise ValueError("audit verdict must be BLOCKED when a hard failure is present")
    if not (has_blocked or has_critical or hard_alignment) and (has_review or uncertain) and result["verdict"] == "PASS":
        raise ValueError("audit verdict cannot PASS while REVIEW/UNCERTAIN evidence remains")


def validate_fixed_dimensions(result: dict) -> None:
    expected_f={"SOURCE_ASSERTIONS","SOURCE_RELATIONS","ENDING_OUTCOME","ENDING_MEANING","ACTOR_OWNERSHIP","KNOWLEDGE_EPISTEMIC","FORM_FUNCTION","TRANSFORMATION_INVENTION"}
    expected_i={"CAUSALITY","KNOWLEDGE_STATE","CHARACTER_RELATIONSHIP_STATE","PHYSICAL_SPATIAL_STATE","SCENE_DEPENDENCIES","PLAYABILITY","SCENE_ECONOMY"}
    got_f={x["id"] for x in result["fidelity"]["dimensions"]}
    got_i={x["id"] for x in result["integrity"]["dimensions"]}
    if got_f != expected_f: raise ValueError(f"fidelity dimensions mismatch: {got_f}")
    if got_i != expected_i: raise ValueError(f"integrity dimensions mismatch: {got_i}")


def render_report(result: dict, meta: dict) -> str:
    lines=["# STS Existing-Screenplay Audit","",f"- **Verdict:** {result['verdict']}",f"- **Confidence:** {result['confidence']:.2f}",f"- **STS audit domain:** {meta['domain_version']}",f"- **Audit bundle SHA-256:** `{meta['audit_bundle_sha256']}`","", "## Summary", "", result["summary"], "", "## Fidelity", ""]
    for d in result["fidelity"]["dimensions"]: lines.append(f"- **{d['id']} — {d['status']}**: {d['summary']}")
    lines += ["", "## Screenplay Integrity", ""]
    for d in result["integrity"]["dimensions"]: lines.append(f"- **{d['id']} — {d['status']}**: {d['summary']}")
    operations={}
    for row in result["assertion_alignment"] + result["relation_alignment"]:
        operations[row["status"]]=operations.get(row["status"],0)+1
    lines += ["", "## Adaptation Operations", ""]
    for key in ("PRESERVED","DRAMATIZED","COMBINED","REORDERED","OMITTED","DRIFTED","UNCERTAIN"):
        lines.append(f"- **{key}:** {operations.get(key,0)}")
    priority={"CRITICAL":0,"MAJOR":1,"MINOR":2}
    repairs=sorted(result["findings"], key=lambda f: priority[f["severity"]])[:5]
    lines += ["", "## Priority Repairs", ""]
    if not repairs: lines.append("No material repair is required by this audit.")
    for i,f in enumerate(repairs,1):
        lines.append(f"{i}. **{f['severity']} / {f['axis']} / {f['category']}** — {f['recommendation']}")
    lines += ["", "## Source Assertion Alignment", ""]
    for a in result["assertion_alignment"]:
        lines += [f"### {a['source_id']} — {a['status']}", f"Recovered: {', '.join(a['recovered_ids']) or '—'}", a['rationale']]
        if a['evidence']: lines.append("Evidence: "+" | ".join(a['evidence']))
        lines.append("")
    lines += ["## Source Relation Alignment", ""]
    for a in result["relation_alignment"]:
        lines += [f"### {a['source_id']} — {a['status']}", f"Recovered: {', '.join(a['recovered_ids']) or '—'}", a['rationale']]
        if a['evidence']: lines.append("Evidence: "+" | ".join(a['evidence']))
        lines.append("")
    lines += ["## Findings", ""]
    if not result["findings"]: lines.append("No material findings.")
    for f in result["findings"]:
        lines += [f"### {f['severity']} / {f['axis']} / {f['category']}",f["issue"],"Evidence: "+(" | ".join(f["evidence"]) or "—"),"Recommendation: "+f["recommendation"],""]
    lines += ["## Reproducibility", "", f"- Synopsis SHA-256: `{meta['synopsis_sha256']}`", f"- Screenplay SHA-256: `{meta['screenplay_sha256']}`", f"- STS model: `{meta['model']}`", f"- Reasoning: `{meta['reasoning']}`", f"- Input formats: `{meta['synopsis_format']}` / `{meta['screenplay_format']}`", "- Screenplay reconstruction was source-blind; comparison received only the two independent IRs.", ""]
    return "\n".join(lines)


