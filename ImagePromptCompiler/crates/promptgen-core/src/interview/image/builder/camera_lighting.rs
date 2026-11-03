use super::{contains_any, image_copy};
use crate::json::JsonValue;

pub(super) fn camera_json(medium: &str) -> JsonValue {
    let fields = match medium {
        "photo" => [
            (
                "depth_of_field",
                "주 피사체 전체는 선명하고 후경은 윤곽만 식별되는 완만한 초점 이탈",
            ),
            (
                "field_of_view",
                "표준 시야, 직선 가장자리에 배럴 왜곡이 보이지 않음",
            ),
            (
                "focus",
                "주 피사체의 전면 가장자리와 핵심 재질 경계에 초점면 고정",
            ),
            (
                "motion_rendering",
                "정지 촬영, 명시된 움직임만 하나의 연속 궤적으로 표현",
            ),
            (
                "perspective",
                "전경이 과장되지 않는 자연 원근과 균형 잡힌 평면 압축",
            ),
        ],
        _ => [
            (
                "depth_of_field",
                "앞·중·뒤 레이어의 겹침이 명확하고 주 피사체 경계는 선명",
            ),
            (
                "field_of_view",
                "장면 전체 배치를 읽을 수 있는 균형 잡힌 시야",
            ),
            ("focus", "주 피사체의 외곽선과 핵심 정보에 시각 초점 고정"),
            (
                "motion_rendering",
                "명시된 움직임만 방향성 있는 형태 반복으로 표현",
            ),
            (
                "perspective",
                "주 피사체와 배경의 상대 크기를 유지하는 통제된 원근",
            ),
        ],
    };
    JsonValue::object(
        fields
            .into_iter()
            .map(|(key, value)| (key, JsonValue::from(value))),
    )
}

pub(super) fn flat_graphic_camera_json(language: &str) -> JsonValue {
    JsonValue::object([
        (
            "depth_of_field",
            JsonValue::from(image_copy(
                language,
                "단일 평면 그래픽으로 모든 셀과 외곽선을 같은 선명도로 유지",
                "Keep every cell and outline equally sharp in a single flat graphic plane",
            )),
        ),
        (
            "field_of_view",
            JsonValue::from(image_copy(
                language,
                "패널·스티커 시트 전체가 잘리지 않고 보이는 정면 시야",
                "Straight-on view showing the entire panel or sticker sheet without cropping",
            )),
        ),
        (
            "focus",
            JsonValue::from(image_copy(
                language,
                "모든 셀의 얼굴, 행동, 다이컷 외곽선에 동일한 시각 선명도",
                "Give every face, action, and die-cut outline equal visual sharpness",
            )),
        ),
        (
            "motion_rendering",
            JsonValue::from(image_copy(
                language,
                "각 셀에 지정된 한 가지 행동만 또렷한 실루엣으로 표현",
                "Render only the assigned action in each cell as one clear silhouette",
            )),
        ),
        (
            "perspective",
            JsonValue::from(image_copy(
                language,
                "정면 직교 투영, 원근 축소와 렌즈 왜곡 없음",
                "Front-facing orthographic projection with no perspective scaling or lens distortion",
            )),
        ),
    ])
}

pub(super) fn flat_illustration_lighting_json(language: &str) -> JsonValue {
    JsonValue::object([
        (
            "exposure",
            JsonValue::from(image_copy(
                language,
                "모든 셀의 지정 팔레트와 외곽선 대비를 균일하게 유지",
                "Keep the specified palette and outline contrast uniform across every cell",
            )),
        ),
        (
            "fill_description",
            JsonValue::from(image_copy(
                language,
                "별도 사진식 필라이트 없이 평면 색을 그대로 유지",
                "Preserve flat colors without photographic fill lighting",
            )),
        ),
        (
            "key_direction",
            JsonValue::from(image_copy(
                language,
                "캔버스 정면 전체에 균일한 그래픽 명도",
                "Uniform graphic luminance across the full front of the canvas",
            )),
        ),
        (
            "key_quality",
            JsonValue::from(image_copy(
                language,
                "광원 방향보다 평면 채색과 일관된 선 굵기를 우선",
                "Prioritize flat color and consistent line weight over directional lighting",
            )),
        ),
        (
            "shadow_character",
            JsonValue::from(image_copy(
                language,
                "입체 캐스트 그림자와 림라이트 없이 필요한 접지 그림자만 단순화",
                "Use only simplified contact shadows where needed; no volumetric cast shadows or rim light",
            )),
        ),
    ])
}

