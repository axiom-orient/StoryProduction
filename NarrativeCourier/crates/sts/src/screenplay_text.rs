//! Borrowed syntax tokens consumed by screenplay format projections; no semantic authority.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token<'a> {
    Heading(&'a str),
    Scene(&'a str),
    Action(&'a str),
    Character(&'a str),
    Dialogue(&'a str),
    Blank,
}

pub(crate) fn parse(content: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut previous_dialogue = false;
    for line in content.lines() {
        let stripped = line.trim();
        if stripped.is_empty() {
            out.push(Token::Blank);
            previous_dialogue = false;
            continue;
        }
        if stripped.starts_with('#') {
            let t = stripped.trim_start_matches('#').trim();
            if !t.is_empty() {
                out.push(Token::Heading(t));
            }
            previous_dialogue = false;
            continue;
        }
        if is_scene_heading(stripped) {
            out.push(Token::Scene(stripped));
            previous_dialogue = false;
            continue;
        }
        let leading = line.len() - line.trim_start_matches(' ').len();
        if leading >= 4 {
            if stripped.starts_with('(') && stripped.ends_with(')') {
                out.push(Token::Dialogue(stripped));
                previous_dialogue = true;
                continue;
            }
            if let Some((character, dialogue)) = split_character_dialogue(line) {
                out.push(Token::Character(character));
                out.push(Token::Dialogue(dialogue));
                previous_dialogue = true;
                continue;
            }
            if previous_dialogue {
                out.push(Token::Dialogue(stripped));
                continue;
            }
            if !stripped.chars().any(|c| c.is_whitespace()) && stripped.chars().count() <= 24 {
                out.push(Token::Character(stripped));
                previous_dialogue = true;
                continue;
            }
        }
        out.push(Token::Action(stripped));
        previous_dialogue = false;
    }
    out
}

fn is_scene_heading(s: &str) -> bool {
    if let Some(rest) = s.strip_prefix("SCN-") {
        let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        return digits >= 3 && rest.as_bytes().get(digits) == Some(&b'.');
    }
    false
}

fn split_character_dialogue(line: &str) -> Option<(&str, &str)> {
    let s = line.trim();
    let mut run_start = None;
    let mut run = 0usize;
    for (idx, ch) in s.char_indices() {
        if ch == ' ' || ch == '\t' {
            if run == 0 {
                run_start = Some(idx);
            }
            run += 1;
            if run >= 2 {
                let start = run_start.unwrap();
                let left = s[..start].trim();
                let right = s[idx + ch.len_utf8()..].trim();
                if !left.is_empty()
                    && !right.is_empty()
                    && !left.contains('(')
                    && left.chars().count() <= 24
                {
                    return Some((left, right));
                }
            }
        } else {
            run = 0;
            run_start = None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repository_same_line_dialogue_contract() {
        let tokens = parse("SCN-001. INT. 방 - 밤\n    민준   여기서 뭐 해?\n    (당황하며)\n    그냥 쉬고 있어.\n행동한다.");
        assert!(tokens.contains(&Token::Scene("SCN-001. INT. 방 - 밤")));
        assert!(tokens.contains(&Token::Character("민준")));
        assert!(tokens.contains(&Token::Dialogue("여기서 뭐 해?")));
        assert!(tokens.contains(&Token::Dialogue("(당황하며)")));
    }
}
