use serde::{Deserialize, Serialize};

/// Bounded, deterministic observations retained for the active 글투.
///
/// This intentionally contains counts and fixed-point ratios only. Imported
/// sample prose is measured in memory and is never part of this structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VoiceMeasurements {
    pub character_count: u64,
    pub word_count: u64,
    pub sentence_count: u64,
    pub paragraph_count: u64,
    pub line_count: u64,
    pub dialogue_line_count: u64,
    pub question_count: u64,
    pub exclamation_count: u64,
    pub comma_count: u64,
    pub ellipsis_count: u64,
    pub hangul_count: u64,
    pub latin_count: u64,
    pub digit_count: u64,
    pub average_sentence_chars_x100: u64,
    pub average_paragraph_chars_x100: u64,
    pub sentences_per_paragraph_x100: u64,
    pub dialogue_ratio_per_mille: u64,
    pub hangul_ratio_per_mille: u64,
    pub latin_ratio_per_mille: u64,
    pub digit_ratio_per_mille: u64,
    pub question_rate_per_mille: u64,
    pub exclamation_rate_per_mille: u64,
    pub comma_rate_per_sentence_x100: u64,
    pub ellipsis_rate_per_sentence_x100: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VoiceProfile {
    pub schema_version: u32,
    pub analyzer_version: String,
    pub profile_id: String,
    pub corpus_digest: String,
    pub sample_count: usize,
    pub corpus_bytes: usize,
    pub measurements: VoiceMeasurements,
    pub summary: String,
    pub guidelines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceComparison {
    pub score: u8,
    pub label: String,
    pub summary: String,
    pub tips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceGuide {
    pub project_id: String,
    pub scene_id: String,
    pub scene_title: String,
    pub source_project_revision: u64,
    pub source_content_revision: u64,
    pub source_scene_revision: u64,
    pub source_scene_digest: String,
    pub external_editor_notice: String,
    pub instruction: String,
}
