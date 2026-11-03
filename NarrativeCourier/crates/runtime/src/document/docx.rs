//! Bounded DOCX body text extraction using WordprocessingML namespace and text elements.

use super::MAX_EXTRACTED_BYTES;
use crate::{Error, Result};
use quick_xml::{events::Event, name::ResolveResult, NsReader};
use std::io::{Cursor, Read};

const WORD_NAMESPACE: &[u8] = b"http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const STRICT_WORD_NAMESPACE: &[u8] = b"http://purl.oclc.org/ooxml/wordprocessingml/main";

#[derive(Clone, Copy)]
struct Frame {
    in_body: bool,
    text: bool,
    paragraph: bool,
}

fn append(output: &mut String, text: &str) -> Result<()> {
    if text.len() > MAX_EXTRACTED_BYTES.saturating_sub(output.len()) {
        return Err(Error::Contract(
            "DOCX extracted text exceeds its byte limit".into(),
        ));
    }
    if text.chars().any(|character| !matches!(character, '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')) {
        return Err(Error::Contract("DOCX text contains an invalid XML 1.0 character".into()));
    }
    output.push_str(text);
    Ok(())
}

pub(super) fn extract(bytes: &[u8]) -> Result<String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut body_parts = 0;
    for index in 0..archive.len() {
        if archive.by_index_raw(index)?.name() == "word/document.xml" {
            body_parts += 1;
        }
    }
    if body_parts != 1 {
        return Err(Error::Contract(
            "DOCX requires exactly one word/document.xml part".into(),
        ));
    }
    let mut part = archive.by_name("word/document.xml")?;
    if part.size() > MAX_EXTRACTED_BYTES as u64 {
        return Err(Error::Contract(
            "DOCX body XML exceeds its uncompressed byte limit".into(),
        ));
    }
    let mut xml = Vec::new();
    (&mut part)
        .take(MAX_EXTRACTED_BYTES as u64 + 1)
        .read_to_end(&mut xml)?;
    if xml.len() > MAX_EXTRACTED_BYTES {
        return Err(Error::Contract(
            "DOCX body XML exceeded its actual read limit".into(),
        ));
    }
    let xml = std::str::from_utf8(&xml)
        .map_err(|error| Error::Contract(format!("DOCX body XML is not UTF-8: {error}")))?;
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Frame> = Vec::new();
    let mut root_seen = false;
    let mut body_seen = false;
    let mut output = String::new();
    loop {
        let (namespace, event) = reader
            .read_resolved_event()
            .map_err(|error| Error::Contract(format!("DOCX XML parse failed: {error}")))?;
        if matches!(&namespace, ResolveResult::Unknown(_)) {
            return Err(Error::Contract(
                "DOCX XML contains an undeclared namespace prefix".into(),
            ));
        }
        let word = matches!(namespace, ResolveResult::Bound(uri) if uri.as_ref() == WORD_NAMESPACE || uri.as_ref() == STRICT_WORD_NAMESPACE);
        match event {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                let (tag, empty) = match event {
                    Event::Start(tag) => (tag, false),
                    Event::Empty(tag) => (tag, true),
                    _ => unreachable!("matched element event"),
                };
                let local = tag.local_name();
                let name = local.as_ref();
                let parent_body = stack.last().is_some_and(|frame| frame.in_body);
                if stack.is_empty() {
                    if root_seen || !word || name != b"document" {
                        return Err(Error::Contract(
                            "DOCX requires one WordprocessingML document root".into(),
                        ));
                    }
                    root_seen = true;
                }
                if stack.last().is_some_and(|frame| frame.text) {
                    return Err(Error::Contract(
                        "DOCX text element cannot contain child elements".into(),
                    ));
                }
                let body = word && name == b"body";
                if body {
                    if stack.len() != 1 || body_seen {
                        return Err(Error::Contract(
                            "DOCX requires one direct document body".into(),
                        ));
                    }
                    body_seen = true;
                }
                if parent_body
                    && word
                    && matches!(
                        name,
                        b"ins" | b"del" | b"moveFrom" | b"moveTo" | b"altChunk"
                    )
                {
                    return Err(Error::Contract("DOCX tracked revisions/embedded body content require resolution before extraction".into()));
                }
                let frame = Frame {
                    in_body: parent_body || body,
                    text: parent_body && word && name == b"t",
                    paragraph: parent_body && word && name == b"p",
                };
                if parent_body && word {
                    if name == b"tab" {
                        append(&mut output, "\t")?;
                    }
                    if name == b"br" || name == b"cr" {
                        append(&mut output, "\n")?;
                    }
                }
                if empty {
                    if frame.paragraph {
                        append(&mut output, "\n")?;
                    }
                } else {
                    stack.push(frame);
                }
            }
            Event::End(_) => {
                let frame = stack.pop().ok_or_else(|| {
                    Error::Contract("DOCX XML has an unmatched close element".into())
                })?;
                if frame.paragraph {
                    append(&mut output, "\n")?;
                }
            }
            Event::Text(text) if stack.last().is_some_and(|frame| frame.text) => {
                append(
                    &mut output,
                    &text.xml10_content().map_err(|error| {
                        Error::Contract(format!("DOCX text decode failed: {error}"))
                    })?,
                )?;
            }
            Event::CData(text) if stack.last().is_some_and(|frame| frame.text) => {
                append(
                    &mut output,
                    &text.xml10_content().map_err(|error| {
                        Error::Contract(format!("DOCX CDATA decode failed: {error}"))
                    })?,
                )?;
            }
            Event::GeneralRef(reference) => {
                let decoded = if let Some(character) =
                    reference.resolve_char_ref().map_err(|error| {
                        Error::Contract(format!("DOCX entity decode failed: {error}"))
                    })? {
                    character.to_string()
                } else {
                    match reference.as_ref() {
                        b"lt" => "<",
                        b"gt" => ">",
                        b"amp" => "&",
                        b"apos" => "'",
                        b"quot" => "\"",
                        _ => {
                            return Err(Error::Contract(
                                "DOCX contains an unsupported named entity".into(),
                            ))
                        }
                    }
                    .to_owned()
                };
                if stack.last().is_some_and(|frame| frame.text) {
                    append(&mut output, &decoded)?;
                }
            }
            Event::DocType(_) => {
                return Err(Error::Contract(
                    "DOCX DTD/entity declarations are unsupported".into(),
                ))
            }
            Event::Eof => {
                if !stack.is_empty() || !root_seen || !body_seen {
                    return Err(Error::Contract("DOCX document/body is incomplete".into()));
                }
                break;
            }
            _ => {}
        }
    }
    Ok(output)
}
