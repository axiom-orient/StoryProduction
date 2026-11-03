use super::{InterviewOutcome, InterviewRequest};

#[derive(Clone, Copy)]
struct AspectRatioPreset {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    width: u32,
    height: u32,
}

const ASPECT_RATIO_PRESETS: &[AspectRatioPreset] = &[
    AspectRatioPreset {
        id: "1:1",
        label: "1:1",
        description: "정사각형 1024×1024",
        width: 1024,
        height: 1024,
    },
    AspectRatioPreset {
        id: "2:3",
        label: "2:3",
        description: "세로형 1024×1536",
        width: 1024,
        height: 1536,
    },
    AspectRatioPreset {
        id: "3:2",
        label: "3:2",
        description: "가로형 1536×1024",
        width: 1536,
        height: 1024,
    },
    AspectRatioPreset {
        id: "3:4",
        label: "3:4",
        description: "세로형 1152×1536",
        width: 1152,
        height: 1536,
    },
    AspectRatioPreset {
        id: "4:5",
        label: "4:5",
        description: "세로형 1280×1600",
        width: 1280,
        height: 1600,
    },
    AspectRatioPreset {
        id: "4:3",
        label: "4:3",
        description: "가로형 1536×1152",
        width: 1536,
        height: 1152,
    },
    AspectRatioPreset {
        id: "16:9",
        label: "16:9",
        description: "와이드 1792×1008",
        width: 1792,
        height: 1008,
    },
    AspectRatioPreset {
        id: "9:16",
        label: "9:16",
        description: "세로 와이드 1008×1792",
        width: 1008,
        height: 1792,
    },
    AspectRatioPreset {
        id: "2048:2048",
        label: "2048×2048",
        description: "고해상도 정사각형",
        width: 2048,
        height: 2048,
    },
];
const SUPPORTED_LIGHTING: &[&str] = &[
    "soft_daylight",
    "hard_graphic",
    "low_key",
    "golden_hour",
    "neon_practical",
];
const SUPPORTED_LANGUAGES: &[&str] = &["ko", "en"];
const SUPPORTED_MEDIA: &[&str] = &["photo", "illustration", "3d", "graphic_design", "mixed"];
const SUPPORTED_QUALITY: &[&str] = &["auto", "low", "medium", "high"];
const SUPPORTED_TEXT_POSITIONS: &[&str] = &[
    "top_left",
    "top_center",
    "top_right",
    "middle_left",
    "center",
    "middle_right",
    "bottom_left",
    "bottom_center",
    "bottom_right",
];
const SUPPORTED_TEXT_STYLES: &[&str] = &["geometric", "condensed", "didone", "mono", "brush"];

#[path = "image/builder.rs"]
mod builder;
#[path = "image/flow.rs"]
mod flow;
#[path = "image/inference.rs"]
mod inference;
#[path = "image/questions.rs"]
mod questions;
#[path = "image/route_decisions/mod.rs"]
mod route_decisions;
#[path = "image/text_contract.rs"]
mod text_contract;
#[path = "image/visual_defaults.rs"]
mod visual_defaults;

pub(super) fn run(request: &InterviewRequest) -> InterviewOutcome {
    flow::run(request)
}

#[cfg(test)]
#[path = "image/tests.rs"]
mod tests;
