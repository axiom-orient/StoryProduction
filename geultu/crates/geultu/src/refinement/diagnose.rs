//! Deterministic diagnosis engine for the H5 refinement flow.
//!
//! The rules are engine-owned, span-based Korean-text checks: they scan the
//! diagnosed document only, never call external services, and never sample
//! randomness. [`RefinementFinding::replacement`] is populated exclusively
//! by pure collapse/normalize functions applied to characters scanned out
//! of the document itself (e.g. `"!!!" → "!"`), which is why no code path
//! here can emit translated text.

use serde::{Deserialize, Serialize};

/// One deterministic diagnosis finding over scene text.
///
/// `span_start` / `span_end` are character offsets into the scene text
/// (Korean-safe; never byte offsets). `replacement` is `Some` only when a
/// meaning-preserving mechanical rewrite of the matched span exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementFinding {
    pub code: String,
    pub severity: crate::model::Severity,
    pub message: String,
    pub scene_id: String,
    pub span_start: usize,
    pub span_end: usize,
    pub found: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

/// Diagnoses one text with the deterministic rule set, returning findings
/// ordered by `(span_start, span_end, code)` with overlapping matches
/// dropped (first by position wins) so proposals stay stable.
#[must_use]
pub fn diagnose_text(scene_id: &str, text: &str) -> Vec<RefinementFinding> {
    let chars: Vec<char> = text.chars().collect();
    let mut candidates = whitespace_runs(scene_id, &chars);
    candidates.extend(punctuation_stacks(scene_id, &chars));
    candidates.extend(ellipsis_runs(scene_id, &chars));
    candidates.extend(repeated_hangul_words(scene_id, &chars));
    candidates.sort_by(|left, right| {
        (left.span_start, left.span_end, &left.code).cmp(&(
            right.span_start,
            right.span_end,
            &right.code,
        ))
    });
    let mut accepted: Vec<RefinementFinding> = Vec::new();
    for finding in candidates {
        // Positive spans are sorted by start; accepted spans never overlap.
        let overlaps = accepted
            .last()
            .is_some_and(|kept| finding.span_start < kept.span_end);
        if !overlaps {
            accepted.push(finding);
        }
    }
    accepted
}

fn whitespace_runs(scene_id: &str, chars: &[char]) -> Vec<RefinementFinding> {
    collect_runs(chars, |character| character == ' ' || character == '\t', 2)
        .into_iter()
        .map(|(start, end)| RefinementFinding {
            code: "WHITESPACE_RUN".into(),
            severity: crate::model::Severity::Warning,
            message: "연속된 공백은 한 칸으로 정리할 수 있습니다.".into(),
            scene_id: scene_id.into(),
            span_start: start,
            span_end: end,
            found: chars[start..end].iter().collect(),
            replacement: Some(" ".into()),
        })
        .collect()
}

fn punctuation_stacks(scene_id: &str, chars: &[char]) -> Vec<RefinementFinding> {
    let mut findings = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let character = chars[index];
        if character != '!' && character != '?' {
            index += 1;
            continue;
        }
        let mut end = index + 1;
        while end < chars.len() && chars[end] == character {
            end += 1;
        }
        if end - index >= 2 {
            findings.push(RefinementFinding {
                code: "PUNCTUATION_STACK".into(),
                severity: crate::model::Severity::Warning,
                message: "같은 문장 부호의 반복은 하나로 정리할 수 있습니다.".into(),
                scene_id: scene_id.into(),
                span_start: index,
                span_end: end,
                found: chars[index..end].iter().collect(),
                replacement: Some(character.to_string()),
            });
        }
        index = end;
    }
    findings
}

fn ellipsis_runs(scene_id: &str, chars: &[char]) -> Vec<RefinementFinding> {
    collect_runs(chars, |character| character == '.', 4)
        .into_iter()
        .map(|(start, end)| RefinementFinding {
            code: "ELLIPSIS_RUN".into(),
            severity: crate::model::Severity::Info,
            message: "4개 이상의 마침표는 표준 말줄임표 세 개로 정리할 수 있습니다.".into(),
            scene_id: scene_id.into(),
            span_start: start,
            span_end: end,
            found: chars[start..end].iter().collect(),
            replacement: Some("...".into()),
        })
        .collect()
}

/// Adjacent duplicated Hangul words ("그리고 그리고") are reported without a
/// replacement: collapsing them could change meaning, so the decision is
/// left to the human approver.
fn repeated_hangul_words(scene_id: &str, chars: &[char]) -> Vec<RefinementFinding> {
    let mut findings = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if !chars[index].is_whitespace() {
            let start = index;
            while index < chars.len() && !chars[index].is_whitespace() {
                index += 1;
            }
            let word = &chars[start..index];
            let gap_start = index;
            while index < chars.len() && chars[index] == ' ' {
                index += 1;
            }
            let single_space = index > gap_start && index - gap_start == 1;
            if single_space && index < chars.len() {
                let next_start = index;
                while index < chars.len() && !chars[index].is_whitespace() {
                    index += 1;
                }
                let next = &chars[next_start..index];
                if word.len() >= 2 && word == next && word.iter().all(is_hangul_syllable) {
                    findings.push(RefinementFinding {
                        code: "REPEATED_WORD".into(),
                        severity: crate::model::Severity::Warning,
                        message: "같은 한글 어절이 연달아 반복됩니다; 사람 확인이 필요합니다."
                            .into(),
                        scene_id: scene_id.into(),
                        span_start: start,
                        span_end: index,
                        found: chars[start..index].iter().collect(),
                        replacement: None,
                    });
                }
                // The next word must be examined even if this pair did not match.
                index = next_start;
            }
        } else {
            index += 1;
        }
    }
    findings
}

fn is_hangul_syllable(character: &char) -> bool {
    ('\u{AC00}'..='\u{D7A3}').contains(character)
}

/// Maximal runs of `predicate` characters of length at least `min`,
/// returned as char-index `(start, end)` pairs, scanning left to right.
fn collect_runs(
    chars: &[char],
    predicate: impl Fn(char) -> bool,
    min: usize,
) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if predicate(chars[index]) {
            let start = index;
            while index < chars.len() && predicate(chars[index]) {
                index += 1;
            }
            if index - start >= min {
                runs.push((start, index));
            }
        } else {
            index += 1;
        }
    }
    runs
}

/// Applies every mechanically safe replacement right-to-left so earlier
/// offsets stay valid; diagnose-only findings are skipped.
#[must_use]
pub(super) fn apply_replacements(text: &str, findings: &[RefinementFinding]) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let mut patchable: Vec<_> = findings
        .iter()
        .filter_map(|finding| {
            finding
                .replacement
                .as_ref()
                .map(|replacement| (finding.span_start, finding.span_end, replacement))
        })
        .collect();
    patchable.sort_by_key(|(start, end, _)| (*start, *end));
    for (start, end, replacement) in patchable.into_iter().rev() {
        chars.splice(start..end, replacement.chars());
    }
    chars.into_iter().collect()
}
