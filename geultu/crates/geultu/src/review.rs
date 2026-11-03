//! Deterministic project review and source-to-candidate proof reports.

mod diagnostics;
mod proof;
mod text;

pub use diagnostics::review_project;
pub use proof::verify_proof;
pub use text::style_metrics;

use crate::model::{Finding, Severity};

pub(super) fn finding(
    code: impl Into<String>,
    severity: Severity,
    message: impl Into<String>,
    scene_id: Option<String>,
    evidence: Option<String>,
) -> Finding {
    Finding {
        code: code.into(),
        severity,
        message: message.into(),
        scene_id,
        evidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Fact, Project};

    #[test]
    fn conflicting_facts_are_errors() {
        let mut project = Project::empty("book".into(), "Book".into());
        project.facts.insert(
            "a".into(),
            Fact {
                id: "a".into(),
                subject: "민수".into(),
                predicate: "눈색".into(),
                object: "갈색".into(),
                evidence: Vec::new(),
            },
        );
        project.facts.insert(
            "b".into(),
            Fact {
                id: "b".into(),
                subject: "민수".into(),
                predicate: "눈색".into(),
                object: "파랑".into(),
                evidence: Vec::new(),
            },
        );
        let report = review_project(&project);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "continuity.fact_conflict"
                && finding.severity == Severity::Error));
    }

    #[test]
    fn proof_checks_protected_terms() {
        let report = verify_proof("Axiom은 정본이다.", "정본이다.", &[], &["Axiom".into()]);
        assert!(!report.protected_terms_preserved);
    }
}
