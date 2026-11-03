use std::collections::BTreeSet;

use super::{ASPECT_RATIO_PRESETS, AspectRatioPreset, contains_any, extract_grid, infer_lighting};
use crate::catalog::find_entry;

pub(in crate::interview::image) fn default_composition(
    category: &str,
    _patterns: &[String],
    has_text: bool,
    brief: &str,
    language: &str,
) -> String {
    let lower = brief.to_lowercase();
    if lower.contains("클로즈업") || lower.contains("close-up") {
        return "주 피사체 얼굴 또는 제품 전면이 캔버스 높이의 72%, 시선·기능 면에 초점, 후경은 18% 이하 정보 밀도".to_owned();
    }
    if category == "C4" && contains_any(&lower, &["세로", "vertical", "portrait orientation"]) {
        return "세로 캔버스 중앙의 단일 제품 3/4 히어로 구도, 전면과 측면 재질이 함께 보이며 상하 여백을 균형 있게 유지"
            .to_owned();
    }
    if category == "C10"
        && contains_any(
            &lower,
            &[
                "패널",
                "스티커",
                "그리드",
                "스토리보드",
                "panel",
                "sticker",
                "grid",
                "storyboard",
            ],
        )
    {
        let layout = extract_grid(brief).unwrap_or_else(|| "지정된 패널 grid".to_owned());
        return if language == "en" {
            format!(
                "Arrange every cell in the {layout} at equal size with uniform gutters; assign exactly one action to each cell in reading order"
            )
        } else {
            format!(
                "{layout}의 모든 셀을 균일한 크기와 동일한 여백으로 배치하고, 각 셀에는 읽기 순서대로 한 가지 행동만 배정"
            )
        };
    }
    let base = match category {
        "C1" => {
            "단독 인물 전신 또는 3/4 신체가 캔버스 높이의 68%, 아이레벨 세로 구도, 상하 여백 합계 18%, 실루엣과 의상선이 겹치지 않음"
        }
        "C4" => {
            "제품 또는 패키지가 중앙 52%, 재질과 기능 면이 동시에 보이는 3/4 구도, 좌우 18%는 브랜드 적용 또는 콜아웃 안전 영역"
        }
        "C5" => {
            "헤드라인이 1순위, 히어로가 2순위인 비대칭 캠페인 그리드, 바깥 여백 8%, 문구와 피사체 경계 4% 이상 분리"
        }
        "C6" => {
            "12열 그리드, 바깥 여백 6%, 제목 1개와 핵심 정보 블록 3개 이하, 좌상단에서 우하단으로 읽는 단일 위계"
        }
        "C10" if language == "en" => {
            "Separate the main subject and background in a flat front-facing illustration"
        }
        "C10" => "단일 일러스트의 주 피사체와 배경을 정면 평면 구도로 분리",
        "C11" => {
            "주 피사체가 높이의 64%, 중심보다 6% 한쪽으로 이동, 반대쪽 28%는 타이틀 또는 환경 서사를 위한 네거티브 스페이스"
        }
        _ if has_text => {
            "주 피사체가 높이의 56%, 중심보다 8% 오른쪽, 좌상단 30%는 정확 문구용 네거티브 스페이스, 읽기 순서는 문구→피사체→보조 정보"
        }
        _ => {
            "주 피사체가 높이의 62%, 중심보다 3% 오른쪽, 상단 22%는 비우고 배경 정보 밀도는 피사체의 절반 이하"
        }
    };
    base.to_owned()
}

pub(in crate::interview::image) fn default_lighting(
    category: &str,
    _patterns: &[String],
    brief: &str,
) -> String {
    if let Some(value) = infer_lighting(brief) {
        return value.to_owned();
    }
    match category {
        "C5" | "C6" => "hard_graphic".to_owned(),
        "C11" => "low_key".to_owned(),
        "C10" => "hard_graphic".to_owned(),
        _ => "soft_daylight".to_owned(),
    }
}

