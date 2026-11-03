//! Scene and composition clause extraction from the interview brief.

use super::{contains_any, contains_subject_marker, remove_finish_terms, split_clauses};

fn normalize_scene_clause(value: &str) -> String {
    let lower = value.to_lowercase();
    for marker in ["배경", "background"] {
        let Some(start) = lower.find(marker) else {
            continue;
        };
        let end = start + marker.len();
        if start > 0 {
            let mut scene = value[..end].trim().to_owned();
            if contains_subject_marker(&value[end..]) {
                scene = remove_finish_terms(&scene);
            }
            return scene;
        }
    }
    value.trim().to_owned()
}

pub(in crate::interview::image) fn extract_scene_clause(value: &str) -> Option<String> {
    let markers = [
        "배경",
        "스튜디오",
        "골목",
        "거리",
        "도시",
        "실내",
        "야외",
        "해안",
        "테라스",
        "카페",
        "로스터리",
        "무대",
        "숲",
        "바다",
        "밤",
        "낮",
        "아침",
        "저녁",
        "새벽",
        "비",
        "눈",
        "안개",
        "background",
        "studio",
        "street",
        "alley",
        "city",
        "indoor",
        "outdoor",
        "night",
        "morning",
        "rain",
        "fog",
        "coast",
        "terrace",
    ];
    let matches = split_clauses(value)
        .into_iter()
        .filter(|clause| {
            let lower = clause.to_lowercase();
            markers.iter().any(|marker| lower.contains(marker))
        })
        .map(|clause| normalize_scene_clause(&clause))
        .filter(|clause| !clause.is_empty())
        .take(2)
        .collect::<Vec<_>>();
    (!matches.is_empty()).then(|| matches.join(", "))
}

pub(in crate::interview::image) fn extract_composition_clause(value: &str) -> Option<String> {
    let strong_markers = [
        "구도",
        "여백",
        "중앙",
        "배치",
        "전신",
        "반신",
        "클로즈업",
        "아이레벨",
        "로우앵글",
        "하이앵글",
        "%",
        "composition",
        "negative space",
        "centered",
        "close-up",
        "full body",
        "eye level",
    ];
    split_clauses(value).into_iter().find(|clause| {
        let lower = clause.to_lowercase();
        let strong = strong_markers.iter().any(|marker| lower.contains(marker));
        let directional_placement = contains_any(
            &lower,
            &[
                "왼쪽",
                "오른쪽",
                "상단",
                "하단",
                "left",
                "right",
                "top",
                "bottom",
            ],
        ) && contains_any(
            &lower,
            &[
                "피사체",
                "제품",
                "인물",
                "배치",
                "위치",
                "놓",
                "subject",
                "product",
                "placed",
                "positioned",
            ],
        );
        let lighting_clause = contains_any(
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
            ],
        );
        strong || (directional_placement && !lighting_clause)
    })
}
