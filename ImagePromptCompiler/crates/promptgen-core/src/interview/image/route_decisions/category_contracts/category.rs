//! Category-specific answer contracts and brief-bound defaults.

use std::collections::BTreeMap;

use super::profile::is_app_icon_profile;
use crate::interview::answer;
use crate::interview::image::inference::{contains_subject_marker, parse_subjects};

#[derive(Clone, Copy)]
pub(super) struct CategoryContractSpec {
    pub(super) key: &'static str,
    pub(super) label: &'static str,
    pub(super) help: &'static str,
    pub(super) placeholder: &'static str,
    pub(super) brief_markers: &'static [&'static str],
}

pub(super) fn category_contract_spec(category: &str) -> Option<CategoryContractSpec> {
    match category.to_ascii_uppercase().as_str() {
        "C4" => Some(CategoryContractSpec {
            key: "image.product_guide",
            label: "제품과 브랜드에서 반드시 보여 줄 증거를 정해 주세요.",
            help: "제품·패키지·브랜드 적용물, 수량, 핵심 재질, 보여 줄 면과 라벨 사용 여부를 한 번에 적습니다.",
            placeholder: "무광 알루미늄 헤드폰 1개와 재생지 패키지 1개, 3/4 외관, 이어컵과 헤드밴드 재질 차이, 로고·문구 없음.",
            brief_markers: &[
                "제품",
                "브랜드",
                "패키지",
                "목업",
                "재질",
                "라벨",
                "외관",
                "로고",
            ],
        }),
        "C5" => Some(CategoryContractSpec {
            key: "image.campaign_plan",
            label: "캠페인의 목적과 히어로를 정해 주세요.",
            help: "대상 고객, 전달할 한 가지 메시지, 주 제품 또는 모델, CTA가 들어갈 안전 영역을 한 답변으로 확인합니다.",
            placeholder: "출근길 직장인을 위한 콜드브루 출시 캠페인, 무광 검정 캔 1개가 히어로, 좌하단 CTA 여백 18%.",
            brief_markers: &[
                "캠페인",
                "광고",
                "포스터",
                "키비주얼",
                "브랜드",
                "대상",
                "고객",
                "히어로",
                "제품",
                "모델",
                "cta",
            ],
        }),
        "C6" => Some(CategoryContractSpec {
            key: "image.information_plan",
            label: "전달할 정보와 읽는 순서를 정해 주세요.",
            help: "핵심 메시지, 실제 단계·수치, 각 라벨, 시작에서 끝까지의 읽기 방향을 한 답변으로 적습니다.",
            placeholder: "원두 수확→세척→건조→로스팅→추출 5단계, 단계별 한 줄 설명, 좌상단에서 우하단으로 읽기.",
            brief_markers: &[
                "인포그래픽",
                "단계",
                "수치",
                "데이터",
                "라벨",
                "흐름",
                "순서",
                "과정",
            ],
        }),
        "C10" => Some(CategoryContractSpec {
            key: "image.panel_plan",
            label: "그림의 장면 수와 이야기 흐름을 정해 주세요.",
            help: "단일 장면·패널·스티커·스토리보드 중 형식, 개수, 반복할 캐릭터 특징과 장면별 사건을 적습니다.",
            placeholder: "같은 붉은 목도리 여우가 등장하는 4패널, 좌→우 읽기. 씨앗 발견→심기→비 기다리기→새싹과 인사, 문구 없음.",
            brief_markers: &[
                "일러스트",
                "이야기",
                "만화",
                "패널",
                "스티커",
                "스토리보드",
                "컷",
            ],
        }),
        "C11" => Some(CategoryContractSpec {
            key: "image.keyart_plan",
            label: "컨셉 아트의 세계와 초점을 정해 주세요.",
            help: "장르, 주인공 또는 핵심 오브젝트, 환경 스케일, 감정, 카메라와 안전 영역을 한 답변으로 확인합니다.",
            placeholder: "고대 수몰 도시에 도착한 탐험가 1명, 거대한 석조 아치와 작은 인물의 대비, 경외감, 24mm 와이드, 문구 없음.",
            brief_markers: &[
                "컨셉",
                "키아트",
                "영화",
                "게임",
                "장르",
                "주인공",
                "환경",
                "세계",
            ],
        }),
        _ => None,
    }
}

