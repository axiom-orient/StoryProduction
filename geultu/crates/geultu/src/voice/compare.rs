//! Candidate-to-profile scoring and bounded textual coaching tips.

use super::measurements::measure_text;
use super::profile::validate_profile;
use crate::error::Result;
use crate::model::{VoiceComparison, VoiceProfile};

/// Compares candidate text to the active profile without retaining the text.
pub fn compare_to_profile(profile: &VoiceProfile, candidate: &str) -> Result<VoiceComparison> {
    validate_profile(profile)?;
    let candidate_measurements = measure_text(candidate)?;
    let reference = &profile.measurements;

    let dimensions = [
        (
            "문장 길이",
            reference.average_sentence_chars_x100,
            candidate_measurements.average_sentence_chars_x100,
            24u64,
        ),
        (
            "문단 호흡",
            reference.average_paragraph_chars_x100,
            candidate_measurements.average_paragraph_chars_x100,
            16u64,
        ),
        (
            "문장과 문단의 균형",
            reference.sentences_per_paragraph_x100,
            candidate_measurements.sentences_per_paragraph_x100,
            14u64,
        ),
        (
            "대화문 비중",
            reference.dialogue_ratio_per_mille,
            candidate_measurements.dialogue_ratio_per_mille,
            12u64,
        ),
        (
            "한국어 비중",
            reference.hangul_ratio_per_mille,
            candidate_measurements.hangul_ratio_per_mille,
            14u64,
        ),
        (
            "물음·감탄 표현",
            reference.question_rate_per_mille + reference.exclamation_rate_per_mille,
            candidate_measurements.question_rate_per_mille
                + candidate_measurements.exclamation_rate_per_mille,
            8u64,
        ),
        (
            "쉼표와 말줄임표",
            reference.comma_rate_per_sentence_x100 + reference.ellipsis_rate_per_sentence_x100,
            candidate_measurements.comma_rate_per_sentence_x100
                + candidate_measurements.ellipsis_rate_per_sentence_x100,
            8u64,
        ),
        (
            "영문·숫자 비중",
            reference.latin_ratio_per_mille + reference.digit_ratio_per_mille,
            candidate_measurements.latin_ratio_per_mille
                + candidate_measurements.digit_ratio_per_mille,
            4u64,
        ),
    ];

    let total_weight: u64 = dimensions.iter().map(|dimension| dimension.3).sum();
    let weighted_score: u64 = dimensions
        .iter()
        .map(|(_, reference_value, candidate_value, weight)| {
            u64::from(similarity(*reference_value, *candidate_value)) * *weight
        })
        .sum();
    let score = (weighted_score / total_weight).min(100) as u8;
    let label = match score {
        85..=100 => "아주 가까움",
        70..=84 => "비슷한 편",
        50..=69 => "차이가 보임",
        _ => "차이가 큼",
    };

    let mut tips = Vec::new();
    add_tip(
        &mut tips,
        reference.average_sentence_chars_x100,
        candidate_measurements.average_sentence_chars_x100,
        82,
        |shorter| {
            if shorter {
                "문장을 조금 더 길게 이어 문장 호흡을 맞춰 보세요.".into()
            } else {
                "문장을 조금 더 짧게 나누면 관찰된 글투에 가까워집니다.".into()
            }
        },
    );
    add_tip(
        &mut tips,
        reference.average_paragraph_chars_x100,
        candidate_measurements.average_paragraph_chars_x100,
        82,
        |shorter| {
            if shorter {
                "문단을 조금 더 묶어 한 호흡으로 읽히게 해 보세요.".into()
            } else {
                "문단을 나누어 읽는 호흡을 가볍게 해 보세요.".into()
            }
        },
    );
    add_tip(
        &mut tips,
        reference.dialogue_ratio_per_mille,
        candidate_measurements.dialogue_ratio_per_mille,
        82,
        |shorter| {
            if shorter {
                "대화문이 조금 적습니다. 필요한 대사는 줄바꿈을 살려 보세요.".into()
            } else {
                "대화문이 조금 많습니다. 서술과 대사의 균형을 확인해 보세요.".into()
            }
        },
    );
    add_tip(
        &mut tips,
        reference.hangul_ratio_per_mille,
        candidate_measurements.hangul_ratio_per_mille,
        82,
        |shorter| {
            if shorter {
                "한국어 문장 비중이 낮습니다. 불필요한 외국어 표현을 줄여 보세요.".into()
            } else {
                "한국어 문장 비중이 높습니다. 고유한 표현은 의미를 바꾸지 않는 선에서 확인하세요."
                    .into()
            }
        },
    );
    if tips.len() < 4 {
        add_tip(
            &mut tips,
            reference.comma_rate_per_sentence_x100 + reference.ellipsis_rate_per_sentence_x100,
            candidate_measurements.comma_rate_per_sentence_x100
                + candidate_measurements.ellipsis_rate_per_sentence_x100,
            82,
            |shorter| {
                if shorter {
                    "쉼표와 말줄임표를 조금 더 활용해 문장 사이의 여백을 살펴보세요.".into()
                } else {
                    "쉼표와 말줄임표를 덜어 문장이 또렷하게 흐르는지 확인해 보세요.".into()
                }
            },
        );
    }
    if tips.is_empty() {
        tips.push("전체적인 문장과 문단의 호흡이 관찰된 글투와 잘 맞습니다.".into());
    }

    Ok(VoiceComparison {
        score,
        label: label.to_owned(),
        summary: format!(
            "관찰된 문장·문단·표현 습관을 비교한 결과, {score}점으로 {label}입니다. 이 점수는 문체의 방향을 참고하기 위한 측정값입니다."
        ),
        tips,
    })
}

fn add_tip<F>(tips: &mut Vec<String>, reference: u64, candidate: u64, threshold: u32, message: F)
where
    F: FnOnce(bool) -> String,
{
    if tips.len() >= 4 || similarity(reference, candidate) >= threshold {
        return;
    }
    tips.push(message(candidate < reference));
}

fn similarity(reference: u64, candidate: u64) -> u32 {
    if reference == 0 && candidate == 0 {
        return 100;
    }
    let difference = reference.abs_diff(candidate);
    let largest = reference.max(candidate).max(1);
    let relative_percent = difference.saturating_mul(100) / largest;
    100u32.saturating_sub(relative_percent.min(100) as u32)
}
