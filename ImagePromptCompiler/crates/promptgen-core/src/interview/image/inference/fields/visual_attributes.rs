//! Surface, wardrobe, and lighting observations/defaults.

use super::{contains_any, contains_subject_marker, extract_finish, material_terms, split_clauses};

pub(in crate::interview::image) fn extract_surface_clause(value: &str) -> Option<String> {
    split_clauses(value).into_iter().find_map(|clause| {
        let lower = clause.to_lowercase();
        let materials = material_terms(&clause);
        let finish = extract_finish(&clause);
        let explicit_surface = contains_any(
            &lower,
            &[
                "질감", "표면", "원단", "마감", "texture", "surface", "fabric", "finish",
            ],
        );
        if finish.is_none() && materials.is_empty() && !explicit_surface {
            return None;
        }
        // A finish-only background describes the set, not the product. Only
        // promote it to the primary surface when the same clause names a subject.
        if materials.is_empty()
            && !explicit_surface
            && !contains_subject_marker(&clause)
            && contains_any(&lower, &["배경", "background"])
        {
            return None;
        }
        let mut observations = materials
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if let Some(finish) = finish {
            observations.push(finish);
        }
        (!observations.is_empty()).then(|| observations.join(", "))
    })
}

pub(in crate::interview::image) fn extract_lighting_clause(value: &str) -> Option<String> {
    split_clauses(value).into_iter().find(|clause| {
        let lower = clause.to_lowercase();
        contains_any(
            &lower,
            &[
                "광원",
                "광선",
                "일광",
                "창가",
                "창문광",
                "확산광",
                "측광",
                "역광",
                "조명",
                "빛",
                "그림자",
                "light",
                "lighting",
                "shadow",
                "window",
                "golden hour",
            ],
        ) && !contains_any(
            &lower,
            &[
                "없", "않", "금지", "제외", "without", "avoid", "exclude", "no light",
            ],
        )
    })
}

pub(in crate::interview::image) fn surface_fields(value: &str, category: &str) -> (String, String) {
    let materials = material_terms(value);
    let material = if materials.is_empty() || materials.len() > 2 {
        match category {
            "C1" => "요청에 특정 재료가 명시되지 않음; 피부와 의상 표면을 자연스럽게 구분",
            "C4" => "요청에 특정 재료가 명시되지 않은 일관된 제품 소재",
            _ => "요청에 특정 재료가 명시되지 않은 일관된 주 표면",
        }
        .to_owned()
    } else {
        materials.join(", ")
    };
    let finish =
        extract_finish(value).unwrap_or_else(|| "요청된 재료의 고유한 표면 마감".to_owned());
    (material, finish)
}

pub(in crate::interview::image) fn extract_wardrobe_clause(value: &str) -> Option<String> {
    let markers = [
        "코트",
        "재킷",
        "블레이저",
        "수트",
        "니트",
        "셔츠",
        "드레스",
        "로브",
        "팬츠",
        "스커트",
        "리조트웨어",
        "라운지웨어",
        "의상",
        "착용",
        "coat",
        "jacket",
        "blazer",
        "suit",
        "dress",
        "robe",
        "pants",
        "skirt",
        "wardrobe",
        "wearing",
    ];
    split_clauses(value).into_iter().find(|clause| {
        let lower = clause.to_lowercase();
        markers.iter().any(|marker| lower.contains(marker))
    })
}

pub(in crate::interview::image) fn wardrobe_is_specific(value: &str) -> bool {
    let lower = value.to_lowercase();
    let has_garment = contains_any(
        &lower,
        &[
            "코트",
            "재킷",
            "블레이저",
            "수트",
            "니트",
            "셔츠",
            "드레스",
            "로브",
            "팬츠",
            "슬랙스",
            "스커트",
            "터틀넥",
            "보디스",
            "리조트웨어",
            "라운지웨어",
            "coat",
            "jacket",
            "blazer",
            "suit",
            "knit",
            "shirt",
            "dress",
            "robe",
            "pants",
            "slacks",
            "skirt",
            "bodice",
        ],
    );
    let axes = [
        contains_any(
            &lower,
            &[
                "아이보리",
                "크림",
                "흰",
                "검정",
                "블랙",
                "네이비",
                "갈색",
                "브라운",
                "세이지",
                "베이지",
                "회색",
                "그레이",
                "빨강",
                "레드",
                "파랑",
                "블루",
                "초록",
                "그린",
                "ivory",
                "cream",
                "black",
                "navy",
                "brown",
                "sage",
                "beige",
                "gray",
                "grey",
                "red",
                "blue",
                "green",
                "#",
            ],
        ),
        contains_any(
            &lower,
            &[
                "울",
                "니트",
                "리넨",
                "린넨",
                "새틴",
                "실크",
                "면",
                "데님",
                "가죽",
                "트위드",
                "코튼",
                "wool",
                "linen",
                "satin",
                "silk",
                "cotton",
                "denim",
                "leather",
                "tweed",
            ],
        ),
        contains_any(
            &lower,
            &[
                "오버사이즈",
                "여유로운",
                "슬림",
                "테일러드",
                "구조적",
                "하이웨이스트",
                "발목 길이",
                "무릎 길이",
                "롱",
                "와이드",
                "핏",
                "재단",
                "oversized",
                "relaxed",
                "slim",
                "tailored",
                "structured",
                "high-waist",
                "ankle-length",
                "wide-leg",
                "fit",
                "cut",
            ],
        ),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    has_garment && axes >= 2
}

pub(in crate::interview::image) fn infer_lighting(value: &str) -> Option<&'static str> {
    let lower = split_clauses(value)
        .into_iter()
        .map(|clause| clause.to_lowercase())
        .filter(|clause| {
            !contains_any(
                clause,
                &[
                    "없",
                    "않",
                    "금지",
                    "제외",
                    "피하",
                    "과도",
                    "no ",
                    "not ",
                    "without",
                    "avoid",
                    "exclude",
                    "excessive",
                ],
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    if contains_any(&lower, &["네온", "청록", "마젠타", "neon"]) {
        Some("neon_practical")
    } else if contains_any(
        &lower,
        &["골든아워", "석양", "해질녘", "golden hour", "sunset"],
    ) {
        Some("golden_hour")
    } else if contains_any(
        &lower,
        &["저조도", "로우키", "어두운 조명", "low key", "noir"],
    ) {
        Some("low_key")
    } else if contains_any(&lower, &["하드 라이트", "날카로운 그림자", "hard light"]) {
        Some("hard_graphic")
    } else if contains_any(&lower, &["확산광", "창가", "부드러운 빛", "soft daylight"]) {
        Some("soft_daylight")
    } else {
        None
    }
}
