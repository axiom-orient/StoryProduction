//! Legacy text-board unified diff semantics (three lines of context and popular-line filtering).
//! This presentation is a derived review aid, never a new narrative authority.

use crate::{require, Result, MAX_JSON_BYTES};
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct Code {
    tag: &'static str,
    a0: usize,
    a1: usize,
    b0: usize,
    b1: usize,
}
fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((index, character)) = chars.next() {
        if matches!(
            character,
            '\n' | '\r'
                | '\x0b'
                | '\x0c'
                | '\x1c'
                | '\x1d'
                | '\x1e'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        ) {
            let mut end = index + character.len_utf8();
            if character == '\r' && chars.peek().is_some_and(|(_, next)| *next == '\n') {
                end = chars.next().expect("peeked CRLF").0 + 1;
            }
            lines.push(&text[start..end]);
            start = end;
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}
fn longest(
    a: &[&str],
    b: &[&str],
    index: &HashMap<&str, Vec<usize>>,
    a0: usize,
    a1: usize,
    b0: usize,
    b1: usize,
) -> (usize, usize, usize) {
    let (mut best_a, mut best_b, mut size) = (a0, b0, 0);
    let mut previous = HashMap::new();
    for (i, line) in a.iter().enumerate().take(a1).skip(a0) {
        let mut next = HashMap::new();
        for &j in index.get(line).into_iter().flatten() {
            if j < b0 {
                continue;
            }
            if j >= b1 {
                break;
            }
            let length = j
                .checked_sub(1)
                .and_then(|j| previous.get(&j))
                .copied()
                .unwrap_or(0)
                + 1;
            next.insert(j, length);
            if length > size {
                (best_a, best_b, size) = (i + 1 - length, j + 1 - length, length);
            }
        }
        previous = next;
    }
    while best_a > a0 && best_b > b0 && a[best_a - 1] == b[best_b - 1] {
        best_a -= 1;
        best_b -= 1;
        size += 1;
    }
    while best_a + size < a1 && best_b + size < b1 && a[best_a + size] == b[best_b + size] {
        size += 1;
    }
    (best_a, best_b, size)
}
fn codes(a: &[&str], b: &[&str]) -> Vec<Code> {
    let mut index: HashMap<&str, Vec<usize>> = HashMap::new();
    for (j, line) in b.iter().enumerate() {
        index.entry(line).or_default().push(j);
    }
    if b.len() >= 200 {
        let threshold = b.len() / 100 + 1;
        index.retain(|_, positions| positions.len() <= threshold);
    }
    let mut queue = vec![(0, a.len(), 0, b.len())];
    let mut blocks = Vec::new();
    while let Some((a0, a1, b0, b1)) = queue.pop() {
        let (i, j, length) = longest(a, b, &index, a0, a1, b0, b1);
        if length == 0 {
            continue;
        }
        blocks.push((i, j, length));
        if a0 < i && b0 < j {
            queue.push((a0, i, b0, j));
        }
        if i + length < a1 && j + length < b1 {
            queue.push((i + length, a1, j + length, b1));
        }
    }
    blocks.sort_unstable();
    let mut merged: Vec<(usize, usize, usize)> = Vec::new();
    for (i, j, length) in blocks {
        if let Some(last) = merged.last_mut() {
            if last.0 + last.2 == i && last.1 + last.2 == j {
                last.2 += length;
                continue;
            }
        }
        merged.push((i, j, length));
    }
    merged.push((a.len(), b.len(), 0));
    let mut result = Vec::new();
    let (mut i, mut j) = (0, 0);
    for (ai, bj, length) in merged {
        let tag = if i < ai && j < bj {
            "replace"
        } else if i < ai {
            "delete"
        } else if j < bj {
            "insert"
        } else {
            ""
        };
        if !tag.is_empty() {
            result.push(Code {
                tag,
                a0: i,
                a1: ai,
                b0: j,
                b1: bj,
            });
        }
        (i, j) = (ai + length, bj + length);
        if length > 0 {
            result.push(Code {
                tag: "equal",
                a0: ai,
                a1: i,
                b0: bj,
                b1: j,
            });
        }
    }
    result
}
fn groups(mut codes: Vec<Code>) -> Vec<Vec<Code>> {
    if codes.is_empty() {
        return Vec::new();
    }
    if codes[0].tag == "equal" {
        let first = &mut codes[0];
        first.a0 = first.a1.saturating_sub(3).max(first.a0);
        first.b0 = first.b1.saturating_sub(3).max(first.b0);
    }
    if codes.last().expect("nonempty codes").tag == "equal" {
        let last = codes.last_mut().expect("nonempty codes");
        last.a1 = last.a1.min(last.a0 + 3);
        last.b1 = last.b1.min(last.b0 + 3);
    }
    let mut groups = Vec::new();
    let mut group = Vec::new();
    for mut code in codes {
        if code.tag == "equal" && code.a1 - code.a0 > 6 {
            group.push(Code {
                a1: code.a0 + 3,
                b1: code.b0 + 3,
                ..code
            });
            groups.push(std::mem::take(&mut group));
            code.a0 = code.a1 - 3;
            code.b0 = code.b1 - 3;
        }
        group.push(code);
    }
    if !(group.is_empty() || group.len() == 1 && group[0].tag == "equal") {
        groups.push(group);
    }
    groups
}
fn range(start: usize, stop: usize) -> String {
    let length = stop - start;
    if length == 1 {
        (start + 1).to_string()
    } else if length == 0 {
        format!("{start},0")
    } else {
        format!("{},{length}", start + 1)
    }
}
pub(super) fn unified(before: &str, after: &str) -> Result<String> {
    let a = splitlines(before);
    let b = splitlines(after);
    let groups = groups(codes(&a, &b));
    let mut result = String::new();
    let mut append = |text: &str| -> Result<()> {
        require(
            result.len() + text.len() <= MAX_JSON_BYTES,
            "STORYBOARD_DIFF_TOO_LARGE: select fewer scenes",
        )?;
        result.push_str(text);
        Ok(())
    };
    if !groups.is_empty() {
        append("--- before.md\n+++ candidate.md\n")?;
    }
    for group in groups {
        let first = group.first().expect("nonempty group");
        let last = group.last().expect("nonempty group");
        append(&format!(
            "@@ -{} +{} @@\n",
            range(first.a0, last.a1),
            range(first.b0, last.b1)
        ))?;
        for code in group {
            if code.tag == "equal" {
                for line in &a[code.a0..code.a1] {
                    append(&format!(" {line}"))?;
                }
            }
            if matches!(code.tag, "replace" | "delete") {
                for line in &a[code.a0..code.a1] {
                    append(&format!("-{line}"))?;
                }
            }
            if matches!(code.tag, "replace" | "insert") {
                for line in &b[code.b0..code.b1] {
                    append(&format!("+{line}"))?;
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diff_matches_existing_projection() {
        assert_eq!(
            unified("", "가\n나\n").unwrap(),
            "--- before.md\n+++ candidate.md\n@@ -0,0 +1,2 @@\n+가\n+나\n"
        );
        assert_eq!(
            unified("a\nb\nc\n", "a\nx\nc\n").unwrap(),
            "--- before.md\n+++ candidate.md\n@@ -1,3 +1,3 @@\n a\n-b\n+x\n c\n"
        );
        assert_eq!(unified("same", "same").unwrap(), "");
        assert_eq!(
            splitlines("a\r\nb\u{2028}c"),
            vec!["a\r\n", "b\u{2028}", "c"]
        );
    }
}
