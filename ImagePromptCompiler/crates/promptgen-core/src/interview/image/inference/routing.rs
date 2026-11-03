use std::collections::BTreeMap;

use super::contains_any;
use crate::catalog::{TextRequirement, find_entry};
use crate::interview::answer;

pub(in crate::interview::image) fn infer_category(value: &str) -> Option<&'static str> {
    let lower = value.to_lowercase();
    // Medium words such as "illustration" describe how to render, while product
    // and brand words identify what is being made. Prefer the latter when both
    // are present ("제품 브랜드를 수채화 일러스트로").
    if !contains_any(
        &lower,
        &[
            "캠페인",
            "광고",
            "포스터",
            "키비주얼",
            "campaign",
            "advertisement",
            "poster",
        ],
    ) && contains_any(
        &lower,
        &[
            "앱 아이콘",
            "app icon",
            "application icon",
            "app-icon",
            "제품",
            "패키지",
            "브랜딩",
            "목업",
            "화장품",
            "뷰티",
            "제형",
            "분해도",
            "도감",
            "product",
            "packaging",
            "branding",
            "mockup",
        ],
    ) {
        return Some("C4");
    }
    for (category, terms) in [
        (
            "C10",
            &[
                "스티커",
                "이모지",
                "캐릭터 시트",
                "턴어라운드",
                "스토리보드",
                "키프레임",
                "만화",
                "웹툰",
                "코믹",
                "일러스트",
                "sprite",
                "sticker",
                "character sheet",
                "storyboard",
                "comic",
                "illustration",
            ][..],
        ),
        (
            "C6",
            &[
                "인포그래픽",
                "다이어그램",
                "플로우차트",
                "프레젠테이션",
                "슬라이드",
                "카드뉴스",
                "앱 화면",
                "웹 화면",
                "에이전트 화면",
                "app screen",
                "web screen",
                "agent screen",
                "dashboard",
                "ui 목업",
                "와이어프레임",
                "교육 도해",
                "infographic",
                "diagram",
                "presentation",
                "ui mockup",
            ][..],
        ),
        (
            "C5",
            &[
                "캠페인",
                "광고",
                "포스터",
                "키비주얼",
                "소셜 배너",
                "타이포 포스터",
                "campaign",
                "advertisement",
                "poster",
            ][..],
        ),
        (
            "C11",
            &[
                "키아트",
                "시네마틱",
                "콘셉트 아트",
                "컨셉 아트",
                "게임 아트",
                "판타지",
                "역사 장면",
                "cinematic",
                "key art",
                "concept art",
                "game art",
            ][..],
        ),
        (
            "C1",
            &[
                "인물",
                "패션",
                "화보",
                "룩북",
                "프로필",
                "라이프스타일",
                "에디토리얼",
                "portrait",
                "fashion",
                "lookbook",
                "lifestyle",
                "editorial",
            ][..],
        ),
    ] {
        if terms.iter().any(|term| lower.contains(term)) {
            return Some(category);
        }
    }
    None
}

pub(in crate::interview::image) fn infer_medium(value: &str) -> Option<&'static str> {
    let lower = value.to_lowercase();
    if contains_any(
        &lower,
        &[
            "사진",
            "photo",
            "photoreal",
            "촬영",
            "화보",
            "룩북",
            "film still",
        ],
    ) {
        Some("photo")
    } else if contains_any(&lower, &["3d", "렌더", "클레이", "아이소메트릭", "render"]) {
        Some("3d")
    } else if contains_any(
        &lower,
        &["일러스트", "illustration", "그림", "수채화", "웹툰", "만화"],
    ) {
        Some("illustration")
    } else if contains_any(
        &lower,
        &[
            "그래픽",
            "포스터",
            "타이포",
            "인포그래픽",
            "슬라이드",
            "카드뉴스",
        ],
    ) {
        Some("graphic_design")
    } else {
        None
    }
}

pub(in crate::interview::image) fn infer_profile(
    value: &str,
    category: &str,
) -> Option<&'static str> {
    let lower = value.to_lowercase();
    let category = category.to_ascii_uppercase();
    match category.as_str() {
        "C4" if contains_any(
            &lower,
            &["앱 아이콘", "app icon", "application icon", "app-icon"],
        ) =>
        {
            Some("app_icon")
        }
        "C4" if contains_any(
            &lower,
            &["로고", "브랜드 마크", "brand mark", "logo", "wordmark"],
        ) && !contains_any(
            &lower,
            &[
                "로고 없이",
                "로고 없음",
                "without logo",
                "without logos",
                "without text or logos",
                "no logo",
                "no logos",
                "no text or logos",
            ],
        ) =>
        {
            Some("logo_identity")
        }
        "C6" if contains_any(
            &lower,
            &[
                "앱",
                "웹",
                "ui",
                "ux",
                "app",
                "web",
                "dashboard",
                "wireframe",
                "화면",
                "screen",
            ],
        ) =>
        {
            Some("app_web_ui")
        }
        "C6" if contains_any(
            &lower,
            &[
                "인포그래픽",
                "정보 디자인",
                "information design",
                "diagram",
                "flowchart",
            ],
        ) =>
        {
            Some("information_design")
        }
        "C10" if contains_any(&lower, &["품새", "poomsae", "태권도", "taekwondo"]) => {
            Some("poomsae_pose")
        }
        "C10"
            if contains_any(
                &lower,
                &["캐릭터 자세", "character pose", "pose", "자세", "동작"],
            ) =>
        {
            Some("character_pose")
        }
        _ => None,
    }
}

pub(in crate::interview::image) fn needs_high_quality(category: &str, has_text: bool) -> bool {
    has_text || matches!(category, "C5" | "C6" | "C10" | "C11")
}

pub(in crate::interview::image) fn invalid_pattern_tokens(value: &str) -> bool {
    !value.trim().eq_ignore_ascii_case("none")
}

pub(in crate::interview::image) fn selected_text_requirement(
    answers: &BTreeMap<String, String>,
) -> TextRequirement {
    answer(answers, "image.category")
        .and_then(|category| find_entry(category).ok().flatten())
        .map_or(TextRequirement::Optional, |entry| entry.text_requirement)
}
