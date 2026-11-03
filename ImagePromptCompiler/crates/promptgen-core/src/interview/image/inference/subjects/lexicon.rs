//! Shared subject vocabulary used by parsing and count inference.

use super::contains_any;

pub(super) const SUBJECT_MARKERS: &[&str] = &[
    "피사체",
    "모델",
    "인물",
    "사람",
    "고양이",
    "동물",
    "제품",
    "캔",
    "병",
    "컵",
    "텀블러",
    "레몬",
    "원두",
    "신발",
    "가방",
    "자동차",
    "의자",
    "시계",
    "화장품",
    "스피커",
    "패키지",
    "상자",
    "박스",
    "아이콘",
    "캐릭터",
    "마스코트",
    "스티커",
    "패널",
    "subject",
    "model",
    "person",
    "cat",
    "animal",
    "product",
    "bottle",
    "can",
    "cup",
    "tumbler",
    "lemon",
    "bean",
    "beans",
    "shoe",
    "shoes",
    "bag",
    "car",
    "watch",
    "speaker",
    "package",
    "box",
    "icon",
    "character",
    "mascot",
    "sticker",
    "stickers",
    "panel",
    "robot",
];

/// Converts the observable, count-bearing noun phrases in an interview subject
/// answer into independent typed subjects. The renderer and visual fidelity gate
/// treat `SubjectSpec::count` as authoritative, so keeping a compound phrase in a
/// single `count=1` subject would erase exactly the facts those layers verify.
pub(in crate::interview::image) fn contains_subject_marker(value: &str) -> bool {
    contains_any(&value.to_lowercase(), SUBJECT_MARKERS)
}
