//! Fountain 1.1 practical-subset parser and canonical block normalization.

use super::super::NormalizedScreenplay;
use super::Builder;

fn strip_boneyard_and_notes(body: &str) -> narrative_courier_runtime::Result<String> {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(start) = rest.find("/*") {
        match rest[start..].find("*/") {
            Some(end) => {
                out.push_str(&rest[..start]);
                rest = &rest[start + end + 2..];
            }
            None => {
                return Err(narrative_courier_runtime::Error::Contract(
                    "unterminated Fountain boneyard".into(),
                ));
            }
        }
    }
    out.push_str(rest);
    Ok(out
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !(trimmed.starts_with("[[") && trimmed.ends_with("]]"))
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn split_blocks(cleaned: &str) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    for chunk in cleaned.split("\n\n") {
        let lines: Vec<String> = chunk
            .lines()
            .map(|l| l.trim_end_matches('\r').trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        if !lines.is_empty() {
            blocks.push(lines);
        }
    }
    blocks
}

fn is_title_page_block(lines: &[String]) -> bool {
    !lines.is_empty()
        && lines.iter().all(|line| {
            line.contains(':')
                && matches!(
                    line.split(':')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_ascii_uppercase()
                        .as_str(),
                    "TITLE"
                        | "CREDIT"
                        | "AUTHOR"
                        | "AUTHORS"
                        | "SOURCE"
                        | "DRAFT DATE"
                        | "CONTACT"
                        | "NOTES"
                )
        })
}

fn is_scene_heading(line: &str) -> bool {
    let t = line.trim();
    if t.starts_with('.') && !t.starts_with("..") {
        return true;
    }
    let upper = t.to_ascii_uppercase();
    [
        "INT ", "INT.", "EXT ", "EXT.", "EST ", "EST.", "I/E ", "I/E.",
    ]
    .iter()
    .any(|prefix| upper.starts_with(prefix))
}

fn is_transition(line: &str) -> bool {
    let t = line.trim();
    if t.starts_with('>') && !t.ends_with('<') {
        return true;
    }
    let upper = t.to_ascii_uppercase();
    upper == "FADE OUT." || upper == "FADE IN:" || upper.ends_with(" TO:")
}

fn is_upper_cue(line: &str) -> bool {
    let core = line.trim_end_matches('^').trim();
    !core.is_empty()
        && core
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_whitespace() || "0123456789.'-#".contains(c))
        && core.chars().any(|c| c.is_ascii_alphabetic())
}

pub(super) fn normalize_fountain(
    body: &str,
) -> narrative_courier_runtime::Result<NormalizedScreenplay> {
    let cleaned = strip_boneyard_and_notes(body)?;
    let mut blocks = split_blocks(&cleaned);
    let mut builder = Builder::default();

    let mut title_page = None;
    if let Some(first) = blocks.first() {
        if is_title_page_block(first) {
            title_page = first.iter().find_map(|line| {
                let (key, value) = line.split_once(':')?;
                key.eq_ignore_ascii_case("title")
                    .then(|| value.trim().to_string())
            });
            blocks.remove(0);
        }
    }

    for block in blocks {
        let first = block[0].clone();
        let trimmed_first = first.trim().to_string();
        if is_scene_heading(&trimmed_first) {
            builder.push_heading(trimmed_first.trim_start_matches('.').trim().to_string());
            for line in block.into_iter().skip(1) {
                builder.push_block(line);
            }
            continue;
        }
        if is_transition(&trimmed_first) {
            let label = trimmed_first.trim_start_matches('>').trim();
            builder.push_block(format!("> {}", label.to_ascii_uppercase()));
            for line in block.into_iter().skip(1) {
                builder.push_block(line);
            }
            continue;
        }
        if let Some(cue_line) = trimmed_first.strip_prefix('@') {
            let name = cue_line.trim_end_matches('^').trim().to_string();
            builder.cast.insert(name.clone());
            let mut rendered = vec![name];
            rendered.extend(block.iter().skip(1).cloned());
            builder.push_block(rendered.join("\n"));
            continue;
        }
        if is_upper_cue(&trimmed_first) && block.len() > 1 {
            let name = trimmed_first.trim_end_matches('^').trim().to_string();
            builder.cast.insert(name.clone());
            let mut rendered = vec![name];
            rendered.extend(block.iter().skip(1).cloned());
            builder.push_block(rendered.join("\n"));
            continue;
        }
        for line in &block {
            let trimmed = line.trim();
            if trimmed.starts_with('=') || trimmed.starts_with('#') {
                builder.skipped += 1;
                continue;
            }
            builder.push_block(trimmed.to_string());
        }
    }

    Ok(builder.finish(title_page.filter(|t| !t.is_empty())))
}
