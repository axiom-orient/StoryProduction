use crate::error::{Error, Result};
use crate::model::{Project, Scene};
use std::collections::BTreeSet;

pub fn import_markdown(markdown: &str, start_order: i64) -> Result<Vec<Scene>> {
    if markdown.trim().is_empty() {
        return Err(Error::InvalidInput("markdown is empty".into()));
    }
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut title = "본문".to_string();
    let mut body = String::new();
    let mut fence: Option<(u8, usize)> = None;
    for line in markdown.lines() {
        if let Some((marker, width)) = fence {
            if let Some((next_marker, next_width, rest)) = fence_marker(line) {
                if marker == next_marker && next_width >= width && rest.trim().is_empty() {
                    fence = None;
                }
            }
            body.push_str(line);
            body.push('\n');
            continue;
        }
        if let Some((marker, width, _)) = fence_marker(line) {
            fence = Some((marker, width));
            body.push_str(line);
            body.push('\n');
            continue;
        }
        if let Some(value) = line.strip_prefix("# ").or_else(|| line.strip_prefix("## ")) {
            if !body.trim().is_empty() {
                sections.push((title, body.trim().to_string()));
            }
            title = value.trim().to_string();
            body.clear();
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    if !body.trim().is_empty() {
        sections.push((title, body.trim().to_string()));
    }
    if sections.is_empty() {
        sections.push(("본문".into(), markdown.trim().into()));
    }
    let mut used = BTreeSet::new();
    sections
        .into_iter()
        .enumerate()
        .map(|(index, (title, text))| {
            let base = slug(&title);
            // t33 contract requires ^scene_[A-Za-z0-9_.-]{1,120}$ (valid_scene_id).
            let base_id = if base.is_empty() {
                format!("scene_{}", index + 1)
            } else {
                format!("scene_{base}")
            };
            let mut id = base_id.clone();
            let mut suffix = 2;
            while !used.insert(id.clone()) {
                id = format!("{base_id}-{suffix}");
                suffix += 1;
            }
            let offset = i64::try_from(index)
                .map_err(|_| Error::InvalidInput("too many Markdown scenes".into()))?;
            let order = start_order
                .checked_add(offset)
                .ok_or_else(|| Error::InvalidInput("scene order exceeds i64 capacity".into()))?;
            Ok(Scene {
                id,
                title,
                text,
                order,
                revision: 0,
            })
        })
        .collect()
}

pub fn export_markdown(project: &Project) -> String {
    let mut scenes: Vec<_> = project.scenes.values().collect();
    scenes.sort_by_key(|scene| (scene.order, scene.id.as_str()));
    let mut output = format!("# {}\n\n", project.title);
    for scene in scenes {
        output.push_str("## ");
        output.push_str(&scene.title);
        output.push_str("\n\n");
        output.push_str(scene.text.trim_end());
        output.push_str("\n\n");
    }
    output
}

fn slug(value: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            output.push(character);
            separator = false;
        } else if (character.is_whitespace() || character == '-' || character == '_')
            && !output.is_empty()
            && !separator
        {
            output.push('-');
            separator = true;
        }
    }
    output.trim_matches('-').chars().take(48).collect()
}

/// Recognizes fenced code boundaries without interpreting headings inside them.
fn fence_marker(line: &str) -> Option<(u8, usize, &str)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let marker = *trimmed.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let width = trimmed.bytes().take_while(|byte| *byte == marker).count();
    (width >= 3).then(|| (marker, width, &trimmed[width..]))
}
