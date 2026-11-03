use super::lexicon::{
    LIST_CONNECTOR_TERMS, NEGATION_EMPHASIS_TERMS, NEGATION_SCOPE_BREAK_TERMS, NEGATION_TERMS,
};
use super::matching::{is_ui_token_character, ui_term_spans};

fn non_overlapping_forbidden_spans(value: &str, terms: &[&str]) -> Vec<(usize, usize)> {
    let mut spans = ui_term_spans(value, terms);
    spans.sort_by(|(left_start, left_end), (right_start, right_end)| {
        left_start
            .cmp(right_start)
            .then_with(|| (right_end - right_start).cmp(&(left_end - left_start)))
    });
    let mut selected = Vec::new();
    for span @ (start, end) in spans {
        if selected
            .iter()
            .all(|(selected_start, selected_end)| end <= *selected_start || start >= *selected_end)
        {
            selected.push(span);
        }
    }
    selected.sort_unstable_by_key(|(start, _)| *start);
    selected
}

fn contains_list_connector(value: &str) -> bool {
    !ui_term_spans(value, LIST_CONNECTOR_TERMS).is_empty()
}

fn contains_scope_break(value: &str, terms: &[&str]) -> bool {
    if !ui_term_spans(value, NEGATION_SCOPE_BREAK_TERMS).is_empty() {
        return true;
    }
    // "add" is a scope break after an already-mentioned forbidden item, but
    // remains a normal verb in "do not add readable text".
    ui_term_spans(value, &["add", "adding", "added"])
        .iter()
        .any(|(start, _)| !non_overlapping_forbidden_spans(&value[..*start], terms).is_empty())
}

fn local_negation_start(value: &str) -> Option<(usize, usize)> {
    NEGATION_TERMS
        .iter()
        .flat_map(|negation| {
            value
                .match_indices(negation)
                .map(move |(start, _)| (start, start + negation.len()))
        })
        .max_by_key(|(start, _)| *start)
}

fn starts_with_negation_emphasis(value: &str) -> bool {
    let value = value.trim_start();
    NEGATION_EMPHASIS_TERMS.iter().any(|term| {
        value.strip_prefix(term).is_some_and(|remainder| {
            remainder
                .chars()
                .next()
                .is_none_or(|character| !is_ui_token_character(character))
        })
    })
}

fn starts_with_not_emphasis(value: &str) -> bool {
    let value = value.trim_start();
    ["not", "do not", "don't"].iter().any(|negation| {
        value
            .strip_prefix(negation)
            .is_some_and(starts_with_negation_emphasis)
    })
}

fn term_is_locally_negated(
    segment: &str,
    start: usize,
    end: usize,
    terms: &[&str],
    spans: &[(usize, usize)],
) -> bool {
    let suffix = segment[end..].chars().take(32).collect::<String>();
    if starts_with_not_emphasis(&suffix) {
        return false;
    }
    if NEGATION_TERMS
        .iter()
        .any(|negation| suffix.trim_start().starts_with(negation.trim()))
    {
        return true;
    }
    let Some((_, negation_end)) = local_negation_start(&segment[..start]) else {
        return false;
    };
    let between = &segment[negation_end..start];
    if starts_with_negation_emphasis(between) {
        return false;
    }
    if contains_scope_break(between, terms) {
        return false;
    }
    // "no circles and readable text" is not a list. A connector can inherit
    // negation only after the same segment already named a forbidden item.
    if contains_list_connector(between)
        && !spans
            .iter()
            .any(|(prior_start, prior_end)| *prior_end <= start && *prior_start >= negation_end)
    {
        return false;
    }
    true
}

fn is_forbidden_list_continuation(segment: &str, spans: &[(usize, usize)]) -> bool {
    if spans.is_empty() {
        return false;
    }
    let mut remainder = String::new();
    let mut cursor = 0;
    for (start, end) in spans {
        remainder.push_str(&segment[cursor..*start]);
        cursor = *end;
    }
    remainder.push_str(&segment[cursor..]);
    remainder.split_whitespace().all(|word| {
        let word = word.trim_matches(|character: char| {
            matches!(
                character,
                ',' | '.' | ';' | ':' | '!' | '?' | '"' | '\'' | '(' | ')'
            )
        });
        matches!(
            word,
            "and"
                | "or"
                | "&"
                | "및"
                | "또는"
                | "그리고"
                | "와"
                | "과"
                | "no"
                | "without"
                | "exclude"
                | "avoid"
                | "prohibit"
                | "not"
                | "do"
                | "don't"
                | "없"
                | "없는"
                | "없음"
                | "금지"
                | "제외"
                | "배제"
                | "readable"
        )
    })
}

pub(in crate::image::validate) fn contains_positive_forbidden_term(
    value: &str,
    terms: &[&str],
) -> bool {
    let lower = value.to_lowercase();
    let mut segments = Vec::new();
    let mut start = 0;
    let mut preceded_by_comma = false;
    for (index, character) in lower.char_indices() {
        if matches!(
            character,
            ',' | '，' | ';' | '；' | '.' | '。' | ':' | '!' | '！' | '?' | '？' | '|' | '\n'
        ) {
            segments.push((&lower[start..index], preceded_by_comma));
            start = index + character.len_utf8();
            preceded_by_comma = matches!(character, ',' | '，');
        }
    }
    segments.push((&lower[start..], preceded_by_comma));

    let mut negative_list_active = false;
    for (segment, continued_from_comma) in segments {
        if !continued_from_comma {
            negative_list_active = false;
        }
        let spans = non_overlapping_forbidden_spans(segment, terms);
        let direct_negation = spans
            .iter()
            .map(|(start, end)| term_is_locally_negated(segment, *start, *end, terms, &spans))
            .collect::<Vec<_>>();
        if spans.iter().enumerate().any(|(index, _)| {
            !direct_negation[index]
                && !(negative_list_active && is_forbidden_list_continuation(segment, &spans))
        }) {
            return true;
        }
        let has_scope_break_after_first = spans
            .first()
            .is_some_and(|(_, first_end)| contains_scope_break(&segment[*first_end..], terms));
        negative_list_active = direct_negation.iter().any(|negated| *negated)
            && !has_scope_break_after_first
            || continued_from_comma
                && negative_list_active
                && is_forbidden_list_continuation(segment, &spans);
    }
    false
}
