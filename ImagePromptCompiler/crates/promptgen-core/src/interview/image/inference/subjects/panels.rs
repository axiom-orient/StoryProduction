//! Repeated-panel counts, identity anchors, and subject relationships.

use super::appearance::extract_finish;
use super::counts::{count_candidates, grid_instance_count, series_unit_candidates};
use super::parser::{ParsedSubject, clean_subject_description, subject_id};
use super::{contains_any, is_hangul};

pub(in crate::interview::image) fn repeated_panel_count(value: &str) -> Option<u16> {
    let lower = value.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "패널",
            "스티커",
            "그리드",
            "장면",
            "스토리보드",
            "panel",
            "sticker",
            "grid",
            "scene",
            "storyboard",
        ],
    ) {
        return None;
    }

    let explicit = series_unit_candidates(value)
        .into_iter()
        .chain(count_candidates(value))
        .rfind(|candidate| candidate.count > 1)
        .map(|candidate| candidate.count);
    explicit
        .or_else(|| grid_instance_count(value))
        .filter(|count| *count > 1)
}

/// Extracts the stable identity phrase for one subject repeated across a C10
/// panel plan without teaching the compound-subject parser that two
/// independently quantified objects are the same thing.
pub(in crate::interview::image) fn repeated_panel_identity(
    value: &str,
    instance_count: u16,
) -> ParsedSubject {
    let identity_source = value
        .split([';', '\n'])
        .map(str::trim)
        .find(|segment| {
            let lower = segment.to_lowercase();
            contains_any(
                &lower,
                &["마스코트", "캐릭터", "여우", "mascot", "character", "fox"],
            )
        })
        .or_else(|| {
            value
                .split([';', '\n'])
                .map(str::trim)
                .find(|value| !value.is_empty())
        })
        .unwrap_or(value)
        .trim();
    let mut candidates = count_candidates(identity_source);
    candidates.extend(series_unit_candidates(identity_source));
    candidates.sort_by_key(|candidate| candidate.start);
    let series_start = candidates
        .iter()
        .find(|candidate| candidate.count == instance_count)
        .map(|candidate| candidate.start)
        .filter(|start| identity_source[..*start].trim().chars().count() >= 4);
    let identity_source = series_start.map_or(identity_source, |start| &identity_source[..start]);
    let counts = count_candidates(identity_source);
    let description = clean_subject_description(identity_source, &counts);
    ParsedSubject {
        id: subject_id(&description, 0),
        count: instance_count,
        appearance: extract_finish(identity_source),
        description,
    }
}

pub(in crate::interview::image) fn repeated_identity_anchors(description: &str) -> Vec<String> {
    description
        .split([',', ';'])
        .map(str::trim)
        .filter(|anchor| !anchor.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(in crate::interview::image) fn subject_relationship(value: &str) -> Option<String> {
    let lower = value.to_lowercase();
    if contains_any(
        &lower,
        &[
            "그리드",
            "grid",
            "패널",
            "panel",
            "스티커 팩",
            "sticker pack",
        ],
    ) {
        let layout = extract_grid(value).unwrap_or_else(|| "지정된 반복 레이아웃".to_owned());
        return Some(if value.chars().any(is_hangul) {
            format!("모든 인스턴스를 {layout} 안에 서로 겹치지 않게 배치")
        } else {
            format!("Arrange every instance without overlap in the {layout}")
        });
    }
    if contains_any(
        &lower,
        &[
            "함께",
            "나란히",
            "옆에",
            "주위",
            "둘러",
            "together",
            "side by side",
            "beside",
            "next to",
            "around",
        ],
    ) {
        return Some(if value.chars().any(is_hangul) {
            "모든 피사체를 요청에 명시된 상대 위치로 함께 배치".to_owned()
        } else {
            "Arrange all subjects together in the stated relative positions".to_owned()
        });
    }
    None
}

pub(in crate::interview::image) fn extract_grid(value: &str) -> Option<String> {
    let chars = value.char_indices().collect::<Vec<_>>();
    for (index, (_, ch)) in chars.iter().enumerate() {
        if !matches!(ch, 'x' | 'X' | '×') || index == 0 || index + 1 >= chars.len() {
            continue;
        }
        let (left_start, _) = chars[..index]
            .iter()
            .rev()
            .take_while(|(_, ch)| ch.is_ascii_digit())
            .last()
            .copied()?;
        let right_start = chars[index + 1].0;
        let right_end = chars[index + 1..]
            .iter()
            .take_while(|(_, ch)| ch.is_ascii_digit())
            .last()
            .map_or(right_start, |(offset, ch)| offset + ch.len_utf8());
        if left_start < chars[index].0 && right_end > right_start {
            return Some(format!("{} grid", &value[left_start..right_end]));
        }
    }
    None
}
