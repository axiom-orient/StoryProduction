use super::common::{PromptLanguage, heading, label, quote};
use crate::catalog::{CatalogEntry, DirectiveSlot};

use super::lut::ResolvedPhotoLut;
use super::model::{
    CanvasPlacement, CanvasZone, ImageProfile, ImagePromptRequest, ImageRenderProfile,
};

mod capture_sections;
mod constraints;
mod editorial_flat;
mod exact_text;
mod profiles;
mod scene_sections;
#[cfg(test)]
use self::editorial_flat::clause_segments;

use self::editorial_flat::render_editorial_flat_prompt;
use self::exact_text::render_exact_text_section;
use self::profiles::{
    render_category_profile_contract, render_cinematic_storyboard_contract,
    render_consistency_contract, render_pose_fidelity_contract,
};

const TIER2_ASSERT: &str = "all depicted people are adults aged 25+, original fictional characters, non-nude fashion editorial styling, fully opaque garments, covered chest lines, composed upright editorial poses";
const EDITORIAL_TIER2_ASSERT: &str = "adult subject aged 25+, original fictional character, non-nude fashion editorial, fully opaque garment, covered chest line, composed upright pose";
const TIER2_TAIL: &str = "no nudity, no nipple or genital exposure, no wardrobe malfunction, no extra people, no text, no watermark";
const TIER2_TAIL_WITH_EXACT_TEXT: &str = "no nudity, no nipple or genital exposure, no wardrobe malfunction, no extra people, no watermark";

pub(crate) fn render_image_prompt(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
) -> String {
    match request.render_profile {
        ImageRenderProfile::Structured => render_structured_prompt(request, selected),
        ImageRenderProfile::EditorialFlat => render_editorial_flat_prompt(request, selected),
        ImageRenderProfile::PoseTransfer => render_structured_prompt(request, selected),
        ImageRenderProfile::CinematicStoryboard => render_structured_prompt(request, selected),
    }
}

fn render_structured_prompt(request: &ImagePromptRequest, selected: &[&CatalogEntry]) -> String {
    let language = request.language;
    let mut sections = Vec::new();
    sections.push(format!(
        "{}\n- {}: {}\n- {}: {}\n- {}: {}",
        heading(language, "목적", "PURPOSE"),
        label(language, "용도", "Use case"),
        visual(&request.use_case),
        label(language, "작업 모드", "Task mode"),
        request.task_mode.as_str(),
        label(language, "매체", "Medium"),
        request.medium.as_str()
    ));
    if request.profile != ImageProfile::Standard {
        sections.push(render_category_profile_contract(request));
    }

    if !request.references.is_empty() {
        let references = request
            .references
            .iter()
            .map(|reference| {
                format!(
                    "- image_{}: role={}; description={}; use_only_for=[{}]",
                    reference.index,
                    reference.role.as_str(),
                    quote(&reference.description),
                    reference
                        .use_for
                        .iter()
                        .map(|value| quote(value))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!(
            "{}\n{}",
            heading(language, "입력 참조 이미지", "INPUT REFERENCES"),
            references
        ));
    }

    if !request.change_contract.change_only.is_empty()
        || !request.change_contract.preserve.is_empty()
    {
        let mut contract = Vec::new();
        if !request.change_contract.change_only.is_empty() {
            contract.push(format!(
                "- {}:",
                label(language, "다음 항목만 변경", "Change only")
            ));
            contract.extend(
                request
                    .change_contract
                    .change_only
                    .iter()
                    .map(|value| format!("  - {}", visual(value))),
            );
        }
        if !request.change_contract.preserve.is_empty() {
            contract.push(format!(
                "- {}:",
                label(language, "반드시 보존", "Preserve exactly")
            ));
            contract.extend(
                request
                    .change_contract
                    .preserve
                    .iter()
                    .map(|value| format!("  - {}", visual(value))),
            );
        }
        contract.push(format!(
            "- {}",
            label(
                language,
                "명시한 변경 외에는 아무것도 바꾸지 않는다. 이후 반복에서도 이 보존 목록을 그대로 적용한다.",
                "Change nothing outside the declared edits. Reapply this complete preserve list in every later iteration."
            )
        ));
        sections.push(format!(
            "{}\n{}",
            heading(language, "변경 계약", "CHANGE CONTRACT"),
            contract.join("\n")
        ));
    }

    if request.render_profile == ImageRenderProfile::PoseTransfer {
        sections.push(render_pose_fidelity_contract(request));
    }
    if request.render_profile == ImageRenderProfile::CinematicStoryboard
        && let Some(storyboard) = &request.storyboard
    {
        sections.push(render_cinematic_storyboard_contract(storyboard, language));
    }
    if let Some(consistency) = &request.consistency {
        sections.push(render_consistency_contract(consistency, language));
    }

    if !selected.is_empty() && request.profile != ImageProfile::AppIcon {
        let textless_ui =
            request.profile == ImageProfile::AppWebUi && request.text_elements.is_empty();
        let mut lines = Vec::new();
        lines.push(format!(
            "- {}",
            label(
                language,
                "권위 규칙: 사용자가 확정한 구조화 필드가 장면·배치·색·정확 문구·출력의 최종 권위다. 아래 directive는 선택한 시각 문법을 관찰 가능한 제작 규칙으로 구체화하며, 충돌 시 구조화 필드가 우선한다.",
                "Authority rule: user-confirmed structured fields are final authority for scene, placement, color, exact text, and output. The directives below turn the selected visual grammar into observable production rules; structured fields win on conflict."
            )
        ));
        for entry in selected {
            for directive in &entry.prompt_directives {
                if textless_ui
                    && matches!(
                        directive.slot,
                        crate::catalog::DirectiveSlot::Typography
                            | crate::catalog::DirectiveSlot::Constraint
                    )
                {
                    continue;
                }
                lines.push(format!(
                    "- {}: {}",
                    directive_slot_label(language, directive.slot),
                    visual(&directive.text)
                ));
            }
        }
        sections.push(format!(
            "{}\n{}",
            heading(language, "선택한 시각 시스템", "SELECTED VISUAL SYSTEM"),
            lines.join("\n")
        ));
    }

    // Keep this order stable: scene/composition, exact text, capture, then constraints/output.
    sections.extend(scene_sections::render_scene_subject_composition(
        request, language,
    ));

    if let Some(text_section) = render_exact_text_section(request, selected) {
        sections.push(text_section);
    }

    sections.extend(capture_sections::render_capture_sections(request, language));

    sections.extend(constraints::render_constraints_and_output(
        request, language,
    ));

    sections.join("\n\n")
}

fn tier_two_tail(has_exact_text: bool) -> &'static str {
    if has_exact_text {
        // The original editorial contract explicitly permits a
        // sequence-preserving subset of NEGATIVE_TAIL. Exact-text work removes
        // only `no text`; all remaining clauses retain their canonical order.
        TIER2_TAIL_WITH_EXACT_TEXT
    } else {
        TIER2_TAIL
    }
}

fn visual(value: &str) -> String {
    super::common::prompt_text(value)
}

const fn directive_slot_label(language: PromptLanguage, slot: DirectiveSlot) -> &'static str {
    match (language, slot) {
        (PromptLanguage::Korean, DirectiveSlot::Subject) => "피사체 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Scene) => "장면 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Composition) => "구도 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Camera) => "카메라 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Lighting) => "조명 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Color) => "색상 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Material) => "재질 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Typography) => "타이포그래피 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Motion) => "움직임 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Constraint) => "제약 규칙",
        (PromptLanguage::English, DirectiveSlot::Subject) => "Subject rule",
        (PromptLanguage::English, DirectiveSlot::Scene) => "Scene rule",
        (PromptLanguage::English, DirectiveSlot::Composition) => "Composition rule",
        (PromptLanguage::English, DirectiveSlot::Camera) => "Camera rule",
        (PromptLanguage::English, DirectiveSlot::Lighting) => "Lighting rule",
        (PromptLanguage::English, DirectiveSlot::Color) => "Color rule",
        (PromptLanguage::English, DirectiveSlot::Material) => "Material rule",
        (PromptLanguage::English, DirectiveSlot::Typography) => "Typography rule",
        (PromptLanguage::English, DirectiveSlot::Motion) => "Motion rule",
        (PromptLanguage::English, DirectiveSlot::Constraint) => "Constraint rule",
    }
}

