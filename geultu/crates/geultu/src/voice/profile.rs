//! Request-local profile construction and stored-profile invariants.

use super::measurements::{measure_text, validate_measurements};
use super::{
    VoiceSample, MAX_VOICE_CORPUS_BYTES, MAX_VOICE_SAMPLES, MAX_VOICE_SAMPLE_BYTES,
    VOICE_ANALYZER_VERSION,
};
use crate::contracts::validate::all_hex;
use crate::domain::validation::validate_id;
use crate::error::{Error, Result};
use crate::model::{VoiceMeasurements, VoiceProfile, VOICE_PROFILE_SCHEMA_VERSION};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Builds a bounded profile from request-local samples.
pub fn build_profile(profile_id: &str, samples: &[VoiceSample]) -> Result<VoiceProfile> {
    validate_id("voice profile id", profile_id)?;
    if samples.is_empty() || samples.len() > MAX_VOICE_SAMPLES {
        return Err(Error::InvalidInput(format!(
            "voice samples must contain 1..={MAX_VOICE_SAMPLES} items"
        )));
    }

    let mut sample_ids = BTreeSet::new();
    let mut corpus_bytes = 0usize;
    let mut measurements = VoiceMeasurements::default();
    let mut digest = Sha256::new();
    digest.update(b"geultu-voice-corpus-v1\0");

    for sample in samples {
        validate_id("voice sample id", &sample.id)?;
        if !sample_ids.insert(sample.id.as_str()) {
            return Err(Error::InvalidInput(format!(
                "voice sample '{}' is duplicated",
                sample.id
            )));
        }
        if sample.text.trim().is_empty() {
            return Err(Error::InvalidInput(format!(
                "voice sample '{}' must not be empty",
                sample.id
            )));
        }
        if sample.text.len() > MAX_VOICE_SAMPLE_BYTES {
            return Err(Error::InvalidInput(format!(
                "voice sample '{}' exceeds {MAX_VOICE_SAMPLE_BYTES} bytes",
                sample.id
            )));
        }
        corpus_bytes = corpus_bytes
            .checked_add(sample.text.len())
            .ok_or_else(|| Error::InvalidInput("voice sample corpus size overflowed".into()))?;
        if corpus_bytes > MAX_VOICE_CORPUS_BYTES {
            return Err(Error::InvalidInput(format!(
                "voice sample corpus exceeds {MAX_VOICE_CORPUS_BYTES} bytes"
            )));
        }

        // Length-prefixing makes the identity independent of concatenation
        // ambiguity while keeping the source text request-local.
        digest.update((sample.id.len() as u64).to_le_bytes());
        digest.update(sample.id.as_bytes());
        digest.update((sample.text.len() as u64).to_le_bytes());
        digest.update(sample.text.as_bytes());
        measurements.add(&measure_text(&sample.text)?);
    }
    measurements.recalculate();

    let profile = VoiceProfile {
        schema_version: VOICE_PROFILE_SCHEMA_VERSION,
        analyzer_version: VOICE_ANALYZER_VERSION.into(),
        profile_id: profile_id.into(),
        corpus_digest: format!("{:x}", digest.finalize()),
        sample_count: samples.len(),
        corpus_bytes,
        summary: build_summary(samples.len(), corpus_bytes, &measurements),
        guidelines: build_guidelines(&measurements),
        measurements,
    };
    validate_profile(&profile)?;
    Ok(profile)
}

/// Validates a stored profile without requiring any source prose.
pub fn validate_profile(profile: &VoiceProfile) -> Result<()> {
    if profile.schema_version != VOICE_PROFILE_SCHEMA_VERSION {
        return Err(Error::InvalidInput(format!(
            "unsupported voice profile schema {}",
            profile.schema_version
        )));
    }
    if profile.analyzer_version.is_empty() || profile.analyzer_version.len() > 128 {
        return Err(Error::InvalidInput(
            "voice analyzer version is empty or too long".into(),
        ));
    }
    validate_id("voice profile id", &profile.profile_id)?;
    if profile.corpus_digest.len() != 64 || !all_hex(&profile.corpus_digest) {
        return Err(Error::InvalidInput(
            "voice corpus digest must be a 64-character hexadecimal value".into(),
        ));
    }
    if profile.sample_count == 0 || profile.sample_count > MAX_VOICE_SAMPLES {
        return Err(Error::InvalidInput(format!(
            "voice sample count must be 1..={MAX_VOICE_SAMPLES}"
        )));
    }
    if profile.corpus_bytes == 0 || profile.corpus_bytes > MAX_VOICE_CORPUS_BYTES {
        return Err(Error::InvalidInput(format!(
            "voice corpus must be 1..={MAX_VOICE_CORPUS_BYTES} bytes"
        )));
    }
    if profile.summary.trim().is_empty() || profile.summary.len() > 2_048 {
        return Err(Error::InvalidInput(
            "voice summary must contain 1..=2048 bytes".into(),
        ));
    }
    if profile.guidelines.is_empty() || profile.guidelines.len() > 16 {
        return Err(Error::InvalidInput(
            "voice guidelines must contain 1..=16 items".into(),
        ));
    }
    if profile
        .guidelines
        .iter()
        .any(|guideline| guideline.trim().is_empty() || guideline.len() > 512)
    {
        return Err(Error::InvalidInput(
            "voice guideline is empty or too long".into(),
        ));
    }
    validate_measurements(&profile.measurements, profile.corpus_bytes)
}

fn build_summary(
    sample_count: usize,
    corpus_bytes: usize,
    measurements: &VoiceMeasurements,
) -> String {
    let sentence_shape = if measurements.average_sentence_chars_x100 < 1_800 {
        "짧은 문장 호흡"
    } else if measurements.average_sentence_chars_x100 > 3_200 {
        "긴 문장 호흡"
    } else {
        "중간 길이의 문장 호흡"
    };
    let paragraph_shape = if measurements.average_paragraph_chars_x100 < 450 {
        "짧은 문단"
    } else if measurements.average_paragraph_chars_x100 > 1_200 {
        "긴 문단"
    } else {
        "중간 길이의 문단"
    };
    let dialogue_shape = if measurements.dialogue_ratio_per_mille >= 250 {
        "대화가 자주 섞인"
    } else {
        "서술 중심의"
    };
    format!(
        "{sample_count}개 샘플({corpus_bytes}바이트)을 관찰했습니다. {sentence_shape}, {paragraph_shape}, {dialogue_shape} 글투의 방향이 보입니다."
    )
}

fn build_guidelines(measurements: &VoiceMeasurements) -> Vec<String> {
    vec![
        format!(
            "문장 길이는 평균 약 {}자 안팎의 호흡을 참고합니다.",
            measurements.average_sentence_chars_x100 / 100
        ),
        format!(
            "문단은 평균 약 {}자 안팎으로 묶어 읽는 흐름을 유지합니다.",
            measurements.average_paragraph_chars_x100 / 100
        ),
        if measurements.dialogue_ratio_per_mille >= 250 {
            "대화와 서술이 함께 흐르도록 대사의 줄바꿈과 간격을 살립니다.".into()
        } else {
            "서술의 흐름을 우선하고 대사는 필요한 곳에서만 또렷하게 구분합니다.".into()
        },
        format!(
            "쉼표·물음표·감탄표·말줄임표의 사용 빈도는 관찰값을 참고하되 과하게 늘리지 않습니다(쉼표 약 {}회/문장).",
            measurements.comma_rate_per_sentence_x100 / 100
        ),
        "원문의 사실, 이름, 사건 순서와 의미를 글투보다 우선해 보존합니다.".into(),
    ]
}
