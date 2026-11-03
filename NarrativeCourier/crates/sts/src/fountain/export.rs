//! Canonical-screenplay serializers for Fountain and Final Draft.

fn escape_xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(character),
        }
    }
    out
}

/// Export the canonical screenplay into Fountain 1.1 text.
///
/// Scene headings are force-marked (`.`) because our `SCN-###.` prefix is not
/// an auto-detectable slug; character cues are force-marked (`@`) so Korean
/// names survive round-trips that uppercase heuristics cannot.
pub fn to_fountain(
    markdown: &str,
    title: Option<&str>,
) -> Result<String, narrative_courier_runtime::Error> {
    let mut out = String::new();
    if let Some(title) = title {
        out.push_str(&format!("Title: {}\n\n", title.trim()));
    }
    let mut blocks: Vec<String> = Vec::new();
    for token in crate::screenplay_text::parse(markdown) {
        match token {
            crate::screenplay_text::Token::Heading(value) => {
                if !value.is_empty() {
                    blocks.push(format!("# {value}"));
                }
            }
            crate::screenplay_text::Token::Scene(value) => blocks.push(format!(".{value}")),
            crate::screenplay_text::Token::Action(value) => blocks.push(value.to_owned()),
            crate::screenplay_text::Token::Character(name) => blocks.push(format!("@{name}")),
            crate::screenplay_text::Token::Dialogue(line) => match blocks.last_mut() {
                Some(block) if block.starts_with('@') => {
                    block.push('\n');
                    block.push_str(line);
                }
                _ => blocks.push(line.to_owned()),
            },
            crate::screenplay_text::Token::Blank => {}
        }
    }
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            out.push_str("\n\n");
        }
        out.push_str(block);
    }
    Ok(out + "\n")
}

/// Export the canonical screenplay into a minimal Final Draft `.fdx` document.
pub fn to_fdx(
    markdown: &str,
    title: Option<&str>,
) -> Result<String, narrative_courier_runtime::Error> {
    let mut paragraphs = String::new();
    let mut open_character = false;
    for token in crate::screenplay_text::parse(markdown) {
        let emit = |kind: &str, text: &str, paragraphs: &mut String| {
            paragraphs.push_str(&format!(
                "      <Paragraph Type=\"{}\">\n        <Text>{}</Text>\n      </Paragraph>\n",
                kind,
                escape_xml(text)
            ));
        };
        match token {
            crate::screenplay_text::Token::Heading(value) => {
                if !value.is_empty() {
                    emit("Action", &format!("# {value}"), &mut paragraphs);
                }
                open_character = false;
            }
            crate::screenplay_text::Token::Scene(value) => {
                emit("Scene Heading", value, &mut paragraphs);
                open_character = false;
            }
            crate::screenplay_text::Token::Action(value) => {
                emit("Action", value, &mut paragraphs);
                open_character = false;
            }
            crate::screenplay_text::Token::Character(name) => {
                emit("Character", name, &mut paragraphs);
                open_character = true;
            }
            crate::screenplay_text::Token::Dialogue(line) => {
                let kind = if open_character
                    && line.starts_with('(')
                    && line.ends_with(')')
                    && line.len() > 2
                {
                    "Parenthetical"
                } else if open_character {
                    "Dialogue"
                } else {
                    "Action"
                };
                emit(kind, line, &mut paragraphs);
                if kind == "Action" {
                    open_character = false;
                }
            }
            crate::screenplay_text::Token::Blank => {
                open_character = false;
            }
        }
    }
    let title_text = title.map(str::trim).filter(|t| !t.is_empty());
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"1\">\n  <Content>\n{}  </Content>\n</FinalDraft>\n",
        title_text
            .map(|t| format!(
                "      <Paragraph Type=\"Action\">\n        <Text># {}</Text>\n      </Paragraph>\n",
                escape_xml(t)
            ))
            .unwrap_or_default()
            + &paragraphs
    ))
}