pub(in crate::interview::image) fn default_surface(
    category: &str,
    _patterns: &[String],
    language: &str,
) -> String {
    let base = match category {
        "C1" => "자연스러운 피부 결, 보이는 모공, 의상 원단의 직조와 재단선, 미세한 필름 그레인",
        "C4" => {
            "제품과 패키지 재료별 표면 차이, 단면 가장자리와 조립 틈, 하이라이트 폭으로 구분되는 금속·플라스틱·종이·유리"
        }
        "C5" | "C6" => {
            "인쇄 또는 화면 그래픽의 평면 마감, 레이어 경계와 정보 위계를 분리하는 균일한 잉크·픽셀 가장자리"
        }
        "C10" if language == "en" => {
            "consistent line weight with ink or brush texture, crisp color-plane boundaries, and stable tone across panels"
        }
        "C10" => "선 두께가 일정한 잉크 또는 브러시 질감, 면색 경계가 또렷하고 패널 간 톤이 일관됨",
        "C11" => {
            "피부·직물·금속·환경 표면이 서로 다른 반사 폭을 가지며, 어두운 영역에도 미세 재질층이 남음"
        }
        _ => {
            "주 피사체 재료를 식별할 수 있는 미세 요철, 가장자리 마감, 곡률에 맞는 연속 하이라이트와 접점 그림자"
        }
    };
    base.to_owned()
}

pub(in crate::interview::image) fn catalog_palette(
    category: &str,
    patterns: &[String],
) -> Option<Vec<String>> {
    let mut ids = patterns.to_vec();
    ids.push(category.to_owned());
    let mut colors = Vec::new();
    let mut seen = BTreeSet::new();
    for id in ids {
        let Some(entry) = find_entry(&id).ok().flatten() else {
            continue;
        };
        for color in &entry.suggested_palette {
            if seen.insert(color.clone()) {
                colors.push(color.clone());
            }
            if colors.len() == 5 {
                return Some(colors);
            }
        }
    }
    if colors.len() >= 3 {
        Some(colors)
    } else {
        None
    }
}