pub(super) fn lighting_json(value: &str, detail: Option<&str>) -> JsonValue {
    let (direction, quality, kelvin, color, ratio, fill, rim, shadow, exposure) = match value {
        "hard_graphic" => (
            "화면 왼쪽 9시 방향",
            "경계 전이가 4mm 이하인 하드 키 라이트",
            5600_u64,
            "#F3EEE2",
            5.0,
            "카메라 오른쪽의 약한 중성 반사광",
            "후면 가장자리에 폭 2mm의 차가운 림",
            "오른쪽 아래로 길게 이어지는 모서리가 선명한 캐스트 그림자",
            "밝은 면 세부와 짙은 그림자 내부의 재질 경계를 함께 보존",
        ),
        "low_key" => (
            "화면 왼쪽 상단 10시 방향",
            "폭이 좁은 키 라이트와 제한된 확산",
            4300,
            "#D8B18A",
            6.0,
            "카메라 축의 매우 약한 중성 필",
            "후면 오른쪽의 얇은 청회색 림",
            "배경에 깊은 그림자 풀을 만들되 피사체 외곽은 분리",
            "하이라이트 클리핑 없이 중간톤 대비를 유지",
        ),
        "golden_hour" => (
            "화면 오른쪽 낮은 3시 방향",
            "낮은 각도의 따뜻한 측광과 부드러운 가장자리",
            4200,
            "#F2B77B",
            3.0,
            "하늘 방향의 차가운 확산 필",
            "피사체 반대편 외곽의 약한 골드 림",
            "화면 왼쪽으로 길게 이어지는 부드러운 그림자",
            "따뜻한 하이라이트의 층과 중성 피부·재질 색을 함께 보존",
        ),
        "neon_practical" => (
            "화면 왼쪽의 청록 프랙티컬과 오른쪽 후면의 마젠타 림",
            "두 색 광원이 피사체 면에서 겹치지 않도록 분리",
            4800,
            "#36D7D1",
            2.0,
            "카메라 축의 낮은 강도 중성 필",
            "오른쪽 후면 #E05AA8 림",
            "바닥 접점은 짙게 유지하고 반사면에 두 색의 길쭉한 반사",
            "네온 색은 포화되지만 표면 재질과 어두운 영역의 층을 보존",
        ),
        _ => (
            "화면 왼쪽 상단 10시 방향",
            "폭이 넓고 가장자리 전이가 25mm 이상인 확산 키 라이트",
            5200,
            "#F3D9B5",
            3.0,
            "카메라 오른쪽의 중성 반사광이 그림자 내부 재질을 유지",
            "후면 오른쪽에서 외곽에 폭 2mm의 따뜻한 림",
            "접점에서 시작해 오른쪽으로 이어지는 부드러운 그림자",
            "밝은 면의 세부와 어두운 재질의 색층을 함께 보존",
        ),
    };
    let direction = detail
        .and_then(lighting_detail_direction)
        .unwrap_or_else(|| direction.to_owned());
    let quality = detail
        .and_then(lighting_detail_quality)
        .unwrap_or_else(|| quality.to_owned());
    JsonValue::object([
        ("exposure", JsonValue::from(exposure)),
        ("fill_description", JsonValue::from(fill)),
        ("key_color_hex", JsonValue::from(color)),
        ("key_direction", JsonValue::from(direction)),
        ("key_quality", JsonValue::from(quality)),
        ("key_temperature_kelvin", JsonValue::from(kelvin)),
        ("key_to_fill_ratio", JsonValue::Number(format!("{ratio}"))),
        ("rim_description", JsonValue::from(rim)),
        ("shadow_character", JsonValue::from(shadow)),
    ])
}

pub(super) fn lighting_detail_direction(detail: &str) -> Option<String> {
    let lower = detail.to_lowercase();
    let direction = if contains_any(
        &lower,
        &["왼쪽 위", "왼쪽 상단", "좌상", "top left", "upper left"],
    ) {
        "화면 왼쪽 상단"
    } else if contains_any(
        &lower,
        &[
            "오른쪽 위",
            "오른쪽 상단",
            "우상",
            "top right",
            "upper right",
        ],
    ) {
        "화면 오른쪽 상단"
    } else if contains_any(&lower, &["왼쪽", "좌측", "from left", "left side"]) {
        "화면 왼쪽"
    } else if contains_any(&lower, &["오른쪽", "우측", "from right", "right side"]) {
        "화면 오른쪽"
    } else if contains_any(&lower, &["상단", "위에서", "from above", "top down"]) {
        "화면 상단"
    } else {
        return None;
    };
    let source = if contains_any(&lower, &["창가", "창문", "window"]) {
        "창가광"
    } else {
        "키 라이트"
    };
    Some(format!("{direction}에서 들어오는 {source}"))
}

pub(super) fn lighting_detail_quality(detail: &str) -> Option<String> {
    let lower = detail.to_lowercase();
    let window = contains_any(&lower, &["창가", "창문", "window"]);
    let soft = contains_any(&lower, &["부드러운", "확산", "soft", "diffused", "diffuse"]);
    match (window, soft) {
        (true, true) => Some("부드러운 창가 확산광, 넓고 완만한 명암 전이".to_owned()),
        (true, false) => Some("창을 통과한 자연광, 창의 크기에 맞는 명암 전이".to_owned()),
        (false, true) => Some("부드러운 확산광, 넓고 완만한 명암 전이".to_owned()),
        (false, false) => None,
    }
}
