//! Lexical fidelity, grounding, repetition, and protected-term checks.

use super::finding;
use super::text::{normalize_sentence, split_sentences};
use crate::model::{ProofReport, Severity};
use std::collections::BTreeSet;

pub fn verify_proof(
    source: &str,
    candidate: &str,
    evidence: &[String],
    protected_terms: &[String],
) -> ProofReport {
    let source_tokens = token_set(source);
    let candidate_tokens = token_set(candidate);
    let evidence_tokens = token_set(&evidence.join("\n"));
    let intersection = source_tokens.intersection(&candidate_tokens).count();
    let union = source_tokens.union(&candidate_tokens).count();
    let fidelity = if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    };

    let supported = candidate_tokens
        .iter()
        .filter(|token| source_tokens.contains(*token) || evidence_tokens.contains(*token))
        .count();
    let grounding = if candidate_tokens.is_empty() {
        1.0
    } else {
        supported as f64 / candidate_tokens.len() as f64
    };

    let sentences = split_sentences(candidate);
    let unique: BTreeSet<_> = sentences
        .iter()
        .map(|sentence| normalize_sentence(sentence))
        .collect();
    let repetition_ratio = if sentences.is_empty() {
        0.0
    } else {
        1.0 - unique.len() as f64 / sentences.len() as f64
    };
    let naturalness = (1.0 - repetition_ratio).clamp(0.0, 1.0);

    let missing_terms: Vec<_> = protected_terms
        .iter()
        .filter(|term| !term.is_empty() && !candidate.contains(term.as_str()))
        .map(String::as_str)
        .collect();
    let mut findings = Vec::new();
    if !missing_terms.is_empty() {
        findings.push(finding(
            "proof.protected_term_missing",
            Severity::Error,
            "Candidate removed protected terms",
            None,
            Some(missing_terms.join(", ")),
        ));
    }
    if fidelity < 0.25 {
        findings.push(finding(
            "proof.low_fidelity",
            Severity::Warning,
            "Candidate has low lexical overlap with the source",
            None,
            Some(format!("score={fidelity:.3}")),
        ));
    }
    if grounding < 0.55 {
        findings.push(finding(
            "proof.low_grounding",
            Severity::Warning,
            "Large portions of the candidate are unsupported by source or evidence",
            None,
            Some(format!("score={grounding:.3}")),
        ));
    }
    if naturalness < 0.8 {
        findings.push(finding(
            "proof.repetition",
            Severity::Warning,
            "Candidate repeats identical sentences",
            None,
            Some(format!("score={naturalness:.3}")),
        ));
    }
    ProofReport {
        fidelity,
        grounding,
        naturalness,
        protected_terms_preserved: missing_terms.is_empty(),
        findings,
    }
}

fn token_set(text: &str) -> BTreeSet<String> {
    text.split(|character: char| character.is_whitespace() || character.is_ascii_punctuation())
        .map(|value| value.trim().to_lowercase())
        .filter(|value| value.chars().count() >= 2)
        .collect()
}
