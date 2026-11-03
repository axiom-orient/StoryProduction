//! STEP 0 source-text traceability and lossless coverage admission.

use super::source_inventory::parse_source_items;
use super::text::{has_duplicates, identifiers, is_span_id, is_table_separator, table_cells};
use narrative_courier_runtime::{Error, Result};
use std::collections::BTreeSet;

const COVERAGE_HEADER: &str = "| Source span | Exact source text | SRC/REL ID | Unmapped reason |";

#[derive(Debug)]
struct CoverageSpan {
    id: String,
    exact_text: String,
    source_ids: Vec<String>,
    unmapped_reason: String,
}

/// Requires the Source IR to expose every non-whitespace synopsis byte as an
/// exact, human-reviewable span and to bind every typed obligation to one span.
/// This is intentionally a source-stage check; STEP 8D remains source-blind.
pub(in crate::engine::workflow) fn validate_step_00_source_coverage(
    synopsis: &str,
    source_ir: &str,
) -> Result<()> {
    if synopsis.trim().is_empty() {
        return Err(source_contract("approved synopsis is empty"));
    }

    let source_items = parse_source_items(source_ir).map_err(source_context)?;
    let spans = parse_coverage_spans(source_ir)?;
    let mut span_ids = BTreeSet::new();
    let mut mapped_source_ids = BTreeSet::new();
    let mut covered_ranges = Vec::with_capacity(spans.len());

    for span in spans {
        if !is_span_id(&span.id) {
            return Err(source_contract(format!(
                "invalid source span identifier {}",
                span.id
            )));
        }
        if span.exact_text.trim().is_empty() {
            return Err(source_contract(format!(
                "source span {} has empty source text",
                span.id
            )));
        }
        if !span_ids.insert(span.id.clone()) {
            return Err(source_contract(format!(
                "source span {} is declared more than once",
                span.id
            )));
        }

        let unmapped_heading = span.source_ids.is_empty()
            && span.unmapped_reason.trim() == "MARKDOWN_HEADING"
            && is_markdown_heading(&span.exact_text);
        if span.source_ids.is_empty() && !unmapped_heading {
            return Err(source_contract(format!(
                "unmapped source span {} must be a Markdown heading marked MARKDOWN_HEADING",
                span.id
            )));
        }
        if !span.source_ids.is_empty() && span.unmapped_reason.trim() != "—" {
            return Err(source_contract(format!(
                "mapped source span {} must use — for unmapped reason",
                span.id
            )));
        }

        let mut matches = synopsis
            .match_indices(&span.exact_text)
            .filter(|(start, _)| {
                !unmapped_heading
                    || is_complete_source_line(synopsis, *start, span.exact_text.len())
            });
        let (start, _) = matches.next().ok_or_else(|| {
            source_contract(format!(
                "source span {} is not an exact synopsis substring or complete unmapped heading line",
                span.id
            ))
        })?;
        if !unmapped_heading && matches.next().is_some() {
            return Err(source_contract(format!(
                "source span {} is ambiguous; quote a longer exact span",
                span.id
            )));
        }
        covered_ranges.push(start..start + span.exact_text.len());
        if unmapped_heading {
            covered_ranges.extend(matches.map(|(start, _)| start..start + span.exact_text.len()));
            continue;
        }

        for source_id in span.source_ids {
            let item = source_items.get(&source_id).ok_or_else(|| {
                source_contract(format!(
                    "source span {} references undeclared obligation {source_id}",
                    span.id
                ))
            })?;
            if item.evidence_span != span.id {
                return Err(source_contract(format!(
                    "{source_id} declares evidence={} but coverage maps it to {}",
                    item.evidence_span, span.id
                )));
            }
            if !mapped_source_ids.insert(source_id.clone()) {
                return Err(source_contract(format!(
                    "source obligation {source_id} is mapped more than once"
                )));
            }
        }
    }

    for (source_id, item) in &source_items {
        if !span_ids.contains(&item.evidence_span) {
            return Err(source_contract(format!(
                "{source_id} references missing source span {}",
                item.evidence_span
            )));
        }
        if !mapped_source_ids.contains(source_id) {
            return Err(source_contract(format!(
                "source obligation {source_id} is absent from the coverage map"
            )));
        }
    }

    covered_ranges.sort_by_key(|range| (range.start, range.end));
    let mut covered_end = 0;
    for range in covered_ranges {
        if range.start > covered_end {
            reject_uncovered_text(synopsis, covered_end, range.start)?;
        }
        covered_end = covered_end.max(range.end);
    }
    reject_uncovered_text(synopsis, covered_end, synopsis.len())?;

    Ok(())
}

