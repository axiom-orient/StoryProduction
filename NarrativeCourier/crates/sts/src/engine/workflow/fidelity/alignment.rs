//! STEP 8D source-to-screenplay reconstruction alignment.

use super::source_inventory::{parse_source_items, ItemKind, SourceItem};
use super::text::{
    contract, has_duplicates, identifiers, is_table_separator, leading_id, strip_markdown_prefix,
    table_cells,
};
use crate::model::workflow::Node;
use narrative_courier_runtime::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const SOURCE_IR_INPUT: &str = "output/step_00_source_ir.md";
const SCREENPLAY_IR_INPUT: &str = "output/step_08_recovered_screenplay_ir.md";
const ALIGNMENT_HEADER: &str =
    "| Source ID | State | Recovered Screenplay IR IDs | Screenplay scene evidence | Rationale |";

#[derive(Debug)]
struct RecoveredItem {
    kind: ItemKind,
    scene_ids: BTreeSet<String>,
}

#[derive(Debug)]
struct AlignmentRow {
    source_id: String,
    state: String,
    recovered_ids: Vec<String>,
    scene_ids: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Ready,
    Blocked,
}

/// Re-check STEP 8D's references at both submission and final human release.
/// The inputs are the exact durable files resolved from the node manifest.
pub(in crate::engine::workflow) fn validate_step_08(
    node: &Node,
    inputs: &[PathBuf],
    report: &str,
) -> Result<()> {
    if node.id != "step_08_verify" {
        return Ok(());
    }
    let result = (|| {
        let source_path = declared_input(node, inputs, SOURCE_IR_INPUT)?;
        let screenplay_path = declared_input(node, inputs, SCREENPLAY_IR_INPUT)?;
        let source_ir = fs::read_to_string(source_path)?;
        let screenplay_ir = fs::read_to_string(screenplay_path)?;
        validate_alignment(&source_ir, &screenplay_ir, report)
    })();
    result.map_err(step_08_context)
}

fn step_08_context(error: Error) -> Error {
    match error {
        Error::Contract(message) => {
            Error::Contract(format!("STEP 8D fidelity alignment: {message}"))
        }
        other => other,
    }
}

fn declared_input<'a>(node: &Node, inputs: &'a [PathBuf], declared: &str) -> Result<&'a PathBuf> {
    if !node.inputs.iter().any(|input| input == declared) {
        return Err(contract(format!(
            "verification input is not declared: {declared}"
        )));
    }
    let mut matches = inputs
        .iter()
        .filter(|input| input.ends_with(Path::new(declared)));
    let path = matches
        .next()
        .ok_or_else(|| contract(format!("verification input did not resolve: {declared}")))?;
    if matches.next().is_some() {
        return Err(contract(format!(
            "verification input resolves more than once: {declared}"
        )));
    }
    Ok(path)
}

fn validate_alignment(source_ir: &str, screenplay_ir: &str, report: &str) -> Result<()> {
    let source_items = parse_source_items(source_ir)?;
    let recovered_items = parse_recovered_items(screenplay_ir)?;
    let (verdict, rows) = parse_report(report)?;
    let mut rows_by_source = BTreeMap::new();

    for row in &rows {
        let source_item = source_items.get(&row.source_id).ok_or_else(|| {
            contract(format!(
                "alignment references unknown source ID {}",
                row.source_id
            ))
        })?;
        if rows_by_source.insert(row.source_id.as_str(), row).is_some() {
            return Err(contract(format!(
                "source ID {} has duplicate alignment rows",
                row.source_id
            )));
        }
        validate_row(source_item, &recovered_items, row)?;
    }

    for (source_id, item) in &source_items {
        if item.must_be_aligned() && !rows_by_source.contains_key(source_id.as_str()) {
            return Err(contract(format!(
                "material source ID {source_id} has no alignment row"
            )));
        }
    }

    if verdict == Verdict::Ready {
        for (source_id, row) in &rows_by_source {
            let item = source_items.get(*source_id).ok_or_else(|| {
                contract(format!(
                    "alignment references unknown source ID {source_id}"
                ))
            })?;
            if row.state == "UNCERTAIN" {
                return Err(contract(format!(
                    "READY cannot include uncertain source alignment {source_id}"
                )));
            }
            if matches!(row.state.as_str(), "OMITTED" | "DRIFTED") && item.omission_blocks_ready() {
                return Err(contract(format!(
                    "READY cannot include protected or CORE source drift {source_id}"
                )));
            }
        }
    }
    Ok(())
}