pub(super) fn app_icon_contract_spec() -> CategoryContractSpec {
    CategoryContractSpec {
        key: "image.app_icon_concept",
        label: "앱 아이콘의 제품 목적과 핵심 시각 은유를 정해 주세요.",
        help: "제품이 무엇을 돕는지, 하나의 중심 상징과 그 상징을 이루는 간결한 형태·색·깊이를 적습니다. 글자·UI·디바이스 프레임은 넣지 않습니다.",
        placeholder: "개인 기억 앱, 작은 별 안에 접힌 종이 형태를 넣은 단일 중심 마크, 남청색 바탕과 따뜻한 노랑 포인트, 문구·UI·기기 프레임 없음.",
        brief_markers: &[
            "앱 아이콘",
            "app icon",
            "application icon",
            "상징",
            "은유",
            "symbol",
            "metaphor",
            "제품 목적",
            "product purpose",
        ],
    }
}

pub(in crate::interview::image) fn infer_category_contract_from_brief(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    let Some(category) = answer(answers, "image.category") else {
        return;
    };
    let spec = if category.eq_ignore_ascii_case("C4") && is_app_icon_profile(answers) {
        app_icon_contract_spec()
    } else {
        let Some(spec) = category_contract_spec(category) else {
            return;
        };
        spec
    };
    if answer(answers, spec.key).is_some() {
        return;
    }

    let trimmed = brief.trim();
    if trimmed.chars().count() < 6 {
        return;
    }
    let normalized = trimmed.to_lowercase();
    let marker_count = spec
        .brief_markers
        .iter()
        .filter(|marker| normalized.contains(**marker))
        .count();
    let observable_c4_subject = category.eq_ignore_ascii_case("C4")
        && contains_subject_marker(trimmed)
        && parse_subjects(trimmed)
            .iter()
            .any(|subject| subject.count > 0);
    if marker_count < 2 && !observable_c4_subject {
        return;
    }

    answers.insert(spec.key.to_owned(), trimmed.to_owned());
    inferences.push(format!(
        "요청에서 {}개의 결과물 핵심 단서를 찾아 {} 답변으로 사용했다.",
        marker_count.max(usize::from(observable_c4_subject)),
        spec.key
    ));
}

pub(in crate::interview::image) fn category_contract_value(
    answers: &BTreeMap<String, String>,
) -> Option<(&'static str, &str)> {
    let category = answer(answers, "image.category")?.to_ascii_uppercase();
    if category == "C4" && is_app_icon_profile(answers) {
        return answer(answers, "image.app_icon_concept").map(|value| ("앱 아이콘 계약", value));
    }
    match category.as_str() {
        "C1" => answer(answers, "image.wardrobe").map(|value| ("의상 계약", value)),
        _ => {
            let spec = category_contract_spec(&category)?;
            answer(answers, spec.key).map(|value| (spec.label, value))
        }
    }
}

pub(in crate::interview::image::route_decisions) fn default_route_scene(
    category: &str,
) -> &'static str {
    match category {
        "C1" => "인물의 행동·표정·의상과 환경 관계가 분리되는 자연스러운 라이프스타일 세트",
        "C4" => "제품·패키지·브랜드 적용물의 형태와 재질을 비교하는 중립 스튜디오",
        "C5" => "히어로와 카피·CTA 안전 영역이 분리되는 캠페인 캔버스",
        "C6" => "단계·수치·UI 상태를 읽기 순서대로 배치하는 정보 캔버스",
        "C10" => "캐릭터 정체성과 장면 순서가 유지되는 일러스트 보드",
        "C11" => "주인공·핵심 오브젝트·거대한 환경의 스케일이 분리되는 컨셉 아트 캔버스",
        _ => "주 피사체와 정보 구조가 분리되는 중립 제작 캔버스",
    }
}
