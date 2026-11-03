//! Typography choices, explicit copy extraction, and palette parsing.

use std::collections::BTreeSet;

use super::contains_any;

pub(in crate::interview::image) fn infer_text_position(value: &str) -> Option<&'static str> {
    let lower = value.to_lowercase();
    for (zone, terms) in [
        ("top_left", &["좌상", "왼쪽 상단", "top left"][..]),
        ("top_center", &["상단 중앙", "top center"][..]),
        ("top_right", &["우상", "오른쪽 상단", "top right"][..]),
        ("middle_left", &["가운데 왼쪽", "middle left"][..]),
        ("center", &["정중앙", "중앙 배치", "centered text"][..]),
        ("middle_right", &["가운데 오른쪽", "middle right"][..]),
        ("bottom_left", &["좌하", "왼쪽 하단", "bottom left"][..]),
        ("bottom_center", &["하단 중앙", "bottom center"][..]),
        ("bottom_right", &["우하", "오른쪽 하단", "bottom right"][..]),
    ] {
        if terms.iter().any(|term| lower.contains(term)) {
            return Some(zone);
        }
    }
    None
}

pub(in crate::interview::image) fn infer_text_style(value: &str) -> Option<&'static str> {
    let lower = value.to_lowercase();
    if contains_any(&lower, &["콘덴스드", "좁은 산세리프", "condensed"]) {
        Some("condensed")
    } else if contains_any(&lower, &["디돈", "고대비 세리프", "didone"]) {
        Some("didone")
    } else if contains_any(&lower, &["모노", "타자기", "monospace"]) {
        Some("mono")
    } else if contains_any(&lower, &["붓글씨", "브러시", "brush"]) {
        Some("brush")
    } else if contains_any(&lower, &["산세리프", "기하", "sans", "geometric"]) {
        Some("geometric")
    } else {
        None
    }
}

pub(in crate::interview::image) fn extract_hex_palette(value: &str) -> Option<Vec<String>> {
    let bytes = value.as_bytes();
    let mut colors = Vec::new();
    let mut seen = BTreeSet::new();
    let mut index = 0;
    while index + 7 <= bytes.len() {
        if bytes[index] == b'#'
            && bytes[index + 1..index + 7]
                .iter()
                .all(u8::is_ascii_hexdigit)
        {
            let color = value[index..index + 7].to_ascii_uppercase();
            if seen.insert(color.clone()) {
                colors.push(color);
            }
            index += 7;
        } else {
            index += 1;
        }
    }
    (3..=5).contains(&colors.len()).then_some(colors)
}

pub(in crate::interview::image) fn extract_quoted_texts(value: &str) -> Vec<String> {
    let mut texts = Vec::new();
    let mut seen = BTreeSet::new();
    for (open, close) in [('"', '"'), ('“', '”'), ('「', '」'), ('『', '』')] {
        let mut rest = value;
        while let Some(start) = rest.find(open) {
            let after = &rest[start + open.len_utf8()..];
            let Some(end) = after.find(close) else {
                break;
            };
            let text = after[..end].trim();
            if !text.is_empty() && seen.insert(text.to_owned()) {
                texts.push(text.to_owned());
            }
            rest = &after[end + close.len_utf8()..];
            if open == close && rest.is_empty() {
                break;
            }
        }
    }
    texts
}