fn parse_coverage_spans(source_ir: &str) -> Result<Vec<CoverageSpan>> {
    let mut in_section = false;
    let mut section_seen = false;
    let mut header_seen = false;
    let mut spans = Vec::new();

    for line in source_ir.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            if trimmed == "## Source Coverage Audit" {
                if section_seen {
                    return Err(source_contract("Source Coverage Audit section is repeated"));
                }
                section_seen = true;
                in_section = true;
            } else if in_section {
                in_section = false;
            }
            continue;
        }
        if !in_section || trimmed.is_empty() {
            continue;
        }
        if !header_seen {
            if trimmed != COVERAGE_HEADER {
                return Err(source_contract(
                    "Source Coverage Audit has no canonical table header",
                ));
            }
            header_seen = true;
            continue;
        }
        if !trimmed.starts_with('|') {
            return Err(source_contract(
                "Source Coverage Audit contains text outside its table",
            ));
        }

        let cells = table_cells(trimmed).map_err(source_context)?;
        if cells.len() != 4 {
            return Err(source_contract(
                "coverage rows must have exactly four columns",
            ));
        }
        if is_table_separator(&cells) {
            continue;
        }
        let mut source_ids = identifiers(&cells[2], "SRC-").map_err(source_context)?;
        source_ids.extend(identifiers(&cells[2], "REL-").map_err(source_context)?);
        if has_duplicates(&source_ids) {
            return Err(source_contract(format!(
                "coverage row {} repeats a source obligation",
                cells[0]
            )));
        }
        if source_ids.len() > 1 {
            return Err(source_contract(format!(
                "coverage span {} must map one atomic SRC/REL obligation",
                cells[0]
            )));
        }
        spans.push(CoverageSpan {
            id: cells[0].trim().to_ascii_uppercase(),
            exact_text: cells[1].clone(),
            source_ids,
            unmapped_reason: cells[3].clone(),
        });
    }

    if !section_seen || !header_seen || spans.is_empty() {
        return Err(source_contract(
            "Source Coverage Audit must contain a header and at least one source span",
        ));
    }
    Ok(spans)
}

fn is_complete_source_line(source: &str, start: usize, length: usize) -> bool {
    let bytes = source.as_bytes();
    let end = start + length;
    (start == 0 || bytes[start - 1] == b'\n')
        && (end == bytes.len()
            || bytes[end] == b'\n'
            || (bytes[end] == b'\r' && bytes.get(end + 1) == Some(&b'\n')))
}

fn reject_uncovered_text(source: &str, start: usize, end: usize) -> Result<()> {
    if let Some(offset) = source.as_bytes()[start..end]
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
    {
        return Err(source_contract(format!(
            "synopsis contains uncovered source text at byte {}",
            start + offset
        )));
    }
    Ok(())
}

fn source_contract(message: impl Into<String>) -> Error {
    Error::Contract(format!("STEP 0 source coverage: {}", message.into()))
}

fn is_markdown_heading(value: &str) -> bool {
    let level = value
        .chars()
        .take_while(|character| *character == '#')
        .count();
    (1..=6).contains(&level) && value.chars().nth(level) == Some(' ')
}

