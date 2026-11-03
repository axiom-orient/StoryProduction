//! Palette, photo LUT, and color-constraint projection.

use std::collections::BTreeMap;

use super::super::inference::{parse_hex_palette, proportions};
use crate::interview::answer;
use crate::json::JsonValue;

pub(super) fn palette_projection(answers: &BTreeMap<String, String>) -> JsonValue {
    let palette = parse_hex_palette(answer(answers, "image.palette").unwrap_or_default())
        .unwrap_or_else(|| {
            vec![
                "#F2F2EF".to_owned(),
                "#6B655F".to_owned(),
                "#1A1A1A".to_owned(),
            ]
        });
    let proportions = proportions(palette.len());
    let palette_json =
        palette
            .into_iter()
            .zip(proportions)
            .enumerate()
            .map(|(index, (hex, proportion))| {
                JsonValue::object([
                    ("hex", JsonValue::from(hex)),
                    ("proportion_percent", JsonValue::from(u64::from(proportion))),
                    (
                        "usage",
                        JsonValue::from(format!("시각 면적 우선순위 {}의 색", index + 1)),
                    ),
                ])
            });
    JsonValue::array(palette_json)
}

pub(super) fn color_json(
    answers: &BTreeMap<String, String>,
    medium: &str,
    palette: JsonValue,
) -> JsonValue {
    let lut = if medium == "photo" {
        answer(answers, "image.lut")
            .filter(|value| *value != "none")
            .map(|value| {
                JsonValue::object([
                    ("preset", JsonValue::from(value)),
                    ("strength_percent", JsonValue::from(70_u64)),
                ])
            })
    } else {
        None
    };
    let mut color_fields = vec![
        (
            "contrast",
            JsonValue::from("주 피사체와 배경 사이 명도 차이를 20% 이상 유지"),
        ),
        (
            "harmony",
            JsonValue::from("지정 HEX 팔레트만 사용하고 첫 색부터 면적 우선순위를 유지"),
        ),
        ("palette", palette),
        (
            "saturation",
            JsonValue::from("전체 채도는 재질 식별이 가능한 중간 범위로 제한"),
        ),
    ];
    if let Some(lut) = lut {
        color_fields.push(("photo_lut", lut));
    }
    JsonValue::object(color_fields)
}