fn parse_recovered_items(screenplay_ir: &str) -> Result<BTreeMap<String, RecoveredItem>> {
    let mut items = BTreeMap::new();
    let mut current: Option<(String, ItemKind, BTreeSet<String>)> = None;

    for line in screenplay_ir.lines() {
        let body = strip_markdown_prefix(line);
        let next = if let Some(id) = leading_id(body, "OBS-")? {
            Some((id, ItemKind::Assertion))
        } else {
            leading_id(body, "OREL-")?.map(|id| (id, ItemKind::Relation))
        };
        if let Some((id, kind)) = next {
            insert_recovered(&mut items, current.take())?;
            let scene_ids = identifiers(line, "SCN-")?.into_iter().collect();
            current = Some((id, kind, scene_ids));
            continue;
        }
        if line.trim().starts_with('#') {
            insert_recovered(&mut items, current.take())?;
            continue;
        }
        if let Some((_, _, scenes)) = current.as_mut() {
            scenes.extend(identifiers(line, "SCN-")?);
        }
    }
    insert_recovered(&mut items, current)?;
    if items.is_empty() {
        return Err(contract(
            "recovered Screenplay IR has no OBS/OREL declarations",
        ));
    }
    Ok(items)
}

fn insert_recovered(
    items: &mut BTreeMap<String, RecoveredItem>,
    current: Option<(String, ItemKind, BTreeSet<String>)>,
) -> Result<()> {
    let Some((id, kind, scene_ids)) = current else {
        return Ok(());
    };
    if scene_ids.is_empty() {
        return Err(contract(format!("recovered item {id} has no SCN evidence")));
    }
    if items
        .insert(id.clone(), RecoveredItem { kind, scene_ids })
        .is_some()
    {
        return Err(contract(format!("Screenplay IR repeats {id}")));
    }
    Ok(())
}

fn parse_report(report: &str) -> Result<(Verdict, Vec<AlignmentRow>)> {
    let mut header_line = None;
    let mut status_line = None;
    let mut status = None;
    let mut in_table = false;
    let mut rows = Vec::new();

    for (line_number, line) in report.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed == ALIGNMENT_HEADER {
            if header_line.replace(line_number).is_some() {
                return Err(contract(
                    "verification report repeats the alignment table header",
                ));
            }
            in_table = true;
            continue;
        }
        if trimmed.starts_with("STS_STATUS:") {
            if status_line.replace(line_number).is_some() {
                return Err(contract(
                    "verification report must contain one STS_STATUS line",
                ));
            }
            status = match trimmed {
                "STS_STATUS: READY" => Some(Verdict::Ready),
                "STS_STATUS: BLOCKED" => Some(Verdict::Blocked),
                _ => {
                    return Err(contract(
                        "verification report has an unsupported STS_STATUS",
                    ))
                }
            };
            in_table = false;
            continue;
        }
        if !in_table || trimmed.is_empty() {
            continue;
        }
        if !trimmed.starts_with('|') {
            in_table = false;
            continue;
        }
        let cells = table_cells(trimmed)?;
        if cells.len() != 5 {
            return Err(contract("alignment rows must have exactly five cells"));
        }
        if is_table_separator(&cells) {
            continue;
        }
        rows.push(parse_alignment_row(&cells)?);
    }

    let header_line =
        header_line.ok_or_else(|| contract("verification report has no alignment table"))?;
    let status_line =
        status_line.ok_or_else(|| contract("verification report has no STS_STATUS"))?;
    if status_line <= header_line {
        return Err(contract(
            "STS_STATUS must follow the evidence alignment table",
        ));
    }
    let last_nonempty_line = report
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, _)| index)
        .last();
    if last_nonempty_line != Some(status_line) {
        return Err(contract(
            "STS_STATUS must be the final non-empty report line",
        ));
    }
    let status =
        status.ok_or_else(|| contract("verification report has no supported STS_STATUS"))?;
    Ok((status, rows))
}

fn parse_alignment_row(cells: &[String]) -> Result<AlignmentRow> {
    if cells.len() != 5 {
        return Err(contract("alignment rows must have exactly five cells"));
    }
    let mut source_ids = identifiers(&cells[0], "SRC-")?;
    source_ids.extend(identifiers(&cells[0], "REL-")?);
    if source_ids.len() != 1 {
        return Err(contract(
            "alignment row must name exactly one SRC or REL ID",
        ));
    }
    let state = cells[1].trim().trim_matches('`').to_ascii_uppercase();
    if !matches!(
        state.as_str(),
        "PRESERVED" | "DRAMATIZED" | "COMBINED" | "REORDERED" | "OMITTED" | "DRIFTED" | "UNCERTAIN"
    ) {
        return Err(contract(format!("alignment has unsupported state {state}")));
    }
    if cells[4].trim().is_empty() {
        return Err(contract(format!(
            "alignment row {} has no rationale",
            source_ids[0]
        )));
    }
    let mut recovered_ids = identifiers(&cells[2], "OBS-")?;
    recovered_ids.extend(identifiers(&cells[2], "OREL-")?);
    if has_duplicates(&recovered_ids) {
        return Err(contract(format!(
            "alignment row {} repeats a recovered ID",
            source_ids[0]
        )));
    }
    let scene_ids = identifiers(&cells[3], "SCN-")?.into_iter().collect();
    Ok(AlignmentRow {
        source_id: source_ids[0].clone(),
        state,
        recovered_ids,
        scene_ids,
    })
}

