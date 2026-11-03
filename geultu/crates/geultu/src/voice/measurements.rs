//! Bounded text feature extraction and fixed-point aggregation.

use crate::error::{Error, Result};
use crate::model::{VoiceMeasurements, MAX_TEXT_BYTES};

pub(super) fn measure_text(text: &str) -> Result<VoiceMeasurements> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(Error::InvalidInput(format!(
            "voice comparison text exceeds {MAX_TEXT_BYTES} bytes"
        )));
    }
    if text.trim().is_empty() {
        return Err(Error::InvalidInput(
            "voice comparison text must not be empty".into(),
        ));
    }

    let mut measurements = VoiceMeasurements {
        character_count: 0,
        word_count: text.split_whitespace().count() as u64,
        sentence_count: 0,
        paragraph_count: text
            .split("\n\n")
            .filter(|paragraph| !paragraph.trim().is_empty())
            .count() as u64,
        line_count: 0,
        dialogue_line_count: 0,
        question_count: 0,
        exclamation_count: 0,
        comma_count: 0,
        ellipsis_count: 0,
        hangul_count: 0,
        latin_count: 0,
        digit_count: 0,
        ..VoiceMeasurements::default()
    };
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        measurements.line_count = measurements.line_count.saturating_add(1);
        let trimmed = line.trim_start();
        if trimmed.starts_with('\"')
            || trimmed.starts_with('“')
            || trimmed.starts_with('‘')
            || trimmed.starts_with('「')
            || trimmed.starts_with('『')
            || trimmed.starts_with('-')
        {
            measurements.dialogue_line_count = measurements.dialogue_line_count.saturating_add(1);
        }
    }
    for character in text.chars() {
        measurements.character_count = measurements.character_count.saturating_add(1);
        match character {
            '.' | '。' | '!' | '！' | '?' | '？' => {
                measurements.sentence_count = measurements.sentence_count.saturating_add(1)
            }
            ',' | '，' => measurements.comma_count = measurements.comma_count.saturating_add(1),
            _ => {}
        }
        if character == '?' || character == '？' {
            measurements.question_count = measurements.question_count.saturating_add(1);
        }
        if character == '!' || character == '！' {
            measurements.exclamation_count = measurements.exclamation_count.saturating_add(1);
        }
        if ('가'..='힣').contains(&character) {
            measurements.hangul_count = measurements.hangul_count.saturating_add(1);
        } else if character.is_ascii_alphabetic() {
            measurements.latin_count = measurements.latin_count.saturating_add(1);
        } else if character.is_ascii_digit() {
            measurements.digit_count = measurements.digit_count.saturating_add(1);
        }
    }
    measurements.ellipsis_count =
        text.matches("...").count() as u64 + text.matches('…').count() as u64;
    if measurements.sentence_count == 0 {
        measurements.sentence_count = 1;
    }
    if measurements.paragraph_count == 0 {
        measurements.paragraph_count = 1;
    }
    if measurements.line_count == 0 {
        measurements.line_count = 1;
    }
    measurements.recalculate();
    Ok(measurements)
}

pub(super) fn validate_measurements(
    measurements: &VoiceMeasurements,
    corpus_bytes: usize,
) -> Result<()> {
    if measurements.character_count == 0
        || measurements.word_count == 0
        || measurements.sentence_count == 0
        || measurements.paragraph_count == 0
        || measurements.line_count == 0
        || measurements.character_count > corpus_bytes as u64
    {
        return Err(Error::InvalidInput(
            "voice measurements are inconsistent with corpus bounds".into(),
        ));
    }
    if [
        measurements.word_count,
        measurements.sentence_count,
        measurements.paragraph_count,
        measurements.line_count,
        measurements.question_count,
        measurements.exclamation_count,
        measurements.comma_count,
        measurements.ellipsis_count,
        measurements.hangul_count,
        measurements.latin_count,
        measurements.digit_count,
    ]
    .iter()
    .any(|count| *count > measurements.character_count)
        || measurements.dialogue_line_count > measurements.line_count
        || measurements
            .hangul_count
            .saturating_add(measurements.latin_count)
            .saturating_add(measurements.digit_count)
            > measurements.character_count
        || measurements
            .question_count
            .saturating_add(measurements.exclamation_count)
            > measurements.sentence_count
    {
        return Err(Error::InvalidInput(
            "voice measurement counts exceed their observed character/line bounds".into(),
        ));
    }
    if [
        measurements.dialogue_ratio_per_mille,
        measurements.hangul_ratio_per_mille,
        measurements.latin_ratio_per_mille,
        measurements.digit_ratio_per_mille,
        measurements.question_rate_per_mille,
        measurements.exclamation_rate_per_mille,
    ]
    .iter()
    .any(|value| *value > 1_000)
    {
        return Err(Error::InvalidInput(
            "voice measurement ratios must not exceed 1000 per mille".into(),
        ));
    }
    let mut derived = measurements.clone();
    derived.recalculate();
    if derived != *measurements {
        return Err(Error::InvalidInput(
            "voice measurement derived values do not match their counts".into(),
        ));
    }
    Ok(())
}

impl VoiceMeasurements {
    pub(super) fn add(&mut self, other: &Self) {
        self.character_count = self.character_count.saturating_add(other.character_count);
        self.word_count = self.word_count.saturating_add(other.word_count);
        self.sentence_count = self.sentence_count.saturating_add(other.sentence_count);
        self.paragraph_count = self.paragraph_count.saturating_add(other.paragraph_count);
        self.line_count = self.line_count.saturating_add(other.line_count);
        self.dialogue_line_count = self
            .dialogue_line_count
            .saturating_add(other.dialogue_line_count);
        self.question_count = self.question_count.saturating_add(other.question_count);
        self.exclamation_count = self
            .exclamation_count
            .saturating_add(other.exclamation_count);
        self.comma_count = self.comma_count.saturating_add(other.comma_count);
        self.ellipsis_count = self.ellipsis_count.saturating_add(other.ellipsis_count);
        self.hangul_count = self.hangul_count.saturating_add(other.hangul_count);
        self.latin_count = self.latin_count.saturating_add(other.latin_count);
        self.digit_count = self.digit_count.saturating_add(other.digit_count);
    }

    pub(super) fn recalculate(&mut self) {
        self.average_sentence_chars_x100 = ratio(self.character_count, self.sentence_count, 100);
        self.average_paragraph_chars_x100 = ratio(self.character_count, self.paragraph_count, 100);
        self.sentences_per_paragraph_x100 = ratio(self.sentence_count, self.paragraph_count, 100);
        let line_count = self.line_count.max(1);
        self.dialogue_ratio_per_mille = ratio(self.dialogue_line_count, line_count, 1_000);
        self.hangul_ratio_per_mille = ratio(self.hangul_count, self.character_count, 1_000);
        self.latin_ratio_per_mille = ratio(self.latin_count, self.character_count, 1_000);
        self.digit_ratio_per_mille = ratio(self.digit_count, self.character_count, 1_000);
        self.question_rate_per_mille = ratio(self.question_count, self.sentence_count, 1_000);
        self.exclamation_rate_per_mille = ratio(self.exclamation_count, self.sentence_count, 1_000);
        self.comma_rate_per_sentence_x100 = ratio(self.comma_count, self.sentence_count, 100);
        self.ellipsis_rate_per_sentence_x100 = ratio(self.ellipsis_count, self.sentence_count, 100);
    }
}

fn ratio(numerator: u64, denominator: u64, scale: u64) -> u64 {
    numerator
        .saturating_mul(scale)
        .checked_div(denominator)
        .unwrap_or(0)
}
