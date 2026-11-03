use super::super::common::{heading, label, quote};
use super::super::model::ImagePromptRequest;
use super::placement;
use crate::catalog::CatalogEntry;
use crate::catalog::selection::text_guard_is_repetition_aware;

const TIER1_GUARD: &str = "All text appears once, perfectly legible — no duplicate text, no extra words, no invented glyphs, no watermark.";
const REPETITION_GUARD: &str =
    "The designated headline remains perfectly legible — no invented glyphs, no watermark.";

pub(super) fn render_exact_text_section(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
) -> Option<String> {
    if request.text_elements.is_empty() {
        return None;
    }

    let language = request.language;
    let mut text = Vec::new();
    for (element_index, element) in request.text_elements.iter().enumerate() {
        text.push(format!(
            "- text_element_{}: id={}; role={}; language={}; placement={}; font_family={}; weight={}; alignment={}; size_percent={}%; color={}; letter_spacing={}; treatment={}",
            element_index + 1,
            quote(&element.id),
            element.role.as_str(),
            quote(&element.language),
            placement(&element.placement),
            quote(&element.font_family),
            quote(&element.weight),
            quote(&element.alignment),
            element.size_percent,
            element.color_hex.to_ascii_uppercase(),
            quote(&element.letter_spacing),
            quote(&element.treatment)
        ));
        text.push(format!(
            "  - {}",
            label(
                language,
                "다음 인용문만 지정된 순서와 줄바꿈으로 정확히 한 번 렌더링한다:",
                "Render only these quoted lines, exactly once, in this order and with these line breaks:"
            )
        ));
        for (index, line) in element.lines.iter().enumerate() {
            text.push(format!("    {}. {}", index + 1, quote(line)));
        }
        if let Some(spelling_hint) = &element.spelling_hint {
            text.push(format!(
                "  - spelling_hint={} ({})",
                quote(spelling_hint),
                label(
                    language,
                    "철자 안내 전용이며 별도 문구로 렌더링하지 않는다",
                    "guidance only; do not render it as additional copy"
                )
            ));
        }
    }
    text.push(format!(
        "- {}",
        label(
            language,
            "인용된 정확 문구만 렌더링한다. text_element 메타데이터, 번호, 레이블은 이미지에 표시하지 않는다.",
            "Render only the quoted exact copy. Do not render text_element metadata, numbering, or labels."
        )
    ));
    text.push(format!(
        "- {}",
        if text_guard_is_repetition_aware(selected) {
            REPETITION_GUARD
        } else {
            TIER1_GUARD
        }
    ));
    Some(format!(
        "{}\n{}",
        heading(language, "이미지 안 정확 문구", "EXACT TEXT IN IMAGE"),
        text.join("\n")
    ))
}