fn validate_row(
    source_item: &SourceItem,
    recovered_items: &BTreeMap<String, RecoveredItem>,
    row: &AlignmentRow,
) -> Result<()> {
    let supported = matches!(
        row.state.as_str(),
        "PRESERVED" | "DRAMATIZED" | "COMBINED" | "REORDERED"
    );
    if supported && (row.recovered_ids.is_empty() || row.scene_ids.is_empty()) {
        return Err(contract(format!(
            "{} requires recovered IR and SCN evidence for state {}",
            row.source_id, row.state
        )));
    }
    if row.recovered_ids.is_empty() && !row.scene_ids.is_empty() {
        return Err(contract(format!(
            "{} cites SCN evidence without a recovered IR ID",
            row.source_id
        )));
    }
    if row.state == "DRIFTED" && row.recovered_ids.is_empty() {
        return Err(contract(format!(
            "{} DRIFTED row has no contradictory IR evidence",
            row.source_id
        )));
    }

    let mut cited_scene_union = BTreeSet::new();
    for recovered_id in &row.recovered_ids {
        let recovered = recovered_items.get(recovered_id).ok_or_else(|| {
            contract(format!(
                "{} cites unknown recovered ID {recovered_id}",
                row.source_id
            ))
        })?;
        if recovered.kind != source_item.kind {
            return Err(contract(format!(
                "{} cites wrong-kind recovered ID {recovered_id}",
                row.source_id
            )));
        }
        cited_scene_union.extend(recovered.scene_ids.iter().map(String::as_str));
        if !recovered
            .scene_ids
            .iter()
            .any(|scene_id| row.scene_ids.contains(scene_id))
        {
            return Err(contract(format!(
                "{} scene evidence does not support recovered ID {recovered_id}",
                row.source_id
            )));
        }
    }
    if !row
        .scene_ids
        .iter()
        .all(|scene| cited_scene_union.contains(scene.as_str()))
    {
        return Err(contract(format!(
            "{} cites a scene absent from its recovered IR evidence",
            row.source_id
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE_IR: &str = "## SRC Assertions\n- SRC-001 — 민준은 범인이 아니다.; polarity=NEGATE; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-001\n## REL Relations\n- REL-001 — 진실 확인 BEFORE 출발; relation=BEFORE; materiality=CORE; evidence=SPAN-002\n";
    const SCREENPLAY_IR: &str = "## OBS Assertions\n- OBS-001: 지우가 기록으로 민준의 무죄를 확인한다. evidence=SCN-001\n## OREL Relations\n- OREL-001: 확인 BEFORE 출발. evidence=SCN-001\n";
    const VALID_READY: &str = "# Verification\n| Source ID | State | Recovered Screenplay IR IDs | Screenplay scene evidence | Rationale |\n|---|---|---|---|---|\n| SRC-001 | DRAMATIZED | OBS-001 | SCN-001: 기록 확인 | 핵심 사실을 대본에서 복원한다. |\n| REL-001 | PRESERVED | OREL-001 | SCN-001: 확인 뒤 출발 | 사건 순서를 보존한다. |\n\nSTS_STATUS: READY\n";

    #[test]
    fn complete_alignment_with_bound_ids_and_scenes_is_admitted() {
        assert!(validate_alignment(SOURCE_IR, SCREENPLAY_IR, VALID_READY).is_ok());
    }

    #[test]
    fn incomplete_source_coverage_is_rejected() {
        let report = VALID_READY.replace(
            "| REL-001 | PRESERVED | OREL-001 | SCN-001: 확인 뒤 출발 | 사건 순서를 보존한다. |\n",
            "",
        );
        let error = validate_alignment(SOURCE_IR, SCREENPLAY_IR, &report).unwrap_err();
        assert!(error.to_string().contains("material source ID REL-001"));
    }

    #[test]
    fn unknown_recovered_id_is_rejected() {
        let report = VALID_READY.replace("OBS-001", "OBS-999");
        let error = validate_alignment(SOURCE_IR, SCREENPLAY_IR, &report).unwrap_err();
        assert!(error.to_string().contains("unknown recovered ID OBS-999"));
    }

    #[test]
    fn scene_citation_must_belong_to_the_recovered_item() {
        let report = VALID_READY.replace("SCN-001: 기록 확인", "SCN-999: 기록 확인");
        let error = validate_alignment(SOURCE_IR, SCREENPLAY_IR, &report).unwrap_err();
        assert!(error
            .to_string()
            .contains("scene evidence does not support recovered ID OBS-001"));
    }

    #[test]
    fn ready_cannot_pass_protected_core_drift() {
        let report = VALID_READY.replace("SRC-001 | DRAMATIZED", "SRC-001 | DRIFTED");
        let error = validate_alignment(SOURCE_IR, SCREENPLAY_IR, &report).unwrap_err();
        assert!(error
            .to_string()
            .contains("protected or CORE source drift SRC-001"));
    }
}
