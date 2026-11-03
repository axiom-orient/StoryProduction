use std::collections::BTreeMap;

use super::inference::{
    aspect_ratio_preset, catalog_palette, default_aspect_ratio, default_composition,
    default_exclusions, default_lighting, default_lut, default_surface, fallback_palette,
    needs_high_quality, parse_hex_palette, parse_patterns,
};
use super::questions::{
    aspect_ratio_options, lighting_options, lut_options, medium_options, option, quality_options,
    text_position_options, text_style_options,
};
use super::route_decisions::category_contracts::profile::{
    is_app_icon_profile, profile_medium_options, profile_medium_question,
};
use super::{
    SUPPORTED_LANGUAGES, SUPPORTED_LIGHTING, SUPPORTED_MEDIA, SUPPORTED_QUALITY,
    SUPPORTED_TEXT_POSITIONS, SUPPORTED_TEXT_STYLES,
};
use crate::image::lut::PRESET_NAMES;
use crate::interview::{InterviewQuestion, answer};

pub(super) fn apply_visual_defaults(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    let category = answer(answers, "image.category")
        .unwrap_or("C5")
        .to_ascii_uppercase();
    let language = answer(answers, "language").unwrap_or("ko").to_owned();
    let patterns = parse_patterns(answer(answers, "image.pattern").unwrap_or("none"));
    let has_text = answer(answers, "image.text_mode") == Some("exact");
    let is_app_icon = is_app_icon_profile(answers);
    let composition_context = if category == "C10" {
        format!(
            "{brief}\n{}",
            answer(answers, "image.panel_plan").unwrap_or_default()
        )
    } else {
        brief.to_owned()
    };

    if category == "C1" {
        infer_default(
            answers,
            inferences,
            "image.face",
            "차분한 자신감이 드러나는 표정, 자연스러운 피부 결, 보이는 모공, 과도한 보정이 없는 균형 잡힌 이목구비".to_owned(),
            "패션 룩북의 얼굴·피부 기본값",
        );
        infer_default(
            answers,
            inferences,
            "image.hair",
            "의상 실루엣을 가리지 않는 정돈된 헤어, 잔머리 몇 가닥과 실제 모발 결이 보임"
                .to_owned(),
            "패션 룩북의 헤어 기본값",
        );
    }

    infer_default(
        answers,
        inferences,
        "image.composition",
        if is_app_icon {
            "one dominant centered mark filling roughly 80% of the square; simple 2–3 visual planes with no secondary objects".to_owned()
        } else {
            default_composition(
                &category,
                &patterns,
                has_text,
                &composition_context,
                &language,
            )
        },
        "선택한 결과물의 구도 문법",
    );
    infer_default(
        answers,
        inferences,
        "image.lighting",
        if is_app_icon {
            "hard_graphic".to_owned()
        } else {
            default_lighting(&category, &patterns, brief)
        },
        "요청과 선택한 결과물의 조명 문법",
    );
    infer_default(
        answers,
        inferences,
        "image.surface",
        if is_app_icon {
            "soft matte artwork with restrained dimensionality and crisp component edges".to_owned()
        } else {
            default_surface(&category, &patterns, &language)
        },
        "선택한 결과물의 재질 문법",
    );
    if answer(answers, "image.palette").is_none() {
        let palette =
            catalog_palette(&category, &patterns).unwrap_or_else(|| fallback_palette(&category));
        let value = palette.join(", ");
        answers.insert("image.palette".to_owned(), value.clone());
        inferences.push(format!(
            "카탈로그의 HEX 또는 결과물 기본 팔레트 {value}를 적용했다."
        ));
    }
    infer_default(
        answers,
        inferences,
        "image.aspect_ratio",
        if is_app_icon {
            "1:1".to_owned()
        } else {
            default_aspect_ratio(&category, &patterns, brief)
        },
        "선택한 결과물의 화면 비율 기본값",
    );
    infer_default(
        answers,
        inferences,
        "image.quality",
        if is_app_icon || needs_high_quality(&category, has_text) {
            "high".to_owned()
        } else {
            "medium".to_owned()
        },
        "텍스트·정보 밀도에 따른 품질 규칙",
    );
    if answer(answers, "image.lut").is_none() && answer(answers, "image.medium") == Some("photo") {
        let (lut, reason) = default_lut(&patterns, brief);
        answers.insert("image.lut".to_owned(), lut.to_owned());
        inferences.push(format!(
            "{reason}에 따라 사진 색감 프리셋 {lut}을 적용했다."
        ));
    }
    infer_default(
        answers,
        inferences,
        "image.exclusions",
        if is_app_icon {
            "text, letters, numerals, pseudo-writing, readable copy, UI screenshot, device mockup, rounded-square mask, app-icon frame, watermark, secondary objects".to_owned()
        } else {
            default_exclusions(has_text)
        },
        "프레임 밖 불필요 요소 배제 규칙",
    );
}

