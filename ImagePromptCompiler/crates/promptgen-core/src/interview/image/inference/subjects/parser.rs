//! Subject phrase extraction, typed subject parsing, and normalization.

use std::collections::BTreeSet;

use super::appearance::{extract_finish, remove_finish_terms};
use super::counts::{CountCandidate, count_candidates};
use super::lexicon::{SUBJECT_MARKERS, contains_subject_marker};
use super::text::replace_ascii_case_insensitive;
use super::{contains_any, split_clauses};

pub(in crate::interview::image) fn extract_subject_clause(value: &str) -> Option<String> {
    let markers = [
        "피사체",
        "모델",
        "인물",
        "사람",
        "남성",
        "여성",
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
        "robot",
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
        "shoe",
        "bag",
        "car",
        "watch",
        "speaker",
        "package",
        "box",
        "mascot",
        "sticker",
    ];
    split_clauses(value)
        .into_iter()
        .filter_map(|clause| {
            let lower = clause.to_lowercase();
            let marker_score = markers
                .iter()
                .filter(|marker| lower.contains(**marker))
                .count();
            let count_score = count_candidates(&clause).len();
            // A quantified object clause carries a stronger subject contract than a
            // generic genre clause such as "제품 사진". Reverse-inferred briefs are
            // intentionally multi-sentence, so letting the genre marker win here
            // erases the exact objects before fidelity validation.
            let score = marker_score * 3 + count_score * 4;
            (score > 0).then_some((score, clause))
        })
        .max_by_key(|(score, clause)| (*score, clause.len()))
        .map(|(_, clause)| normalize_subject_clause(&clause))
        .filter(|clause| !clause.is_empty())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::interview::image) struct ParsedSubject {
    pub id: String,
    pub count: u16,
    pub description: String,
    pub appearance: Option<String>,
}

pub(in crate::interview::image) fn parse_subjects(value: &str) -> Vec<ParsedSubject> {
    let mut parsed = split_subject_segments(value)
        .into_iter()
        .filter_map(|segment| parse_subject_segment(&segment))
        .collect::<Vec<_>>();

    if parsed.is_empty() {
        let appearance = extract_finish(value);
        let description = clean_subject_description(value, &[]);
        parsed.push(ParsedSubject {
            id: subject_id(&description, 0),
            count: 1,
            description,
            appearance,
        });
    }

    let mut used_ids = BTreeSet::new();
    for (index, subject) in parsed.iter_mut().enumerate() {
        let base = subject_id(&subject.description, index);
        let mut id = base.clone();
        let mut suffix = 2_u16;
        while !used_ids.insert(id.clone()) {
            id = format!("{base}_{suffix}");
            suffix += 1;
        }
        subject.id = id;
    }
    parsed
}

/// Resolves the visible instance count for a C10 panel or sticker series.
///
/// This intentionally sits beside, rather than inside, `parse_subjects`: a
/// recurring identity shown in twelve stickers is one typed subject with twelve
/// visible instances, while ordinary compound requests still need independent
/// subject IDs and counts.
fn split_subject_segments(value: &str) -> Vec<String> {
    let mut normalized = value.replace(['\n', ';', '。'], ",");
    for (from, to) in [
        ("개와 ", "개|"),
        ("알과 ", "알|"),
        ("명과 ", "명|"),
        ("마리와 ", "마리|"),
        ("대와 ", "대|"),
        ("병과 ", "병|"),
        ("개 및 ", "개|"),
        ("알 및 ", "알|"),
        ("명 및 ", "명|"),
        (" & ", "|"),
        (" + ", "|"),
    ] {
        normalized = replace_ascii_case_insensitive(&normalized, from, to);
    }
    normalized = split_english_subject_conjunctions(&normalized);
    normalized = split_korean_subject_conjunctions(&normalized);
    normalized
        .split([',', '|'])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_subject_segment(segment: &str) -> Option<ParsedSubject> {
    let candidates = count_candidates(segment);
    let selected = select_subject_count(segment, &candidates);
    if selected.is_none() && !contains_subject_marker(segment) {
        return None;
    }
    let appearance = extract_finish(segment);
    let description = clean_subject_description(segment, &candidates);
    (!description.is_empty()).then(|| ParsedSubject {
        id: String::new(),
        count: selected.map_or(1, |candidate| candidate.count),
        description,
        appearance,
    })
}

fn select_subject_count<'a>(
    segment: &str,
    candidates: &'a [CountCandidate],
) -> Option<&'a CountCandidate> {
    if candidates.is_empty() {
        return None;
    }
    let lower = segment.to_lowercase();
    let repeated_unit = ["스티커", "sticker", "패널", "panel", "컷", "frame"]
        .into_iter()
        .filter_map(|marker| lower.find(marker))
        .min();
    repeated_unit
        .and_then(|unit| {
            candidates
                .iter()
                .filter(|candidate| candidate.start <= unit)
                .min_by_key(|candidate| unit - candidate.start)
        })
        .or_else(|| candidates.last())
}

pub(super) fn clean_subject_description(value: &str, counts: &[CountCandidate]) -> String {
    let mut description = value.trim().to_owned();
    for candidate in counts.iter().rev() {
        if candidate.end <= description.len() {
            description.replace_range(candidate.start..candidate.end, "");
        }
    }
    for phrase in [
        "정확히 ",
        "함께 배치한다",
        "함께 배치",
        "나란히 배치한다",
        "나란히",
        "placed together",
        "arranged together",
        "together",
        "exactly ",
    ] {
        description = replace_ascii_case_insensitive(&description, phrase, "");
    }
    description = remove_finish_terms(&description);
    description = description
        .trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '.' | '·' | '-'))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if description.is_empty() {
        "지정된 시각 피사체".to_owned()
    } else {
        description
    }
}