pub(in crate::interview::image) fn fallback_palette(category: &str) -> Vec<String> {
    let values: &[&str] = match category {
        "C1" => &["#F5F0E8", "#D8CBB8", "#8A7A66", "#2E2A26"],
        "C4" => &["#F4F5F6", "#7C8794", "#2D333A", "#D18B47"],
        "C5" => &["#F1EAD8", "#7A8450", "#111111", "#D8442E"],
        "C6" => &["#F7F8FA", "#243447", "#4F7CAC", "#E2A93B"],
        "C10" => &["#FFF7E8", "#E85D4A", "#2F6FA3", "#20242A"],
        "C11" => &["#0E1420", "#D9A566", "#4A6670", "#ECE7DD"],
        _ => &["#F2F2EF", "#6B655F", "#1A1A1A"],
    };
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub(in crate::interview::image) fn default_aspect_ratio(
    category: &str,
    _patterns: &[String],
    brief: &str,
) -> String {
    if let Some(ratio) = find_ratio(brief) {
        return ratio;
    }
    let lower = brief.to_lowercase();
    if contains_any(&lower, &["세로", "vertical", "portrait orientation"]) {
        return "2:3".to_owned();
    }
    if contains_any(&lower, &["가로", "horizontal", "landscape orientation"]) {
        return "3:2".to_owned();
    }
    if category == "C1" && contains_any(&lower, &["화보", "에디토리얼", "룩북", "editorial"])
    {
        return "2:3".to_owned();
    }
    match category {
        "C1" | "C5" | "C10" => "2:3".to_owned(),
        "C4" => "3:2".to_owned(),
        "C6" | "C11" => "16:9".to_owned(),
        _ => "1:1".to_owned(),
    }
}

pub(in crate::interview::image) fn default_lut(
    _patterns: &[String],
    brief: &str,
) -> (&'static str, &'static str) {
    let lower = brief.to_lowercase();
    if contains_any(
        &lower,
        &[
            "모노크롬 하이 콘트라스트",
            "고대비 흑백",
            "monochrome high contrast",
            "black and white",
            "흑백",
        ],
    ) {
        return ("monochrome_high_contrast", "요청의 고대비 흑백 표현");
    }
    if contains_any(&lower, &["블리치 바이패스", "bleach bypass"]) {
        return ("bleach_bypass", "요청에 명시된 저채도 고대비 영화 색감");
    }
    if contains_any(
        &lower,
        &[
            "절제된 틸 오렌지",
            "틸 오렌지",
            "restrained teal orange",
            "teal orange",
        ],
    ) {
        return (
            "restrained_teal_orange",
            "요청에 명시된 청록 그림자·따뜻한 하이라이트 분리",
        );
    }
    if contains_any(&lower, &["쿨 스틸", "cool steel"]) {
        return ("cool_steel", "요청에 명시된 차가운 기술 색감");
    }
    if contains_any(&lower, &["페이디드 프린트", "바랜 인쇄", "faded print"]) {
        return ("faded_print", "요청에 명시된 바랜 인쇄 색감");
    }
    if contains_any(&lower, &["웜 파스텔 필름", "따뜻한 파스텔", "warm pastel"]) {
        return (
            "warm_pastel_filmic",
            "요청에 명시된 따뜻한 파스텔 필름 색감",
        );
    }
    if contains_any(&lower, &["클린 뉴트럴", "clean neutral"]) {
        return ("clean_neutral", "요청에 명시된 중립 색 재현");
    }
    if contains_any(&lower, &["텅스텐", "야간", "야경", "밤", "전구", "night"]) {
        return ("tungsten_night", "요청의 야간·텅스텐 조명");
    }
    ("clean_neutral", "제품·재질의 중립 색을 보존하는 기본값")
}

pub(in crate::interview::image) fn default_exclusions(has_text: bool) -> String {
    if has_text {
        "지정한 주 피사체와 필수 소품만 화면에 존재\n브랜드 표식이 없는 표면\n읽을 수 있는 문자는 사용자가 지정한 정확 문구의 각 줄만 순서대로 한 번씩 존재\n배경은 구조화된 장면의 요소만 유지"
            .to_owned()
    } else {
        "지정한 주 피사체와 필수 소품만 화면에 존재\n브랜드 표식이 없는 표면\n읽을 수 있는 문자가 없는 깨끗한 이미지\n배경은 구조화된 장면의 요소만 유지"
            .to_owned()
    }
}

pub(in crate::interview::image) fn find_ratio(value: &str) -> Option<String> {
    for token in
        value.split(|c: char| c.is_whitespace() || matches!(c, ',' | '/' | '(' | ')' | '[' | ']'))
    {
        if aspect_ratio_preset(token).is_some() {
            return Some(token.to_owned());
        }
    }
    None
}

pub(in crate::interview::image) fn parse_patterns(_value: &str) -> Vec<String> {
    Vec::new()
}

pub(in crate::interview::image) fn selected_safety_tier(
    category: &str,
    patterns: &[String],
    has_text: bool,
) -> u8 {
    let minimum = std::iter::once(category.to_owned())
        .chain(patterns.iter().cloned())
        .filter_map(|id| find_entry(&id).ok().flatten())
        .map(|entry| entry.default_safety_tier)
        .max()
        .unwrap_or(0);
    minimum.max(u8::from(has_text))
}

pub(in crate::interview::image) fn parse_hex_palette(value: &str) -> Option<Vec<String>> {
    let mut colors = Vec::new();
    let mut unique = BTreeSet::new();
    for token in value
        .split([',', ' ', '\n'])
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let normalized = token.to_ascii_uppercase();
        if normalized.len() != 7
            || !normalized.starts_with('#')
            || !normalized[1..]
                .chars()
                .all(|character| character.is_ascii_hexdigit())
            || !unique.insert(normalized.clone())
        {
            return None;
        }
        colors.push(normalized);
    }
    (3..=5).contains(&colors.len()).then_some(colors)
}

pub(in crate::interview::image) fn dimensions(value: &str) -> (u32, u32) {
    aspect_ratio_preset(value)
        .map(|preset| (preset.width, preset.height))
        .unwrap_or((1024, 1024))
}

pub(in crate::interview::image) fn aspect_ratio_preset(
    value: &str,
) -> Option<&'static AspectRatioPreset> {
    ASPECT_RATIO_PRESETS
        .iter()
        .find(|preset| preset.id == value)
}

pub(in crate::interview::image) fn proportions(count: usize) -> Vec<u8> {
    match count {
        3 => vec![45, 35, 20],
        4 => vec![35, 30, 20, 15],
        5 => vec![30, 25, 20, 15, 10],
        _ => Vec::new(),
    }
}

pub(in crate::interview::image) fn split_nonempty(value: &str) -> Vec<String> {
    value
        .split(['\n', ';'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