pub(super) fn collect_validation_questions(
    answers: &BTreeMap<String, String>,
) -> Vec<InterviewQuestion> {
    let mut questions = Vec::new();
    if let Some(value) = answer(answers, "language")
        && !SUPPORTED_LANGUAGES.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "language",
                "출력 언어를 다시 선택해 주세요.",
                "컴파일러가 지원하는 언어 코드를 정확히 사용합니다.",
                vec![
                    option("ko", "한국어", "컴파일러 소유 제목과 규칙을 한국어로 출력"),
                    option(
                        "en",
                        "English",
                        "Render compiler-owned headings and rules in English",
                    ),
                ],
            )
            .with_error("지원하지 않는 출력 언어입니다."),
        );
    }
    if let Some(value) = answer(answers, "image.medium") {
        if !SUPPORTED_MEDIA.contains(&value) {
            questions.push(
                InterviewQuestion::choice(
                    "image.medium",
                    "시각 매체를 다시 선택해 주세요.",
                    "카메라·재질·렌더링 규칙의 기준이 되는 매체를 선택합니다.",
                    medium_options(),
                )
                .with_error("지원하지 않는 시각 매체입니다."),
            );
        } else if let Some(profile) = answer(answers, "image.profile")
            && let Some(question) = profile_medium_options(profile)
                .filter(|options| !options.iter().any(|option| option.value == value))
                .and_then(|_| profile_medium_question(profile, value))
        {
            questions.push(question);
        }
    }
    if let Some(value) = answer(answers, "image.aspect_ratio")
        && aspect_ratio_preset(value).is_none()
    {
        questions.push(
            InterviewQuestion::choice(
                "image.aspect_ratio",
                "지원하는 화면 비율을 다시 선택해 주세요.",
                "입력한 비율을 다른 비율로 바꾸지 않고, 지원 범위 안에서 정확히 적용합니다.",
                aspect_ratio_options(),
            )
            .with_error("지원하지 않는 화면 비율입니다."),
        );
    }
    if is_app_icon_profile(answers)
        && let Some(value) = answer(answers, "image.aspect_ratio")
        && value != "1:1"
    {
        questions.push(
            InterviewQuestion::choice(
                "image.aspect_ratio",
                "앱 아이콘은 1:1 정사각형만 지원합니다.",
                "1024×1024 PNG 마스터 콘셉트 계약을 위해 다른 비율을 사용하지 않습니다.",
                vec![option("1:1", "1:1", "정사각형 1024×1024")],
            )
            .with_error("app_icon은 image.aspect_ratio=1:1만 허용합니다."),
        );
    }
    if let Some(value) = answer(answers, "image.lighting")
        && !SUPPORTED_LIGHTING.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "image.lighting",
                "조명 프로필을 다시 선택해 주세요.",
                "지원하는 조명 프로필 중 하나를 정확히 적용합니다.",
                lighting_options(),
            )
            .with_error("지원하지 않는 조명 프로필입니다."),
        );
    }
    if answer(answers, "image.text_mode") == Some("exact")
        && let Some(value) = answer(answers, "image.text_style")
        && !SUPPORTED_TEXT_STYLES.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "image.text_style",
                "타이포그래피 스타일을 다시 선택해 주세요.",
                "지원하는 자형 계열 중 하나를 정확히 적용합니다.",
                text_style_options(),
            )
            .with_error("지원하지 않는 타이포그래피 스타일입니다."),
        );
    }
    if answer(answers, "image.text_mode") == Some("exact")
        && let Some(value) = answer(answers, "image.text_position")
        && !SUPPORTED_TEXT_POSITIONS.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "image.text_position",
                "문구 위치를 다시 선택해 주세요.",
                "지원하는 9개 캔버스 영역 중 하나를 정확히 적용합니다.",
                text_position_options(),
            )
            .with_error("지원하지 않는 문구 위치입니다."),
        );
    }
    if let Some(value) = answer(answers, "image.palette")
        && parse_hex_palette(value).is_none()
    {
        questions.push(
            InterviewQuestion::text(
                "image.palette",
                "HEX 팔레트를 3–5개 입력해 주세요.",
                "쉼표로 구분하며 첫 색부터 면적 우선순위가 높습니다.",
                "#F3F0E8, #6B655F, #5B351F, #A87945",
            )
            .with_error("#RRGGBB 형식의 서로 다른 색 3–5개가 필요합니다."),
        );
    }
    if answer(answers, "image.medium") == Some("photo")
        && let Some(value) = answer(answers, "image.lut")
        && value != "none"
        && !PRESET_NAMES.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "image.lut",
                "사진 색감 프리셋을 다시 선택해 주세요.",
                "현재 지원하는 8개 사진 색감 중 하나를 적용합니다. 그래픽·일러스트·3D에는 적용하지 않습니다.",
                lut_options(),
            )
            .optional()
            .with_error("지원하지 않는 사진 색감 프리셋입니다."),
        );
    }
    if let Some(value) = answer(answers, "image.quality")
        && !SUPPORTED_QUALITY.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "image.quality",
                "출력 품질을 다시 선택해 주세요.",
                "지원하는 gpt-image-2 품질 값 중 하나를 사용합니다.",
                quality_options(),
            )
            .with_error("지원하지 않는 출력 품질입니다."),
        );
    }
    questions
}
pub(super) fn infer_default(
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
    key: &str,
    value: String,
    source: &str,
) {
    if answer(answers, key).is_none() {
        answers.insert(key.to_owned(), value.clone());
        inferences.push(format!("{source}에 따라 {key}={value}를 채웠다."));
    }
}
