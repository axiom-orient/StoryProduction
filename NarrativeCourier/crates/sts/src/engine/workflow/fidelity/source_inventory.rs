//! Typed SRC/REL inventory parsing for source-fidelity admission.

use super::text::{assignment, contract, is_span_id, leading_id};
use narrative_courier_runtime::Result;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ItemKind {
    Assertion,
    Relation,
}

impl ItemKind {
    fn source_prefix(self) -> &'static str {
        match self {
            Self::Assertion => "SRC-",
            Self::Relation => "REL-",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Materiality {
    Core,
    Support,
    Texture,
}

#[derive(Clone, Debug)]
pub(super) struct SourceItem {
    pub(super) kind: ItemKind,
    materiality: Materiality,
    protected: bool,
    pub(super) evidence_span: String,
}

impl SourceItem {
    pub(super) fn must_be_aligned(&self) -> bool {
        self.materiality != Materiality::Texture || self.protected
    }

    pub(super) fn omission_blocks_ready(&self) -> bool {
        self.materiality == Materiality::Core || self.protected
    }
}

pub(super) fn parse_source_items(source_ir: &str) -> Result<BTreeMap<String, SourceItem>> {
    let mut section = None;
    let mut items = BTreeMap::new();
    for line in source_ir.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            section = match trimmed {
                "## SRC Assertions" => Some(ItemKind::Assertion),
                "## REL Relations" => Some(ItemKind::Relation),
                _ => None,
            };
            continue;
        }
        let Some(kind) = section else { continue };
        let Some(id) = leading_id(trimmed, kind.source_prefix())? else {
            continue;
        };
        let evidence_span = assignment(line, "evidence")?
            .ok_or_else(|| contract(format!("source item {id} has no evidence span")))?;
        if !is_span_id(&evidence_span) {
            return Err(contract(format!(
                "source item {id} has invalid evidence span {evidence_span}"
            )));
        }
        let materiality = assignment(line, "materiality")?
            .ok_or_else(|| contract(format!("source item {id} has no materiality")))?;
        let materiality = parse_materiality(&materiality)?;
        let protected = if kind == ItemKind::Assertion {
            let polarity = assignment(line, "polarity")?
                .ok_or_else(|| contract(format!("source assertion {id} has no polarity")))?;
            if !matches!(polarity.as_str(), "AFFIRM" | "NEGATE") {
                return Err(contract(format!(
                    "source assertion {id} has unknown polarity {polarity}"
                )));
            }
            let epistemic = assignment(line, "epistemic")?
                .ok_or_else(|| contract(format!("source assertion {id} has no epistemic kind")))?;
            if !matches!(epistemic.as_str(), "FACT" | "BELIEF" | "CLAIM" | "UNKNOWN") {
                return Err(contract(format!(
                    "source assertion {id} has unknown epistemic kind {epistemic}"
                )));
            }
            let protected = assignment(line, "protected")?
                .ok_or_else(|| contract(format!("source assertion {id} has no protected flag")))?;
            parse_protected(&protected)?
        } else {
            let relation = assignment(line, "relation")?
                .ok_or_else(|| contract(format!("source relation {id} has no relation kind")))?;
            if !matches!(
                relation.as_str(),
                "CAUSES"
                    | "ENABLES"
                    | "PREVENTS"
                    | "BEFORE"
                    | "AFTER"
                    | "KNOWS"
                    | "BELIEVES"
                    | "IDENTITY"
                    | "RELATIONSHIP"
                    | "WORLD_RULE"
            ) {
                return Err(contract(format!(
                    "source relation {id} has unknown relation kind {relation}"
                )));
            }
            false
        };
        if items
            .insert(
                id.clone(),
                SourceItem {
                    kind,
                    materiality,
                    protected,
                    evidence_span,
                },
            )
            .is_some()
        {
            return Err(contract(format!("Source IR repeats {id}")));
        }
    }
    if !items.keys().any(|id| id.starts_with("SRC-")) {
        return Err(contract("Source IR has no SRC assertion declarations"));
    }
    Ok(items)
}

fn parse_materiality(value: &str) -> Result<Materiality> {
    match value {
        "CORE" => Ok(Materiality::Core),
        "SUPPORT" => Ok(Materiality::Support),
        "TEXTURE" => Ok(Materiality::Texture),
        _ => Err(contract(format!("unknown source materiality {value}"))),
    }
}

fn parse_protected(value: &str) -> Result<bool> {
    match value {
        "YES" | "TRUE" => Ok(true),
        "NO" | "FALSE" => Ok(false),
        _ => Err(contract(format!("unknown protected flag {value}"))),
    }
}
