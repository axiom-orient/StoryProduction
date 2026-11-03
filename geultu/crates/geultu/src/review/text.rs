//! Shared sentence, paragraph, and fixed output metrics for review reports.

use crate::model::StyleMetrics;

pub fn style_metrics(text: &str) -> StyleMetrics {
    let sentences = split_sentences(text);
    let sentence_count = sentences.len();
    let character_count = text.chars().count();
    let sentence_characters: usize = sentences
        .iter()
        .map(|sentence| sentence.chars().count())
        .sum();
    let (line_count, dialogue_lines) = text.lines().filter(|line| !line.trim().is_empty()).fold(
        (0usize, 0usize),
        |(lines, dialogue), line| {
            let value = line.trim_start();
            let is_dialogue = value.starts_with('\"')
                || value.starts_with('“')
                || value.starts_with('‘')
                || value.starts_with('-');
            (lines + 1, dialogue + usize::from(is_dialogue))
        },
    );
    StyleMetrics {
        character_count,
        sentence_count,
        average_sentence_length: if sentence_count == 0 {
            0.0
        } else {
            sentence_characters as f64 / sentence_count as f64
        },
        dialogue_line_ratio: if line_count == 0 {
            0.0
        } else {
            dialogue_lines as f64 / line_count as f64
        },
        paragraph_count: split_paragraphs(text).len(),
    }
}

pub(super) fn split_paragraphs(text: &str) -> Vec<&str> {
    text.split("\n\n")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect()
}

pub(super) fn split_sentences(text: &str) -> Vec<&str> {
    text.split_inclusive(['.', '!', '?', '。', '！', '？', '\n'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect()
}

pub(super) fn normalize_sentence(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|character| !character.is_whitespace() && !character.is_ascii_punctuation())
        .collect()
}

pub(super) fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}