fn placement(value: &CanvasPlacement) -> String {
    match value {
        CanvasPlacement::Zone(zone) => zone_name(*zone).to_owned(),
        CanvasPlacement::Custom {
            x_percent,
            y_percent,
            width_percent,
            height_percent,
        } => format!(
            "custom_box(x={}%, y={}%, width={}%, height={}%)",
            x_percent, y_percent, width_percent, height_percent
        ),
    }
}

const fn zone_name(zone: CanvasZone) -> &'static str {
    zone.as_str()
}

fn render_lut(lut: &ResolvedPhotoLut, language: PromptLanguage) -> Vec<String> {
    vec![
        format!(
            "- {}: {} at {}% strength",
            label(language, "LUT 프리셋", "LUT preset"),
            lut.preset,
            lut.strength_percent
        ),
        format!(
            "- {}: {} K; tint={}",
            label(language, "LUT 화이트 밸런스", "LUT white balance"),
            lut.white_balance_kelvin,
            lut.tint
        ),
        format!(
            "- {}: tone_curve={}; black_response={}; contrast={}; highlight_rolloff={}",
            label(language, "LUT 톤 응답", "LUT tone response"),
            lut.tone_curve,
            lut.black_response,
            lut.contrast,
            lut.highlight_rolloff
        ),
        format!(
            "- {}: saturation={}%; shadow_bias={}; highlight_bias={}; skin_tone_policy={}",
            label(language, "LUT 색 분리", "LUT color separation"),
            lut.saturation_percent,
            lut.shadow_bias_hex,
            lut.highlight_bias_hex,
            lut.skin_tone_policy
        ),
        format!(
            "- {}: grain={} at {}%; halation={}%; vignette={}%",
            label(language, "LUT 사진 질감", "LUT photographic texture"),
            lut.grain_size,
            lut.grain_amount_percent,
            lut.halation_percent,
            lut.vignette_percent
        ),
    ]
}

#[cfg(test)]
#[path = "render/tests.rs"]
mod tests;