fn source_context(error: Error) -> Error {
    match error {
        Error::Contract(message) => source_contract(message),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYNOPSIS: &str = "어머니는 민준이 불을 냈다고 믿는다.\n화재 원인은 밝혀지지 않았다.\n";
    const SOURCE_IR: &str = concat!(
        "## SRC Assertions\n",
        "SRC-001 — 어머니는 민준이 불을 냈다고 믿는다.; polarity=AFFIRM; epistemic=BELIEF; materiality=CORE; protected=yes; evidence=SPAN-001\n",
        "SRC-002 — 화재 원인은 밝혀지지 않았다.; polarity=AFFIRM; epistemic=UNKNOWN; materiality=CORE; protected=yes; evidence=SPAN-002\n",
        "## Source Coverage Audit\n",
        "| Source span | Exact source text | SRC/REL ID | Unmapped reason |\n",
        "|---|---|---|---|\n",
        "| SPAN-001 | 어머니는 민준이 불을 냈다고 믿는다. | SRC-001 | — |\n",
        "| SPAN-002 | 화재 원인은 밝혀지지 않았다. | SRC-002 | — |\n"
    );

    #[test]
    fn exact_source_spans_cover_and_bind_every_obligation() {
        assert!(validate_step_00_source_coverage(SYNOPSIS, SOURCE_IR).is_ok());
    }

    #[test]
    fn markdown_headings_are_the_only_unmapped_source_spans() {
        let source = "# Story\n\n민준은 범인이 아니다.\n";
        let source_ir = concat!(
            "## SRC Assertions\n",
            "SRC-001 — 민준은 범인이 아니다.; polarity=NEGATE; epistemic=FACT; materiality=CORE; protected=yes; evidence=SPAN-002\n",
            "## Source Coverage Audit\n",
            "| Source span | Exact source text | SRC/REL ID | Unmapped reason |\n",
            "|---|---|---|---|\n",
            "| SPAN-001 | # Story | — | MARKDOWN_HEADING |\n",
            "| SPAN-002 | 민준은 범인이 아니다. | SRC-001 | — |\n"
        );
        assert!(validate_step_00_source_coverage(source, source_ir).is_ok());
    }

    #[test]
    fn uncovered_source_text_is_rejected() {
        let source = format!("{SYNOPSIS}지우는 역을 잠그고 떠난다.\n");
        let error = validate_step_00_source_coverage(&source, SOURCE_IR).unwrap_err();
        assert!(error.to_string().contains("uncovered source text"));
    }

    #[test]
    fn narrative_source_text_cannot_be_hidden_as_unmapped_formatting() {
        let source_ir = SOURCE_IR.replace(
            "| SPAN-001 | 어머니는 민준이 불을 냈다고 믿는다. | SRC-001 | — |",
            "| SPAN-001 | 어머니는 민준이 불을 냈다고 믿는다. | — | MARKDOWN_HEADING |",
        );
        let error = validate_step_00_source_coverage(SYNOPSIS, &source_ir).unwrap_err();
        assert!(error.to_string().contains("must be a Markdown heading"));
    }

    #[test]
    fn unsupported_or_mismatched_coverage_ids_are_rejected() {
        let undeclared = SOURCE_IR.replace("SRC-002 | —", "SRC-999 | —");
        let error = validate_step_00_source_coverage(SYNOPSIS, &undeclared).unwrap_err();
        assert!(error.to_string().contains("undeclared obligation SRC-999"));

        let mismatched = SOURCE_IR.replace(
            "| SPAN-002 | 화재 원인은 밝혀지지 않았다. | SRC-002 | — |",
            "| SPAN-003 | 화재 원인은 밝혀지지 않았다. | SRC-002 | — |",
        );
        let error = validate_step_00_source_coverage(SYNOPSIS, &mismatched).unwrap_err();
        assert!(error.to_string().contains("declares evidence=SPAN-002"));
    }
}
