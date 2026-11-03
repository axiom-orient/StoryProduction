//! Material and surface-finish recognition.

use super::text::replace_ascii_case_insensitive;

fn finish_terms(value: &str) -> Vec<(&'static str, &'static str)> {
    let lower = value.to_lowercase();
    let mut terms = Vec::new();
    for (canonical, aliases) in [
        ("무광", &["무광택", "무광"][..]),
        ("유광", &["고광택", "유광", "광택"][..]),
        ("새틴", &["새틴"][..]),
        ("브러시드", &["브러시드", "헤어라인"][..]),
        ("matte", &["matte", "matt finish"][..]),
        ("glossy", &["high-gloss", "high gloss", "glossy"][..]),
        ("satin", &["satin finish", "satin"][..]),
        ("brushed", &["brushed finish", "brushed"][..]),
    ] {
        if let Some(alias) = aliases.iter().find(|alias| lower.contains(**alias)) {
            terms.push((canonical, *alias));
        }
    }
    terms
}

pub(in crate::interview::image) fn extract_finish(value: &str) -> Option<String> {
    let values = finish_terms(value)
        .into_iter()
        .map(|(canonical, _)| canonical)
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.join(", "))
}

pub(in crate::interview::image) fn remove_finish_terms(value: &str) -> String {
    let mut output = value.to_owned();
    for (_, alias) in finish_terms(value) {
        output = replace_ascii_case_insensitive(&output, alias, "");
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(in crate::interview::image) fn material_terms(value: &str) -> Vec<&'static str> {
    let lower = value.to_lowercase();
    let mut materials = Vec::new();
    for (canonical, aliases) in [
        ("스테인리스 스틸", &["스테인리스 스틸", "스테인리스"][..]),
        ("알루미늄", &["알루미늄"][..]),
        ("도자기", &["도자기", "세라믹"][..]),
        ("유리", &["유리"][..]),
        ("가죽", &["가죽"][..]),
        ("종이", &["재생지", "종이"][..]),
        ("플라스틱", &["플라스틱"][..]),
        ("실리콘", &["실리콘"][..]),
        ("고무", &["고무"][..]),
        ("목재", &["목재", "나무"][..]),
        ("금속", &["금속"][..]),
        ("stainless steel", &["stainless steel"][..]),
        ("aluminum", &["aluminium", "aluminum"][..]),
        ("ceramic", &["ceramic", "porcelain"][..]),
        ("glass", &["glass"][..]),
        ("leather", &["leather"][..]),
        ("paper", &["recycled paper", "paper"][..]),
        ("plastic", &["plastic"][..]),
        ("silicone", &["silicone"][..]),
        ("rubber", &["rubber"][..]),
        ("wood", &["wooden", "wood"][..]),
        ("metal", &["metal"][..]),
    ] {
        if aliases.iter().any(|alias| lower.contains(*alias)) && !materials.contains(&canonical) {
            materials.push(canonical);
        }
    }
    materials
}
