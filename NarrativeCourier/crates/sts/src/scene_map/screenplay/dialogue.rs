//! Dialogue cue, inline-line, and parenthetical recognition.

pub(super) fn inline_dialogue(line: &str) -> Option<(&str, &str)> {
    let indented = line
        .chars()
        .next()
        .is_some_and(|character| character.is_whitespace());
    let trimmed = line.trim();

    // Korean and Latin screenplay writers commonly use `SPEAKER: line`.
    if let Some((character, dialogue)) = trimmed.split_once(':') {
        let character = character.trim();
        let dialogue = dialogue.trim();
        if valid_speaker_token(character)
            && !dialogue.is_empty()
            && dialogue.chars().count() <= 20_000
        {
            return Some((character, dialogue));
        }
    }

    if !indented {
        return None;
    }
    let mut parts = trimmed.splitn(2, |character: char| character.is_whitespace());
    let character = parts.next()?.trim();
    let dialogue = parts.next()?.trim();
    if !valid_speaker_token(character) || dialogue.is_empty() || dialogue.chars().count() > 20_000 {
        return None;
    }
    Some((character, dialogue))
}

pub(super) fn cue(line: &str) -> Option<&str> {
    if line.len() <= 1 || line.starts_with('-') || line.contains(':') || is_parenthetical(line) {
        return None;
    }

    // Accept an optional parenthetical suffix (`지우 (속삭이며)`), while
    // returning only the stable speaker name to the scene map.
    let speaker = line
        .strip_suffix(')')
        .and_then(|value| value.rfind('(').map(|index| value[..index].trim()))
        .filter(|value| !value.is_empty())
        .unwrap_or(line);
    if speaker != line || valid_speaker_token(speaker) {
        let is_ascii_cue = speaker == speaker.to_ascii_uppercase()
            && speaker.bytes().any(|byte| byte.is_ascii_alphabetic())
            && !speaker.ends_with(['.', '!', '?']);
        let is_korean_cue = speaker.chars().any(is_hangul)
            && speaker.chars().count() <= 64
            && !speaker.chars().any(char::is_whitespace)
            && valid_speaker_token(speaker);
        if is_ascii_cue || is_korean_cue {
            return Some(speaker);
        }
    }
    None
}

pub(super) fn is_parenthetical(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.len() > 2
}

fn valid_speaker_token(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.chars().count() <= 64
        && value
            .chars()
            .any(|character| character.is_alphanumeric() || is_hangul(character))
        && value.chars().all(|character| {
            character.is_alphanumeric() || is_hangul(character) || matches!(character, '_' | '-')
        })
}

fn is_hangul(character: char) -> bool {
    matches!(character, '\u{1100}'..='\u{11ff}' | '\u{3130}'..='\u{318f}' | '\u{ac00}'..='\u{d7af}')
}
