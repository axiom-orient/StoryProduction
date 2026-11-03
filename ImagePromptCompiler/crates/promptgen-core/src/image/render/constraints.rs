//! Required/omitted elements, safety constraints, and output parameters.

use super::super::common::{PromptLanguage, heading, label};
use super::super::model::ImagePromptRequest;
use super::{TIER2_ASSERT, tier_two_tail, visual};

pub(super) fn render_constraints_and_output(
    request: &ImagePromptRequest,
    language: PromptLanguage,
) -> Vec<String> {
    let mut sections = Vec::with_capacity(2);
    let required = if request.constraints.required_elements.is_empty() {
        label(language, "없음", "none").to_owned()
    } else {
        request
            .constraints
            .required_elements
            .iter()
            .map(|value| visual(value))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let omitted = if request.constraints.excluded_elements.is_empty() {
        label(language, "없음", "none").to_owned()
    } else {
        request
            .constraints
            .excluded_elements
            .iter()
            .map(|value| visual(value))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let mut constraints = vec![
        format!(
            "- {}: {}",
            label(language, "필수 요소", "Required elements"),
            required
        ),
        format!(
            "- {}: {}",
            label(
                language,
                "프레임 밖으로 유지할 범주",
                "Categories kept outside the frame"
            ),
            omitted
        ),
        format!(
            "- {}: {}",
            label(language, "안전 티어", "Safety tier"),
            request.constraints.safety_tier
        ),
        format!(
            "- {}: {}",
            label(language, "프레임 경계", "Frame boundary"),
            label(
                language,
                "지정한 피사체·필수 요소·표면만 화면에 존재하며, 배경은 구조화된 장면 설명을 그대로 따른다.",
                "Only the specified subjects, required elements, and surfaces occupy the frame; the background follows the structured scene description exactly."
            )
        ),
    ];
    if request.constraints.original_characters_only {
        constraints.push(format!(
            "- {}",
            label(
                language,
                "인물은 실존 인물과 무관한 가상 오리지널 캐릭터다.",
                "Every person is an original fictional character unrelated to any real person."
            )
        ));
    }
    if request.constraints.clean_unbranded_finish {
        constraints.push(format!(
            "- {}",
            label(
                language,
                "브랜드 표식이 없는 깨끗하고 무표식인 마감.",
                "Clean, unbranded, unmarked finish."
            )
        ));
    }
    if request.constraints.safety_tier == 2 {
        constraints.push(format!("- SAFETY_ASSERT: {TIER2_ASSERT}"));
        constraints.push(format!(
            "- COMPLIANCE_TAIL: {}",
            tier_two_tail(!request.text_elements.is_empty())
        ));
    }
    sections.push(format!(
        "{}\n{}",
        heading(language, "제약 및 안전", "CONSTRAINTS AND SAFETY"),
        constraints.join("\n")
    ));

    sections.push(format!(
        "{}\n- model={}\n- width={}\n- height={}\n- quality={}\n- background={}\n- format={}\n- AR {}",
        heading(language, "생성 파라미터", "GENERATION PARAMETERS"),
        request.output.model,
        request.output.width,
        request.output.height,
        request.output.quality.as_str(),
        request.output.background.as_str(),
        request.output.format,
        request.output.aspect_ratio_label()
    ));

    sections
}
