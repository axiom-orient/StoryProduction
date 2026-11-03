//! Structured scene, subject, and composition sections.

use super::super::common::{PromptLanguage, heading, label, quote};
use super::super::model::ImagePromptRequest;
use super::{placement, visual};

pub(super) fn render_scene_subject_composition(
    request: &ImagePromptRequest,
    language: PromptLanguage,
) -> Vec<String> {
    let mut sections = Vec::with_capacity(3);
    let mut scene = Vec::new();
    if request.scene.environment.trim() == request.scene.background.trim() {
        scene.push(format!(
            "- {}: {}",
            label(language, "환경 및 배경", "Environment and background"),
            visual(&request.scene.environment)
        ));
    } else {
        scene.extend([
            format!(
                "- {}: {}",
                label(language, "환경", "Environment"),
                visual(&request.scene.environment)
            ),
            format!(
                "- {}: {}",
                label(language, "배경", "Background"),
                visual(&request.scene.background)
            ),
        ]);
    }
    scene.push(format!(
        "- {}: {}",
        label(language, "분위기", "Atmosphere"),
        visual(&request.scene.atmosphere)
    ));
    if let Some(value) = &request.scene.time_of_day {
        scene.push(format!(
            "- {}: {}",
            label(language, "시간대", "Time of day"),
            visual(value)
        ));
    }
    if let Some(value) = &request.scene.weather {
        scene.push(format!(
            "- {}: {}",
            label(language, "날씨", "Weather"),
            visual(value)
        ));
    }
    sections.push(format!(
        "{}\n{}",
        heading(language, "장면", "SCENE"),
        scene.join("\n")
    ));

    let mut subjects = Vec::new();
    for subject in &request.subjects {
        let mut fields = vec![
            format!("id={}", quote(&subject.id)),
            format!("count={}", subject.count),
            format!("description={}", quote(&subject.description)),
            format!("placement={}", placement(&subject.placement)),
            format!("scale={}", quote(&subject.scale)),
            format!("pose={}", quote(&subject.pose)),
            format!("action={}", quote(&subject.action)),
        ];
        if !subject.gaze.starts_with("시선 해당 없음")
            && !subject
                .gaze
                .to_ascii_lowercase()
                .starts_with("not applicable")
        {
            fields.push(format!("gaze={}", quote(&subject.gaze)));
        }
        if let Some(value) = &subject.face {
            fields.push(format!("face={}", quote(value)));
        }
        if let Some(value) = &subject.hair {
            fields.push(format!("hair={}", quote(value)));
        }
        if let Some(value) = &subject.appearance {
            fields.push(format!("appearance={}", quote(value)));
        }
        if !subject.distinguishing_features.is_empty() {
            fields.push(format!(
                "distinguishing_features=[{}]",
                subject
                    .distinguishing_features
                    .iter()
                    .map(|value| quote(value))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        subjects.push(format!("- {}", fields.join("; ")));
    }
    sections.push(format!(
        "{}\n{}",
        heading(language, "피사체", "SUBJECTS"),
        subjects.join("\n")
    ));

    let negative_space = if request.composition.negative_space.is_empty() {
        label(language, "지정 없음", "not specified").to_owned()
    } else {
        request
            .composition
            .negative_space
            .iter()
            .map(|zone| zone.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    sections.push(format!(
        "{}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}",
        heading(language, "구도 및 공간", "COMPOSITION AND SPACE"),
        label(language, "프레이밍", "Framing"),
        visual(&request.composition.framing),
        label(language, "시점", "Viewpoint"),
        visual(&request.composition.viewpoint),
        label(language, "카메라 각도", "Camera angle"),
        visual(&request.composition.camera_angle),
        label(language, "균형", "Balance"),
        visual(&request.composition.balance),
        label(language, "시각 계층", "Visual hierarchy"),
        request
            .composition
            .visual_hierarchy
            .iter()
            .map(|id| quote(id))
            .collect::<Vec<_>>()
            .join(" > "),
        label(language, "깊이 레이어", "Depth layers"),
        request
            .composition
            .depth_layers
            .iter()
            .map(|value| visual(value))
            .collect::<Vec<_>>()
            .join(" | "),
        label(language, "의도된 여백 영역", "Negative-space zones"),
        negative_space
    ));

    sections
}
