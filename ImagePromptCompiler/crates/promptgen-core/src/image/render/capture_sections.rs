//! Camera, lighting, color, and material sections.

use super::super::common::{PromptLanguage, heading, label};
use super::super::lut::resolve_photo_lut;
use super::super::model::ImagePromptRequest;
use super::{quote, render_lut, visual};

pub(super) fn render_capture_sections(
    request: &ImagePromptRequest,
    language: PromptLanguage,
) -> Vec<String> {
    let mut sections = Vec::with_capacity(4);
    if let Some(camera) = &request.camera {
        sections.push(format!(
            "{}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}",
            heading(language, "카메라 결과", "CAMERA OUTCOME"),
            label(language, "시야", "Field of view"),
            visual(&camera.field_of_view),
            label(language, "원근", "Perspective"),
            visual(&camera.perspective),
            label(language, "심도", "Depth of field"),
            visual(&camera.depth_of_field),
            label(language, "초점", "Focus"),
            visual(&camera.focus),
            label(language, "움직임 표현", "Motion rendering"),
            visual(&camera.motion_rendering)
        ));
    }

    let mut lighting = vec![
        format!(
            "- {}: {}",
            label(language, "키 방향", "Key direction"),
            visual(&request.lighting.key_direction)
        ),
        format!(
            "- {}: {}",
            label(language, "키 광질", "Key quality"),
            visual(&request.lighting.key_quality)
        ),
    ];
    if let Some(value) = request.lighting.key_temperature_kelvin {
        lighting.push(format!(
            "- {}: {} K",
            label(language, "키 색온도", "Key temperature"),
            value
        ));
    }
    if let Some(value) = &request.lighting.key_color_hex {
        lighting.push(format!(
            "- {}: {}",
            label(language, "키 색상", "Key color"),
            value.to_ascii_uppercase()
        ));
    }
    if let Some(value) = request.lighting.key_to_fill_ratio {
        lighting.push(format!(
            "- {}: {:.2}:1",
            label(language, "키 대 필 비율", "Key-to-fill ratio"),
            value
        ));
    }
    lighting.extend([
        format!(
            "- {}: {}",
            label(language, "필", "Fill"),
            visual(&request.lighting.fill_description)
        ),
        format!(
            "- {}: {}",
            label(language, "그림자", "Shadow character"),
            visual(&request.lighting.shadow_character)
        ),
        format!(
            "- {}: {}",
            label(language, "노출", "Exposure"),
            visual(&request.lighting.exposure)
        ),
    ]);
    if let Some(value) = &request.lighting.rim_description {
        lighting.push(format!(
            "- {}: {}",
            label(language, "림 라이트", "Rim light"),
            visual(value)
        ));
    }
    sections.push(format!(
        "{}\n{}",
        heading(language, "조명", "LIGHTING"),
        lighting.join("\n")
    ));

    let palette = request
        .color
        .palette
        .iter()
        .map(|entry| {
            format!(
                "{} {}% ({})",
                entry.hex.to_ascii_uppercase(),
                entry.proportion_percent,
                visual(&entry.usage)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let mut color = vec![
        format!("- {}: {}", label(language, "팔레트", "Palette"), palette),
        format!(
            "- {}: {}",
            label(language, "조화", "Harmony"),
            visual(&request.color.harmony)
        ),
        format!(
            "- {}: {}",
            label(language, "대비", "Contrast"),
            visual(&request.color.contrast)
        ),
        format!(
            "- {}: {}",
            label(language, "채도", "Saturation"),
            visual(&request.color.saturation)
        ),
    ];
    if let Some(selection) = &request.color.photo_lut
        && let Some(resolved) = resolve_photo_lut(selection)
    {
        color.extend(render_lut(&resolved, language));
    }
    sections.push(format!(
        "{}\n{}",
        heading(language, "색 및 사진 LUT", "COLOR AND PHOTO LUT"),
        color.join("\n")
    ));

    if !request.surfaces.is_empty() {
        let surfaces = request
            .surfaces
            .iter()
            .map(|surface| {
                format!(
                    "- id={}; material={}; finish={}; micro_detail={}; light_response={}",
                    quote(&surface.id),
                    quote(&surface.material),
                    quote(&surface.finish),
                    quote(&surface.micro_detail),
                    quote(&surface.light_response)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!(
            "{}\n{}",
            heading(language, "재질 및 표면", "MATERIALS AND SURFACES"),
            surfaces
        ));
    }

    sections
}
