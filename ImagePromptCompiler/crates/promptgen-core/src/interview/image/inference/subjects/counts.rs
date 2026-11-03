use super::lexicon::contains_subject_marker;

#[derive(Clone, Copy, Debug)]
pub(super) struct CountCandidate {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) count: u16,
}

pub(super) fn series_unit_candidates(value: &str) -> Vec<CountCandidate> {
    let bytes = value.as_bytes();
    let mut candidates = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let end = index;
        let suffix = value[end..]
            .trim_start()
            .trim_start_matches(['-', '_'])
            .to_lowercase();
        if [
            "패널",
            "컷",
            "장면",
            "스티커",
            "panel",
            "frame",
            "scene",
            "sticker",
        ]
        .iter()
        .any(|unit| suffix.starts_with(unit))
            && let Ok(count) = value[start..end].parse::<u16>()
        {
            candidates.push(CountCandidate { start, end, count });
        }
    }
    candidates
}

pub(super) fn count_candidates(value: &str) -> Vec<CountCandidate> {
    let mut candidates = Vec::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let digit_end = index;
        let before = value[..start].chars().next_back();
        let after = value[digit_end..].chars().next();
        if before.is_some_and(|ch| matches!(ch, ':' | 'x' | 'X' | '×'))
            || after.is_some_and(|ch| matches!(ch, ':' | 'x' | 'X' | '×' | '%' | '°'))
        {
            continue;
        }
        let Ok(count) = value[start..digit_end].parse::<u16>() else {
            continue;
        };
        let suffix = &value[digit_end..];
        let counter = korean_counter_after(suffix);
        if counter
            .is_some_and(|(length, counter)| korean_age_decade(value, digit_end, length, counter))
        {
            continue;
        }
        if counter.is_none() && !is_english_numeric_count(value, start, digit_end, count) {
            continue;
        }
        let mut end = digit_end + counter.map_or(0, |(length, _)| length);
        if value[end..].starts_with(['을', '를', '의']) {
            end += value[end..].chars().next().map_or(0, char::len_utf8);
        }
        candidates.push(CountCandidate { start, end, count });
    }

    let lower = value.to_lowercase();
    for (word, count) in [
        ("one", 1),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
    ] {
        let mut offset = 0;
        while let Some(relative) = lower[offset..].find(word) {
            let start = offset + relative;
            let end = start + word.len();
            let boundary_before =
                start == 0 || !lower.as_bytes()[start - 1].is_ascii_alphanumeric();
            let boundary_after =
                end == lower.len() || !lower.as_bytes()[end].is_ascii_alphanumeric();
            if boundary_before && boundary_after && contains_subject_marker(value) {
                candidates.push(CountCandidate { start, end, count });
            }
            offset = end;
        }
    }
    for (word, count) in [
        ("한", 1),
        ("하나", 1),
        ("두", 2),
        ("둘", 2),
        ("세", 3),
        ("셋", 3),
        ("네", 4),
        ("넷", 4),
        ("다섯", 5),
        ("여섯", 6),
        ("일곱", 7),
        ("여덟", 8),
        ("아홉", 9),
        ("열", 10),
        ("열한", 11),
        ("열두", 12),
    ] {
        for counter in [
            "마리", "켤레", "개", "알", "명", "종", "대", "병", "장", "점", "권", "벌",
        ] {
            for separator in ["", " "] {
                let phrase = format!("{word}{separator}{counter}");
                let mut offset = 0;
                while let Some(relative) = value[offset..].find(&phrase) {
                    let start = offset + relative;
                    let mut end = start + phrase.len();
                    let boundary_before = value[..start]
                        .chars()
                        .next_back()
                        .is_none_or(|ch| !ch.is_alphanumeric());
                    if boundary_before {
                        if value[end..].starts_with(['을', '를', '의']) {
                            end += value[end..].chars().next().map_or(0, char::len_utf8);
                        }
                        candidates.push(CountCandidate { start, end, count });
                    }
                    offset = start + phrase.len();
                }
            }
        }
    }
    candidates.sort_by_key(|candidate| candidate.start);
    candidates.dedup_by_key(|candidate| (candidate.start, candidate.end));
    candidates
}

fn korean_counter_after(value: &str) -> Option<(usize, &'static str)> {
    let whitespace = value.len() - value.trim_start().len();
    let suffix = &value[whitespace..];
    [
        "마리", "켤레", "개", "알", "명", "종", "대", "병", "장", "점", "권", "벌",
    ]
    .into_iter()
    .find(|counter| suffix.starts_with(counter))
    .map(|counter| (whitespace + counter.len(), counter))
}

fn korean_age_decade(value: &str, digit_end: usize, counter_length: usize, counter: &str) -> bool {
    if counter != "대" {
        return false;
    }
    let following = value[digit_end + counter_length..].trim_start();
    [
        "초반", "중반", "후반", "성인", "남성", "여성", "모델", "인물", "사람",
    ]
    .iter()
    .any(|marker| following.starts_with(marker))
}

fn is_english_numeric_count(value: &str, start: usize, end: usize, count: u16) -> bool {
    let previous_words = ascii_words(&value[..start]);
    let following_words = ascii_words(&value[end..]);
    let suffix_count = previous_words
        .last()
        .is_some_and(|word| is_english_subject_noun(word));
    if suffix_count {
        return true;
    }

    let prefix_allowed = previous_words.last().is_none_or(|word| {
        matches!(
            word.as_str(),
            "exactly" | "about" | "add" | "show" | "place" | "render" | "include" | "with"
        )
    });
    if !prefix_allowed {
        return false;
    }
    following_words.iter().take(4).any(|word| {
        is_english_subject_noun(word) && (count == 1 || is_english_plural_count_noun(word))
    })
}

fn ascii_words(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn is_english_subject_noun(word: &str) -> bool {
    matches!(
        word,
        "subject"
            | "subjects"
            | "model"
            | "models"
            | "person"
            | "people"
            | "cat"
            | "cats"
            | "animal"
            | "animals"
            | "product"
            | "products"
            | "bottle"
            | "bottles"
            | "can"
            | "cans"
            | "cup"
            | "cups"
            | "tumbler"
            | "tumblers"
            | "lemon"
            | "lemons"
            | "bean"
            | "beans"
            | "shoe"
            | "shoes"
            | "bag"
            | "bags"
            | "car"
            | "cars"
            | "watch"
            | "watches"
            | "speaker"
            | "speakers"
            | "package"
            | "packages"
            | "box"
            | "boxes"
            | "icon"
            | "icons"
            | "character"
            | "characters"
            | "mascot"
            | "mascots"
            | "sticker"
            | "stickers"
            | "panel"
            | "panels"
            | "robot"
            | "robots"
    )
}

fn is_english_plural_count_noun(word: &str) -> bool {
    word == "people" || word.ends_with('s')
}

pub(super) fn grid_instance_count(value: &str) -> Option<u16> {
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
        let separator_start = chars[index].0;
        let right_start = chars[index + 1].0;
        let right_end = chars[index + 1..]
            .iter()
            .take_while(|(_, ch)| ch.is_ascii_digit())
            .last()
            .map_or(right_start, |(offset, ch)| offset + ch.len_utf8());
        if left_start >= separator_start || right_end <= right_start {
            continue;
        }
        let rows = value[left_start..separator_start].parse::<u16>().ok()?;
        let columns = value[right_start..right_end].parse::<u16>().ok()?;
        return rows.checked_mul(columns);
    }
    None
}
