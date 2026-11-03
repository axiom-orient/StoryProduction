//! Supported Final Draft XML paragraphs, parsed as XML rather than substring tags.

use super::super::NormalizedScreenplay;
use super::Builder;
use narrative_courier_runtime::{Error, Result};
use quick_xml::{events::Event, Reader};

pub(super) fn normalize_fdx(body: &str) -> Result<NormalizedScreenplay> {
    let mut reader = Reader::from_str(body);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut root_seen = false;
    let mut content_seen = false;
    let mut paragraph: Option<(String, String)> = None;
    let mut text_depth = None;
    let mut dialogue_open = false;
    let mut builder = Builder::default();
    loop {
        let event = reader
            .read_event()
            .map_err(|error| xml_error(error.to_string()))?;
        match event {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                let (tag, empty) = match event {
                    Event::Start(tag) => (tag, false),
                    Event::Empty(tag) => (tag, true),
                    _ => unreachable!("opening XML event matched above"),
                };
                let name = tag.name().as_ref().to_vec();
                let mut paragraph_type = None;
                for attribute in tag.attributes() {
                    let attribute = attribute.map_err(|error| xml_error(error.to_string()))?;
                    let value = attribute
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|error| xml_error(error.to_string()))?;
                    if attribute.key.as_ref() == b"Type" {
                        paragraph_type = Some(value.into_owned());
                    }
                }
                if stack.is_empty() {
                    if root_seen || name != b"FinalDraft" || empty {
                        return Err(xml_error("document must have one FinalDraft root"));
                    }
                    root_seen = true;
                }
                if name == b"Content" && stack.len() == 1 {
                    if content_seen {
                        return Err(xml_error("repeated Content"));
                    }
                    content_seen = true;
                }
                let in_content = stack.iter().any(|name| name == b"Content");
                if name == b"Paragraph" && in_content {
                    if paragraph.is_some() {
                        return Err(xml_error("nested Paragraph"));
                    }
                    let kind =
                        paragraph_type.ok_or_else(|| xml_error("Paragraph Type is required"))?;
                    if !matches!(
                        kind.as_str(),
                        "Scene Heading"
                            | "Character"
                            | "Dialogue"
                            | "Action"
                            | "General"
                            | "Parenthetical"
                            | "Transition"
                    ) {
                        return Err(xml_error(format!("unsupported paragraph type: {kind}")));
                    }
                    paragraph = Some((kind, String::new()));
                    if empty {
                        paragraph = None;
                    }
                }
                if name == b"Text" && paragraph.is_some() {
                    if text_depth.is_some() {
                        return Err(xml_error("nested Text"));
                    }
                    if !empty {
                        text_depth = Some(stack.len() + 1);
                    }
                }
                if !empty {
                    stack.push(name);
                }
            }
            Event::End(tag) => {
                if text_depth == Some(stack.len()) {
                    text_depth = None;
                }
                if tag.name().as_ref() == b"Paragraph" {
                    if let Some((kind, text)) = paragraph.take() {
                        emit(&mut builder, &kind, &text, &mut dialogue_open)?;
                    }
                }
                let opened = stack
                    .pop()
                    .ok_or_else(|| xml_error("unexpected closing tag"))?;
                if opened != tag.name().as_ref() {
                    return Err(xml_error("mismatched closing tag"));
                }
            }
            Event::Text(text) => {
                let text = text
                    .xml10_content()
                    .map_err(|error| xml_error(error.to_string()))?;
                if text_depth.is_some() {
                    paragraph
                        .as_mut()
                        .ok_or_else(|| xml_error("Text outside Paragraph"))?
                        .1
                        .push_str(&text);
                } else if stack.is_empty() && !text.trim().is_empty() {
                    return Err(xml_error("text outside FinalDraft root"));
                }
            }
            Event::CData(text) if text_depth.is_some() => {
                let text = text
                    .decode()
                    .map_err(|error| xml_error(error.to_string()))?;
                paragraph
                    .as_mut()
                    .ok_or_else(|| xml_error("CDATA outside Paragraph"))?
                    .1
                    .push_str(&text);
            }
            Event::GeneralRef(reference) if text_depth.is_some() => {
                let reference = std::str::from_utf8(reference.as_ref())
                    .map_err(|error| xml_error(error.to_string()))?;
                let entity = format!("&{reference};");
                let decoded = quick_xml::escape::unescape(&entity)
                    .map_err(|error| xml_error(error.to_string()))?;
                paragraph
                    .as_mut()
                    .ok_or_else(|| xml_error("entity outside Paragraph"))?
                    .1
                    .push_str(&decoded);
            }
            Event::DocType(_) => {
                return Err(xml_error("DOCTYPE/entity declarations are unsupported"))
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !root_seen
        || !content_seen
        || !stack.is_empty()
        || paragraph.is_some()
        || text_depth.is_some()
    {
        return Err(xml_error("incomplete FinalDraft document"));
    }
    Ok(builder.finish(None))
}

fn emit(builder: &mut Builder, kind: &str, raw: &str, dialogue_open: &mut bool) -> Result<()> {
    let content = raw.trim();
    if content.is_empty() {
        return Ok(());
    }
    match kind {
        "Character" => {
            let name = content.trim_end_matches('^').trim();
            if name.is_empty() {
                return Err(xml_error("empty Character"));
            }
            builder.cast.insert(name.to_owned());
            builder.push_block(name.to_owned());
            *dialogue_open = true;
        }
        "Dialogue" | "Parenthetical" => {
            if !*dialogue_open {
                return Err(xml_error("Dialogue/Parenthetical has no Character"));
            }
            let text = if kind == "Parenthetical"
                && !(content.starts_with('(') && content.ends_with(')'))
            {
                format!("({content})")
            } else {
                content.to_owned()
            };
            let block = builder
                .blocks
                .last_mut()
                .ok_or_else(|| xml_error("missing Character block"))?;
            block.push('\n');
            block.push_str(&text);
        }
        "Scene Heading" => {
            builder.push_heading(content.to_owned());
            *dialogue_open = false;
        }
        "Transition" => {
            builder.push_block(format!("> {}", content.to_ascii_uppercase()));
            *dialogue_open = false;
        }
        "Action" | "General" => {
            builder.push_block(content.to_owned());
            *dialogue_open = false;
        }
        _ => return Err(xml_error(format!("unsupported paragraph type: {kind}"))),
    }
    Ok(())
}

fn xml_error(message: impl Into<String>) -> Error {
    Error::Contract(format!("FDX input: {}", message.into()))
}
