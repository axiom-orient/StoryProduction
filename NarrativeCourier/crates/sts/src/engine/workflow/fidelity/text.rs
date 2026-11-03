//! Shared closed Markdown/identifier grammar for fidelity admission.

use std::collections::BTreeSet;

use narrative_courier_runtime::{Error, Result};

pub(super) fn table_cells(line: &str) -> Result<Vec<String>> {
    if line.len() < 2 || !line.starts_with('|') || !line.ends_with('|') {
        return Err(contract("alignment row is not a pipe-delimited table row"));
    }
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for character in line[1..line.len() - 1].chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '|' {
            cells.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(character);
        }
    }
    if escaped {
        current.push('\\');
    }
    cells.push(current.trim().to_string());
    Ok(cells)
}

pub(super) fn is_table_separator(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let separator = cell.trim().trim_matches(':');
            separator.len() >= 3 && separator.bytes().all(|byte| byte == b'-')
        })
}

pub(super) fn leading_id(line: &str, prefix: &str) -> Result<Option<String>> {
    let body = strip_markdown_prefix(line);
    if !body.starts_with(prefix) {
        return Ok(None);
    }
    let suffix = &body[prefix.len()..];
    let digit_count = suffix
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digit_count < 3 {
        return Err(contract(format!(
            "malformed identifier beginning with {prefix}"
        )));
    }
    if let Some(next) = suffix[digit_count..].chars().next() {
        if !(next.is_whitespace() || matches!(next, ':' | ';' | ',' | '—' | '–')) {
            return Err(contract(format!(
                "malformed identifier beginning with {prefix}"
            )));
        }
    }
    Ok(Some(format!("{prefix}{}", &suffix[..digit_count])))
}

pub(super) fn strip_markdown_prefix(line: &str) -> &str {
    let mut body = line.trim();
    while let Some(rest) = body.strip_prefix('#') {
        body = rest.trim_start();
    }
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = body.strip_prefix(marker) {
            return rest.trim_start();
        }
    }
    body
}

pub(super) fn identifiers(text: &str, prefix: &str) -> Result<Vec<String>> {
    let mut identifiers = Vec::new();
    for token in
        text.split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
    {
        if !token.starts_with(prefix) {
            continue;
        }
        let suffix = &token[prefix.len()..];
        if suffix.len() < 3 || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(contract(format!("malformed evidence identifier {token}")));
        }
        identifiers.push(token.to_string());
    }
    Ok(identifiers)
}

pub(super) fn is_span_id(value: &str) -> bool {
    value
        .strip_prefix("SPAN-")
        .is_some_and(|digits| digits.len() >= 3 && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

pub(super) fn assignment(line: &str, key: &str) -> Result<Option<String>> {
    let mut found = None;
    for part in line.split([';', ',', '|']) {
        let Some((name, value)) = part.split_once('=').or_else(|| part.split_once(':')) else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case(key) {
            if found.is_some() {
                return Err(contract(format!("source item repeats assignment {key}")));
            }
            let atom = value.trim().trim_matches('`');
            if atom.is_empty() || atom.chars().any(char::is_whitespace) {
                return Err(contract(format!(
                    "source assignment {key} requires one nonempty atom"
                )));
            }
            found = Some(atom.to_ascii_uppercase());
        }
    }
    Ok(found)
}

pub(super) fn has_duplicates(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(value))
}

pub(super) fn contract(message: impl Into<String>) -> Error {
    Error::Contract(message.into())
}