pub(super) fn subject_id(description: &str, index: usize) -> String {
    let lower = description.to_lowercase();
    for (id, markers) in [
        ("beans", &["원두", "coffee bean", "beans"][..]),
        ("cup", &["컵", " cup"][..]),
        ("tumbler", &["텀블러", "tumbler"][..]),
        ("lemon", &["레몬", "lemon"][..]),
        ("speaker", &["스피커", "speaker"][..]),
        ("package_box", &["패키지 박스", "package box"][..]),
        ("stickers", &["스티커", "sticker"][..]),
        ("mascot", &["마스코트", "mascot"][..]),
        ("character", &["캐릭터", "character"][..]),
        ("person", &["모델", "인물", "사람", "person", "model"][..]),
        ("bottle", &["병", "bottle"][..]),
        ("can", &["캔", " can"][..]),
        ("box", &["상자", "박스", " box"][..]),
    ] {
        if markers.iter().any(|marker| lower.contains(marker)) {
            return id.to_owned();
        }
    }
    format!("subject_{}", index + 1)
}

fn split_korean_subject_conjunctions(value: &str) -> String {
    let mut output = value.to_owned();
    let mut search_from = 0;
    loop {
        let next = ["와 ", "과 "]
            .into_iter()
            .filter_map(|separator| {
                output[search_from..]
                    .find(separator)
                    .map(|relative| (search_from + relative, separator))
            })
            .min_by_key(|(index, _)| *index);
        let Some((index, separator)) = next else {
            break;
        };
        let left_start = output[..index]
            .rfind([',', '|'])
            .map_or(0, |boundary| boundary + 1);
        let right_start = index + separator.len();
        let right_end = output[right_start..]
            .find([',', '|'])
            .map_or(output.len(), |boundary| right_start + boundary);
        let left = &output[left_start..index];
        let right = &output[right_start..right_end];
        if ends_with_subject_marker(left) && contains_subject_marker(right) {
            output.replace_range(index..right_start, "|");
            search_from = index + 1;
        } else {
            search_from = right_start;
        }
    }
    output
}

fn split_english_subject_conjunctions(value: &str) -> String {
    let mut output = value.to_owned();
    let mut search_from = 0;
    loop {
        let lower = output.to_lowercase();
        let Some(relative) = lower[search_from..].find(" and ") else {
            break;
        };
        let index = search_from + relative;
        let right_start = index + " and ".len();
        let left_start = output[..index]
            .rfind([',', '|'])
            .map_or(0, |boundary| boundary + 1);
        let right_end = output[right_start..]
            .find([',', '|'])
            .map_or(output.len(), |boundary| right_start + boundary);
        let left = &output[left_start..index];
        let right = &output[right_start..right_end];
        if ends_with_subject_marker(left) && contains_subject_marker(right) {
            output.replace_range(index..right_start, "|");
            search_from = index + 1;
        } else {
            search_from = right_start;
        }
    }
    output
}

fn ends_with_subject_marker(value: &str) -> bool {
    let normalized = value
        .trim()
        .trim_end_matches(|character: char| {
            character.is_ascii_digit()
                || character.is_whitespace()
                || matches!(character, '.' | ',' | ':' | ';' | '-' | '_')
        })
        .to_lowercase();
    SUBJECT_MARKERS
        .iter()
        .copied()
        .any(|marker| normalized.ends_with(marker))
}

fn subject_after_background(value: &str) -> Option<String> {
    for marker in ["배경", "background"] {
        let lower = value.to_lowercase();
        let Some(marker_start) = lower.find(marker) else {
            continue;
        };
        let marker_end = marker_start + marker.len();
        let tail = &value[marker_end..];
        for separator in [
            " 위에 ",
            " 위 ",
            " 앞에 ",
            "에 놓인 ",
            "에 ",
            "에서 ",
            " with ",
            " featuring ",
        ] {
            if let Some(index) = tail.to_lowercase().find(separator) {
                let candidate = tail[index + separator.len()..].trim();
                if contains_subject_marker(candidate) {
                    let finish = extract_finish(&value[..marker_start]);
                    return Some(match finish {
                        Some(finish) => format!("{finish} {candidate}"),
                        None => candidate.to_owned(),
                    });
                }
            }
        }
    }
    None
}

pub(in crate::interview::image) fn normalize_subject_clause(value: &str) -> String {
    let mut subject = value.trim().to_owned();
    if let Some(extracted) = subject_after_background(&subject) {
        subject = extracted;
    }
    for separator in ["없이 ", "without "] {
        if let Some(index) = subject.to_lowercase().rfind(separator) {
            subject = subject[index + separator.len()..].trim_start().to_owned();
        }
    }
    let lower = subject.to_lowercase();
    if (lower.contains("모델")
        || lower.contains("인물")
        || lower.contains("person")
        || lower.contains("model"))
        && let Some(index) = subject.find("의 ")
    {
        let suffix = subject[index + "의 ".len()..].to_lowercase();
        if contains_any(
            &suffix,
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
                "스커트",
                "리조트웨어",
                "라운지웨어",
                "wardrobe",
                "wearing",
            ],
        ) {
            subject.truncate(index);
        }
    }
    for suffix in [
        " 세로 화보",
        " 가로 화보",
        " 화보",
        " 룩북",
        " 에디토리얼",
        " 포스터",
        " editorial",
        " lookbook",
        " poster",
    ] {
        if subject.to_lowercase().ends_with(suffix) {
            let keep = subject.len().saturating_sub(suffix.len());
            subject.truncate(keep);
            subject = subject.trim_end().to_owned();
        }
    }
    subject
}
